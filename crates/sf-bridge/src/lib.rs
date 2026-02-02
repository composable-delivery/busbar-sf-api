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

pub use error::{Error, Result};

use std::sync::Arc;

use busbar_sf_bulk::BulkApiClient;
use busbar_sf_metadata::MetadataClient;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_tooling::ToolingClient;
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
    pub(crate) bulk_client: BulkApiClient,
    pub(crate) tooling_client: ToolingClient,
    pub(crate) instance_url: Arc<str>,
    pub(crate) access_token: Arc<str>,
    pub(crate) handle: tokio::runtime::Handle,
}

impl BridgeState {
    /// Construct a fresh MetadataClient. MetadataClient is not Clone,
    /// so we build one on-demand from stored credentials.
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
    rest_client: SalesforceRestClient,
    bulk_client: BulkApiClient,
    tooling_client: ToolingClient,
    instance_url: Arc<str>,
    access_token: Arc<str>,
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
        Self::with_handle(wasm_bytes, rest_client, handle)
    }

    /// Create a new bridge, providing a specific tokio runtime handle.
    ///
    /// Use this when constructing the bridge outside of a tokio context.
    pub fn with_handle(
        wasm_bytes: Vec<u8>,
        rest_client: SalesforceRestClient,
        handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        let inner = rest_client.inner();
        let instance_url: Arc<str> = inner.instance_url().to_string().into();
        let access_token: Arc<str> = inner.access_token().to_string().into();

        let bulk_client = BulkApiClient::from_client(inner.clone());
        let tooling_client = ToolingClient::from_client(inner.clone());

        Ok(Self {
            wasm_bytes: Arc::new(wasm_bytes),
            rest_client,
            bulk_client,
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
        let rest_client = self.rest_client.clone();
        let bulk_client = self.bulk_client.clone();
        let tooling_client = self.tooling_client.clone();
        let instance_url = Arc::clone(&self.instance_url);
        let access_token = Arc::clone(&self.access_token);
        let handle = self.handle.clone();
        let function = function.to_string();

        // Run the plugin on a blocking thread so host functions can
        // safely use Handle::block_on() for async Salesforce operations.
        tokio::task::spawn_blocking(move || {
            let state = BridgeState {
                rest_client,
                bulk_client,
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

/// Create an Extism plugin with all Salesforce host functions registered.
fn create_plugin(wasm_bytes: &[u8], state: BridgeState) -> Result<Plugin> {
    let manifest = Manifest::new([Wasm::data(wasm_bytes.to_vec())]);
    let user_data = UserData::new(state);

    let plugin = PluginBuilder::new(manifest)
        .with_wasi(true)
        // =====================================================================
        // REST API host functions
        // =====================================================================
        .with_function(
            host_fn_names::QUERY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_query,
        )
        .with_function(
            host_fn_names::QUERY_MORE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_query_more,
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
            host_fn_names::COMPOSITE_BATCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_batch,
        )
        .with_function(
            host_fn_names::COMPOSITE_TREE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_tree,
        )
        .with_function(
            host_fn_names::CREATE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_create_multiple,
        )
        .with_function(
            host_fn_names::UPDATE_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_update_multiple,
        )
        .with_function(
            host_fn_names::GET_MULTIPLE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_multiple,
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
        .with_function(
            host_fn_names::VERSIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_versions,
        )
        // =====================================================================
        // REST API: Process & Approvals host functions
        // =====================================================================
        .with_function(
            host_fn_names::LIST_PROCESS_RULES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_process_rules,
        )
        .with_function(
            host_fn_names::LIST_PROCESS_RULES_FOR_SOBJECT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_process_rules_for_sobject,
        )
        .with_function(
            host_fn_names::TRIGGER_PROCESS_RULES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_trigger_process_rules,
        )
        .with_function(
            host_fn_names::LIST_PENDING_APPROVALS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_pending_approvals,
        )
        .with_function(
            host_fn_names::SUBMIT_APPROVAL,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_submit_approval,
        )
        // =====================================================================
        // REST API: List Views host functions
        // =====================================================================
        .with_function(
            host_fn_names::LIST_VIEWS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_views,
        )
        .with_function(
            host_fn_names::GET_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_list_view,
        )
        .with_function(
            host_fn_names::DESCRIBE_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_list_view,
        )
        .with_function(
            host_fn_names::EXECUTE_LIST_VIEW,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_execute_list_view,
        )
        // =====================================================================
        // REST API: Quick Actions host functions
        // =====================================================================
        .with_function(
            host_fn_names::LIST_GLOBAL_QUICK_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_global_quick_actions,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global_quick_action,
        )
        .with_function(
            host_fn_names::LIST_QUICK_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_quick_actions,
        )
        .with_function(
            host_fn_names::DESCRIBE_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_quick_action,
        )
        .with_function(
            host_fn_names::INVOKE_QUICK_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_quick_action,
        )
        // =====================================================================
        // REST API: Sync (Get Deleted/Updated) host functions
        // =====================================================================
        .with_function(
            host_fn_names::GET_DELETED,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_deleted,
        )
        .with_function(
            host_fn_names::GET_UPDATED,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_updated,
        )
        // =====================================================================
        // Bulk API host functions
        // =====================================================================
        .with_function(
            host_fn_names::BULK_CREATE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_create_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_UPLOAD_JOB_DATA,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_upload_job_data,
        )
        .with_function(
            host_fn_names::BULK_CLOSE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_close_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_ABORT_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_abort_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_GET_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_GET_JOB_RESULTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_job_results,
        )
        .with_function(
            host_fn_names::BULK_DELETE_INGEST_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_delete_ingest_job,
        )
        .with_function(
            host_fn_names::BULK_GET_ALL_INGEST_JOBS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_all_ingest_jobs,
        )
        .with_function(
            host_fn_names::BULK_ABORT_QUERY_JOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_abort_query_job,
        )
        .with_function(
            host_fn_names::BULK_GET_QUERY_RESULTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_bulk_get_query_results,
        )
        // =====================================================================
        // Tooling API host functions
        // =====================================================================
        .with_function(
            host_fn_names::TOOLING_QUERY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_query,
        )
        .with_function(
            host_fn_names::TOOLING_EXECUTE_ANONYMOUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_execute_anonymous,
        )
        .with_function(
            host_fn_names::TOOLING_GET,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_get,
        )
        .with_function(
            host_fn_names::TOOLING_CREATE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_create,
        )
        .with_function(
            host_fn_names::TOOLING_DELETE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tooling_delete,
        )
        // =====================================================================
        // Metadata API host functions
        // =====================================================================
        .with_function(
            host_fn_names::METADATA_DEPLOY,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_deploy,
        )
        .with_function(
            host_fn_names::METADATA_CHECK_DEPLOY_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_check_deploy_status,
        )
        .with_function(
            host_fn_names::METADATA_RETRIEVE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_retrieve,
        )
        .with_function(
            host_fn_names::METADATA_CHECK_RETRIEVE_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_check_retrieve_status,
        )
        .with_function(
            host_fn_names::METADATA_LIST,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_list,
        )
        .with_function(
            host_fn_names::METADATA_DESCRIBE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_metadata_describe,
        )
        // =====================================================================
        // Priority 2: Invocable Actions
        // =====================================================================
        .with_function(
            host_fn_names::LIST_STANDARD_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_standard_actions,
        )
        .with_function(
            host_fn_names::LIST_CUSTOM_ACTION_TYPES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_custom_action_types,
        )
        .with_function(
            host_fn_names::LIST_CUSTOM_ACTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_list_custom_actions,
        )
        .with_function(
            host_fn_names::DESCRIBE_STANDARD_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_standard_action,
        )
        .with_function(
            host_fn_names::DESCRIBE_CUSTOM_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_custom_action,
        )
        .with_function(
            host_fn_names::INVOKE_STANDARD_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_standard_action,
        )
        .with_function(
            host_fn_names::INVOKE_CUSTOM_ACTION,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_invoke_custom_action,
        )
        // =====================================================================
        // Priority 2: Layouts
        // =====================================================================
        .with_function(
            host_fn_names::DESCRIBE_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_NAMED_LAYOUT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_named_layout,
        )
        .with_function(
            host_fn_names::DESCRIBE_APPROVAL_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_approval_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_COMPACT_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_compact_layouts,
        )
        .with_function(
            host_fn_names::DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_describe_global_publisher_layouts,
        )
        // =====================================================================
        // Priority 2: Knowledge
        // =====================================================================
        .with_function(
            host_fn_names::KNOWLEDGE_SETTINGS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_knowledge_settings,
        )
        .with_function(
            host_fn_names::KNOWLEDGE_ARTICLES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_knowledge_articles,
        )
        .with_function(
            host_fn_names::DATA_CATEGORY_GROUPS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_data_category_groups,
        )
        .with_function(
            host_fn_names::DATA_CATEGORIES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_data_categories,
        )
        // =====================================================================
        // Priority 2: Standalone
        // =====================================================================
        .with_function(
            host_fn_names::TABS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_tabs,
        )
        .with_function(
            host_fn_names::THEME,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_theme,
        )
        .with_function(
            host_fn_names::APP_MENU,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_app_menu,
        )
        .with_function(
            host_fn_names::RECENT_ITEMS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_recent_items,
        )
        .with_function(
            host_fn_names::RELEVANT_ITEMS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_relevant_items,
        )
        .with_function(
            host_fn_names::COMPACT_LAYOUTS_MULTI,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_compact_layouts_multi,
        )
        .with_function(
            host_fn_names::PLATFORM_EVENT_SCHEMA,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_platform_event_schema,
        )
        .with_function(
            host_fn_names::LIGHTNING_TOGGLE_METRICS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_lightning_toggle_metrics,
        )
        .with_function(
            host_fn_names::LIGHTNING_USAGE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_lightning_usage,
        )
        // =====================================================================
        // Priority 2: User Password
        // =====================================================================
        .with_function(
            host_fn_names::GET_USER_PASSWORD_STATUS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_user_password_status,
        )
        .with_function(
            host_fn_names::SET_USER_PASSWORD,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_set_user_password,
        )
        .with_function(
            host_fn_names::RESET_USER_PASSWORD,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_reset_user_password,
        )
        // =====================================================================
        // Priority 2: Scheduler
        // =====================================================================
        .with_function(
            host_fn_names::APPOINTMENT_CANDIDATES,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_appointment_candidates,
        )
        .with_function(
            host_fn_names::APPOINTMENT_SLOTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_appointment_slots,
        )
        // =====================================================================
        // Priority 2: Consent
        // =====================================================================
        .with_function(
            host_fn_names::READ_CONSENT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_read_consent,
        )
        .with_function(
            host_fn_names::WRITE_CONSENT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_write_consent,
        )
        .with_function(
            host_fn_names::READ_MULTI_CONSENT,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_read_multi_consent,
        )
        // =====================================================================
        // Priority 2: Binary
        // =====================================================================
        .with_function(
            host_fn_names::GET_BLOB,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_blob,
        )
        .with_function(
            host_fn_names::GET_RICH_TEXT_IMAGE,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_rich_text_image,
        )
        .with_function(
            host_fn_names::GET_RELATIONSHIP,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_relationship,
        )
        // =====================================================================
        // Priority 2: Embedded Service
        // =====================================================================
        .with_function(
            host_fn_names::GET_EMBEDDED_SERVICE_CONFIG,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_get_embedded_service_config,
        )
        // =====================================================================
        // Priority 2: Search Enhancements
        // =====================================================================
        .with_function(
            host_fn_names::PARAMETERIZED_SEARCH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_parameterized_search,
        )
        .with_function(
            host_fn_names::SEARCH_SUGGESTIONS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_suggestions,
        )
        .with_function(
            host_fn_names::SEARCH_SCOPE_ORDER,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_scope_order,
        )
        .with_function(
            host_fn_names::SEARCH_RESULT_LAYOUTS,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_search_result_layouts,
        )
        // =====================================================================
        // Priority 2: Composite Enhancement
        // =====================================================================
        .with_function(
            host_fn_names::COMPOSITE_GRAPH,
            [ValType::I64],
            [ValType::I64],
            user_data.clone(),
            host_fn_composite_graph,
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

    let output_bytes = rmp_serde::to_vec_named(&result)
        .map_err(|e| extism::Error::msg(format!("serialize response: {e}")))?;
    let mem_handle = plugin.memory_new(&output_bytes)?;
    outputs[0] = plugin.memory_to_val(mem_handle);
    Ok(())
}

// =============================================================================
// REST API host function callbacks
// =============================================================================

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

fn host_fn_query_more(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_query_more(&s.rest_client, r))
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

fn host_fn_composite_batch(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_batch(&s.rest_client, r))
    })
}

fn host_fn_composite_tree(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_tree(&s.rest_client, r))
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

fn host_fn_update_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_update_multiple(&s.rest_client, r))
    })
}

fn host_fn_get_multiple(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_multiple(&s.rest_client, r))
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

fn host_fn_versions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_versions(&s.rest_client))
    })
}

