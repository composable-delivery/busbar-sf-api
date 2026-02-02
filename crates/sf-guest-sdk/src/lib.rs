//! # busbar-sf-guest-sdk
//!
//! Guest SDK for building WASM plugins that interact with Salesforce APIs
//! through the busbar bridge.
//!
//! This crate is compiled to `wasm32-unknown-unknown` and loaded by a host
//! running [`busbar-sf-bridge`]. All Salesforce operations are executed by
//! the host - this SDK just provides ergonomic wrappers around the host
//! function imports.
//!
//! ## Security
//!
//! Your plugin code **never sees Salesforce credentials**. The host manages
//! all authentication. You call functions like [`query`] and get results back.
//! There is no way to extract the access token from within the WASM sandbox.
//!
//! ## APIs Available
//!
//! - **REST API**: SOQL queries, CRUD, composite, collections, search, limits
//! - **Bulk API**: Ingest jobs, query jobs, CSV upload/download
//! - **Tooling API**: Apex execution, tooling SOQL, tooling CRUD
//! - **Metadata API**: Deploy, retrieve, list, describe metadata
//!
//! ## Example Plugin
//!
//! ```rust,ignore
//! use busbar_sf_guest_sdk::*;
//! use extism_pdk::*;
//!
//! #[plugin_fn]
//! pub fn run(_input: String) -> FnResult<Json<Vec<serde_json::Value>>> {
//!     let accounts = query("SELECT Id, Name FROM Account LIMIT 10")?;
//!     Ok(Json(accounts.records))
//! }
//! ```
//!
//! ## Testing Strategy
//!
//! Traditional unit tests are not feasible for this crate because:
//! 1. It depends on extism-pdk which requires the Extism WASM runtime
//! 2. The host functions (sf_query, sf_create, etc.) are only available
//!    when running inside a WASM plugin loaded by the bridge
//! 3. The helper functions (call_host_fn, call_host_fn_no_input) require
//!    these host functions to be present
//!
//! This crate is thoroughly tested via:
//! - Integration tests in sf-bridge that load actual WASM plugins
//! - The example wasm-guest-plugin that exercises all APIs
//! - Type safety enforced by the compiler (shared types with sf-wasm-types)

pub use busbar_sf_wasm_types::*;
use extism_pdk::*;

// =============================================================================
// Host function imports
//
// These are provided by the sf-bridge host at runtime. The `extern "ExtismHost"`
// block declares them so the WASM module knows to import them.
// =============================================================================

