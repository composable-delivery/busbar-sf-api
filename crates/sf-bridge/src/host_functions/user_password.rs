//! User password management host function handlers.
use super::error::*;
use busbar_sf_wasm_types::*;

pub async fn handle_get_user_password_status(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.get_user_password_status(&req.id).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_set_user_password(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: SetUserPasswordRequest,
) -> BridgeResult<()> {
    let sf_request = busbar_sf_rest::SetPasswordRequest {
        new_password: req.password,
    };
    match rest.set_user_password(&req.user_id, &sf_request).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_reset_user_password(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: GetRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.reset_user_password(&req.id).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