// =============================================================================
// REST API: Process & Approvals host function callbacks
// =============================================================================

fn host_fn_list_process_rules(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_process_rules(&s.rest_client))
    })
}

fn host_fn_list_process_rules_for_sobject(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_process_rules_for_sobject(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_trigger_process_rules(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_trigger_process_rules(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_list_pending_approvals(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_pending_approvals(
                &s.rest_client,
            ))
    })
}

fn host_fn_submit_approval(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_submit_approval(&s.rest_client, r))
    })
}

// =============================================================================
// REST API: List Views host function callbacks
// =============================================================================

fn host_fn_list_views(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_views(&s.rest_client, r))
    })
}

fn host_fn_get_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_list_view(&s.rest_client, r))
    })
}

fn host_fn_describe_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_list_view(&s.rest_client, r))
    })
}

fn host_fn_execute_list_view(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_execute_list_view(&s.rest_client, r))
    })
}

// =============================================================================
// REST API: Quick Actions host function callbacks
// =============================================================================

fn host_fn_list_global_quick_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_global_quick_actions(
                &s.rest_client,
            ))
    })
}

fn host_fn_describe_global_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_global_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_list_quick_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_quick_actions(&s.rest_client, r))
    })
}

fn host_fn_describe_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_quick_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_quick_action(
                &s.rest_client,
                r,
            ))
    })
}

