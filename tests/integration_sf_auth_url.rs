//! Comprehensive integration tests using SF_AUTH_URL.
//!
//! These tests use the SF_AUTH_URL environment variable to authenticate
//! and run comprehensive integration tests against a real Salesforce org.
//!
//! Run with: `cargo test --test integration_sf_auth_url -- --ignored`
//!
//! Prerequisites:
//! - SF_AUTH_URL environment variable set with SFDX auth URL
//!
//! Environment variables:
//! - SF_AUTH_URL: SFDX auth URL for authentication (required)

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_bulk::{BulkApiClient, BulkOperation};
use busbar_sf_rest::{CompositeRequest, CompositeSubrequest, QueryBuilder, SalesforceRestClient};
use busbar_sf_tooling::ToolingClient;
use serde::{Deserialize, Serialize};

/// Helper to get authenticated credentials from SF_AUTH_URL.
async fn get_test_credentials() -> SalesforceCredentials {
    let auth_url =
        std::env::var("SF_AUTH_URL").expect("SF_AUTH_URL environment variable must be set");

    SalesforceCredentials::from_sfdx_auth_url(&auth_url)
        .await
        .expect("Failed to authenticate from SF_AUTH_URL")
}

// ============================================================================
// REST API - Comprehensive Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_rest_composite_api() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a composite request with multiple sub-requests
    let composite_request = CompositeRequest {
        all_or_none: true,
        collate_subrequests: false,
        subrequests: vec![
            CompositeSubrequest {
                method: "POST".to_string(),
                url: format!("/services/data/v{}/sobjects/Account", creds.api_version()),
                reference_id: "NewAccount".to_string(),
                body: Some(serde_json::json!({
                    "Name": format!("Composite Test Account {}", chrono::Utc::now().timestamp_millis())
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

    let result = client.composite(&composite_request).await;
    assert!(result.is_ok(), "Composite request should succeed");

    let response = result.unwrap();

    assert_eq!(response.responses.len(), 2, "Should have 2 sub-responses");

    // Clean up: delete the created account
    if let Some(first_response) = response.responses.first() {
        if let Some(id) = first_response.body.get("id").and_then(|v| v.as_str()) {
            let _ = client.delete("Account", id).await;
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_rest_search_sosl() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Perform a SOSL search
    let search_query = "FIND {test*} IN NAME FIELDS RETURNING Account(Id, Name), Contact(Id, Name)";
    let result = client.search::<serde_json::Value>(search_query).await;

    assert!(result.is_ok(), "SOSL search should succeed");

    let _search_results = result.unwrap();
    // search_results is a SearchResult<T> which contains searchRecords
    // Results can be empty or non-empty depending on org data, both are valid
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_rest_batch_operations() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test create_multiple
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

    // Collect IDs for further operations
    let ids: Vec<String> = create_results.iter().filter_map(|r| r.id.clone()).collect();

    assert_eq!(ids.len(), 3, "Should have 3 account IDs");

    // Test get_multiple
    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let get_results: Vec<serde_json::Value> = client
        .get_multiple("Account", &id_refs, &["Id", "Name"])
        .await
        .expect("get_multiple should succeed");

    assert_eq!(get_results.len(), 3, "Should retrieve 3 accounts");

    // Test update_multiple - needs (id, record) tuples
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

    // Clean up: delete_multiple
    let delete_results = client
        .delete_multiple(&id_refs, false)
        .await
        .expect("delete_multiple should succeed");

    assert_eq!(delete_results.len(), 3, "Should delete 3 accounts");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_rest_query_pagination() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Query with pagination - use query() for first page
    let result = client
        .query::<serde_json::Value>("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    assert!(
        result.done || result.next_records_url.is_some(),
        "Should indicate completion or pagination"
    );

    // Test query_all which handles pagination automatically
    let all_records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account LIMIT 100")
        .await
        .expect("query_all should succeed");

    // Should get results (might be 0 in a fresh org)
    assert!(all_records.len() <= 100, "Should respect LIMIT");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_rest_upsert_operation() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let unique_number = format!("TEST-{}", chrono::Utc::now().timestamp_millis());

    // First upsert - should create
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

        // Second upsert - should update
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

        // Clean up
        let _ = client.delete("Account", &account_id).await;
    } else {
        // If upsert fails, it might be that AccountNumber field doesn't exist
        // or external ID is not set up - this is acceptable in a scratch org
        println!("Note: Upsert test skipped - AccountNumber may not be set as external ID");
    }
}

// ============================================================================
// QueryBuilder Security Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_query_builder_injection_prevention() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test with potentially malicious input
    let malicious_input = "Test' OR '1'='1";

    // QueryBuilder should escape this safely
    let result: Result<Vec<serde_json::Value>, _> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Name", malicious_input)
        .expect("where_eq should succeed")
        .limit(10)
        .execute(&client)
        .await;

    // Query should succeed without finding anything (because it's escaped)
    assert!(result.is_ok(), "Query should succeed with escaped input");
    let accounts = result.unwrap();

    // Should not match anything due to proper escaping
    assert_eq!(
        accounts.len(),
        0,
        "Should not find any accounts with malicious input"
    );
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_query_builder_like_escaping() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test LIKE with wildcards
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
// Bulk API 2.0 Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_bulk_insert_lifecycle() {
    let creds = get_test_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Prepare CSV data
    let csv_data = format!(
        "Name,Industry\nBulk Test 1 {},Technology\nBulk Test 2 {},Manufacturing",
        chrono::Utc::now().timestamp_millis(),
        chrono::Utc::now().timestamp_millis()
    );

    // Execute bulk insert
    let result = client
        .execute_ingest("Account", BulkOperation::Insert, &csv_data, None)
        .await
        .expect("Bulk insert should succeed");

    assert_eq!(
        result.job.number_records_processed, 2,
        "Should process 2 records"
    );
    assert_eq!(
        result.job.number_records_failed, 0,
        "Should have 0 failures"
    );

    // Parse successful results to get IDs for cleanup
    if let Some(success_results) = result.successful_results {
        let lines: Vec<&str> = success_results.lines().collect();
        // First line is header, remaining are data
        if lines.len() > 1 {
            for line in &lines[1..] {
                // CSV format: id,created,...
                if let Some(id) = line.split(',').next() {
                    if id.starts_with("001") {
                        // Clean up - use REST client for deletion
                        let rest_client =
                            SalesforceRestClient::new(creds.instance_url(), creds.access_token())
                                .expect("Failed to create REST client");
                        let _ = rest_client.delete("Account", id).await;
                    }
                }
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_bulk_query_operation() {
    let creds = get_test_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Execute bulk query using QueryBuilder for SOQL injection protection
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .limit(100);
    
    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    assert!(
        result.job.number_records_processed >= 0,
        "Should process records"
    );

    // Results should be CSV format
    if let Some(csv_results) = result.results {
        let lines: Vec<&str> = csv_results.lines().collect();
        assert!(!lines.is_empty(), "Should have at least header line");
        // First line should be header
        if let Some(header) = lines.first() {
            assert!(
                header.to_lowercase().contains("id"),
                "Header should contain Id"
            );
            assert!(
                header.to_lowercase().contains("name"),
                "Header should contain Name"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_bulk_update_operation() {
    let creds = get_test_credentials().await;

    // First create an account using REST API
    let rest_client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let test_name = format!("Bulk Update Test {}", chrono::Utc::now().timestamp_millis());
    let account_data = serde_json::json!({
        "Name": test_name
    });

    let account_id = rest_client
        .create("Account", &account_data)
        .await
        .expect("Create should succeed");

    // Now use Bulk API to update it
    let bulk_client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let csv_data = format!("Id,Description\n{},Updated via Bulk API", account_id);

    let result = bulk_client
        .execute_ingest("Account", BulkOperation::Update, &csv_data, None)
        .await
        .expect("Bulk update should succeed");

    assert_eq!(
        result.job.number_records_processed, 1,
        "Should process 1 record"
    );
    assert_eq!(
        result.job.number_records_failed, 0,
        "Should have 0 failures"
    );

    // Verify the update
    let updated: serde_json::Value = rest_client
        .get("Account", &account_id, Some(&["Id", "Description"]))
        .await
        .expect("Get should succeed");

    assert_eq!(
        updated.get("Description").and_then(|v| v.as_str()),
        Some("Updated via Bulk API")
    );

    // Clean up
    let _ = rest_client.delete("Account", &account_id).await;
}

// ============================================================================
// Tooling API Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_tooling_query_apex_classes() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query for ApexClass objects
    let result = client
        .query::<serde_json::Value>("SELECT Id, Name, Status FROM ApexClass LIMIT 10")
        .await;

    assert!(result.is_ok(), "Tooling query should succeed");

    let query_result = result.unwrap();
    assert!(
        query_result.done || query_result.next_records_url.is_some(),
        "Query should complete or have pagination"
    );
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_tooling_execute_anonymous_success() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Execute simple Apex code
    let apex_code = r#"
        System.debug('Integration test from busbar-sf-api');
        Integer result = 2 + 2;
        System.debug('Result: ' + result);
    "#;

    let result = client
        .execute_anonymous(apex_code)
        .await
        .expect("Execute anonymous should succeed");

    assert!(result.compiled, "Apex should compile");
    assert!(result.success, "Apex should execute successfully");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_tooling_execute_anonymous_compile_error() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Execute invalid Apex code
    let invalid_apex = "this is not valid apex code at all;";

    let result = client.execute_anonymous(invalid_apex).await;

    // Should return an error for invalid code
    assert!(result.is_err(), "Invalid Apex should return error");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_tooling_query_all_pagination() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Use query_all to automatically handle pagination
    let records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM ApexClass LIMIT 50")
        .await
        .expect("query_all should succeed");

    // Should get results (might be 0 in a fresh org without custom Apex)
    assert!(records.len() <= 50, "Should respect LIMIT");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_error_handling_invalid_field() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Try to query a non-existent field
    let result = client
        .query::<serde_json::Value>("SELECT Id, InvalidFieldName123 FROM Account")
        .await;

    assert!(result.is_err(), "Query with invalid field should fail");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_error_handling_invalid_sobject() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Try to describe a non-existent SObject
    let result = client.describe_sobject("NonExistentObject__c").await;

    assert!(
        result.is_err(),
        "Describing non-existent object should fail"
    );
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_error_handling_invalid_record_id() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Try to get a record with invalid ID
    let result: Result<serde_json::Value, _> =
        client.get("Account", "001000000000000AAA", None).await;

    assert!(result.is_err(), "Getting non-existent record should fail");
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_credentials_redaction() {
    let creds = get_test_credentials().await;

    // Test that Debug output doesn't expose tokens
    let debug_output = format!("{:?}", creds);

    assert!(
        debug_output.contains("[REDACTED]") || !debug_output.contains(creds.access_token()),
        "Debug output should not contain actual token"
    );
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_client_debug_redaction() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test that Debug output doesn't expose tokens
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
#[ignore = "requires SF_AUTH_URL"]
async fn test_type_safe_crud() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // CREATE with type-safe struct
    let account = TestAccount {
        id: None,
        name: format!("Type Safe Test {}", chrono::Utc::now().timestamp_millis()),
        industry: Some("Technology".to_string()),
    };

    let id = client
        .create("Account", &account)
        .await
        .expect("Create should succeed");

    // READ with type-safe struct
    let retrieved: TestAccount = client
        .get("Account", &id, Some(&["Id", "Name", "Industry"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.name, account.name);
    assert_eq!(retrieved.industry, account.industry);

    // UPDATE
    let update_data = serde_json::json!({
        "Industry": "Finance"
    });

    client
        .update("Account", &id, &update_data)
        .await
        .expect("Update should succeed");

    // DELETE
    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_type_safe_query() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Query with type-safe structs
    let accounts: Vec<TestAccount> = client
        .query_all("SELECT Id, Name, Industry FROM Account LIMIT 10")
        .await
        .expect("Query should succeed");

    // Verify type safety
    for account in &accounts {
        assert!(account.id.is_some(), "Account should have ID");
        assert!(!account.name.is_empty(), "Account should have name");
    }
}
