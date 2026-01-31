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
// Internal helpers
// =============================================================================

/// Call a host function with serialization/deserialization.
fn call_host_fn<Req, Resp>(
    host_fn: impl FnOnce(Vec<u8>) -> Result<Vec<u8>, Error>,
    request: &Req,
) -> Result<Resp, Error>
where
    Req: serde::Serialize,
    Resp: serde::de::DeserializeOwned,
{
    let input = serde_json::to_vec(request)
        .map_err(|e| Error::msg(format!("serialize error: {e}")))?;
    let output = host_fn(input)?;
    let result: BridgeResult<Resp> = serde_json::from_slice(&output)
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
    let input = serde_json::to_vec(&serde_json::Value::Null)
        .map_err(|e| Error::msg(format!("serialize error: {e}")))?;
    let output = host_fn(input)?;
    let result: BridgeResult<Resp> = serde_json::from_slice(&output)
        .map_err(|e| Error::msg(format!("deserialize error: {e}")))?;
    result
        .into_result()
        .map_err(|e| Error::msg(e.to_string()))
}