// =============================================================================
// REST API: Sync (Get Deleted/Updated) host function callbacks
// =============================================================================

fn host_fn_get_deleted(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_deleted(&s.rest_client, r))
    })
}

fn host_fn_get_updated(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_updated(&s.rest_client, r))
    })
}

// =============================================================================
// Bulk API host function callbacks
// =============================================================================

fn host_fn_bulk_create_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_create_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_upload_job_data(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_upload_job_data(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_close_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_close_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_abort_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_abort_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_job_results(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_job_results(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_delete_ingest_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_delete_ingest_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_all_ingest_jobs(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_bulk_get_all_ingest_jobs(
                &s.bulk_client,
            ))
    })
}

fn host_fn_bulk_abort_query_job(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_abort_query_job(
                &s.bulk_client,
                r,
            ))
    })
}

fn host_fn_bulk_get_query_results(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_bulk_get_query_results(
                &s.bulk_client,
                r,
            ))
    })
}

// =============================================================================
// Tooling API host function callbacks
// =============================================================================

fn host_fn_tooling_query(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_query(&s.tooling_client, r))
    })
}

fn host_fn_tooling_execute_anonymous(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_execute_anonymous(
                &s.tooling_client,
                r,
            ))
    })
}

fn host_fn_tooling_get(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_get(&s.tooling_client, r))
    })
}

