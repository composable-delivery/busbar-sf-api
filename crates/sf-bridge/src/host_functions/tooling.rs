//! Tooling API host function handlers.
use super::error::*;
use busbar_sf_tooling::ToolingClient;
use busbar_sf_wasm_types::*;

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
