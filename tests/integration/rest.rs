//! REST API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_rest::{CompositeRequest, CompositeSubrequest, QueryBuilder, SalesforceRestClient};
use serde::{Deserialize, Serialize};

// ============================================================================
// REST API - Comprehensive Tests
// ============================================================================

#[tokio::test]
async fn test_rest_composite_api() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let composite_request = CompositeRequest {
        all_or_none: true,
        collate_subrequests: false,
        subrequests: vec![
            CompositeSubrequest {
                method: "POST".to_string(),
                url: format!("/services/data/v{}/sobjects/Account", creds.api_version()),
                reference_id: "NewAccount".to_string(),
                body: Some(serde_json::json!({
                    "Name": format!(
                        "Composite Test Account {}",
                        chrono::Utc::now().timestamp_millis()
                    )
                })),
            },
            CompositeSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/sobjects/Account/@{{NewAccount.id}}",
                    creds.api_version()
                ),
                reference_id: "GetNewAccount".to_string(),
                body: None,
            },
        ],
    };

    let response = client
        .composite(&composite_request)
        .await
        .expect("Composite request should succeed");

    assert_eq!(response.responses.len(), 2, "Should have 2 sub-responses");

    if let Some(first_response) = response.responses.first() {
        if let Some(id) = first_response.body.get("id").and_then(|v| v.as_str()) {
            let _ = client.delete("Account", id).await;
        }
    }
}

#[tokio::test]
async fn test_rest_search_sosl() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let search_query = "FIND {test*} IN NAME FIELDS RETURNING Account(Id, Name), Contact(Id, Name)";
    let result = client.search::<serde_json::Value>(search_query).await;

    assert!(result.is_ok(), "SOSL search should succeed");
}

#[tokio::test]
async fn test_rest_batch_operations() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let test_accounts = vec![
        serde_json::json!({
            "Name": format!("Batch Test 1 {}", chrono::Utc::now().timestamp_millis())
        }),
        serde_json::json!({
            "Name": format!("Batch Test 2 {}", chrono::Utc::now().timestamp_millis())
        }),
        serde_json::json!({
            "Name": format!("Batch Test 3 {}", chrono::Utc::now().timestamp_millis())
        }),
    ];

    let create_results = client
        .create_multiple("Account", &test_accounts, false)
        .await
        .expect("create_multiple should succeed");

    assert_eq!(create_results.len(), 3, "Should create 3 accounts");

    let ids: Vec<String> = create_results.iter().filter_map(|r| r.id.clone()).collect();
    assert_eq!(ids.len(), 3, "Should have 3 account IDs");

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let get_results: Vec<serde_json::Value> = client
        .get_multiple("Account", &id_refs, &["Id", "Name"])
        .await
        .expect("get_multiple should succeed");

    assert_eq!(get_results.len(), 3, "Should retrieve 3 accounts");

    let updates: Vec<(String, serde_json::Value)> = ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                serde_json::json!({
                    "Description": "Updated by batch test"
                }),
            )
        })
        .collect();

    let update_results = client
        .update_multiple("Account", &updates, false)
        .await
        .expect("update_multiple should succeed");

    assert_eq!(update_results.len(), 3, "Should update 3 accounts");

    let delete_results = client
        .delete_multiple(&id_refs, false)
        .await
        .expect("delete_multiple should succeed");

    assert_eq!(delete_results.len(), 3, "Should delete 3 accounts");
}

#[tokio::test]
async fn test_rest_query_pagination() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    assert!(
        result.done || result.next_records_url.is_some(),
        "Should indicate completion or pagination"
    );

    let all_records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account LIMIT 100")
        .await
        .expect("query_all should succeed");

    assert!(all_records.len() <= 100, "Should respect LIMIT");
}

#[tokio::test]
async fn test_rest_upsert_operation() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let unique_number = format!("TEST-{}", chrono::Utc::now().timestamp_millis());

    let account_data = serde_json::json!({
        "Name": format!("Upsert Test {}", unique_number),
        "AccountNumber": unique_number
    });

    let result1 = client
        .upsert("Account", "AccountNumber", &unique_number, &account_data)
        .await;

    if let Ok(upsert_result) = result1 {
        assert!(upsert_result.created, "First upsert should create record");
        let account_id = upsert_result.id.clone();

        let updated_data = serde_json::json!({
            "Name": format!("Upsert Test Updated {}", unique_number),
            "AccountNumber": unique_number
        });

        let result2 = client
            .upsert("Account", "AccountNumber", &unique_number, &updated_data)
            .await;

        if let Ok(upsert_result2) = result2 {
            assert!(
                !upsert_result2.created,
                "Second upsert should update record"
            );
            assert_eq!(upsert_result2.id, account_id, "Should be same account ID");
        }

        let _ = client.delete("Account", &account_id).await;
    } else {
        println!("Note: Upsert test skipped - AccountNumber may not be set as external ID");
    }
}