fn host_fn_tooling_create(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_create(&s.tooling_client, r))
    })
}

fn host_fn_tooling_delete(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_tooling_delete(&s.tooling_client, r))
    })
}

// =============================================================================
// Metadata API host function callbacks
// =============================================================================

fn host_fn_metadata_deploy(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_deploy(&client, r))
    })
}

fn host_fn_metadata_check_deploy_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_check_deploy_status(
                &client, r,
            ))
    })
}

fn host_fn_metadata_retrieve(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_retrieve(&client, r))
    })
}

fn host_fn_metadata_check_retrieve_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_check_retrieve_status(
                &client, r,
            ))
    })
}

fn host_fn_metadata_list(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_list(&client, r))
    })
}

fn host_fn_metadata_describe(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        let client = s.metadata_client();
        s.handle
            .block_on(host_functions::handle_metadata_describe(&client))
    })
}

// =============================================================================
// Priority 2 host function wrappers
// =============================================================================

fn host_fn_list_standard_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_standard_actions(&s.rest_client))
    })
}

fn host_fn_list_custom_action_types(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_list_custom_action_types(
                &s.rest_client,
            ))
    })
}

fn host_fn_list_custom_actions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_list_custom_actions(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_standard_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_standard_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_custom_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_custom_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_standard_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_standard_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_invoke_custom_action(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_invoke_custom_action(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_layouts(&s.rest_client, r))
    })
}