#[host_fn]
extern "ExtismHost" {
    // REST API
    fn sf_query(input: Vec<u8>) -> Vec<u8>;
    fn sf_query_more(input: Vec<u8>) -> Vec<u8>;
    fn sf_create(input: Vec<u8>) -> Vec<u8>;
    fn sf_get(input: Vec<u8>) -> Vec<u8>;
    fn sf_update(input: Vec<u8>) -> Vec<u8>;
    fn sf_delete(input: Vec<u8>) -> Vec<u8>;
    fn sf_upsert(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_global(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_sobject(input: Vec<u8>) -> Vec<u8>;
    fn sf_search(input: Vec<u8>) -> Vec<u8>;
    fn sf_composite(input: Vec<u8>) -> Vec<u8>;
    fn sf_composite_batch(input: Vec<u8>) -> Vec<u8>;
    fn sf_composite_tree(input: Vec<u8>) -> Vec<u8>;
    fn sf_create_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_update_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_get_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_delete_multiple(input: Vec<u8>) -> Vec<u8>;
    fn sf_limits(input: Vec<u8>) -> Vec<u8>;
    fn sf_versions(input: Vec<u8>) -> Vec<u8>;

    // REST API: Process & Approvals
    fn sf_list_process_rules(input: Vec<u8>) -> Vec<u8>;
    fn sf_list_process_rules_for_sobject(input: Vec<u8>) -> Vec<u8>;
    fn sf_trigger_process_rules(input: Vec<u8>) -> Vec<u8>;
    fn sf_list_pending_approvals(input: Vec<u8>) -> Vec<u8>;
    fn sf_submit_approval(input: Vec<u8>) -> Vec<u8>;

    // REST API: List Views
    fn sf_list_views(input: Vec<u8>) -> Vec<u8>;
    fn sf_get_list_view(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_list_view(input: Vec<u8>) -> Vec<u8>;
    fn sf_execute_list_view(input: Vec<u8>) -> Vec<u8>;

    // REST API: Quick Actions
    fn sf_list_global_quick_actions(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_global_quick_action(input: Vec<u8>) -> Vec<u8>;
    fn sf_list_quick_actions(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_quick_action(input: Vec<u8>) -> Vec<u8>;
    fn sf_invoke_quick_action(input: Vec<u8>) -> Vec<u8>;

    // REST API: Sync
    fn sf_get_deleted(input: Vec<u8>) -> Vec<u8>;
    fn sf_get_updated(input: Vec<u8>) -> Vec<u8>;

    // Bulk API
    fn sf_bulk_create_ingest_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_upload_job_data(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_close_ingest_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_abort_ingest_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_get_ingest_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_get_job_results(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_delete_ingest_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_get_all_ingest_jobs(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_abort_query_job(input: Vec<u8>) -> Vec<u8>;
    fn sf_bulk_get_query_results(input: Vec<u8>) -> Vec<u8>;

    // Tooling API
    fn sf_tooling_query(input: Vec<u8>) -> Vec<u8>;
    fn sf_tooling_execute_anonymous(input: Vec<u8>) -> Vec<u8>;
    fn sf_tooling_get(input: Vec<u8>) -> Vec<u8>;
    fn sf_tooling_create(input: Vec<u8>) -> Vec<u8>;
    fn sf_tooling_delete(input: Vec<u8>) -> Vec<u8>;

    // Metadata API
    fn sf_metadata_deploy(input: Vec<u8>) -> Vec<u8>;
    fn sf_metadata_check_deploy_status(input: Vec<u8>) -> Vec<u8>;
    fn sf_metadata_retrieve(input: Vec<u8>) -> Vec<u8>;
    fn sf_metadata_check_retrieve_status(input: Vec<u8>) -> Vec<u8>;
    fn sf_metadata_list(input: Vec<u8>) -> Vec<u8>;
    fn sf_metadata_describe(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Invocable Actions
    fn sf_list_standard_actions(input: Vec<u8>) -> Vec<u8>;
    fn sf_list_custom_action_types(input: Vec<u8>) -> Vec<u8>;
    fn sf_list_custom_actions(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_standard_action(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_custom_action(input: Vec<u8>) -> Vec<u8>;
    fn sf_invoke_standard_action(input: Vec<u8>) -> Vec<u8>;
    fn sf_invoke_custom_action(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Layouts
    fn sf_describe_layouts(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_named_layout(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_approval_layouts(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_compact_layouts(input: Vec<u8>) -> Vec<u8>;
    fn sf_describe_global_publisher_layouts(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Knowledge
    fn sf_knowledge_settings(input: Vec<u8>) -> Vec<u8>;
    fn sf_knowledge_articles(input: Vec<u8>) -> Vec<u8>;
    fn sf_data_category_groups(input: Vec<u8>) -> Vec<u8>;
    fn sf_data_categories(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Standalone
    fn sf_tabs(input: Vec<u8>) -> Vec<u8>;
    fn sf_theme(input: Vec<u8>) -> Vec<u8>;
    fn sf_app_menu(input: Vec<u8>) -> Vec<u8>;
    fn sf_recent_items(input: Vec<u8>) -> Vec<u8>;
    fn sf_relevant_items(input: Vec<u8>) -> Vec<u8>;
    fn sf_compact_layouts_multi(input: Vec<u8>) -> Vec<u8>;
    fn sf_platform_event_schema(input: Vec<u8>) -> Vec<u8>;
    fn sf_lightning_toggle_metrics(input: Vec<u8>) -> Vec<u8>;
    fn sf_lightning_usage(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: User Password
    fn sf_get_user_password_status(input: Vec<u8>) -> Vec<u8>;
    fn sf_set_user_password(input: Vec<u8>) -> Vec<u8>;
    fn sf_reset_user_password(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Scheduler
    fn sf_appointment_candidates(input: Vec<u8>) -> Vec<u8>;
    fn sf_appointment_slots(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Consent
    fn sf_read_consent(input: Vec<u8>) -> Vec<u8>;
    fn sf_write_consent(input: Vec<u8>) -> Vec<u8>;
    fn sf_read_multi_consent(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Binary
    fn sf_get_blob(input: Vec<u8>) -> Vec<u8>;
    fn sf_get_rich_text_image(input: Vec<u8>) -> Vec<u8>;
    fn sf_get_relationship(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Embedded Service
    fn sf_get_embedded_service_config(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Search Enhancements
    fn sf_parameterized_search(input: Vec<u8>) -> Vec<u8>;
    fn sf_search_suggestions(input: Vec<u8>) -> Vec<u8>;
    fn sf_search_scope_order(input: Vec<u8>) -> Vec<u8>;
    fn sf_search_result_layouts(input: Vec<u8>) -> Vec<u8>;

    // Priority 2: Composite Enhancement
    fn sf_composite_graph(input: Vec<u8>) -> Vec<u8>;
}

// =============================================================================
// REST API wrappers
// =============================================================================

/// Execute a SOQL query.
///
/// Returns the first page of results. Check `done` and `next_records_url`
/// for pagination.
///
/// # Example
///
/// ```rust,ignore
/// let result = query("SELECT Id, Name FROM Account LIMIT 10")?;
/// for record in &result.records {
///     // process records...
/// }
/// ```
pub fn query(soql: &str) -> Result<QueryResponse, Error> {
    let request = QueryRequest {
        soql: soql.to_string(),
        include_deleted: false,
    };
    call_host_fn(|input| unsafe { sf_query(input) }, &request)
}

/// Execute a SOQL query including deleted/archived records.
pub fn query_all(soql: &str) -> Result<QueryResponse, Error> {
    let request = QueryRequest {
        soql: soql.to_string(),
        include_deleted: true,
    };
    call_host_fn(|input| unsafe { sf_query(input) }, &request)
}

/// Fetch the next page of query results.
///
/// Use the `next_records_url` from a previous [`query`] response.
pub fn query_more(next_records_url: &str) -> Result<QueryResponse, Error> {
    let request = QueryMoreRequest {
        next_records_url: next_records_url.to_string(),
    };
    call_host_fn(|input| unsafe { sf_query_more(input) }, &request)
}

/// Create a new record.
///
/// Returns the result including the new record's ID.
///
/// # Example
///
/// ```rust,ignore
/// let result = create("Account", &serde_json::json!({"Name": "Acme Corp"}))?;
/// let new_id = result.id;
/// ```
pub fn create(sobject: &str, record: &serde_json::Value) -> Result<CreateResponse, Error> {
    let request = CreateRequest {
        sobject: sobject.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_create(input) }, &request)
}

/// Get a record by ID.
///
/// # Example
///
/// ```rust,ignore
/// let record = get("Account", "001xx000003DgAAAS", None)?;
/// let name = &record["Name"];
/// ```
pub fn get(
    sobject: &str,
    id: &str,
    fields: Option<Vec<String>>,
) -> Result<serde_json::Value, Error> {
    let request = GetRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        fields,
    };
    call_host_fn(|input| unsafe { sf_get(input) }, &request)
}

/// Update a record.
///
/// # Example
///
/// ```rust,ignore
/// update("Account", "001xx000003DgAAAS", &serde_json::json!({"Name": "New Name"}))?;
/// ```
pub fn update(sobject: &str, id: &str, record: &serde_json::Value) -> Result<(), Error> {
    let request = UpdateRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_update(input) }, &request)
}

/// Delete a record.
///
/// # Example
///
/// ```rust,ignore
/// delete("Account", "001xx000003DgAAAS")?;
/// ```
pub fn delete(sobject: &str, id: &str) -> Result<(), Error> {
    let request = DeleteRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_delete(input) }, &request)
}

/// Upsert a record using an external ID.
///
/// Creates the record if it doesn't exist, updates it if it does.
pub fn upsert(
    sobject: &str,
    external_id_field: &str,
    external_id_value: &str,
    record: &serde_json::Value,
) -> Result<UpsertResponse, Error> {
    let request = UpsertRequest {
        sobject: sobject.to_string(),
        external_id_field: external_id_field.to_string(),
        external_id_value: external_id_value.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_upsert(input) }, &request)
}

/// Get metadata for all SObjects in the org.
pub fn describe_global() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_describe_global(input) })
}

/// Get metadata for a specific SObject.
pub fn describe_sobject(sobject: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_sobject(input) }, &request)
}

/// Execute a SOSL full-text search.
pub fn search(sosl: &str) -> Result<SearchResponse, Error> {
    let request = SearchRequest {
        sosl: sosl.to_string(),
    };
    call_host_fn(|input| unsafe { sf_search(input) }, &request)
}

/// Execute a composite API request.
///
/// Allows multiple subrequests in a single API call. Subrequests can
/// reference results from earlier subrequests using `@{referenceId}`.
pub fn composite(request: &CompositeRequest) -> Result<CompositeResponse, Error> {
    call_host_fn(|input| unsafe { sf_composite(input) }, request)
}

/// Execute a composite batch API request.
///
/// Groups multiple independent requests into a single API call.
pub fn composite_batch(request: &CompositeBatchRequest) -> Result<CompositeBatchResponse, Error> {
    call_host_fn(|input| unsafe { sf_composite_batch(input) }, request)
}

/// Execute a composite tree API request.
///
/// Creates a tree of related records in a single API call.
pub fn composite_tree(request: &CompositeTreeRequest) -> Result<CompositeTreeResponse, Error> {
    call_host_fn(|input| unsafe { sf_composite_tree(input) }, request)
}

/// Create multiple records in a single request (up to 200).
pub fn create_multiple(
    sobject: &str,
    records: Vec<serde_json::Value>,
    all_or_none: bool,
) -> Result<Vec<CollectionResult>, Error> {
    let request = CreateMultipleRequest {
        sobject: sobject.to_string(),
        records,
        all_or_none,
    };
    call_host_fn(|input| unsafe { sf_create_multiple(input) }, &request)
}

/// Update multiple records in a single request (up to 200).
pub fn update_multiple(
    sobject: &str,
    records: Vec<UpdateMultipleRecord>,
    all_or_none: bool,
) -> Result<Vec<CollectionResult>, Error> {
    let request = UpdateMultipleRequest {
        sobject: sobject.to_string(),
        records,
        all_or_none,
    };
    call_host_fn(|input| unsafe { sf_update_multiple(input) }, &request)
}

/// Get multiple records by ID in a single request.
pub fn get_multiple(
    sobject: &str,
    ids: Vec<String>,
    fields: Vec<String>,
) -> Result<Vec<serde_json::Value>, Error> {
    let request = GetMultipleRequest {
        sobject: sobject.to_string(),
        ids,
        fields,
    };
    call_host_fn(|input| unsafe { sf_get_multiple(input) }, &request)
}

/// Delete multiple records in a single request (up to 200).
pub fn delete_multiple(
    ids: Vec<String>,
    all_or_none: bool,
) -> Result<Vec<CollectionResult>, Error> {
    let request = DeleteMultipleRequest { ids, all_or_none };
    call_host_fn(|input| unsafe { sf_delete_multiple(input) }, &request)
}

/// Get API limits for the org.
pub fn limits() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_limits(input) })
}

/// Get available API versions.
pub fn versions() -> Result<Vec<ApiVersion>, Error> {
    call_host_fn_no_input(|input| unsafe { sf_versions(input) })
}

// =============================================================================
// REST API: Process & Approvals wrappers
// =============================================================================

/// List all process rules.
pub fn list_process_rules() -> Result<ProcessRuleCollection, Error> {
    call_host_fn_no_input(|input| unsafe { sf_list_process_rules(input) })
}

/// List process rules for a specific SObject.
pub fn list_process_rules_for_sobject(sobject: &str) -> Result<Vec<ProcessRule>, Error> {
    let request = ListProcessRulesForSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_list_process_rules_for_sobject(input) },
        &request,
    )
}

/// Trigger process rules for records.
pub fn trigger_process_rules(context_ids: Vec<String>) -> Result<ProcessRuleResult, Error> {
    let request = ProcessRuleRequest { context_ids };
    call_host_fn(
        |input| unsafe { sf_trigger_process_rules(input) },
        &request,
    )
}

/// List pending approvals.
pub fn list_pending_approvals() -> Result<PendingApprovalCollection, Error> {
    call_host_fn_no_input(|input| unsafe { sf_list_pending_approvals(input) })
}

/// Submit, approve, or reject an approval.
pub fn submit_approval(request: &ApprovalRequest) -> Result<ApprovalResult, Error> {
    call_host_fn(|input| unsafe { sf_submit_approval(input) }, request)
}

// =============================================================================
// REST API: List Views wrappers
// =============================================================================

/// List all list views for an SObject.
pub fn list_views(sobject: &str) -> Result<ListViewsResult, Error> {
    let request = ListViewsRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_list_views(input) }, &request)
}

/// Get a specific list view by ID.
pub fn get_list_view(sobject: &str, list_view_id: &str) -> Result<ListView, Error> {
    let request = ListViewRequest {
        sobject: sobject.to_string(),
        list_view_id: list_view_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_list_view(input) }, &request)
}

/// Describe a list view (get columns, filters, etc.).
pub fn describe_list_view(sobject: &str, list_view_id: &str) -> Result<ListViewDescribe, Error> {
    let request = ListViewRequest {
        sobject: sobject.to_string(),
        list_view_id: list_view_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_list_view(input) }, &request)
}

/// Execute a list view and return its results.
pub fn execute_list_view(sobject: &str, list_view_id: &str) -> Result<serde_json::Value, Error> {
    let request = ListViewRequest {
        sobject: sobject.to_string(),
        list_view_id: list_view_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_execute_list_view(input) }, &request)
}

// =============================================================================
// REST API: Quick Actions wrappers
// =============================================================================

/// List all global quick actions.
pub fn list_global_quick_actions() -> Result<Vec<QuickActionMetadata>, Error> {
    call_host_fn_no_input(|input| unsafe { sf_list_global_quick_actions(input) })
}

/// Describe a global quick action.
pub fn describe_global_quick_action(action: &str) -> Result<QuickActionDescribe, Error> {
    let request = DescribeGlobalQuickActionRequest {
        action: action.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_describe_global_quick_action(input) },
        &request,
    )
}

