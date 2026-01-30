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
//! │                                                 │
//! │  CANNOT: see tokens, make raw HTTP, access fs   │
//! └──────────────┬──────────────────────────────────┘
//!                │  Extism host function ABI (JSON over shared memory)
//!                ▼
//! ┌─────────────────────────────────────────────────┐
//! │  SfBridge (this crate)                          │
//! │                                                 │
//! │  - Owns SalesforceRestClient (with credentials) │
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
//! module. The underlying `SalesforceRestClient` shares a connection pool.
//!
//! ```text
//! tokio runtime (multi-threaded)
//! ├── task: guest_a.wasm → sf_query()  → host awaits API → returns
//! ├── task: guest_b.wasm → sf_create() → host awaits API → returns
//! └── task: guest_c.wasm → sf_query()  → host awaits API → returns
//! ```
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

pub use error::{Error, Result};

use std::sync::Arc;

use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::host_fn_names;
use extism::{Manifest, Plugin, PluginBuilder, UserData, ValType, Wasm};
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
    pub(crate) rest_client: SalesforceRestClient,
    pub(crate) handle: tokio::runtime::Handle,
}

/// The main bridge between WASM guests and Salesforce APIs.
///
/// Create one `SfBridge` per WASM module. Call [`SfBridge::call`] to invoke
/// exported guest functions. The bridge is `Send + Sync` and safe to share
/// across tokio tasks.
pub struct SfBridge {
    wasm_bytes: Arc<Vec<u8>>,
    rest_client: SalesforceRestClient,
    handle: tokio::runtime::Handle,
}

impl SfBridge {
    /// Create a new bridge with the given WASM module bytes and REST client.
    ///
    /// The `rest_client` must already be authenticated. The bridge does not
    /// perform authentication -- that's the caller's responsibility.
    ///
    /// Must be called from within a tokio runtime context.
    pub fn new(wasm_bytes: Vec<u8>, rest_client: SalesforceRestClient) -> Result<Self> {
        let handle = tokio::runtime::Handle::current();
        Ok(Self {
            wasm_bytes: Arc::new(wasm_bytes),
            rest_client,
            handle,
        })
    }

    /// Create a new bridge, providing a specific tokio runtime handle.
    ///
    /// Use this when constructing the bridge outside of a tokio context.
    pub fn with_handle(
        wasm_bytes: Vec<u8>,
        rest_client: SalesforceRestClient,
        handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        Ok(Self {
            wasm_bytes: Arc::new(wasm_bytes),
            rest_client,
            handle,
        })
    }

    /// Call an exported function in the WASM guest.
    ///
    /// Each call creates a fresh plugin instance (cheap -- the module is
    /// pre-compiled by Extism/Wasmtime). The host functions are wired up
    /// with the bridge's Salesforce client.
    ///
    /// Safe to call concurrently from multiple tokio tasks.
    #[instrument(skip(self, input), fields(function = %function))]
    pub async fn call(
        &self,
        function: &str,
        input: impl AsRef<[u8]> + Send + 'static,
    ) -> Result<Vec<u8>> {
        let wasm_bytes = self.wasm_bytes.clone();
        let rest_client = self.rest_client.clone();
        let handle = self.handle.clone();
        let function = function.to_string();

        // Run the plugin on a blocking thread so host functions can
        // safely use Handle::block_on() for async Salesforce operations.
        tokio::task::spawn_blocking(move || {
            let state = BridgeState {
                rest_client,
                handle,
            };
            let mut plugin = create_plugin(&wasm_bytes, state)?;
            let result = plugin.call::<&[u8], &[u8]>(&function, input.as_ref())?;
            Ok(result.to_vec())
        })
        .await?
    }
}

