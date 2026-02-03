//! Composite API host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

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

pub async fn handle_composite_graph(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: serde_json::Value,
) -> BridgeResult<serde_json::Value> {
    let sf_request: busbar_sf_rest::CompositeGraphRequest = match serde_json::from_value(req) {
        Ok(r) => r,
        Err(e) => return BridgeResult::err("INVALID_REQUEST", e.to_string()),
    };
    match rest.composite_graph(&sf_request).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
