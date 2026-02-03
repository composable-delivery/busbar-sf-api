//! Consent API host function handlers.
use super::error::*;
use busbar_sf_wasm_types::*;

pub async fn handle_read_consent(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: ReadConsentRequest,
) -> BridgeResult<serde_json::Value> {
    let ids: Vec<&str> = req.ids.iter().map(|s| s.as_str()).collect();
    match rest.read_consent(&req.action, &ids).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_write_consent(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: WriteConsentRequest,
) -> BridgeResult<()> {
    let sf_request = busbar_sf_rest::ConsentWriteRequest {
        records: req
            .records
            .into_iter()
            .map(|r| busbar_sf_rest::ConsentWriteRecord {
                id: r.id,
                result: r.result,
            })
            .collect(),
    };
    match rest.write_consent(&req.action, &sf_request).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_read_multi_consent(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: ReadMultiConsentRequest,
) -> BridgeResult<serde_json::Value> {
    let actions: Vec<&str> = req.actions.iter().map(|s| s.as_str()).collect();
    let ids: Vec<&str> = req.ids.iter().map(|s| s.as_str()).collect();
    match rest.read_multi_consent(&actions, &ids).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