/// List quick actions available for an SObject.
pub fn list_quick_actions(sobject: &str) -> Result<Vec<QuickActionMetadata>, Error> {
    let request = ListQuickActionsRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_list_quick_actions(input) }, &request)
}

/// Describe a specific quick action on an SObject.
pub fn describe_quick_action(
    sobject: &str,
    action: &str,
) -> Result<QuickActionDescribe, Error> {
    let request = DescribeQuickActionRequest {
        sobject: sobject.to_string(),
        action: action.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_describe_quick_action(input) },
        &request,
    )
}

/// Invoke a quick action on an SObject.
pub fn invoke_quick_action(
    sobject: &str,
    action: &str,
    record_id: Option<&str>,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let request = InvokeQuickActionRequest {
        sobject: sobject.to_string(),
        action: action.to_string(),
        record_id: record_id.map(|s| s.to_string()),
        body: body.clone(),
    };
    call_host_fn(|input| unsafe { sf_invoke_quick_action(input) }, &request)
}

// =============================================================================
// REST API: Sync (Get Deleted/Updated) wrappers
// =============================================================================

/// Get deleted records for an SObject within a date range.
///
/// The start and end parameters should be ISO 8601 date-time strings
/// (e.g., "2024-01-01T00:00:00Z").
pub fn get_deleted(sobject: &str, start: &str, end: &str) -> Result<GetDeletedResult, Error> {
    let request = GetDeletedRequest {
        sobject: sobject.to_string(),
        start: start.to_string(),
        end: end.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_deleted(input) }, &request)
}

