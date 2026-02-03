//! Bulk API 2.0 host function handlers.
use super::error::*;
use busbar_sf_bulk::BulkApiClient;
use busbar_sf_wasm_types::*;

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
// Utility functions
// =============================================================================

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
