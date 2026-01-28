//! Integration tests for example programs.
//!
//! These tests verify that all example programs work correctly against
//! a real Salesforce org using the SF_AUTH_URL environment variable.
//!
//! Run with: `cargo test --test integration_examples -- --ignored`
//!
//! Prerequisites:
//! - SF_AUTH_URL environment variable set with SFDX auth URL

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_bulk::BulkApiClient;
use busbar_sf_client::QueryResult;
use busbar_sf_rest::{QueryBuilder, SalesforceRestClient};
use busbar_sf_tooling::ToolingClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper to get authenticated credentials from SF_AUTH_URL.
async fn get_test_credentials() -> SalesforceCredentials {
    let auth_url =
        std::env::var("SF_AUTH_URL").expect("SF_AUTH_URL environment variable must be set");

    SalesforceCredentials::from_sfdx_auth_url(&auth_url)
        .await
        .expect("Failed to authenticate from SF_AUTH_URL")
}

// ============================================================================
// basic_auth.rs Example Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_basic_auth_from_sfdx_auth_url() {
    // Test that we can authenticate from SF_AUTH_URL
    let creds = get_test_credentials().await;

    assert!(creds.is_valid(), "Credentials should be valid");
    assert!(
        !creds.instance_url().is_empty(),
        "Instance URL should be set"
    );
    assert!(
        !creds.access_token().is_empty(),
        "Access token should be set"
    );

    // Verify the instance URL is a Salesforce URL
    let instance_url = creds.instance_url();
    assert!(
        instance_url.contains(".salesforce.com") || instance_url.contains(".my.salesforce.com"),
        "Instance URL should be a Salesforce domain: {}",
        instance_url
    );

    println!("✓ Successfully authenticated from SF_AUTH_URL");
    println!("  Instance URL: {}", creds.instance_url());
    println!("  API Version: {}", creds.api_version());
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_basic_auth_credentials_redaction() {
    let creds = get_test_credentials().await;

    // Ensure Debug output doesn't expose the token (from basic_auth.rs example)
    let debug_output = format!("{:?}", creds);
    assert!(
        debug_output.contains("[REDACTED]") || !debug_output.contains(creds.access_token()),
        "Debug should redact token"
    );
}

