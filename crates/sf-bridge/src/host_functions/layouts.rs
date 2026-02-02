//! Layout describe host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

pub async fn handle_describe_layouts(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.describe_layouts(&req.sobject).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_named_layout(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeNamedLayoutRequest,
) -> BridgeResult<serde_json::Value> {
    match rest
        .describe_named_layout(&req.sobject, &req.layout_name)
        .await
    {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_approval_layouts(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.describe_approval_layouts(&req.sobject).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_compact_layouts(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.describe_compact_layouts(&req.sobject).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_global_publisher_layouts(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.describe_global_publisher_layouts().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_compact_layouts_multi(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: CompactLayoutsMultiRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.compact_layouts(&req.sobject_list).await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
