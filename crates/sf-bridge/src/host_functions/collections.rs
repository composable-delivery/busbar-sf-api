//! SObject Collections API host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

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

// =============================================================================
// Utility functions
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
