//! Appointment Scheduler API host function handlers.
use super::error::*;
use busbar_sf_wasm_types::*;

pub async fn handle_appointment_candidates(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: serde_json::Value,
) -> BridgeResult<serde_json::Value> {
    let sf_request: busbar_sf_rest::AppointmentCandidatesRequest = match serde_json::from_value(req)
    {
        Ok(r) => r,
        Err(e) => return BridgeResult::err("INVALID_REQUEST", e.to_string()),
    };
    match rest.appointment_candidates(&sf_request).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_appointment_slots(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: serde_json::Value,
) -> BridgeResult<serde_json::Value> {
    match rest.appointment_slots(&req).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
