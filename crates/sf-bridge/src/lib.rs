//! # busbar-sf-bridge
//!
//! Extism host bridge for sandboxed WASM access to Salesforce APIs.
//!
//! This crate provides [`SfBridge`], which loads WASM guest plugins via Extism
//! and exposes Salesforce API operations as host functions. Credentials are
//! managed entirely on the host side -- WASM guests never see tokens.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  WASM Guest (compiled with sf-guest-sdk)        │
//! │                                                 │
//! │  Calls host functions:                          │
//! │    sf_query("SELECT Id FROM Account")           │
//! │    sf_create("Contact", {fields...})            │
//! │    sf_bulk_create_ingest_job(...)                │
//! │    sf_tooling_query(...)                         │
//! │    sf_metadata_deploy(...)                       │
//! │                                                 │
//! │  CANNOT: see tokens, make raw HTTP, access fs   │
//! └──────────────┬──────────────────────────────────┘
//!                │  Extism host function ABI (JSON over shared memory)
//!                ▼
//! ┌─────────────────────────────────────────────────┐
//! │  SfBridge (this crate)                          │
//! │                                                 │
//! │  - Owns all Salesforce clients (with creds)     │
//! │  - REST, Bulk, Tooling, Metadata APIs           │
//! │  - Registers host functions per the ABI         │
//! │  - Validates inputs, executes API calls         │
//! │  - Returns results to guest                     │
//! │  - Full async, retry, tracing, rate limiting    │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! - **Credential isolation**: Access tokens live only in the host process.
//!   They never appear in WASM linear memory.
//! - **Sandboxed guests**: WASM modules cannot make raw HTTP calls, read
//!   environment variables, or access the filesystem.
//! - **Input validation**: All inputs from the guest are validated using
//!   sf-client's security utilities (SOQL injection prevention, etc.)
//!   before being forwarded to Salesforce.
//!
//! ## Concurrency
//!
//! `SfBridge::call` is safe to invoke from multiple tokio tasks concurrently.
//! Each invocation creates a fresh WASM plugin instance from a pre-compiled
//! module. The underlying clients share connection pools.
//!
//! ## Example
//!
//! ```rust,ignore
//! use busbar_sf_bridge::SfBridge;
//! use busbar_sf_rest::SalesforceRestClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = SalesforceRestClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token_here",
//!     )?;
//!
//!     let wasm_bytes = std::fs::read("my_plugin.wasm")?;
//!     let bridge = SfBridge::new(wasm_bytes, client)?;
//!
//!     // Call the guest's exported "run" function
//!     let result = bridge.call("run", b"input data").await?;
//!     println!("Guest returned: {}", String::from_utf8_lossy(&result));
//!
//!     Ok(())
//! }
//! ```

mod error;
mod host_functions;
mod registration;

#[cfg(feature = "busbar")]
mod capability;

pub use error::{Error, Result};

use std::sync::Arc;

#[cfg(feature = "bulk")]
use busbar_sf_bulk::BulkApiClient;
#[cfg(feature = "metadata")]
use busbar_sf_metadata::MetadataClient;
#[cfg(feature = "rest")]
use busbar_sf_rest::SalesforceRestClient;
#[cfg(feature = "tooling")]
use busbar_sf_tooling::ToolingClient;

use extism::{Manifest, Plugin, PluginBuilder, UserData, Wasm};
use tracing::instrument;

/// Shared state passed to all host functions via `UserData<BridgeState>`.
///
/// Accessed in host function callbacks as:
/// `let state = user_data.get()?.lock().unwrap();`
///
/// The Mutex serializes access per-plugin instance. The tokio handle
/// allows bridging from sync host function callbacks to async Salesforce
/// API calls via `state.handle.block_on(...)`.
pub(crate) struct BridgeState {
    #[cfg(feature = "rest")]
    pub(crate) rest_client: SalesforceRestClient,
    #[cfg(feature = "bulk")]
    pub(crate) bulk_client: BulkApiClient,
    #[cfg(feature = "tooling")]
    pub(crate) tooling_client: ToolingClient,
    pub(crate) instance_url: Arc<str>,
    pub(crate) access_token: Arc<str>,
    pub(crate) handle: tokio::runtime::Handle,
}

impl BridgeState {
    /// Construct a fresh MetadataClient. MetadataClient is not Clone,
    /// so we build one on-demand from stored credentials.
    #[cfg(feature = "metadata")]
    pub(crate) fn metadata_client(&self) -> MetadataClient {
        MetadataClient::from_parts(&*self.instance_url, &*self.access_token)
    }
}

/// The main bridge between WASM guests and Salesforce APIs.
///
/// Create one `SfBridge` per WASM module. Call [`SfBridge::call`] to invoke
/// exported guest functions. The bridge is `Send + Sync` and safe to share
/// across tokio tasks.
pub struct SfBridge {
    wasm_bytes: Arc<Vec<u8>>,
    #[cfg(feature = "rest")]
    pub(crate) rest_client: SalesforceRestClient,
    #[cfg(feature = "bulk")]
    pub(crate) bulk_client: BulkApiClient,
    #[cfg(feature = "tooling")]
    pub(crate) tooling_client: ToolingClient,
    pub(crate) instance_url: Arc<str>,
    pub(crate) access_token: Arc<str>,
    pub(crate) handle: tokio::runtime::Handle,
}