/// Get updated record IDs for an SObject within a date range.
///
/// The start and end parameters should be ISO 8601 date-time strings
/// (e.g., "2024-01-01T00:00:00Z").
pub fn get_updated(sobject: &str, start: &str, end: &str) -> Result<GetUpdatedResult, Error> {
    let request = GetUpdatedRequest {
        sobject: sobject.to_string(),
        start: start.to_string(),
        end: end.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_updated(input) }, &request)
}

// =============================================================================
// Bulk API wrappers
// =============================================================================

/// Create a bulk ingest job.
///
/// # Example
///
/// ```rust,ignore
/// let job = bulk_create_ingest_job("Account", "insert", None, "COMMA", "LF")?;
/// let job_id = job.id;
/// ```
pub fn bulk_create_ingest_job(
    sobject: &str,
    operation: &str,
    external_id_field: Option<String>,
    column_delimiter: &str,
    line_ending: &str,
) -> Result<BulkJobResponse, Error> {
    let request = BulkCreateIngestJobRequest {
        sobject: sobject.to_string(),
        operation: operation.to_string(),
        external_id_field,
        column_delimiter: column_delimiter.to_string(),
        line_ending: line_ending.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_create_ingest_job(input) },
        &request,
    )
}

/// Upload CSV data to a bulk ingest job.
pub fn bulk_upload_job_data(job_id: &str, csv_data: &str) -> Result<(), Error> {
    let request = BulkUploadJobDataRequest {
        job_id: job_id.to_string(),
        csv_data: csv_data.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_upload_job_data(input) },
        &request,
    )
}

