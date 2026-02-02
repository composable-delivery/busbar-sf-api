//! REST API host function handlers.
//!
//! Handles CRUD operations, queries, describe, and basic REST endpoints.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_wasm_types::*;

/// Execute a SOQL query.
pub(crate) async fn handle_query(
    client: &SalesforceRestClient,
    request: QueryRequest,
) -> BridgeResult<QueryResponse> {
    let result = if request.include_deleted {
        client
            .query_all_including_deleted::<serde_json::Value>(&request.soql)
            .await
    } else {
        client.query::<serde_json::Value>(&request.soql).await
    };

    match result {
        Ok(qr) => BridgeResult::ok(QueryResponse {
            total_size: qr.total_size,
            done: qr.done,
            records: qr.records,
            next_records_url: qr.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Fetch the next page of query results.
pub(crate) async fn handle_query_more(
    client: &SalesforceRestClient,
    request: QueryMoreRequest,
) -> BridgeResult<QueryResponse> {
    match client
        .query_more::<serde_json::Value>(&request.next_records_url)
        .await
    {
        Ok(qr) => BridgeResult::ok(QueryResponse {
            total_size: qr.total_size,
            done: qr.done,
            records: qr.records,
            next_records_url: qr.next_records_url,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Create a new record.
pub(crate) async fn handle_create(
    client: &SalesforceRestClient,
    request: CreateRequest,
) -> BridgeResult<CreateResponse> {
    match client.create(&request.sobject, &request.record).await {
        Ok(id) => BridgeResult::ok(CreateResponse {
            id,
            success: true,
            errors: vec![],
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get a record by ID.
pub(crate) async fn handle_get(
    client: &SalesforceRestClient,
    request: GetRequest,
) -> BridgeResult<serde_json::Value> {
    let fields: Option<Vec<&str>> = request
        .fields
        .as_ref()
        .map(|f| f.iter().map(|s| s.as_str()).collect());

    let result: Result<serde_json::Value, _> = client
        .get(&request.sobject, &request.id, fields.as_deref())
        .await;

    match result {
        Ok(record) => BridgeResult::ok(record),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Update a record.
pub(crate) async fn handle_update(
    client: &SalesforceRestClient,
    request: UpdateRequest,
) -> BridgeResult<()> {
    match client
        .update(&request.sobject, &request.id, &request.record)
        .await
    {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Delete a record.
pub(crate) async fn handle_delete(
    client: &SalesforceRestClient,
    request: DeleteRequest,
) -> BridgeResult<()> {
    match client.delete(&request.sobject, &request.id).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Upsert a record using an external ID.
pub(crate) async fn handle_upsert(
    client: &SalesforceRestClient,
    request: UpsertRequest,
) -> BridgeResult<UpsertResponse> {
    match client
        .upsert(
            &request.sobject,
            &request.external_id_field,
            &request.external_id_value,
            &request.record,
        )
        .await
    {
        Ok(result) => BridgeResult::ok(UpsertResponse {
            id: result.id,
            success: result.success,
            created: result.created,
            errors: result
                .errors
                .into_iter()
                .map(|e| SalesforceApiError {
                    status_code: e.status_code,
                    message: e.message,
                    fields: e.fields,
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe all SObjects.
pub(crate) async fn handle_describe_global(
    client: &SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match client.describe_global().await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(v) => BridgeResult::ok(v),
            Err(e) => BridgeResult::err("SERIALIZATION_ERROR", e.to_string()),
        },
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Describe a specific SObject.
pub(crate) async fn handle_describe_sobject(
    client: &SalesforceRestClient,
    request: DescribeSObjectRequest,
) -> BridgeResult<serde_json::Value> {
    match client.describe_sobject(&request.sobject).await {
        Ok(result) => match serde_json::to_value(result) {
            Ok(v) => BridgeResult::ok(v),
            Err(e) => BridgeResult::err("SERIALIZATION_ERROR", e.to_string()),
        },
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Execute a SOSL search.
pub(crate) async fn handle_search(
    client: &SalesforceRestClient,
    request: SearchRequest,
) -> BridgeResult<SearchResponse> {
    match client.search::<serde_json::Value>(&request.sosl).await {
        Ok(result) => BridgeResult::ok(SearchResponse {
            search_records: result.search_records,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get API limits.
pub(crate) async fn handle_limits(
    client: &SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match client.limits().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get API versions.
pub(crate) async fn handle_versions(
    client: &SalesforceRestClient,
) -> BridgeResult<Vec<ApiVersion>> {
    match client.versions().await {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|v| ApiVersion {
                    label: v.label,
                    url: v.url,
                    version: v.version,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get deleted records.
pub(crate) async fn handle_get_deleted(
    client: &SalesforceRestClient,
    request: GetDeletedRequest,
) -> BridgeResult<GetDeletedResult> {
    match client
        .get_deleted(&request.sobject, &request.start, &request.end)
        .await
    {
        Ok(result) => BridgeResult::ok(GetDeletedResult {
            deleted_records: result
                .deleted_records
                .into_iter()
                .map(|r| DeletedRecord {
                    id: r.id,
                    deleted_date: r.deleted_date,
                })
                .collect(),
            earliest_date_available: result.earliest_date_available,
            latest_date_covered: result.latest_date_covered,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Get updated records.
pub(crate) async fn handle_get_updated(
    client: &SalesforceRestClient,
    request: GetUpdatedRequest,
) -> BridgeResult<GetUpdatedResult> {
    match client
        .get_updated(&request.sobject, &request.start, &request.end)
        .await
    {
        Ok(result) => BridgeResult::ok(GetUpdatedResult {
            ids: result.ids,
            latest_date_covered: result.latest_date_covered,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}