fn host_fn_describe_named_layout(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_named_layout(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_approval_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_approval_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_compact_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_describe_compact_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_describe_global_publisher_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_describe_global_publisher_layouts(
                &s.rest_client,
            ))
    })
}

fn host_fn_knowledge_settings(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_knowledge_settings(&s.rest_client))
    })
}

fn host_fn_knowledge_articles(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_knowledge_articles(&s.rest_client, r))
    })
}

fn host_fn_data_category_groups(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_data_category_groups(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_data_categories(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_data_categories(&s.rest_client, r))
    })
}

fn host_fn_tabs(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_tabs(&s.rest_client))
    })
}

fn host_fn_theme(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_theme(&s.rest_client))
    })
}

fn host_fn_app_menu(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_app_menu(&s.rest_client, r))
    })
}

fn host_fn_recent_items(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_recent_items(&s.rest_client))
    })
}

fn host_fn_relevant_items(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_relevant_items(&s.rest_client))
    })
}

fn host_fn_compact_layouts_multi(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_compact_layouts_multi(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_platform_event_schema(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_platform_event_schema(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_lightning_toggle_metrics(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_lightning_toggle_metrics(
                &s.rest_client,
            ))
    })
}

fn host_fn_lightning_usage(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_lightning_usage(&s.rest_client))
    })
}

fn host_fn_get_user_password_status(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_user_password_status(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_set_user_password(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_set_user_password(&s.rest_client, r))
    })
}

fn host_fn_reset_user_password(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_reset_user_password(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_appointment_candidates(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_appointment_candidates(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_appointment_slots(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_appointment_slots(&s.rest_client, r))
    })
}

fn host_fn_read_consent(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_read_consent(&s.rest_client, r))
    })
}

fn host_fn_write_consent(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_write_consent(&s.rest_client, r))
    })
}

fn host_fn_read_multi_consent(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_read_multi_consent(&s.rest_client, r))
    })
}

fn host_fn_get_blob(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_blob(&s.rest_client, r))
    })
}

fn host_fn_get_rich_text_image(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_rich_text_image(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_get_relationship(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_relationship(&s.rest_client, r))
    })
}

fn host_fn_get_embedded_service_config(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_get_embedded_service_config(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_parameterized_search(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_parameterized_search(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_search_suggestions(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search_suggestions(&s.rest_client, r))
    })
}

fn host_fn_search_scope_order(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn_no_input(plugin, inputs, outputs, user_data, |s| {
        s.handle
            .block_on(host_functions::handle_search_scope_order(&s.rest_client))
    })
}

fn host_fn_search_result_layouts(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_search_result_layouts(
                &s.rest_client,
                r,
            ))
    })
}

fn host_fn_composite_graph(
    plugin: &mut extism::CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<BridgeState>,
) -> std::result::Result<(), extism::Error> {
    bridge_host_fn(plugin, inputs, outputs, user_data, |s, r| {
        s.handle
            .block_on(host_functions::handle_composite_graph(&s.rest_client, r))
    })
}
