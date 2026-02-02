//! Embedded Service config host function handlers.
use super::error::*;
use busbar_sf_wasm_types::*;

pub async fn handle_get_embedded_service_config(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.get_embedded_service_config(&req.id).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