/// Close a bulk ingest job (marks it ready for processing).
pub fn bulk_close_ingest_job(job_id: &str) -> Result<BulkJobResponse, Error> {
    let request = BulkJobIdRequest {
        job_id: job_id.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_close_ingest_job(input) },
        &request,
    )
}

/// Abort a bulk ingest job.
pub fn bulk_abort_ingest_job(job_id: &str) -> Result<BulkJobResponse, Error> {
    let request = BulkJobIdRequest {
        job_id: job_id.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_abort_ingest_job(input) },
        &request,
    )
}

/// Get the status of a bulk ingest job.
pub fn bulk_get_ingest_job(job_id: &str) -> Result<BulkJobResponse, Error> {
    let request = BulkJobIdRequest {
        job_id: job_id.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_get_ingest_job(input) },
        &request,
    )
}

/// Get job results (successful, failed, or unprocessed records).
///
/// `result_type` must be one of: `"successful"`, `"failed"`, `"unprocessed"`.
pub fn bulk_get_job_results(
    job_id: &str,
    result_type: &str,
) -> Result<BulkJobResultsResponse, Error> {
    let request = BulkJobResultsRequest {
        job_id: job_id.to_string(),
        result_type: result_type.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_get_job_results(input) },
        &request,
    )
}

/// Delete a bulk ingest job.
pub fn bulk_delete_ingest_job(job_id: &str) -> Result<(), Error> {
    let request = BulkJobIdRequest {
        job_id: job_id.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_delete_ingest_job(input) },
        &request,
    )
}

/// List all ingest jobs.
pub fn bulk_get_all_ingest_jobs() -> Result<BulkJobListResponse, Error> {
    call_host_fn_no_input(|input| unsafe { sf_bulk_get_all_ingest_jobs(input) })
}

/// Abort a bulk query job.
pub fn bulk_abort_query_job(job_id: &str) -> Result<BulkJobResponse, Error> {
    let request = BulkJobIdRequest {
        job_id: job_id.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_bulk_abort_query_job(input) },
        &request,
    )
}

/// Get query job results as CSV.
pub fn bulk_get_query_results(
    job_id: &str,
    locator: Option<String>,
    max_records: Option<u64>,
) -> Result<BulkQueryResultsResponse, Error> {
    let request = BulkQueryResultsRequest {
        job_id: job_id.to_string(),
        locator,
        max_records,
    };
    call_host_fn(
        |input| unsafe { sf_bulk_get_query_results(input) },
        &request,
    )
}

