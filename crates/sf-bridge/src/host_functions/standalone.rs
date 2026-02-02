//! Standalone REST API endpoints (tabs, theme, etc.).
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

pub async fn handle_tabs(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<Vec<serde_json::Value>> {
    match rest.tabs().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_theme(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.theme().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_app_menu(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: AppMenuRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.app_menu(&req.app_menu_type).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_recent_items(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<Vec<serde_json::Value>> {
    match rest.recent_items().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_relevant_items(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.relevant_items().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_platform_event_schema(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: PlatformEventSchemaRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.platform_event_schema(&req.event_name).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_lightning_toggle_metrics(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.lightning_toggle_metrics().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_lightning_usage(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.lightning_usage().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
