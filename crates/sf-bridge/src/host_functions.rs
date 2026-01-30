//! Host function implementations.
//!
//! These functions contain the business logic for each bridge operation.
//! They are pure async functions that take typed requests and return typed
//! responses. The Extism wiring (memory management, serialization at the
//! ABI boundary) is handled in `lib.rs`.
//!
//! ## Security
//!
//! - Credentials never cross the WASM boundary
//! - All inputs are validated using sf-client's security utilities
//! - Errors are sanitized before returning to the guest

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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

/// Delete a record.
pub(crate) async fn handle_delete(
    client: &SalesforceRestClient,
    request: DeleteRequest,
) -> BridgeResult<()> {
    match client.delete(&request.sobject, &request.id).await {
        Ok(()) => BridgeResult::ok(()),
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
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
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

/// Execute a composite API request.
pub(crate) async fn handle_composite(
    client: &SalesforceRestClient,
    request: CompositeRequest,
) -> BridgeResult<CompositeResponse> {
    // Convert bridge types to sf-rest types
    let sf_request = busbar_sf_rest::CompositeRequest {
        all_or_none: request.all_or_none,
        collate_subrequests: false,
        subrequests: request
            .subrequests
            .into_iter()
            .map(|s| busbar_sf_rest::CompositeSubrequest {
                method: s.method,
                url: s.url,
                reference_id: s.reference_id,
                body: s.body,
            })
            .collect(),
    };

    match client.composite(&sf_request).await {
        Ok(result) => BridgeResult::ok(CompositeResponse {
            responses: result
                .responses
                .into_iter()
                .map(|r| CompositeSubresponse {
                    body: r.body,
                    http_status_code: r.http_status_code,
                    reference_id: r.reference_id,
                })
                .collect(),
        }),
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

/// Create multiple records.
pub(crate) async fn handle_create_multiple(
    client: &SalesforceRestClient,
    request: CreateMultipleRequest,
) -> BridgeResult<Vec<CollectionResult>> {
    match client
        .create_multiple(&request.sobject, &request.records, request.all_or_none)
        .await
    {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|r| CollectionResult {
                    id: r.id,
                    success: r.success,
                    errors: r
                        .errors
                        .into_iter()
                        .map(|e| SalesforceApiError {
                            status_code: e.status_code,
                            message: e.message,
                            fields: e.fields,
                        })
                        .collect(),
                    created: r.created,
                })
                .collect(),
        ),
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

/// Delete multiple records.
pub(crate) async fn handle_delete_multiple(
    client: &SalesforceRestClient,
    request: DeleteMultipleRequest,
) -> BridgeResult<Vec<CollectionResult>> {
    let ids: Vec<&str> = request.ids.iter().map(|s| s.as_str()).collect();
    match client.delete_multiple(&ids, request.all_or_none).await {
        Ok(results) => BridgeResult::ok(
            results
                .into_iter()
                .map(|r| CollectionResult {
                    id: r.id,
                    success: r.success,
                    errors: r
                        .errors
                        .into_iter()
                        .map(|e| SalesforceApiError {
                            status_code: e.status_code,
                            message: e.message,
                            fields: e.fields,
                        })
                        .collect(),
                    created: r.created,
                })
                .collect(),
        ),
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

/// Get API limits.
pub(crate) async fn handle_limits(
    client: &SalesforceRestClient,
) -> BridgeResult<serde_json::Value> {
    match client.limits().await {
        Ok(result) => BridgeResult::ok(result),
        Err(e) => BridgeResult::err(format!("{:?}", e.kind), e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Host function tests require a running Salesforce org.
    // Unit tests verify the type conversion logic.

    #[test]
    fn test_bridge_result_serialization_for_query() {
        let response = QueryResponse {
            total_size: 1,
            done: true,
            records: vec![serde_json::json!({"Id": "001xx"})],
            next_records_url: None,
        };
        let result: BridgeResult<QueryResponse> = BridgeResult::ok(response);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("total_size"));
        assert!(json.contains("001xx"));
    }

    #[test]
    fn test_bridge_error_result_for_query() {
        let result: BridgeResult<QueryResponse> =
            BridgeResult::err("INVALID_SOQL", "unexpected token");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("INVALID_SOQL"));
        assert!(json.contains("unexpected token"));
    }

    #[test]
    fn test_create_response_serialization() {
        let response = CreateResponse {
            id: "001xx000003DgAAAS".to_string(),
            success: true,
            errors: vec![],
        };
        let result: BridgeResult<CreateResponse> = BridgeResult::ok(response);
        let json = serde_json::to_string(&result).unwrap();
        let parsed: BridgeResult<CreateResponse> = serde_json::from_str(&json).unwrap();
        match parsed {
            BridgeResult::Ok(r) => assert_eq!(r.id, "001xx000003DgAAAS"),
            _ => panic!("expected Ok"),
        }
    }

    #[test]
    fn test_composite_type_conversion() {
        let bridge_req = CompositeRequest {
            all_or_none: true,
            subrequests: vec![CompositeSubrequest {
                method: "POST".to_string(),
                url: "/services/data/v62.0/sobjects/Account".to_string(),
                reference_id: "ref1".to_string(),
                body: Some(serde_json::json!({"Name": "Test"})),
            }],
        };

        // Verify we can convert to sf-rest types
        let sf_request = busbar_sf_rest::CompositeRequest {
            all_or_none: bridge_req.all_or_none,
            collate_subrequests: false,
            subrequests: bridge_req
                .subrequests
                .into_iter()
                .map(|s| busbar_sf_rest::CompositeSubrequest {
                    method: s.method,
                    url: s.url,
                    reference_id: s.reference_id,
                    body: s.body,
                })
                .collect(),
        };
        assert!(sf_request.all_or_none);
        assert_eq!(sf_request.subrequests.len(), 1);
    }
}