// =============================================================================
// Tooling API wrappers
// =============================================================================

/// Execute a Tooling API SOQL query.
pub fn tooling_query(soql: &str) -> Result<QueryResponse, Error> {
    let request = ToolingQueryRequest {
        soql: soql.to_string(),
    };
    call_host_fn(|input| unsafe { sf_tooling_query(input) }, &request)
}

/// Execute anonymous Apex code.
///
/// # Example
///
/// ```rust,ignore
/// let result = tooling_execute_anonymous("System.debug('Hello');")?;
/// assert!(result.success);
/// ```
pub fn tooling_execute_anonymous(apex_code: &str) -> Result<ExecuteAnonymousResponse, Error> {
    let request = ExecuteAnonymousRequest {
        apex_code: apex_code.to_string(),
    };
    call_host_fn(
        |input| unsafe { sf_tooling_execute_anonymous(input) },
        &request,
    )
}

/// Get a Tooling API record by ID.
pub fn tooling_get(sobject: &str, id: &str) -> Result<serde_json::Value, Error> {
    let request = ToolingGetRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_tooling_get(input) }, &request)
}

/// Create a Tooling API record.
pub fn tooling_create(
    sobject: &str,
    record: &serde_json::Value,
) -> Result<CreateResponse, Error> {
    let request = ToolingCreateRequest {
        sobject: sobject.to_string(),
        record: record.clone(),
    };
    call_host_fn(|input| unsafe { sf_tooling_create(input) }, &request)
}

/// Delete a Tooling API record.
pub fn tooling_delete(sobject: &str, id: &str) -> Result<(), Error> {
    let request = ToolingDeleteRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_tooling_delete(input) }, &request)
}

// =============================================================================
// Metadata API wrappers
// =============================================================================

/// Deploy a metadata package (zip file as base64).
///
/// Returns an async process ID to track the deployment.
pub fn metadata_deploy(
    zip_base64: &str,
    options: MetadataDeployOptions,
) -> Result<MetadataDeployResponse, Error> {
    let request = MetadataDeployRequest {
        zip_base64: zip_base64.to_string(),
        options,
    };
    call_host_fn(|input| unsafe { sf_metadata_deploy(input) }, &request)
}

/// Check the status of a metadata deployment.
pub fn metadata_check_deploy_status(
    async_process_id: &str,
    include_details: bool,
) -> Result<MetadataDeployResult, Error> {
    let request = MetadataCheckDeployStatusRequest {
        async_process_id: async_process_id.to_string(),
        include_details,
    };
    call_host_fn(
        |input| unsafe { sf_metadata_check_deploy_status(input) },
        &request,
    )
}

/// Retrieve metadata as a zip package.
///
/// For unpackaged retrieves, specify `types` with the metadata types and members.
/// For packaged retrieves, set `is_packaged` to true and provide `package_name`.
pub fn metadata_retrieve(request: &MetadataRetrieveRequest) -> Result<MetadataRetrieveResponse, Error> {
    call_host_fn(|input| unsafe { sf_metadata_retrieve(input) }, request)
}

/// Check the status of a metadata retrieve operation.
pub fn metadata_check_retrieve_status(
    async_process_id: &str,
    include_zip: bool,
) -> Result<MetadataRetrieveResult, Error> {
    let request = MetadataCheckRetrieveStatusRequest {
        async_process_id: async_process_id.to_string(),
        include_zip,
    };
    call_host_fn(
        |input| unsafe { sf_metadata_check_retrieve_status(input) },
        &request,
    )
}

/// List metadata components of a given type.
pub fn metadata_list(
    metadata_type: &str,
    folder: Option<String>,
) -> Result<Vec<MetadataComponentInfo>, Error> {
    let request = MetadataListRequest {
        metadata_type: metadata_type.to_string(),
        folder,
    };
    call_host_fn(|input| unsafe { sf_metadata_list(input) }, &request)
}

/// Describe available metadata types.
pub fn metadata_describe() -> Result<MetadataDescribeResult, Error> {
    call_host_fn_no_input(|input| unsafe { sf_metadata_describe(input) })
}

// =============================================================================
// Priority 2: Invocable Actions wrappers
// =============================================================================

pub fn list_standard_actions() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_list_standard_actions(input) })
}

pub fn list_custom_action_types() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_list_custom_action_types(input) })
}

pub fn list_custom_actions(action_type: &str) -> Result<serde_json::Value, Error> {
    let request = ListCustomActionsRequest {
        action_type: action_type.to_string(),
    };
    call_host_fn(|input| unsafe { sf_list_custom_actions(input) }, &request)
}

pub fn describe_standard_action(action_name: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: action_name.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_standard_action(input) }, &request)
}

pub fn describe_custom_action(action_type: &str, action_name: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeCustomActionRequest {
        action_type: action_type.to_string(),
        action_name: action_name.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_custom_action(input) }, &request)
}