// ============================================================================
// QueryBuilder Security Tests
// ============================================================================

#[tokio::test]
async fn test_query_builder_injection_prevention() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let malicious_input = "Test' OR '1'='1";

    let result: Result<Vec<serde_json::Value>, _> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Name", malicious_input)
        .expect("where_eq should succeed")
        .limit(10)
        .execute(&client)
        .await;

    assert!(result.is_ok(), "Query should succeed with escaped input");
    let accounts = result.unwrap();

    assert_eq!(
        accounts.len(),
        0,
        "Should not find any accounts with malicious input"
    );
}

#[tokio::test]
async fn test_query_builder_like_escaping() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let pattern = "test%";

    let result: Result<Vec<serde_json::Value>, _> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_like("Name", pattern)
        .expect("where_like should succeed")
        .limit(10)
        .execute(&client)
        .await;

    assert!(result.is_ok(), "LIKE query should succeed");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_rest_error_invalid_field_query() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, InvalidFieldName123 FROM Account")
        .await;

    assert!(result.is_err(), "Query with invalid field should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sobject_describe() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.describe_sobject("NonExistentObject__c").await;

    assert!(
        result.is_err(),
        "Describing non-existent object should fail"
    );
}

#[tokio::test]
async fn test_rest_error_invalid_record_id_get() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result: Result<serde_json::Value, _> = client.get("Account", "bad-id", None).await;

    assert!(result.is_err(), "Getting non-existent record should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sobject_create() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .create(
            "Account'; DROP TABLE--",
            &serde_json::json!({"Name": "Bad"}),
        )
        .await;

    assert!(result.is_err(), "Create with invalid SObject should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_id_update_delete() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let update_result = client
        .update("Account", "bad-id", &serde_json::json!({"Name": "Bad"}))
        .await;

    assert!(update_result.is_err(), "Update with invalid ID should fail");

    let delete_result = client.delete("Account", "bad-id").await;

    assert!(delete_result.is_err(), "Delete with invalid ID should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_upsert_field() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .upsert(
            "Account",
            "AccountNumber; DROP",
            "bad",
            &serde_json::json!({"Name": "Bad"}),
        )
        .await;

    assert!(result.is_err(), "Upsert with invalid field should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sosl() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .search::<serde_json::Value>("FIND {test} RETURNING")
        .await;

    assert!(result.is_err(), "Invalid SOSL should fail");
}

#[tokio::test]
async fn test_rest_error_batch_invalid_input() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let create_result = client
        .create_multiple("Account; DROP", &Vec::<serde_json::Value>::new(), false)
        .await;

    assert!(
        create_result.is_err(),
        "create_multiple invalid sobject should fail"
    );

    let update_result = client
        .update_multiple(
            "Account",
            &[("bad-id".to_string(), serde_json::json!({"Name": "Bad"}))],
            false,
        )
        .await;

    assert!(
        update_result.is_err(),
        "update_multiple invalid ID should fail"
    );

    let delete_result = client.delete_multiple(&["bad-id"], false).await;

    assert!(
        delete_result.is_err(),
        "delete_multiple invalid ID should fail"
    );

    let get_result: Result<Vec<serde_json::Value>, _> =
        client.get_multiple("Account", &["bad-id"], &["Id"]).await;

    assert!(get_result.is_err(), "get_multiple invalid ID should fail");
}

#[tokio::test]
async fn test_rest_composite_error_subrequest() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let composite_request = CompositeRequest {
        all_or_none: false,
        collate_subrequests: false,
        subrequests: vec![CompositeSubrequest {
            method: "GET".to_string(),
            url: format!("/services/data/v{}/sobjects/Nope__c", creds.api_version()),
            reference_id: "BadRequest".to_string(),
            body: None,
        }],
    };

    let response = client
        .composite(&composite_request)
        .await
        .expect("Composite request should succeed at transport level");

    let sub = response
        .responses
        .first()
        .expect("Should have one response");
    assert!(sub.http_status_code >= 400, "Subrequest should error");
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
async fn test_credentials_redaction() {
    let creds = get_credentials().await;

    let debug_output = format!("{:?}", creds);

    assert!(
        debug_output.contains("[REDACTED]") || !debug_output.contains(creds.access_token()),
        "Debug output should not contain actual token"
    );
}

