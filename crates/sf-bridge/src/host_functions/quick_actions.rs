//! Quick Actions (Invocable Actions) API host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

/// List global quick actions.
pub(crate) async fn handle_list_global_quick_actions(
    client: &SalesforceRestClient,
) -> BridgeResult<Vec<QuickActionMetadata>> {
    match client.list_global_quick_actions().await {
        Ok(actions) => BridgeResult::ok(
            actions
                .into_iter()
                .map(|a| QuickActionMetadata {
                    name: a.name,
                    label: a.label,
                    action_type: a.action_type,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe a global quick action.
pub(crate) async fn handle_describe_global_quick_action(
    client: &SalesforceRestClient,
    request: DescribeGlobalQuickActionRequest,
) -> BridgeResult<QuickActionDescribe> {
    match client.describe_global_quick_action(&request.action).await {
        Ok(desc) => BridgeResult::ok(QuickActionDescribe {
            name: desc.name,
            label: desc.label,
            action_type: desc.action_type,
            target_sobject_type: desc.target_sobject_type,
            target_record_type_id: desc.target_record_type_id,
            target_parent_field: desc.target_parent_field,
            layout: desc.layout,
            default_values: desc.default_values,
            icons: desc
                .icons
                .into_iter()
                .map(|i| serde_json::to_value(&i).unwrap_or(serde_json::Value::Null))
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List quick actions for an SObject.
pub(crate) async fn handle_list_quick_actions(
    client: &SalesforceRestClient,
    request: ListQuickActionsRequest,
) -> BridgeResult<Vec<QuickActionMetadata>> {
    match client.list_quick_actions(&request.sobject).await {
        Ok(actions) => BridgeResult::ok(
            actions
                .into_iter()
                .map(|a| QuickActionMetadata {
                    name: a.name,
                    label: a.label,
                    action_type: a.action_type,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe a quick action.
pub(crate) async fn handle_describe_quick_action(
    client: &SalesforceRestClient,
    request: DescribeQuickActionRequest,
) -> BridgeResult<QuickActionDescribe> {
    match client
        .describe_quick_action(&request.sobject, &request.action)
        .await
    {
        Ok(desc) => BridgeResult::ok(QuickActionDescribe {
            name: desc.name,
            label: desc.label,
            action_type: desc.action_type,
            target_sobject_type: desc.target_sobject_type,
            target_record_type_id: desc.target_record_type_id,
            target_parent_field: desc.target_parent_field,
            layout: desc.layout,
            default_values: desc.default_values,
            icons: desc
                .icons
                .into_iter()
                .map(|i| serde_json::to_value(&i).unwrap_or(serde_json::Value::Null))
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Invoke a quick action.
pub(crate) async fn handle_invoke_quick_action(
    client: &SalesforceRestClient,
    request: InvokeQuickActionRequest,
) -> BridgeResult<serde_json::Value> {
    match client
        .invoke_quick_action(&request.sobject, &request.action, &request.body)
        .await
    {
        Ok(result) => {
            let value = serde_json::to_value(&result)
                .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
            BridgeResult::ok(value)
        }
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

pub async fn handle_list_standard_actions(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.list_standard_actions().await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_list_custom_action_types(
    rest: &busbar_sf_rest::SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match rest.list_custom_action_types().await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_list_custom_actions(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: ListCustomActionsRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.list_custom_actions(&req.action_type).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_standard_action(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match rest.describe_standard_action(&req.sobject).await {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_describe_custom_action(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: DescribeCustomActionRequest,
) -> BridgeResult<serde_json::Value> {
    match rest
        .describe_custom_action(&req.action_type, &req.action_name)
        .await
    {
        Ok(result) => BridgeResult::ok(serde_json::to_value(result).unwrap()),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_invoke_standard_action(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: InvokeActionRequest,
) -> BridgeResult<Vec<serde_json::Value>> {
    let sf_request = busbar_sf_rest::InvocableActionRequest { inputs: req.inputs };
    match rest
        .invoke_standard_action(&req.action_name, &sf_request)
        .await
    {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap())
                .collect(),
        ),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}

pub async fn handle_invoke_custom_action(
    rest: &busbar_sf_rest::SalesforceRestClient,
    req: InvokeCustomActionRequest,
) -> BridgeResult<Vec<serde_json::Value>> {
    let sf_request = busbar_sf_rest::InvocableActionRequest { inputs: req.inputs };
    match rest
        .invoke_custom_action(&req.action_type, &req.action_name, &sf_request)
        .await
    {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|r| serde_json::to_value(r).unwrap())
                .collect(),
        ),
        Err(e) => {
            let (code, msg) = sanitize_rest_error(&e);
            BridgeResult::err(code, msg)
        }
    }
}