pub fn invoke_standard_action(action_name: &str, inputs: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>, Error> {
    let request = InvokeActionRequest {
        action_name: action_name.to_string(),
        inputs,
    };
    call_host_fn(|input| unsafe { sf_invoke_standard_action(input) }, &request)
}

pub fn invoke_custom_action(action_type: &str, action_name: &str, inputs: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>, Error> {
    let request = InvokeCustomActionRequest {
        action_type: action_type.to_string(),
        action_name: action_name.to_string(),
        inputs,
    };
    call_host_fn(|input| unsafe { sf_invoke_custom_action(input) }, &request)
}

// =============================================================================
// Priority 2: Layouts wrappers
// =============================================================================

pub fn describe_layouts(sobject: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_layouts(input) }, &request)
}

pub fn describe_named_layout(sobject: &str, layout_name: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeNamedLayoutRequest {
        sobject: sobject.to_string(),
        layout_name: layout_name.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_named_layout(input) }, &request)
}

pub fn describe_approval_layouts(sobject: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_approval_layouts(input) }, &request)
}

pub fn describe_compact_layouts(sobject: &str) -> Result<serde_json::Value, Error> {
    let request = DescribeSObjectRequest {
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_describe_compact_layouts(input) }, &request)
}

pub fn describe_global_publisher_layouts() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_describe_global_publisher_layouts(input) })
}

// =============================================================================
// Priority 2: Knowledge wrappers
// =============================================================================

pub fn knowledge_settings() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_knowledge_settings(input) })
}

pub fn knowledge_articles(query: Option<String>, channel: Option<String>) -> Result<serde_json::Value, Error> {
    let request = KnowledgeArticlesRequest { query, channel };
    call_host_fn(|input| unsafe { sf_knowledge_articles(input) }, &request)
}

pub fn data_category_groups(sobject: Option<String>) -> Result<serde_json::Value, Error> {
    let request = DataCategoryGroupsRequest { sobject };
    call_host_fn(|input| unsafe { sf_data_category_groups(input) }, &request)
}

pub fn data_categories(group: &str, sobject: Option<String>) -> Result<serde_json::Value, Error> {
    let request = DataCategoriesRequest {
        group: group.to_string(),
        sobject,
    };
    call_host_fn(|input| unsafe { sf_data_categories(input) }, &request)
}

// =============================================================================
// Priority 2: Standalone wrappers
// =============================================================================

pub fn tabs() -> Result<Vec<serde_json::Value>, Error> {
    call_host_fn_no_input(|input| unsafe { sf_tabs(input) })
}

pub fn theme() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_theme(input) })
}

pub fn app_menu(app_menu_type: &str) -> Result<serde_json::Value, Error> {
    let request = AppMenuRequest {
        app_menu_type: app_menu_type.to_string(),
    };
    call_host_fn(|input| unsafe { sf_app_menu(input) }, &request)
}

pub fn recent_items() -> Result<Vec<serde_json::Value>, Error> {
    call_host_fn_no_input(|input| unsafe { sf_recent_items(input) })
}

pub fn relevant_items() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_relevant_items(input) })
}

pub fn compact_layouts_multi(sobject_list: &str) -> Result<serde_json::Value, Error> {
    let request = CompactLayoutsMultiRequest {
        sobject_list: sobject_list.to_string(),
    };
    call_host_fn(|input| unsafe { sf_compact_layouts_multi(input) }, &request)
}

pub fn platform_event_schema(event_name: &str) -> Result<serde_json::Value, Error> {
    let request = PlatformEventSchemaRequest {
        event_name: event_name.to_string(),
    };
    call_host_fn(|input| unsafe { sf_platform_event_schema(input) }, &request)
}

pub fn lightning_toggle_metrics() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_lightning_toggle_metrics(input) })
}

pub fn lightning_usage() -> Result<serde_json::Value, Error> {
    call_host_fn_no_input(|input| unsafe { sf_lightning_usage(input) })
}

// =============================================================================
// Priority 2: User Password wrappers
// =============================================================================

pub fn get_user_password_status(user_id: &str) -> Result<serde_json::Value, Error> {
    let request = GetRequest {
        id: user_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_user_password_status(input) }, &request)
}

pub fn set_user_password(user_id: &str, password: &str) -> Result<(), Error> {
    let request = SetUserPasswordRequest {
        user_id: user_id.to_string(),
        password: password.to_string(),
    };
    call_host_fn(|input| unsafe { sf_set_user_password(input) }, &request)
}

pub fn reset_user_password(user_id: &str) -> Result<serde_json::Value, Error> {
    let request = GetRequest {
        id: user_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_reset_user_password(input) }, &request)
}

// =============================================================================
// Priority 2: Scheduler wrappers
// =============================================================================

pub fn appointment_candidates(request: serde_json::Value) -> Result<serde_json::Value, Error> {
    call_host_fn(|input| unsafe { sf_appointment_candidates(input) }, &request)
}