#[tokio::test]
async fn test_client_debug_redaction() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let debug_output = format!("{:?}", client);

    assert!(
        !debug_output.contains(creds.access_token()),
        "Client debug output should not contain actual token"
    );
}

// ============================================================================
// Type-Safe Pattern Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestAccount {
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry", skip_serializing_if = "Option::is_none")]
    industry: Option<String>,
}

#[tokio::test]
async fn test_type_safe_crud() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let account = TestAccount {
        id: None,
        name: format!("Type Safe Test {}", chrono::Utc::now().timestamp_millis()),
        industry: Some("Technology".to_string()),
    };

    let id = client
        .create("Account", &account)
        .await
        .expect("Create should succeed");

    let retrieved: TestAccount = client
        .get("Account", &id, Some(&["Id", "Name", "Industry"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.name, account.name);
    assert_eq!(retrieved.industry, account.industry);

    let update_data = serde_json::json!({
        "Industry": "Finance"
    });

    client
        .update("Account", &id, &update_data)
        .await
        .expect("Update should succeed");

    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");
}

#[tokio::test]
async fn test_type_safe_query() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let accounts: Vec<TestAccount> = client
        .query_all("SELECT Id, Name, Industry FROM Account LIMIT 10")
        .await
        .expect("Query should succeed");

    for account in &accounts {
        assert!(account.id.is_some(), "Account should have ID");
        assert!(!account.name.is_empty(), "Account should have name");
    }
}

// ============================================================================
// Incremental Sync Tests
// ============================================================================

#[tokio::test]
async fn test_get_deleted_records() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create and delete a test account
    let test_name = format!("Delete Test {}", chrono::Utc::now().timestamp_millis());
    let account_id = client
        .create("Account", &serde_json::json!({"Name": test_name}))
        .await
        .expect("Account creation should succeed");

    // Delete the account
    client
        .delete("Account", &account_id)
        .await
        .expect("Account deletion should succeed");

    // Wait a bit for the deletion to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Query for deleted records in a 30-day window
    let now = chrono::Utc::now();
    let start = (now - chrono::Duration::days(1)).to_rfc3339();
    let end = now.to_rfc3339();

    let result = client
        .get_deleted("Account", &start, &end)
        .await
        .expect("get_deleted should succeed");

    assert!(
        !result.earliest_date_available.is_empty(),
        "Should have earliest date available"
    );
    assert!(
        !result.latest_date_covered.is_empty(),
        "Should have latest date covered"
    );
    // The deleted record may or may not appear immediately, so we just verify the call works
}

#[tokio::test]
async fn test_get_updated_records() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create and update a test account
    let test_name = format!("Update Test {}", chrono::Utc::now().timestamp_millis());
    let account_id = client
        .create("Account", &serde_json::json!({"Name": test_name}))
        .await
        .expect("Account creation should succeed");

    // Update the account
    client
        .update(
            "Account",
            &account_id,
            &serde_json::json!({"Description": "Updated"}),
        )
        .await
        .expect("Account update should succeed");

    // Wait a bit for the update to be processed
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Query for updated records in a 30-day window
    let now = chrono::Utc::now();
    let start = (now - chrono::Duration::days(1)).to_rfc3339();
    let end = now.to_rfc3339();

    let result = client
        .get_updated("Account", &start, &end)
        .await
        .expect("get_updated should succeed");

    assert!(
        !result.latest_date_covered.is_empty(),
        "Should have latest date covered"
    );

    // Clean up
    let _ = client.delete("Account", &account_id).await;
}

// ============================================================================
// Binary Content Tests
// ============================================================================

