//! Knowledge Article host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

pub async fn handle_knowledge_settings(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.knowledge_settings().await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_knowledge_articles(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: KnowledgeArticlesRequest,
) -> BridgeResult<serde_json::Value> {
    match rest
        .knowledge_articles(req.query.as_deref(), req.channel.as_deref())
        .await
    {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_data_category_groups(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DataCategoryGroupsRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.data_category_groups(req.sobject.as_deref()).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_data_categories(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DataCategoriesRequest,
) -> BridgeResult<serde_json::Value> {
    match rest
        .data_categories(&req.group, req.sobject.as_deref())
        .await
    {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