pub fn appointment_slots(request: serde_json::Value) -> Result<serde_json::Value, Error> {
    call_host_fn(|input| unsafe { sf_appointment_slots(input) }, &request)
}

// =============================================================================
// Priority 2: Consent wrappers
// =============================================================================

pub fn read_consent(action: &str, ids: Vec<String>) -> Result<serde_json::Value, Error> {
    let request = ReadConsentRequest {
        action: action.to_string(),
        ids,
    };
    call_host_fn(|input| unsafe { sf_read_consent(input) }, &request)
}

pub fn write_consent(action: &str, records: Vec<ConsentWriteRecord>) -> Result<(), Error> {
    let request = WriteConsentRequest {
        action: action.to_string(),
        records,
    };
    call_host_fn(|input| unsafe { sf_write_consent(input) }, &request)
}

pub fn read_multi_consent(actions: Vec<String>, ids: Vec<String>) -> Result<serde_json::Value, Error> {
    let request = ReadMultiConsentRequest { actions, ids };
    call_host_fn(|input| unsafe { sf_read_multi_consent(input) }, &request)
}

// =============================================================================
// Priority 2: Binary wrappers
// =============================================================================

pub fn get_blob(sobject: &str, id: &str, field: &str) -> Result<GetBlobResponse, Error> {
    let request = GetBlobRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        field: field.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_blob(input) }, &request)
}

pub fn get_rich_text_image(sobject: &str, id: &str, field: &str, content_reference_id: &str) -> Result<GetRichTextImageResponse, Error> {
    let request = GetRichTextImageRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        field: field.to_string(),
        content_reference_id: content_reference_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_rich_text_image(input) }, &request)
}

pub fn get_relationship(sobject: &str, id: &str, relationship_name: &str) -> Result<serde_json::Value, Error> {
    let request = GetRelationshipRequest {
        sobject: sobject.to_string(),
        id: id.to_string(),
        relationship_name: relationship_name.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_relationship(input) }, &request)
}

// =============================================================================
// Priority 2: Embedded Service wrappers
// =============================================================================

pub fn get_embedded_service_config(config_id: &str) -> Result<serde_json::Value, Error> {
    let request = GetRequest {
        id: config_id.to_string(),
    };
    call_host_fn(|input| unsafe { sf_get_embedded_service_config(input) }, &request)
}

// =============================================================================
// Priority 2: Search Enhancements wrappers
// =============================================================================

pub fn parameterized_search(request: serde_json::Value) -> Result<serde_json::Value, Error> {
    call_host_fn(|input| unsafe { sf_parameterized_search(input) }, &request)
}

pub fn search_suggestions(query: &str, sobject: &str) -> Result<serde_json::Value, Error> {
    let request = SearchSuggestionsRequest {
        query: query.to_string(),
        sobject: sobject.to_string(),
    };
    call_host_fn(|input| unsafe { sf_search_suggestions(input) }, &request)
}

pub fn search_scope_order() -> Result<Vec<serde_json::Value>, Error> {
    call_host_fn_no_input(|input| unsafe { sf_search_scope_order(input) })
}

pub fn search_result_layouts(sobjects: Vec<String>) -> Result<Vec<serde_json::Value>, Error> {
    let request = SearchResultLayoutsRequest { sobjects };
    call_host_fn(|input| unsafe { sf_search_result_layouts(input) }, &request)
}

// =============================================================================
// Priority 2: Composite Enhancement wrappers
// =============================================================================

pub fn composite_graph(request: serde_json::Value) -> Result<serde_json::Value, Error> {
    call_host_fn(|input| unsafe { sf_composite_graph(input) }, &request)
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Call a host function with serialization/deserialization.
///
/// Uses MessagePack for the WASM boundary (faster and smaller than JSON).
/// The host side deserializes with the same format.
fn call_host_fn<Req, Resp>(
    host_fn: impl FnOnce(Vec<u8>) -> Result<Vec<u8>, Error>,
    request: &Req,
) -> Result<Resp, Error>
where
    Req: serde::Serialize,
    Resp: serde::de::DeserializeOwned,
{
    let input = rmp_serde::to_vec_named(request)
        .map_err(|e| Error::msg(format!("serialize error: {e}")))?;
    let output = host_fn(input)?;
    let result: BridgeResult<Resp> = rmp_serde::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}

/// Call a host function that takes no meaningful input.
fn call_host_fn_no_input<Resp>(
    host_fn: impl FnOnce(Vec<u8>) -> Result<Vec<u8>, Error>,
) -> Result<Resp, Error>
where
    Resp: serde::de::DeserializeOwned,
{
    let input = rmp_serde::to_vec_named(&())
        .map_err(|e| Error::msg(format!("serialize error: {e}")))?;
    let output = host_fn(input)?;
    let result: BridgeResult<Resp> = rmp_serde::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}
