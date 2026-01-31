//! Host function implementations.
//!
//! These functions contain the business logic for each bridge operation.
//! They are pure async functions that take typed requests and return typed
//! responses. The Extism wiring (memory management, serialization at the
//! ABI boundary) is handled in `lib.rs`.
//!
//! ## Security
//!
//! - Credentials never cross the WASM boundary
//! - WASM guests are responsible for SOQL injection prevention in queries
//!   they construct (using sf-guest-sdk's security utilities or QueryBuilder)
//! - IDs, SObject names, and field names in structured requests are validated
//! - Errors are sanitized before returning to the guest

use base64::{engine::general_purpose, Engine as _};
use busbar_sf_bulk::BulkApiClient;
use busbar_sf_metadata::MetadataClient;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_tooling::ToolingClient;
use busbar_sf_wasm_types::*;

// =============================================================================
// Error Sanitization
// =============================================================================

/// Sanitize an error for safe return to WASM guests.
///
/// Maps internal error types to stable, non-leaking error codes.
/// The message is preserved as it typically contains user-actionable info,
/// but the code is sanitized to avoid exposing internal type names.
fn sanitize_rest_error(err: &busbar_sf_rest::Error) -> (String, String) {
    use busbar_sf_client::ErrorKind as ClientErrorKind;
    use busbar_sf_rest::ErrorKind as RestErrorKind;

    let code = match &err.kind {
        RestErrorKind::Client(_msg) => {
            // Check if the source is a client error with more specific kind
            if let Some(source) = &err.source {
                if let Some(client_err) = source.downcast_ref::<busbar_sf_client::Error>() {
                    match &client_err.kind {
                        ClientErrorKind::Http { status, .. } => format!("HTTP_{}", status),
                        ClientErrorKind::RateLimited { .. } => "RATE_LIMITED".to_string(),
                        ClientErrorKind::Authentication(_) => "AUTH_ERROR".to_string(),
                        ClientErrorKind::Authorization(_) => "AUTHORIZATION_ERROR".to_string(),
                        ClientErrorKind::NotFound(_) => "NOT_FOUND".to_string(),
                        ClientErrorKind::PreconditionFailed(_) => "PRECONDITION_FAILED".to_string(),
                        ClientErrorKind::Timeout => "TIMEOUT".to_string(),
                        ClientErrorKind::Connection(_) => "CONNECTION_ERROR".to_string(),
                        ClientErrorKind::Json(_) => "JSON_ERROR".to_string(),
                        ClientErrorKind::InvalidUrl(_) => "INVALID_URL".to_string(),
                        ClientErrorKind::Serialization(_) => "SERIALIZATION_ERROR".to_string(),
                        ClientErrorKind::Config(_) => "CONFIG_ERROR".to_string(),
                        ClientErrorKind::SalesforceApi { error_code, .. } => error_code.clone(),
                        ClientErrorKind::RetriesExhausted { .. } => "RETRIES_EXHAUSTED".to_string(),
                        ClientErrorKind::Other(_) => "CLIENT_ERROR".to_string(),
                    }
                } else {
                    "CLIENT_ERROR".to_string()
                }
            } else {
                "CLIENT_ERROR".to_string()
            }
        }
        RestErrorKind::Auth(_) => "AUTH_ERROR".to_string(),
        RestErrorKind::Salesforce { error_code, .. } => error_code.clone(),
        RestErrorKind::Other(_) => "OTHER_ERROR".to_string(),
    };

    (code, err.to_string())
}

/// Sanitize bulk API errors.
fn sanitize_bulk_error(err: &busbar_sf_bulk::Error) -> (String, String) {
    // Bulk errors typically wrap client/rest errors, so try to extract those
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    // Fallback to generic bulk error code
    ("BULK_ERROR".to_string(), err.to_string())
}

/// Sanitize tooling API errors.
fn sanitize_tooling_error(err: &busbar_sf_tooling::Error) -> (String, String) {
    // Tooling errors typically wrap client/rest errors
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    ("TOOLING_ERROR".to_string(), err.to_string())
}

/// Sanitize metadata API errors.
fn sanitize_metadata_error(err: &busbar_sf_metadata::Error) -> (String, String) {
    // Metadata errors wrap various error types
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    ("METADATA_ERROR".to_string(), err.to_string())
}

// =============================================================================
// REST API Handlers
// =============================================================================