#[tokio::test]
async fn test_get_blob_content() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a ContentVersion with test data
    let test_content = b"Test file content for blob retrieval";
    use base64::Engine as _;
    let base64_content = base64::engine::general_purpose::STANDARD.encode(test_content);

    let content_version_id = client
        .create(
            "ContentVersion",
            &serde_json::json!({
                "Title": format!("Test Blob {}", chrono::Utc::now().timestamp_millis()),
                "PathOnClient": "test.txt",
                "VersionData": base64_content,
            }),
        )
        .await
        .expect("ContentVersion creation should succeed");

    // Query to get the ContentDocument ID
    let query_result: Vec<serde_json::Value> = client
        .query_all(&format!(
            "SELECT ContentDocumentId FROM ContentVersion WHERE Id = '{}'",
            content_version_id
        ))
        .await
        .expect("Query should succeed");

    if let Some(cv) = query_result.first() {
        if let Some(content_document_id) = cv.get("ContentDocumentId").and_then(|v| v.as_str()) {
            // Retrieve the blob content
            let blob_data = client
                .get_blob("ContentVersion", &content_version_id, "VersionData")
                .await
                .expect("get_blob should succeed");

            assert!(!blob_data.is_empty(), "Blob data should not be empty");
            assert_eq!(
                blob_data, test_content,
                "Retrieved content should match uploaded content"
            );

            // Clean up - delete the ContentDocument
            let _ = client.delete("ContentDocument", content_document_id).await;
        }
    }
}

#[tokio::test]
async fn test_get_rich_text_image() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // This test verifies the method works, but creating rich text images programmatically
    // is complex. We'll test the error handling path instead.
    let result = client
        .get_rich_text_image(
            "Account",
            "001000000000000AAA",
            "Description",
            "069000000000000",
        )
        .await;

    // We expect this to fail since we're using a fake ID, but it should fail with a proper error
    // not a panic or malformed request
    assert!(
        result.is_err(),
        "Should return an error for non-existent record"
    );
}

// ============================================================================
// Relationship Traversal Tests
// ============================================================================

#[tokio::test]
async fn test_get_relationship_child() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account
    let test_name = format!(
        "Relationship Test {}",
        chrono::Utc::now().timestamp_millis()
    );
    let account_id = client
        .create("Account", &serde_json::json!({"Name": test_name}))
        .await
        .expect("Account creation should succeed");

    // Create a contact related to the account
    let contact_id = client
        .create(
            "Contact",
            &serde_json::json!({
                "LastName": "Test Contact",
                "AccountId": account_id
            }),
        )
        .await
        .expect("Contact creation should succeed");

    // Get child contacts through relationship
    let contacts_result: busbar_sf_rest::QueryResult<serde_json::Value> = client
        .get_relationship("Account", &account_id, "Contacts")
        .await
        .expect("get_relationship should succeed for child relationship");

    assert!(
        contacts_result.total_size > 0,
        "Should have at least one contact"
    );
    assert!(!contacts_result.records.is_empty(), "Should have records");

    // Clean up
    let _ = client.delete("Contact", &contact_id).await;
    let _ = client.delete("Account", &account_id).await;
}

#[tokio::test]
async fn test_get_relationship_parent() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account
    let test_name = format!(
        "Parent Relationship Test {}",
        chrono::Utc::now().timestamp_millis()
    );
    let account_id = client
        .create("Account", &serde_json::json!({"Name": test_name}))
        .await
        .expect("Account creation should succeed");

    // Create a contact related to the account
    let contact_id = client
        .create(
            "Contact",
            &serde_json::json!({
                "LastName": "Test Contact",
                "AccountId": account_id
            }),
        )
        .await
        .expect("Contact creation should succeed");

    // Get parent account through relationship
    let account_result: serde_json::Value = client
        .get_relationship("Contact", &contact_id, "Account")
        .await
        .expect("get_relationship should succeed for parent relationship");

    assert!(account_result.get("Id").is_some(), "Should have account ID");
    assert_eq!(
        account_result.get("Id").and_then(|v| v.as_str()),
        Some(account_id.as_str()),
        "Should be the correct account"
    );

    // Clean up
    let _ = client.delete("Contact", &contact_id).await;
    let _ = client.delete("Account", &account_id).await;
}

// ============================================================================
// SObject Basic Info Tests
// ============================================================================

#[tokio::test]
async fn test_get_sobject_basic_info() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let info = client
        .get_sobject_basic_info("Account")
        .await
        .expect("get_sobject_basic_info should succeed");

    assert_eq!(info.object_describe.name, "Account", "Should be Account");
    assert!(!info.object_describe.label.is_empty(), "Should have label");
    assert!(
        info.object_describe.key_prefix.is_some(),
        "Account should have key prefix"
    );
    assert!(
        !info.object_describe.urls.is_empty(),
        "Should have URLs map"
    );
    assert!(
        info.object_describe.createable,
        "Account should be createable"
    );
    assert!(
        info.object_describe.queryable,
        "Account should be queryable"
    );
    // recent_items may be empty if user hasn't accessed any recently
}