impl SfBridge {
    /// Create a new bridge with the given WASM module bytes and REST client.
    ///
    /// The `rest_client` must already be authenticated. The bridge does not
    /// perform authentication -- that's the caller's responsibility.
    ///
    /// Must be called from within a tokio runtime context.
    #[cfg(feature = "rest")]
    pub fn new(wasm_bytes: Vec<u8>, rest_client: SalesforceRestClient) -> Result<Self> {
        let handle = tokio::runtime::Handle::current();
        Self::with_handle(wasm_bytes, rest_client, handle)
    }

    /// Create a new bridge, providing a specific tokio runtime handle.
    ///
    /// Use this when constructing the bridge outside of a tokio context.
    #[cfg(feature = "rest")]
    pub fn with_handle(
        wasm_bytes: Vec<u8>,
        rest_client: SalesforceRestClient,
        handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        let inner = rest_client.inner();
        let instance_url: Arc<str> = inner.instance_url().to_string().into();
        let access_token: Arc<str> = inner.access_token().to_string().into();

        #[cfg(feature = "bulk")]
        let bulk_client = BulkApiClient::from_client(inner.clone());
        #[cfg(feature = "tooling")]
        let tooling_client = ToolingClient::from_client(inner.clone());

        Ok(Self {
            wasm_bytes: Arc::new(wasm_bytes),
            #[cfg(feature = "rest")]
            rest_client,
            #[cfg(feature = "bulk")]
            bulk_client,
            #[cfg(feature = "tooling")]
            tooling_client,
            instance_url,
            access_token,
            handle,
        })
    }

    /// Call an exported function in the WASM guest.
    ///
    /// Each call creates a fresh plugin instance (cheap -- the module is
    /// pre-compiled by Extism/Wasmtime). The host functions are wired up
    /// with the bridge's Salesforce clients.
    ///
    /// Safe to call concurrently from multiple tokio tasks.
    #[instrument(skip(self, input), fields(function = %function))]
    pub async fn call(
        &self,
        function: &str,
        input: impl AsRef<[u8]> + Send + 'static,
    ) -> Result<Vec<u8>> {
        let wasm_bytes = self.wasm_bytes.clone();
        #[cfg(feature = "rest")]
        let rest_client = self.rest_client.clone();
        #[cfg(feature = "bulk")]
        let bulk_client = self.bulk_client.clone();
        #[cfg(feature = "tooling")]
        let tooling_client = self.tooling_client.clone();
        let instance_url = Arc::clone(&self.instance_url);
        let access_token = Arc::clone(&self.access_token);
        let handle = self.handle.clone();
        let function = function.to_string();

        // Run the plugin on a blocking thread so host functions can
        // safely use Handle::block_on() for async Salesforce operations.
        tokio::task::spawn_blocking(move || {
            let state = BridgeState {
                #[cfg(feature = "rest")]
                rest_client,
                #[cfg(feature = "bulk")]
                bulk_client,
                #[cfg(feature = "tooling")]
                tooling_client,
                instance_url,
                access_token,
                handle,
            };
            let mut plugin = create_plugin(&wasm_bytes, state)?;
            let result = plugin.call::<&[u8], &[u8]>(&function, input.as_ref())?;
            Ok(result.to_vec())
        })
        .await?
    }
}

/// Create an Extism plugin with all enabled Salesforce host functions registered.
fn create_plugin(wasm_bytes: &[u8], state: BridgeState) -> Result<Plugin> {
    let manifest = Manifest::new([Wasm::data(wasm_bytes.to_vec())]);
    let user_data = UserData::new(state);

    let builder = PluginBuilder::new(manifest).with_wasi(true);

    // Register all enabled host functions based on feature flags
    let plugin = registration::register_all(builder, &user_data).build()?;

    Ok(plugin)
}

/// Helper for host functions that take a request argument.
pub(crate) fn bridge_host_fn<Req, Resp>(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
    handler: impl FnOnce(&BridgeState, Req) -> busbar_sf_wasm_types::BridgeResult<Resp>,
) -> std::result::Result<(), extism::Error>
where
    Req: serde::de::DeserializeOwned,
    Resp: serde::Serialize,
{
    let state_arc = user_data.get()?;
    let state = state_arc.lock().unwrap();

    let input_bytes: Vec<u8> = plugin.memory_get_val(&inputs[0])?;
    let request: Req = rmp_serde::from_slice(&input_bytes)
        .map_err(|e| extism::Error::msg(format!("deserialize request: {e}")))?;

    let result = handler(&state, request);

    let output_bytes = rmp_serde::to_vec_named(&result)
        .map_err(|e| extism::Error::msg(format!("serialize response: {e}")))?;
    let mem_handle = plugin.memory_new(&output_bytes)?;
    outputs[0] = plugin.memory_to_val(mem_handle);
    Ok(())
}

/// Helper for host functions that take no meaningful input.
pub(crate) fn bridge_host_fn_no_input<Resp>(
    plugin: &mut extism::CurrentPlugin,
    _inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
    handler: impl FnOnce(&BridgeState) -> busbar_sf_wasm_types::BridgeResult<Resp>,
) -> std::result::Result<(), extism::Error>
where
    Resp: serde::Serialize,
{
    let state_arc = user_data.get()?;
    let state = state_arc.lock().unwrap();

    let result = handler(&state);

    let output_bytes = rmp_serde::to_vec_named(&result)
        .map_err(|e| extism::Error::msg(format!("serialize response: {e}")))?;
    let mem_handle = plugin.memory_new(&output_bytes)?;
    outputs[0] = plugin.memory_to_val(mem_handle);
    Ok(())
}