/// Create an Extism plugin with all Salesforce host functions registered.
fn create_plugin(wasm_bytes: &[u8], state: BridgeState) -> Result<Plugin> {
    let manifest = Manifest::new([Wasm::data(wasm_bytes.to_vec())]);
    let user_data = UserData::new(state);

    let plugin = PluginBuilder::new(manifest)
        .with_wasi(true)
        .with_function(
            host_fn_names::QUERY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_query,
        )
        .with_function(
            host_fn_names::CREATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_create,
        )
        .with_function(
            host_fn_names::GET,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get,
        )
        .with_function(
            host_fn_names::UPDATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_update,
        )
        .with_function(
            host_fn_names::DELETE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_delete,
        )
        .with_function(
            host_fn_names::UPSERT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_upsert,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global,
        )
        .with_function(
            host_fn_names::DESCRIBE_SOBJECT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_sobject,
        )
        .with_function(
            host_fn_names::SEARCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search,
        )
        .with_function(
            host_fn_names::COMPOSITE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite,
        )
        .with_function(
            host_fn_names::CREATE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_create_multiple,
        )
        .with_function(
            host_fn_names::DELETE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_delete_multiple,
        )
        .with_function(
            host_fn_names::LIMITS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_limits,
        )
        .build()?;

    Ok(plugin)
}

// =============================================================================
// Host Function Implementations
//
// Each function follows the same pattern:
// 1. Lock UserData to access BridgeState
// 2. Read input bytes from WASM memory (memory_get_val)
// 3. Deserialize the typed request from JSON
// 4. Bridge to async via Handle::block_on() (safe inside spawn_blocking)
// 5. Serialize the BridgeResult response as JSON
// 6. Write output bytes to WASM memory (memory_new + memory_to_val)
// =============================================================================

/// Helper: read input, call synchronous handler (which internally block_on's),
/// write output. The handler receives `&BridgeState` so it can call
/// `state.handle.block_on(async_fn(&state.rest_client, req))` in one scope,
/// avoiding the lifetime issues of returning a future that borrows the client.
fn bridge_host_fn<Req, Resp>(
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
    let request: Req = serde_json::from_slice(&input_bytes)
        .map_err(|e| extism::Error::msg(format!("deserialize request: {e}")))?;

    let result = handler(&state, request);

    let output_bytes = serde_json::to_vec(&result)
        .map_err(|e| extism::Error::msg(format!("serialize response: {e}")))?;
    let mem_handle = plugin.memory_new(&output_bytes)?;
    outputs[0] = plugin.memory_to_val(mem_handle);
    Ok(())
}

/// Helper for host functions that take no meaningful input.
fn bridge_host_fn_no_input<Resp>(
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

    let output_bytes = serde_json::to_vec(&result)
        .map_err(|e| extism::Error::msg(format!("serialize response: {e}")))?;
    let mem_handle = plugin.memory_new(&output_bytes)?;
    outputs[0] = plugin.memory_to_val(mem_handle);
    Ok(())
}

// --- Individual host function callbacks ---
// Each passes a closure that calls block_on() in the same scope as the
// reference to rest_client, avoiding async lifetime issues.

fn host_fn_query(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_query(&s.rest_client, r))
    })
}

fn host_fn_create(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_create(&s.rest_client, r))
    })
}

fn host_fn_get(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get(&s.rest_client, r))
    })
}

fn host_fn_update(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_update(&s.rest_client, r))
    })
}

fn host_fn_delete(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_delete(&s.rest_client, r))
    })
}

fn host_fn_upsert(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_upsert(&s.rest_client, r))
    })
}

fn host_fn_describe_global(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_describe_global(&s.rest_client))
    })
}

fn host_fn_describe_sobject(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_sobject(&s.rest_client, r))
    })
}

fn host_fn_search(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search(&s.rest_client, r))
    })
}

fn host_fn_composite(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite(&s.rest_client, r))
    })
}

fn host_fn_create_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_create_multiple(&s.rest_client, r))
    })
}

fn host_fn_delete_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_delete_multiple(&s.rest_client, r))
    })
}

fn host_fn_limits(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_limits(&s.rest_client))
    })
}
