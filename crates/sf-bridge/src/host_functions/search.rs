//! Search API host function handlers (parameterized, suggestions, etc.).
use super::error::*;
use busbar_sf_wasm_types::*;

pub async fn handle_parameterized_search(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: serde_json::Value,
) -> BridgeResult<serde_json::Value> {
    let sf_request: busbar_sf_rest::ParameterizedSearchRequest = match serde_json::from_value(req) {
        Ok(r) => r,
        Err(e) => return BridgeResult::err("INVALID_REQUEST", e.to_string()),
    };
    match rest.parameterized_search(&sf_request).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_search_suggestions(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: SearchSuggestionsRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.search_suggestions(&req.query, &req.sobject).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_search_scope_order(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<Vec<serde_json::Value>> {
    match rest.search_scope_order().await {
        Ok(result) => BridgeResult::ok(
            result
                .into_iter()
                .map(|s| serde_json::to_value(s).unwrap())
                .collect(),
        ),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_search_result_layouts(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: SearchResultLayoutsRequest,
) -> BridgeResult<Vec<serde_json::Value>> {
    let sobjects: Vec<&str> = req.sobjects.iter().map(|s| s.as_str()).collect();
    match rest.search_result_layouts(&sobjects).await {
        Ok(result) => BridgeResult::ok(
            result
                .into_iter()
                .map(|s| serde_json::to_value(s).unwrap())
                .collect(),
        ),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