/// Execute a SOQL query.
pub(crate) async fn handle_query(
    client: &SalesforceRestClient,
    request: QueryRequest,
) -> BridgeResult<QueryResponse> {
    let result = if request.include_deleted {
        client
            .query_all_including_deleted::<serde_json::Value>(&request.soql)
            .await
    } else {
        client.query::<serde_json::Value>(&request.soql).await
    };

    match result {
        Ok(qr) => BridgeResult::ok(QueryResponse {
            total_size: qr.total_size,
            done: qr.done,
            records: qr.records,
            next_records_url: qr.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Fetch the next page of query results.
pub(crate) async fn handle_query_more(
    client: &SalesforceRestClient,
    request: QueryMoreRequest,
) -> BridgeResult<QueryResponse> {
    match client
        .query_more::<serde_json::Value>(&request.next_records_url)
        .await
    {
        Ok(qr) => BridgeResult::ok(QueryResponse {
            total_size: qr.total_size,
            done: qr.done,
            records: qr.records,
            next_records_url: qr.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Create a new record.
pub(crate) async fn handle_create(
    client: &SalesforceRestClient,
    request: CreateRequest,
) -> BridgeResult<CreateResponse> {
    match client.create(&request.sobject, &request.record).await {
        Ok(id) => BridgeResult::ok(CreateResponse {
            id,
            success: true,
            errors: vec![],
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get a record by ID.
pub(crate) async fn handle_get(
    client: &SalesforceRestClient,
    request: GetRequest,
) -> BridgeResult<serde_json::Value> {
    let fields: Option<Vec<&str>> = request
        .fields
        .as_ref()
        .map(|f| f.iter().map(|s| s.as_str()).collect());

    let result: Result<serde_json::Value, _> = client
        .get(&request.sobject, &request.id, fields.as_deref())
        .await;

    match result {
        Ok(record) => BridgeResult::ok(record),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Update a record.
pub(crate) async fn handle_update(
    client: &SalesforceRestClient,
    request: UpdateRequest,
) -> BridgeResult<()> {
    match client
        .update(&request.sobject, &request.id, &request.record)
        .await
    {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Delete a record.
pub(crate) async fn handle_delete(
    client: &SalesforceRestClient,
    request: DeleteRequest,
) -> BridgeResult<()> {
    match client.delete(&request.sobject, &request.id).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Upsert a record using an external ID.
pub(crate) async fn handle_upsert(
    client: &SalesforceRestClient,
    request: UpsertRequest,
) -> BridgeResult<UpsertResponse> {
    match client
        .upsert(
            &request.sobject,
            &request.external_id_field,
            &request.external_id_value,
            &request.record,
        )
        .await
    {
        Ok(result) => BridgeResult::ok(UpsertResponse {
            id: result.id,
            success: result.success,
            created: result.created,
            errors: result
                .errors
                .into_iter()
                .map(|e| SalesforceApiError {
                    status_code: e.status_code,
                    message: e.message,
                    fields: e.fields,
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe all SObjects.
pub(crate) async fn handle_describe_global(
    client: &SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match client.describe_global().await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(v) => BridgeResult::ok(v),
            Err(e) => BridgeResult::err("SERIALIZATION_ERROR", e.to_string()),
        },
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe a specific SObject.
pub(crate) async fn handle_describe_sobject(
    client: &SalesforceRestClient,
    request: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match client.describe_sobject(&request.sobject).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(v) => BridgeResult::ok(v),
            Err(e) => BridgeResult::err("SERIALIZATION_ERROR", e.to_string()),
        },
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a SOSL search.
pub(crate) async fn handle_search(
    client: &SalesforceRestClient,
    request: SearchRequest,
) -> BridgeResult<SearchResponse> {
    match client.search::<serde_json::Value>(&request.sosl).await {
        Ok(result) => BridgeResult::ok(SearchResponse {
            search_records: result.search_records,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a composite API request.
pub(crate) async fn handle_composite(
    client: &SalesforceRestClient,
    request: CompositeRequest,
) -> BridgeResult<CompositeResponse> {
    let sf_request = busbar_sf_rest::CompositeRequest {
        all_or_none: request.all_or_none,
        collate_subrequests: false,
        subrequests: request
            .subrequests
            .into_iter()
            .map(|s| busbar_sf_rest::CompositeSubrequest {
                method: s.method,
                url: s.url,
                reference_id: s.reference_id,
                body: s.body,
            })
            .collect(),
    };

    match client.composite(&sf_request).await {
        Ok(result) => BridgeResult::ok(CompositeResponse {
            responses: result
                .responses
                .into_iter()
                .map(|r| CompositeSubresponse {
                    body: r.body,
                    http_status_code: r.http_status_code,
                    reference_id: r.reference_id,
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a composite batch API request.
pub(crate) async fn handle_composite_batch(
    client: &SalesforceRestClient,
    request: CompositeBatchRequest,
) -> BridgeResult<CompositeBatchResponse> {
    let sf_request = busbar_sf_rest::CompositeBatchRequest {
        halt_on_error: request.halt_on_error,
        batch_requests: request
            .subrequests
            .into_iter()
            .map(|s| busbar_sf_rest::CompositeBatchSubrequest {
                method: s.method,
                url: s.url,
                rich_input: s.rich_input,
                binary_part_name: None,
                binary_part_name_alias: None,
            })
            .collect(),
    };

    match client.composite_batch(&sf_request).await {
        Ok(result) => BridgeResult::ok(CompositeBatchResponse {
            has_errors: result.has_errors,
            results: result
                .results
                .into_iter()
                .map(|r| CompositeBatchSubresponse {
                    status_code: r.status_code,
                    result: r.result,
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a composite tree API request.
pub(crate) async fn handle_composite_tree(
    client: &SalesforceRestClient,
    request: CompositeTreeRequest,
) -> BridgeResult<CompositeTreeResponse> {
    let records: Vec<busbar_sf_rest::CompositeTreeRecord> = match request
        .records
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(r) => r,
        Err(e) => {
            return BridgeResult::err("INVALID_REQUEST", format!("invalid tree records: {e}"))
        }
    };
    let sf_request = busbar_sf_rest::CompositeTreeRequest { records };

    match client.composite_tree(&request.sobject, &sf_request).await {
        Ok(result) => BridgeResult::ok(CompositeTreeResponse {
            has_errors: result.has_errors,
            results: result
                .results
                .into_iter()
                .map(|r| CompositeTreeResult {
                    reference_id: r.reference_id,
                    id: r.id,
                    errors: r
                        .errors
                        .into_iter()
                        .map(|e| SalesforceApiError {
                            status_code: e.status_code,
                            message: e.message,
                            fields: e.fields,
                        })
                        .collect(),
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Create multiple records.
pub(crate) async fn handle_create_multiple(
    client: &SalesforceRestClient,
    request: CreateMultipleRequest,
) -> BridgeResult<Vec<CollectionResult>> {
    match client
        .create_multiple(&request.sobject, &request.records, request.all_or_none)
        .await
    {
        Ok(results) => BridgeResult::ok(collection_results_to_bridge(results)),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Update multiple records.
pub(crate) async fn handle_update_multiple(
    client: &SalesforceRestClient,
    request: UpdateMultipleRequest,
) -> BridgeResult<Vec<CollectionResult>> {
    let records: Vec<(String, serde_json::Value)> = request
        .records
        .into_iter()
        .map(|r| (r.id, r.fields))
        .collect();
    match client
        .update_multiple(&request.sobject, &records, request.all_or_none)
        .await
    {
        Ok(results) => BridgeResult::ok(collection_results_to_bridge(results)),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get multiple records by ID.
pub(crate) async fn handle_get_multiple(
    client: &SalesforceRestClient,
    request: GetMultipleRequest,
) -> BridgeResult<Vec<serde_json::Value>> {
    let ids: Vec<&str> = request.ids.iter().map(|s| s.as_str()).collect();
    let fields: Vec<&str> = request.fields.iter().map(|s| s.as_str()).collect();
    match client
        .get_multiple::<serde_json::Value>(&request.sobject, &ids, &fields)
        .await
    {
        Ok(results) => BridgeResult::ok(results),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Delete multiple records.
pub(crate) async fn handle_delete_multiple(
    client: &SalesforceRestClient,
    request: DeleteMultipleRequest,
) -> BridgeResult<Vec<CollectionResult>> {
    let ids: Vec<&str> = request.ids.iter().map(|s| s.as_str()).collect();
    match client.delete_multiple(&ids, request.all_or_none).await {
        Ok(results) => BridgeResult::ok(collection_results_to_bridge(results)),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get API limits.
pub(crate) async fn handle_limits(
    client: &SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match client.limits().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get API versions.
pub(crate) async fn handle_versions(
    client: &SalesforceRestClient,
) -> BridgeResult<Vec<ApiVersion>> {
    match client.versions().await {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|v| ApiVersion {
                    label: v.label,
                    url: v.url,
                    version: v.version,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

// =============================================================================
// Bulk API Handlers
// =============================================================================

/// Create a bulk ingest job.
pub(crate) async fn handle_bulk_create_ingest_job(
    client: &BulkApiClient,
    request: BulkCreateIngestJobRequest,
) -> BridgeResult<BulkJobResponse> {
    let operation = match parse_bulk_operation(&request.operation) {
        Ok(op) => op,
        Err(msg) => return BridgeResult::err("INVALID_REQUEST", msg),
    };
    let column_delimiter = match parse_column_delimiter(&request.column_delimiter) {
        Ok(d) => d,
        Err(msg) => return BridgeResult::err("INVALID_REQUEST", msg),
    };
    let line_ending = match parse_line_ending(&request.line_ending) {
        Ok(l) => l,
        Err(msg) => return BridgeResult::err("INVALID_REQUEST", msg),
    };

    let sf_request = busbar_sf_bulk::CreateIngestJobRequest {
        object: request.sobject,
        operation,
        external_id_field_name: request.external_id_field,
        content_type: busbar_sf_bulk::ContentType::default(),
        column_delimiter,
        line_ending,
    };

    match client.create_ingest_job(sf_request).await {
        Ok(job) => BridgeResult::ok(ingest_job_to_bridge(job)),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Upload CSV data to a bulk ingest job.
pub(crate) async fn handle_bulk_upload_job_data(
    client: &BulkApiClient,
    request: BulkUploadJobDataRequest,
) -> BridgeResult<()> {
    match client
        .upload_job_data(&request.job_id, &request.csv_data)
        .await
    {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Close a bulk ingest job (marks it ready for processing).
pub(crate) async fn handle_bulk_close_ingest_job(
    client: &BulkApiClient,
    request: BulkJobIdRequest,
) -> BridgeResult<BulkJobResponse> {
    match client.close_ingest_job(&request.job_id).await {
        Ok(job) => BridgeResult::ok(ingest_job_to_bridge(job)),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Abort a bulk ingest job.
pub(crate) async fn handle_bulk_abort_ingest_job(
    client: &BulkApiClient,
    request: BulkJobIdRequest,
) -> BridgeResult<BulkJobResponse> {
    match client.abort_ingest_job(&request.job_id).await {
        Ok(job) => BridgeResult::ok(ingest_job_to_bridge(job)),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get the status of a bulk ingest job.
pub(crate) async fn handle_bulk_get_ingest_job(
    client: &BulkApiClient,
    request: BulkJobIdRequest,
) -> BridgeResult<BulkJobResponse> {
    match client.get_ingest_job(&request.job_id).await {
        Ok(job) => BridgeResult::ok(ingest_job_to_bridge(job)),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get job results (successful, failed, or unprocessed records).
pub(crate) async fn handle_bulk_get_job_results(
    client: &BulkApiClient,
    request: BulkJobResultsRequest,
) -> BridgeResult<BulkJobResultsResponse> {
    let result = match request.result_type.as_str() {
        "successful" => client.get_successful_results(&request.job_id).await,
        "failed" => client.get_failed_results(&request.job_id).await,
        "unprocessed" => client.get_unprocessed_records(&request.job_id).await,
        other => {
            return BridgeResult::err(
                "INVALID_REQUEST",
                format!("invalid result_type: {other} (expected: successful, failed, unprocessed)"),
            )
        }
    };

    match result {
        Ok(csv_data) => BridgeResult::ok(BulkJobResultsResponse { csv_data }),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Delete a bulk ingest job.
pub(crate) async fn handle_bulk_delete_ingest_job(
    client: &BulkApiClient,
    request: BulkJobIdRequest,
) -> BridgeResult<()> {
    match client.delete_ingest_job(&request.job_id).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List all ingest jobs.
pub(crate) async fn handle_bulk_get_all_ingest_jobs(
    client: &BulkApiClient,
) -> BridgeResult<BulkJobListResponse> {
    match client.get_all_ingest_jobs().await {
        Ok(list) => BridgeResult::ok(BulkJobListResponse {
            records: list.records.into_iter().map(ingest_job_to_bridge).collect(),
            done: list.done,
            next_records_url: list.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Abort a bulk query job.
pub(crate) async fn handle_bulk_abort_query_job(
    client: &BulkApiClient,
    request: BulkJobIdRequest,
) -> BridgeResult<BulkJobResponse> {
    match client.abort_query_job(&request.job_id).await {
        Ok(job) => BridgeResult::ok(query_job_to_bridge(job)),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get query job results.
pub(crate) async fn handle_bulk_get_query_results(
    client: &BulkApiClient,
    request: BulkQueryResultsRequest,
) -> BridgeResult<BulkQueryResultsResponse> {
    match client
        .get_query_results(
            &request.job_id,
            request.locator.as_deref(),
            request.max_records.map(|n| n as usize),
        )
        .await
    {
        Ok(results) => BridgeResult::ok(BulkQueryResultsResponse {
            csv_data: results.csv_data,
            locator: results.locator,
        }),
        Err(e) => {
            let (code, message) = sanitize_bulk_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

// =============================================================================
// Tooling API Handlers
// =============================================================================

/// Execute a Tooling API SOQL query.
pub(crate) async fn handle_tooling_query(
    client: &ToolingClient,
    request: ToolingQueryRequest,
) -> BridgeResult<QueryResponse> {
    match client.query::<serde_json::Value>(&request.soql).await {
        Ok(qr) => BridgeResult::ok(QueryResponse {
            total_size: qr.total_size,
            done: qr.done,
            records: qr.records,
            next_records_url: qr.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_tooling_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute anonymous Apex code.
pub(crate) async fn handle_tooling_execute_anonymous(
    client: &ToolingClient,
    request: ExecuteAnonymousRequest,
) -> BridgeResult<ExecuteAnonymousResponse> {
    match client.execute_anonymous(&request.apex_code).await {
        Ok(result) => BridgeResult::ok(ExecuteAnonymousResponse {
            compiled: result.compiled,
            success: result.success,
            compile_problem: result.compile_problem,
            exception_message: result.exception_message,
            exception_stack_trace: result.exception_stack_trace,
            line: result.line,
            column: result.column,
        }),
        Err(e) => {
            let (code, message) = sanitize_tooling_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get a Tooling API record.
pub(crate) async fn handle_tooling_get(
    client: &ToolingClient,
    request: ToolingGetRequest,
) -> BridgeResult<serde_json::Value> {
    match client
        .get::<serde_json::Value>(&request.sobject, &request.id)
        .await
    {
        Ok(record) => BridgeResult::ok(record),
        Err(e) => {
            let (code, message) = sanitize_tooling_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Create a Tooling API record.
pub(crate) async fn handle_tooling_create(
    client: &ToolingClient,
    request: ToolingCreateRequest,
) -> BridgeResult<CreateResponse> {
    match client.create(&request.sobject, &request.record).await {
        Ok(id) => BridgeResult::ok(CreateResponse {
            id,
            success: true,
            errors: vec![],
        }),
        Err(e) => {
            let (code, message) = sanitize_tooling_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Delete a Tooling API record.
pub(crate) async fn handle_tooling_delete(
    client: &ToolingClient,
    request: ToolingDeleteRequest,
) -> BridgeResult<()> {
    match client.delete(&request.sobject, &request.id).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_tooling_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

// =============================================================================
// Metadata API Handlers
// =============================================================================

/// Deploy a metadata package.
pub(crate) async fn handle_metadata_deploy(
    client: &MetadataClient,
    request: MetadataDeployRequest,
) -> BridgeResult<MetadataDeployResponse> {
    let zip_bytes = match general_purpose::STANDARD.decode(&request.zip_base64) {
        Ok(b) => b,
        Err(e) => return BridgeResult::err("INVALID_REQUEST", format!("invalid base64: {e}")),
    };

    let test_level = match &request.options.test_level {
        Some(tl) => match parse_test_level(tl) {
            Ok(level) => Some(level),
            Err(msg) => return BridgeResult::err("INVALID_REQUEST", msg),
        },
        None => None,
    };

    let options = busbar_sf_metadata::DeployOptions {
        check_only: request.options.check_only,
        rollback_on_error: request.options.rollback_on_error,
        test_level,
        run_tests: request.options.run_tests,
        ..Default::default()
    };

    match client.deploy(&zip_bytes, options).await {
        Ok(async_process_id) => BridgeResult::ok(MetadataDeployResponse { async_process_id }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Check the status of a metadata deployment.
pub(crate) async fn handle_metadata_check_deploy_status(
    client: &MetadataClient,
    request: MetadataCheckDeployStatusRequest,
) -> BridgeResult<MetadataDeployResult> {
    match client
        .check_deploy_status(&request.async_process_id, request.include_details)
        .await
    {
        Ok(result) => BridgeResult::ok(MetadataDeployResult {
            id: result.id,
            done: result.done,
            status: format!("{:?}", result.status),
            success: result.success,
            error_message: result.error_message,
            number_component_errors: result.number_components_errors as i32,
            number_components_deployed: result.number_components_deployed as i32,
            number_components_total: result.number_components_total as i32,
            number_test_errors: result.number_tests_errors as i32,
            number_tests_completed: result.number_tests_completed as i32,
            number_tests_total: result.number_tests_total as i32,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Retrieve metadata as a zip package.
pub(crate) async fn handle_metadata_retrieve(
    client: &MetadataClient,
    request: MetadataRetrieveRequest,
) -> BridgeResult<MetadataRetrieveResponse> {
    let result = if request.is_packaged {
        let package_name = match &request.package_name {
            Some(name) => name.as_str(),
            None => {
                return BridgeResult::err(
                    "INVALID_REQUEST",
                    "package_name is required when is_packaged is true",
                )
            }
        };
        client.retrieve_packaged(package_name).await
    } else {
        let mut manifest = busbar_sf_metadata::PackageManifest::new(request.api_version.clone());
        for t in &request.types {
            manifest = manifest.add_type(t.name.clone(), t.members.clone());
        }
        client.retrieve_unpackaged(&manifest).await
    };

    match result {
        Ok(async_process_id) => BridgeResult::ok(MetadataRetrieveResponse { async_process_id }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Check the status of a metadata retrieve operation.
pub(crate) async fn handle_metadata_check_retrieve_status(
    client: &MetadataClient,
    request: MetadataCheckRetrieveStatusRequest,
) -> BridgeResult<MetadataRetrieveResult> {
    match client
        .check_retrieve_status(&request.async_process_id, request.include_zip)
        .await
    {
        Ok(result) => BridgeResult::ok(MetadataRetrieveResult {
            id: result.id,
            done: result.done,
            status: format!("{:?}", result.status),
            success: result.success,
            zip_base64: result.zip_file,
            error_message: result.error_message,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List metadata components of a given type.
pub(crate) async fn handle_metadata_list(
    client: &MetadataClient,
    request: MetadataListRequest,
) -> BridgeResult<Vec<MetadataComponentInfo>> {
    match client
        .list_metadata(&request.metadata_type, request.folder.as_deref())
        .await
    {
        Ok(components) => BridgeResult::ok(
            components
                .into_iter()
                .map(|c| MetadataComponentInfo {
                    full_name: c.full_name,
                    file_name: c.file_name.unwrap_or_default(),
                    component_type: c.metadata_type,
                    id: c.id.unwrap_or_default(),
                    namespace_prefix: c.namespace_prefix,
                    last_modified_date: c.last_modified_date,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe available metadata types.
pub(crate) async fn handle_metadata_describe(
    client: &MetadataClient,
) -> BridgeResult<MetadataDescribeResult> {
    match client.describe_metadata().await {
        Ok(result) => BridgeResult::ok(MetadataDescribeResult {
            metadata_objects: result
                .metadata_objects
                .into_iter()
                .map(|m| MetadataTypeInfo {
                    xml_name: m.xml_name,
                    directory_name: m.directory_name.unwrap_or_default(),
                    suffix: m.suffix,
                    in_folder: m.in_folder,
                    meta_file: m.meta_file,
                    child_xml_names: m.child_xml_names,
                })
                .collect(),
            organization_namespace: result.organization_namespace.unwrap_or_default(),
            partial_save_allowed: result.partial_save_allowed,
            test_required: result.test_required,
        }),
        Err(e) => {
            let (code, message) = sanitize_metadata_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

// =============================================================================
// Internal Helpers
// =============================================================================

fn collection_results_to_bridge(
    results: Vec<busbar_sf_rest::CollectionResult>,
) -> Vec<CollectionResult> {
    results
        .into_iter()
        .map(|r| CollectionResult {
            id: r.id,
            success: r.success,
            errors: r
                .errors
                .into_iter()
                .map(|e| SalesforceApiError {
                    status_code: e.status_code,
                    message: e.message,
                    fields: e.fields,
                })
                .collect(),
            created: r.created,
        })
        .collect()
}

fn ingest_job_to_bridge(job: busbar_sf_bulk::IngestJob) -> BulkJobResponse {
    BulkJobResponse {
        id: job.id,
        state: format!("{:?}", job.state),
        object: job.object,
        operation: job.operation,
        number_records_processed: job.number_records_processed,
        number_records_failed: job.number_records_failed,
        created_date: job.created_date,
        system_modstamp: job.system_modstamp,
        error_message: job.error_message,
    }
}

fn query_job_to_bridge(job: busbar_sf_bulk::QueryJob) -> BulkJobResponse {
    BulkJobResponse {
        id: job.id,
        state: format!("{:?}", job.state),
        object: String::new(),
        operation: job.operation,
        number_records_processed: job.number_records_processed,
        number_records_failed: 0,
        created_date: job.created_date,
        system_modstamp: job.system_modstamp,
        error_message: job.error_message,
    }
}

fn parse_bulk_operation(s: &str) -> Result<busbar_sf_bulk::BulkOperation, String> {
    match s.to_lowercase().as_str() {
        "insert" => Ok(busbar_sf_bulk::BulkOperation::Insert),
        "update" => Ok(busbar_sf_bulk::BulkOperation::Update),
        "upsert" => Ok(busbar_sf_bulk::BulkOperation::Upsert),
        "delete" => Ok(busbar_sf_bulk::BulkOperation::Delete),
        "harddelete" => Ok(busbar_sf_bulk::BulkOperation::HardDelete),
        _ => Err(format!("invalid bulk operation: {s}")),
    }
}

fn parse_column_delimiter(s: &str) -> Result<busbar_sf_bulk::ColumnDelimiter, String> {
    match s {
        "COMMA" => Ok(busbar_sf_bulk::ColumnDelimiter::Comma),
        "TAB" => Ok(busbar_sf_bulk::ColumnDelimiter::Tab),
        "SEMICOLON" => Ok(busbar_sf_bulk::ColumnDelimiter::Semicolon),
        "PIPE" => Ok(busbar_sf_bulk::ColumnDelimiter::Pipe),
        "BACKQUOTE" => Ok(busbar_sf_bulk::ColumnDelimiter::Backquote),
        "CARET" => Ok(busbar_sf_bulk::ColumnDelimiter::Caret),
        _ => Err(format!("invalid column delimiter: {s}")),
    }
}

fn parse_line_ending(s: &str) -> Result<busbar_sf_bulk::LineEnding, String> {
    match s {
        "LF" => Ok(busbar_sf_bulk::LineEnding::Lf),
        "CRLF" => Ok(busbar_sf_bulk::LineEnding::Crlf),
        _ => Err(format!("invalid line ending: {s}")),
    }
}

fn parse_test_level(s: &str) -> Result<busbar_sf_metadata::TestLevel, String> {
    match s {
        "NoTestRun" => Ok(busbar_sf_metadata::TestLevel::NoTestRun),
        "RunLocalTests" => Ok(busbar_sf_metadata::TestLevel::RunLocalTests),
        "RunAllTestsInOrg" => Ok(busbar_sf_metadata::TestLevel::RunAllTestsInOrg),
        "RunSpecifiedTests" => Ok(busbar_sf_metadata::TestLevel::RunSpecifiedTests),
        _ => Err(format!("invalid test level: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_rest_client() -> (MockServer, SalesforceRestClient) {
        let server = MockServer::start().await;
        let client = SalesforceRestClient::new(server.uri(), "test_token").unwrap();
        (server, client)
    }

    async fn mock_bulk_client() -> (MockServer, BulkApiClient) {
        let server = MockServer::start().await;
        let client = BulkApiClient::new(server.uri(), "test_token").unwrap();
        (server, client)
    }

    async fn mock_tooling_client() -> (MockServer, ToolingClient) {
        let server = MockServer::start().await;
        let client = ToolingClient::new(server.uri(), "test_token").unwrap();
        (server, client)
    }

    // =========================================================================
    // Helper function tests
    // =========================================================================

    #[test]
    fn test_parse_bulk_operation_valid() {
        assert!(parse_bulk_operation("insert").is_ok());
        assert!(parse_bulk_operation("INSERT").is_ok());
        assert!(parse_bulk_operation("update").is_ok());
        assert!(parse_bulk_operation("upsert").is_ok());
        assert!(parse_bulk_operation("delete").is_ok());
        assert!(parse_bulk_operation("harddelete").is_ok());
        assert!(parse_bulk_operation("HardDelete").is_ok());
    }

    #[test]
    fn test_parse_bulk_operation_invalid() {
        assert!(parse_bulk_operation("invalid").is_err());
        assert!(parse_bulk_operation("").is_err());
    }

    #[test]
    fn test_parse_column_delimiter_valid() {
        assert!(parse_column_delimiter("COMMA").is_ok());
        assert!(parse_column_delimiter("TAB").is_ok());
        assert!(parse_column_delimiter("SEMICOLON").is_ok());
        assert!(parse_column_delimiter("PIPE").is_ok());
        assert!(parse_column_delimiter("BACKQUOTE").is_ok());
        assert!(parse_column_delimiter("CARET").is_ok());
    }

    #[test]
    fn test_parse_column_delimiter_invalid() {
        assert!(parse_column_delimiter("INVALID").is_err());
    }

    #[test]
    fn test_parse_line_ending_valid() {
        assert!(parse_line_ending("LF").is_ok());
        assert!(parse_line_ending("CRLF").is_ok());
    }

    #[test]
    fn test_parse_line_ending_invalid() {
        assert!(parse_line_ending("CR").is_err());
    }

    #[test]
    fn test_parse_test_level_valid() {
        assert!(parse_test_level("NoTestRun").is_ok());
        assert!(parse_test_level("RunLocalTests").is_ok());
        assert!(parse_test_level("RunAllTestsInOrg").is_ok());
        assert!(parse_test_level("RunSpecifiedTests").is_ok());
    }

    #[test]
    fn test_parse_test_level_invalid() {
        assert!(parse_test_level("BadLevel").is_err());
    }

    #[test]
    fn test_collection_results_to_bridge() {
        let results = vec![busbar_sf_rest::CollectionResult {
            id: Some("001xx".to_string()),
            success: true,
            errors: vec![],
            created: Some(true),
        }];
        let bridge = collection_results_to_bridge(results);
        assert_eq!(bridge.len(), 1);
        assert!(bridge[0].success);
        assert_eq!(bridge[0].id, Some("001xx".to_string()));
    }

    #[test]
    fn test_ingest_job_to_bridge() {
        let job = busbar_sf_bulk::IngestJob {
            id: "750xx".to_string(),
            state: busbar_sf_bulk::JobState::Open,
            object: "Account".to_string(),
            operation: "insert".to_string(),
            number_records_processed: 0,
            number_records_failed: 0,
            created_date: None,
            system_modstamp: None,
            total_processing_time: None,
            api_version: None,
            concurrency_mode: None,
            error_message: None,
        };
        let bridge = ingest_job_to_bridge(job);
        assert_eq!(bridge.id, "750xx");
        assert_eq!(bridge.object, "Account");
    }

    #[test]
    fn test_query_job_to_bridge() {
        let job = busbar_sf_bulk::QueryJob {
            id: "750xx".to_string(),
            state: busbar_sf_bulk::JobState::JobComplete,
            query: Some("SELECT Id FROM Account".to_string()),
            operation: "query".to_string(),
            number_records_processed: 100,
            created_date: None,
            system_modstamp: None,
            total_processing_time: None,
            error_message: None,
        };
        let bridge = query_job_to_bridge(job);
        assert_eq!(bridge.id, "750xx");
        assert_eq!(bridge.number_records_processed, 100);
    }

    // =========================================================================
    // REST API Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_handle_query_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalSize": 1, "done": true, "records": [{"Id": "001xx"}]
            })))
            .mount(&server)
            .await;

        let req = QueryRequest {
            soql: "SELECT Id FROM Account".to_string(),
            include_deleted: false,
        };
        let result = handle_query(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_query_include_deleted() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/queryAll"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalSize": 0, "done": true, "records": []
            })))
            .mount(&server)
            .await;

        let req = QueryRequest {
            soql: "SELECT Id FROM Account".to_string(),
            include_deleted: true,
        };
        let result = handle_query(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_query_error() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/query"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!([{
                "errorCode": "MALFORMED_QUERY", "message": "bad query"
            }])))
            .mount(&server)
            .await;

        let req = QueryRequest {
            soql: "BAD SOQL".to_string(),
            include_deleted: false,
        };
        let result = handle_query(&client, req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_query_more_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/query/01gxx"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalSize": 100, "done": true, "records": [{"Id": "001xx"}]
            })))
            .mount(&server)
            .await;

        let req = QueryMoreRequest {
            next_records_url: "/services/data/v62.0/query/01gxx-2000".to_string(),
        };
        let result = handle_query_more(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_create_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/sobjects/Account"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": "001xx000003DgAAAS", "success": true, "errors": []
            })))
            .mount(&server)
            .await;

        let req = CreateRequest {
            sobject: "Account".to_string(),
            record: json!({"Name": "Test"}),
        };
        let result = handle_create(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_get_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex(
                "/services/data/.*/sobjects/Account/001xx000003DGbY",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"Id": "001xx000003DGbY", "Name": "Acme"})),
            )
            .mount(&server)
            .await;

        let req = GetRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DGbY".to_string(),
            fields: None,
        };
        let result = handle_get(&client, req).await;
        assert!(result.is_ok(), "handle_get failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_update_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("PATCH"))
            .and(path_regex(
                "/services/data/.*/sobjects/Account/001xx000003DGbY",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let req = UpdateRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DGbY".to_string(),
            record: json!({"Name": "Updated"}),
        };
        let result = handle_update(&client, req).await;
        assert!(result.is_ok(), "handle_update failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_delete_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("DELETE"))
            .and(path_regex(
                "/services/data/.*/sobjects/Account/001xx000003DGbY",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let req = DeleteRequest {
            sobject: "Account".to_string(),
            id: "001xx000003DGbY".to_string(),
        };
        let result = handle_delete(&client, req).await;
        assert!(result.is_ok(), "handle_delete failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_upsert_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("PATCH"))
            .and(path_regex(
                "/services/data/.*/sobjects/Account/External_Id__c/EXT001",
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": "001xx000003DGbY", "success": true, "created": true, "errors": []
            })))
            .mount(&server)
            .await;

        let req = UpsertRequest {
            sobject: "Account".to_string(),
            external_id_field: "External_Id__c".to_string(),
            external_id_value: "EXT001".to_string(),
            record: json!({"Name": "Upserted"}),
        };
        let result = handle_upsert(&client, req).await;
        assert!(result.is_ok(), "handle_upsert failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_describe_global_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/sobjects$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "encoding": "UTF-8",
                "maxBatchSize": 200,
                "sobjects": [{
                    "name": "Account",
                    "label": "Account",
                    "labelPlural": "Accounts",
                    "custom": false,
                    "queryable": true,
                    "createable": true,
                    "updateable": true,
                    "deletable": true,
                    "searchable": true,
                    "retrieveable": true
                }]
            })))
            .mount(&server)
            .await;

        let result = handle_describe_global(&client).await;
        assert!(result.is_ok(), "describe_global failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_describe_sobject_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/sobjects/Account/describe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "Account",
                "label": "Account",
                "custom": false,
                "fields": []
            })))
            .mount(&server)
            .await;

        let req = DescribeSObjectRequest {
            sobject: "Account".to_string(),
        };
        let result = handle_describe_sobject(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_search_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "searchRecords": [{"Id": "001xx", "attributes": {"type": "Account"}}]
            })))
            .mount(&server)
            .await;

        let req = SearchRequest {
            sosl: "FIND {Acme} IN ALL FIELDS".to_string(),
        };
        let result = handle_search(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_composite_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/composite$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "compositeResponse": [{
                    "body": {"id": "001xx"},
                    "httpHeaders": {},
                    "httpStatusCode": 201,
                    "referenceId": "ref1"
                }]
            })))
            .mount(&server)
            .await;

        let req = CompositeRequest {
            all_or_none: false,
            subrequests: vec![CompositeSubrequest {
                method: "POST".to_string(),
                url: "/services/data/v62.0/sobjects/Account".to_string(),
                reference_id: "ref1".to_string(),
                body: Some(json!({"Name": "Test"})),
            }],
        };
        let result = handle_composite(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_composite_batch_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/composite/batch"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hasErrors": false,
                "results": [{"statusCode": 200, "result": {"Id": "001xx"}}]
            })))
            .mount(&server)
            .await;

        let req = CompositeBatchRequest {
            halt_on_error: false,
            subrequests: vec![CompositeBatchSubrequest {
                method: "GET".to_string(),
                url: "/services/data/v62.0/sobjects/Account/001xx".to_string(),
                rich_input: None,
            }],
        };
        let result = handle_composite_batch(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_composite_tree_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/composite/tree/Account"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "hasErrors": false,
                "results": [{"referenceId": "ref1", "id": "001xx", "errors": []}]
            })))
            .mount(&server)
            .await;

        let req = CompositeTreeRequest {
            sobject: "Account".to_string(),
            records: vec![json!({
                "attributes": {"type": "Account"},
                "referenceId": "ref1",
                "Name": "Test"
            })],
        };
        let result = handle_composite_tree(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_composite_tree_invalid_records() {
        let (server, client) = mock_rest_client().await;
        // No mock needed since we fail before making a request
        let _ = &server;

        let req = CompositeTreeRequest {
            sobject: "Account".to_string(),
            records: vec![json!("not_an_object")],
        };
        let result = handle_composite_tree(&client, req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_create_multiple_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/composite/sobjects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": "001xx1", "success": true, "errors": []},
                {"id": "001xx2", "success": true, "errors": []}
            ])))
            .mount(&server)
            .await;

        let req = CreateMultipleRequest {
            sobject: "Account".to_string(),
            records: vec![json!({"Name": "A1"}), json!({"Name": "A2"})],
            all_or_none: false,
        };
        let result = handle_create_multiple(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_delete_multiple_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("DELETE"))
            .and(path_regex("/services/data/.*/composite/sobjects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": "001xx000003DGbY", "success": true, "errors": []}
            ])))
            .mount(&server)
            .await;

        let req = DeleteMultipleRequest {
            ids: vec!["001xx000003DGbY".to_string()],
            all_or_none: false,
        };
        let result = handle_delete_multiple(&client, req).await;
        assert!(
            result.is_ok(),
            "handle_delete_multiple failed: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_handle_limits_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/limits"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "DailyApiRequests": {"Max": 15000, "Remaining": 14500}
            })))
            .mount(&server)
            .await;

        let result = handle_limits(&client).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_versions_success() {
        let (server, client) = mock_rest_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"label": "Winter '25", "url": "/services/data/v62.0", "version": "62.0"}
            ])))
            .mount(&server)
            .await;

        let result = handle_versions(&client).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Bulk API Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_handle_bulk_create_ingest_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/jobs/ingest$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "750xx", "state": "Open", "object": "Account",
                "operation": "insert", "numberRecordsProcessed": 0,
                "numberRecordsFailed": 0
            })))
            .mount(&server)
            .await;

        let req = BulkCreateIngestJobRequest {
            sobject: "Account".to_string(),
            operation: "insert".to_string(),
            external_id_field: None,
            column_delimiter: "COMMA".to_string(),
            line_ending: "LF".to_string(),
        };
        let result = handle_bulk_create_ingest_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_create_ingest_job_invalid_operation() {
        let (server, client) = mock_bulk_client().await;
        let _ = &server;

        let req = BulkCreateIngestJobRequest {
            sobject: "Account".to_string(),
            operation: "invalid".to_string(),
            external_id_field: None,
            column_delimiter: "COMMA".to_string(),
            line_ending: "LF".to_string(),
        };
        let result = handle_bulk_create_ingest_job(&client, req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_bulk_upload_job_data_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("PUT"))
            .and(path_regex("/services/data/.*/jobs/ingest/750xx/batches"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let req = BulkUploadJobDataRequest {
            job_id: "750xx".to_string(),
            csv_data: "Name\nAcme\n".to_string(),
        };
        let result = handle_bulk_upload_job_data(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_close_ingest_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("PATCH"))
            .and(path_regex("/services/data/.*/jobs/ingest/750xx$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "750xx", "state": "UploadComplete", "object": "Account",
                "operation": "insert", "numberRecordsProcessed": 0,
                "numberRecordsFailed": 0
            })))
            .mount(&server)
            .await;

        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let result = handle_bulk_close_ingest_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_abort_ingest_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("PATCH"))
            .and(path_regex("/services/data/.*/jobs/ingest/750xx$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "750xx", "state": "Aborted", "object": "Account",
                "operation": "insert", "numberRecordsProcessed": 0,
                "numberRecordsFailed": 0
            })))
            .mount(&server)
            .await;

        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let result = handle_bulk_abort_ingest_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_ingest_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/jobs/ingest/750xx$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "750xx", "state": "JobComplete", "object": "Account",
                "operation": "insert", "numberRecordsProcessed": 100,
                "numberRecordsFailed": 2
            })))
            .mount(&server)
            .await;

        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let result = handle_bulk_get_ingest_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_job_results_successful() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex(
                "/services/data/.*/jobs/ingest/750xx/successfulResults",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("\"sf__Id\",\"Name\"\n\"001xx\",\"Acme\""),
            )
            .mount(&server)
            .await;

        let req = BulkJobResultsRequest {
            job_id: "750xx".to_string(),
            result_type: "successful".to_string(),
        };
        let result = handle_bulk_get_job_results(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_job_results_failed() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex(
                "/services/data/.*/jobs/ingest/750xx/failedResults",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("\"sf__Error\"\n\"DUPLICATE\""),
            )
            .mount(&server)
            .await;

        let req = BulkJobResultsRequest {
            job_id: "750xx".to_string(),
            result_type: "failed".to_string(),
        };
        let result = handle_bulk_get_job_results(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_job_results_unprocessed() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex(
                "/services/data/.*/jobs/ingest/750xx/unprocessedrecords",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_string("\"Name\"\n\"Pending\""))
            .mount(&server)
            .await;

        let req = BulkJobResultsRequest {
            job_id: "750xx".to_string(),
            result_type: "unprocessed".to_string(),
        };
        let result = handle_bulk_get_job_results(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_job_results_invalid_type() {
        let (server, client) = mock_bulk_client().await;
        let _ = &server;

        let req = BulkJobResultsRequest {
            job_id: "750xx".to_string(),
            result_type: "invalid".to_string(),
        };
        let result = handle_bulk_get_job_results(&client, req).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_bulk_delete_ingest_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("DELETE"))
            .and(path_regex("/services/data/.*/jobs/ingest/750xx$"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let result = handle_bulk_delete_ingest_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_all_ingest_jobs_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/jobs/ingest$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "done": true,
                "records": [{
                    "id": "750a", "state": "JobComplete", "object": "Account",
                    "operation": "insert", "numberRecordsProcessed": 50,
                    "numberRecordsFailed": 0
                }],
                "nextRecordsUrl": null
            })))
            .mount(&server)
            .await;

        let result = handle_bulk_get_all_ingest_jobs(&client).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_abort_query_job_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("PATCH"))
            .and(path_regex("/services/data/.*/jobs/query/750xx$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "750xx", "state": "Aborted", "operation": "query",
                "numberRecordsProcessed": 0
            })))
            .mount(&server)
            .await;

        let req = BulkJobIdRequest {
            job_id: "750xx".to_string(),
        };
        let result = handle_bulk_abort_query_job(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_bulk_get_query_results_success() {
        let (server, client) = mock_bulk_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/jobs/query/750xx/results"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("\"Id\",\"Name\"\n\"001xx\",\"Acme\""),
            )
            .mount(&server)
            .await;

        let req = BulkQueryResultsRequest {
            job_id: "750xx".to_string(),
            locator: None,
            max_records: None,
        };
        let result = handle_bulk_get_query_results(&client, req).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Tooling API Handler Tests
    // =========================================================================

    #[tokio::test]
    async fn test_handle_tooling_query_success() {
        let (server, client) = mock_tooling_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/tooling/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "totalSize": 1, "done": true,
                "records": [{"Id": "01pxx", "Name": "MyClass"}]
            })))
            .mount(&server)
            .await;

        let req = ToolingQueryRequest {
            soql: "SELECT Id, Name FROM ApexClass".to_string(),
        };
        let result = handle_tooling_query(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tooling_execute_anonymous_success() {
        let (server, client) = mock_tooling_client().await;
        Mock::given(method("GET"))
            .and(path_regex("/services/data/.*/tooling/executeAnonymous"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "compiled": true, "success": true,
                "compileProblem": null, "exceptionMessage": null,
                "exceptionStackTrace": null, "line": -1, "column": -1
            })))
            .mount(&server)
            .await;

        let req = ExecuteAnonymousRequest {
            apex_code: "System.debug('hello');".to_string(),
        };
        let result = handle_tooling_execute_anonymous(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tooling_get_success() {
        let (server, client) = mock_tooling_client().await;
        Mock::given(method("GET"))
            .and(path_regex(
                "/services/data/.*/tooling/sobjects/ApexClass/01pxx000003DGbY",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Id": "01pxx000003DGbY", "Name": "MyClass", "Body": "public class MyClass {}"
            })))
            .mount(&server)
            .await;

        let req = ToolingGetRequest {
            sobject: "ApexClass".to_string(),
            id: "01pxx000003DGbY".to_string(),
        };
        let result = handle_tooling_get(&client, req).await;
        assert!(result.is_ok(), "handle_tooling_get failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_handle_tooling_create_success() {
        let (server, client) = mock_tooling_client().await;
        Mock::given(method("POST"))
            .and(path_regex("/services/data/.*/tooling/sobjects/ApexClass"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id": "01pxx", "success": true, "errors": []
            })))
            .mount(&server)
            .await;

        let req = ToolingCreateRequest {
            sobject: "ApexClass".to_string(),
            record: json!({"Body": "public class Test {}"}),
        };
        let result = handle_tooling_create(&client, req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_tooling_delete_success() {
        let (server, client) = mock_tooling_client().await;
        Mock::given(method("DELETE"))
            .and(path_regex(
                "/services/data/.*/tooling/sobjects/ApexClass/01pxx000003DGbY",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let req = ToolingDeleteRequest {
            sobject: "ApexClass".to_string(),
            id: "01pxx000003DGbY".to_string(),
        };
        let result = handle_tooling_delete(&client, req).await;
        assert!(result.is_ok(), "handle_tooling_delete failed: {:?}", result);
    }

    // =========================================================================
    // Metadata API Handler Tests
    // =========================================================================

    #[test]
    fn test_metadata_deploy_invalid_base64() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = MetadataClient::from_parts("https://example.com", "test_token");

        let req = MetadataDeployRequest {
            zip_base64: "!!!not-valid-base64!!!".to_string(),
            options: MetadataDeployOptions::default(),
        };
        let result = rt.block_on(handle_metadata_deploy(&client, req));
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_retrieve_missing_package_name() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = MetadataClient::from_parts("https://example.com", "test_token");

        let req = MetadataRetrieveRequest {
            is_packaged: true,
            package_name: None,
            types: vec![],
            api_version: "62.0".to_string(),
        };
        let result = rt.block_on(handle_metadata_retrieve(&client, req));
        assert!(result.is_err());
    }

    // =========================================================================
    // Bridge Result Serialization Tests
    // =========================================================================

    #[test]
    fn test_bridge_result_serialization_for_query() {
        let response = QueryResponse {
            total_size: 1,
            done: true,
            records: vec![json!({"Id": "001xx"})],
            next_records_url: None,
        };
        let result: BridgeResult<QueryResponse> = BridgeResult::ok(response);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("total_size"));
        assert!(json.contains("001xx"));
    }

    #[test]
    fn test_bridge_error_result_for_query() {
        let result: BridgeResult<QueryResponse> =
            BridgeResult::err("INVALID_SOQL", "unexpected token");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("INVALID_SOQL"));
        assert!(json.contains("unexpected token"));
    }

    #[test]
    fn test_create_response_serialization() {
        let response = CreateResponse {
            id: "001xx000003DgAAAS".to_string(),
            success: true,
            errors: vec![],
        };
        let result: BridgeResult<CreateResponse> = BridgeResult::ok(response);
        let json = serde_json::to_string(&result).unwrap();
        let parsed: BridgeResult<CreateResponse> = serde_json::from_str(&json).unwrap();
        match parsed {
            BridgeResult::Ok(r) => assert_eq!(r.id, "001xx000003DgAAAS"),
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn test_composite_type_conversion() {
        let bridge_req = CompositeRequest {
            all_or_none: true,
            subrequests: vec![CompositeSubrequest {
                method: "POST".to_string(),
                url: "/services/data/v62.0/sobjects/Account".to_string(),
                reference_id: "ref1".to_string(),
                body: Some(json!({"Name": "Test"})),
            }],
        };

        let sf_request = busbar_sf_rest::CompositeRequest {
            all_or_none: bridge_req.all_or_none,
            collate_subrequests: false,
            subrequests: bridge_req
                .subrequests
                .into_iter()
                .map(|s| busbar_sf_rest::CompositeSubrequest {
                    method: s.method,
                    url: s.url,
                    reference_id: s.reference_id,
                    body: s.body,
                })
                .collect(),
        };
        assert!(sf_request.all_or_none);
        assert_eq!(sf_request.subrequests.len(), 1);
    }
}