// ============================================================================
// rest_crud.rs Example Tests
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ExampleAccount {
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry", skip_serializing_if = "Option::is_none")]
    industry: Option<String>,
    #[serde(rename = "Phone", skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_rest_crud_typed() {
    // Test type-safe CRUD operations from rest_crud.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // CREATE with type-safe struct
    let account = ExampleAccount {
        id: None,
        name: format!(
            "Example Test Corp {}",
            chrono::Utc::now().timestamp_millis()
        ),
        industry: Some("Technology".to_string()),
        phone: Some("+1-555-0100".to_string()),
    };

    let id = client
        .create("Account", &account)
        .await
        .expect("Create should succeed");

    println!("✓ Created account: {}", id);

    // READ with type-safe struct
    let retrieved: ExampleAccount = client
        .get("Account", &id, Some(&["Id", "Name", "Industry", "Phone"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.name, account.name);
    println!("✓ Retrieved account: {:?}", retrieved.name);

    // UPDATE
    let updates = serde_json::json!({
        "Phone": "+1-555-0101"
    });

    client
        .update("Account", &id, &updates)
        .await
        .expect("Update should succeed");

    println!("✓ Updated account: {}", id);

    // DELETE
    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");

    println!("✓ Deleted account: {}", id);
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_rest_crud_dynamic() {
    // Test dynamic JSON CRUD operations from rest_crud.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // CREATE with dynamic JSON
    let account = serde_json::json!({
        "Name": format!("Dynamic Test {}", chrono::Utc::now().timestamp_millis()),
        "Industry": "Technology"
    });

    let id = client
        .create("Account", &account)
        .await
        .expect("Create should succeed");

    println!("✓ Created account with dynamic JSON: {}", id);

    // READ with dynamic JSON
    let retrieved: serde_json::Value = client
        .get("Account", &id, Some(&["Id", "Name", "Industry"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.get("Name"), account.get("Name"));

    println!("✓ Retrieved account: {}", retrieved["Name"]);

    // Clean up
    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_rest_crud_multiple() {
    // Test create_multiple from rest_crud.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let accounts = vec![
        ExampleAccount {
            id: None,
            name: format!("Multi Test 1 {}", chrono::Utc::now().timestamp_millis()),
            industry: Some("Technology".to_string()),
            phone: None,
        },
        ExampleAccount {
            id: None,
            name: format!("Multi Test 2 {}", chrono::Utc::now().timestamp_millis()),
            industry: Some("Retail".to_string()),
            phone: None,
        },
    ];

    // Create multiple
    let results = client
        .create_multiple("Account", &accounts, true)
        .await
        .expect("create_multiple should succeed");

    assert_eq!(results.len(), 2, "Should create 2 accounts");

    let ids: Vec<&str> = results.iter().filter_map(|r| r.id.as_deref()).collect();
    assert_eq!(ids.len(), 2, "Should have 2 IDs");

    println!("✓ Created {} accounts", ids.len());

    // Clean up
    if !ids.is_empty() {
        let _ = client.delete_multiple(&ids, false).await;
        println!("✓ Cleaned up {} accounts", ids.len());
    }
}

// ============================================================================
// queries.rs Example Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_queries_query_builder() {
    // Test QueryBuilder from queries.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test with potentially dangerous input (should be escaped)
    let user_input = "Test'--";

    let accounts: Vec<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Name", user_input)
        .expect("where_eq should succeed")
        .limit(10)
        .execute(&client)
        .await
        .expect("Query should succeed");

    println!(
        "✓ QueryBuilder with escaped input: found {} accounts",
        accounts.len()
    );

    // Test WHERE IN
    let industries = vec!["Technology", "Finance"];
    let accounts2: Vec<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .where_in("Industry", &industries)
        .expect("where_in should succeed")
        .limit(10)
        .execute(&client)
        .await
        .expect("Query should succeed");

    println!(
        "✓ QueryBuilder with WHERE IN: found {} accounts",
        accounts2.len()
    );
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_queries_basic_query() {
    // Test basic queries from queries.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Type-safe query
    let result: QueryResult<HashMap<String, serde_json::Value>> = client
        .query("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    println!("✓ Basic query: found {} records", result.records.len());

    // query_all with automatic pagination
    let all_accounts: Vec<HashMap<String, serde_json::Value>> = client
        .query_all("SELECT Id, Name FROM Account LIMIT 50")
        .await
        .expect("query_all should succeed");

    println!("✓ query_all: found {} records", all_accounts.len());
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_queries_relationship_query() {
    // Test relationship queries from queries.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let contacts: Vec<serde_json::Value> = client
        .query_all(
            "SELECT Id, Name, Email, Account.Name FROM Contact WHERE Account.Name != null LIMIT 10",
        )
        .await
        .expect("Relationship query should succeed");

    println!("✓ Relationship query: found {} contacts", contacts.len());

    // Verify structure
    for contact in contacts.iter().take(3) {
        if let Some(account) = contact.get("Account") {
            if let Some(_name) = account.get("Name") {
                println!("  - Contact with related Account");
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_queries_aggregate() {
    // Test aggregate queries from queries.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let results: Vec<serde_json::Value> = client
        .query_all("SELECT Industry, COUNT(Id) total FROM Account WHERE Industry != null GROUP BY Industry LIMIT 10")
        .await
        .expect("Aggregate query should succeed");

    println!("✓ Aggregate query: found {} groups", results.len());

    for result in results.iter().take(3) {
        if let (Some(industry), Some(count)) = (
            result.get("Industry").and_then(|v| v.as_str()),
            result.get("total").and_then(|v| v.as_i64()),
        ) {
            println!("  - {}: {} accounts", industry, count);
        }
    }
}

// ============================================================================
// bulk_operations.rs Example Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_bulk_insert() {
    // Test bulk insert from bulk_operations.rs example
    let creds = get_test_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let csv_data = format!(
        "Name,Industry\nBulk Example 1 {},Technology\nBulk Example 2 {},Manufacturing",
        chrono::Utc::now().timestamp_millis(),
        chrono::Utc::now().timestamp_millis()
    );

    let result = client
        .execute_ingest(
            "Account",
            busbar_sf_bulk::BulkOperation::Insert,
            &csv_data,
            None,
        )
        .await
        .expect("Bulk insert should succeed");

    println!("✓ Bulk insert completed");
    println!("  Job ID: {}", result.job.id);
    println!(
        "  Records processed: {}",
        result.job.number_records_processed
    );
    println!("  Records failed: {}", result.job.number_records_failed);

    assert_eq!(result.job.number_records_processed, 2);
    assert_eq!(result.job.number_records_failed, 0);

    // Clean up created records
    if let Some(success_results) = result.successful_results {
        let rest_client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
            .expect("Failed to create REST client");

        for line in success_results.lines().skip(1) {
            if let Some(id) = line.split(',').next() {
                if id.starts_with("001") {
                    let _ = rest_client.delete("Account", id).await;
                }
            }
        }
        println!("✓ Cleaned up test records");
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_bulk_query() {
    // Test bulk query from bulk_operations.rs example
    let creds = get_test_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .limit(100);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    println!("✓ Bulk query completed");
    println!("  Job ID: {}", result.job.id);
    println!(
        "  Records processed: {}",
        result.job.number_records_processed
    );

    if let Some(csv_results) = result.results {
        let line_count = csv_results.lines().count();
        println!("  Total lines: {}", line_count);
        assert!(line_count >= 1, "Should have at least header");
    }
}

// ============================================================================
// error_handling.rs Example Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_error_handling_basic() {
    // Test basic error handling from error_handling.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Try to get a non-existent record
    let result: Result<serde_json::Value, _> =
        client.get("Account", "001000000000000AAA", None).await;

    match result {
        Ok(_) => {
            println!("Unexpectedly found account");
        }
        Err(e) => {
            println!("✓ Correctly received error: {}", e);
            println!("  Error type: {:?}", e.kind);
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_error_handling_limits() {
    // Test rate limit checking from error_handling.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let limits = client.limits().await.expect("Should retrieve limits");

    println!("✓ Retrieved org limits");

    // Check API usage
    if let Some(daily_api) = limits.get("DailyApiRequests") {
        if let (Some(max), Some(remaining)) = (
            daily_api.get("Max").and_then(|v| v.as_i64()),
            daily_api.get("Remaining").and_then(|v| v.as_i64()),
        ) {
            let usage_percent = ((max - remaining) as f64 / max as f64) * 100.0;
            println!(
                "  Daily API Usage: {:.1}% ({}/{})",
                usage_percent,
                max - remaining,
                max
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_example_error_handling_invalid_query() {
    // Test error handling for invalid queries from error_handling.rs example
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Invalid query
    let result = client
        .query::<serde_json::Value>("SELECT InvalidField FROM Account")
        .await;

    assert!(result.is_err(), "Invalid query should return error");
    println!("✓ Invalid query correctly returned error");
}

// ============================================================================
// Integration Test: All Examples Work Together
// ============================================================================

#[tokio::test]
#[ignore = "requires SF_AUTH_URL"]
async fn test_all_examples_integration() {
    println!("\n=== Running All Examples Integration Test ===\n");

    // 1. Authenticate (basic_auth.rs)
    let creds = get_test_credentials().await;
    println!("✓ Authentication successful");

    // 2. Create clients
    let rest_client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");
    let bulk_client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");
    let tooling_client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // 3. REST CRUD (rest_crud.rs)
    let account = serde_json::json!({
        "Name": format!("Integration Test {}", chrono::Utc::now().timestamp_millis())
    });
    let account_id = rest_client
        .create("Account", &account)
        .await
        .expect("Create should succeed");
    println!("✓ REST: Created account {}", account_id);

    // 4. Query with QueryBuilder (queries.rs)
    let accounts: Vec<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Id", &account_id)
        .expect("where_eq should succeed")
        .execute(&rest_client)
        .await
        .expect("Query should succeed");
    assert_eq!(accounts.len(), 1);
    println!("✓ Queries: Found account with QueryBuilder");

    // 5. Bulk query (bulk_operations.rs)
    let bulk_query: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Id", &account_id)
        .expect("where_eq should succeed");
    let bulk_result = bulk_client
        .execute_query(bulk_query)
        .await
        .expect("Bulk query should succeed");
    println!(
        "✓ Bulk: Query completed, {} records",
        bulk_result.job.number_records_processed
    );

    // 6. Tooling API (error_handling.rs uses this)
    let tooling_result = tooling_client
        .execute_anonymous("System.debug('All examples test');")
        .await
        .expect("Execute anonymous should succeed");
    assert!(tooling_result.compiled && tooling_result.success);
    println!("✓ Tooling: Executed anonymous Apex");

    // 7. Get limits (error_handling.rs)
    let limits = rest_client.limits().await.expect("Should get limits");
    assert!(limits.is_object());
    println!("✓ Error Handling: Retrieved limits");

    // 8. Clean up
    rest_client
        .delete("Account", &account_id)
        .await
        .expect("Delete should succeed");
    println!("✓ REST: Cleaned up test account");

    println!("\n=== All Examples Integration Test Passed ===\n");
}
