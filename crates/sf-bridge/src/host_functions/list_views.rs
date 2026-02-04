//! List Views API host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

/// List views for an SObject.
pub(crate) async fn handle_list_views(
    client: &SalesforceRestClient,
    request: ListViewsRequest,
) -> BridgeResult<ListViewsResult> {
    match client.list_views(&request.sobject).await {
        Ok(result) => BridgeResult::ok(ListViewsResult {
            done: result.done,
            next_records_url: result.next_records_url,
            listviews: result
                .listviews
                .into_iter()
                .map(|lv| ListView {
                    id: lv.id,
                    developer_name: lv.developer_name,
                    label: lv.label,
                    describe_url: lv.describe_url,
                    results_url: lv.results_url,
                    sobject_type: lv.sobject_type,
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get a list view by ID.
pub(crate) async fn handle_get_list_view(
    client: &SalesforceRestClient,
    request: ListViewRequest,
) -> BridgeResult<ListView> {
    match client
        .get_list_view(&request.sobject, &request.list_view_id)
        .await
    {
        Ok(lv) => BridgeResult::ok(ListView {
            id: lv.id,
            developer_name: lv.developer_name,
            label: lv.label,
            describe_url: lv.describe_url,
            results_url: lv.results_url,
            sobject_type: lv.sobject_type,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe a list view.
pub(crate) async fn handle_describe_list_view(
    client: &SalesforceRestClient,
    request: ListViewRequest,
) -> BridgeResult<ListViewDescribe> {
    match client
        .describe_list_view(&request.sobject, &request.list_view_id)
        .await
    {
        Ok(desc) => BridgeResult::ok(ListViewDescribe {
            id: desc.id,
            developer_name: desc.developer_name,
            label: desc.label,
            sobject_type: desc.sobject_type,
            query: desc.query,
            columns: desc
                .columns
                .into_iter()
                .map(|c| ListViewColumn {
                    field_name_or_path: c.field_name_or_path,
                    label: c.label,
                    sortable: c.sortable,
                    field_type: c.field_type,
                })
                .collect(),
            order_by: desc
                .order_by
                .into_iter()
                .map(|ob| serde_json::to_value(&ob).unwrap_or(serde_json::Value::Null))
                .collect(),
            where_condition: desc.where_condition,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a list view.
pub(crate) async fn handle_execute_list_view(
    client: &SalesforceRestClient,
    request: ListViewRequest,
) -> BridgeResult<serde_json::Value> {
    match client
        .execute_list_view::<serde_json::Value>(&request.sobject, &request.list_view_id)
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
