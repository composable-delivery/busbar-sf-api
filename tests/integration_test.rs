//! Integration tests for busbar-sf-api.
//!
//! These tests require a Salesforce scratch org to be authenticated.
//! Run with: `cargo test --test integration_test -- --ignored`
//!
//! Prerequisites:
//! - SF CLI installed and authenticated
//! - A scratch org with the alias specified in ORG_ALIAS constant
//!
//! Environment variables:
//! - SF_TEST_ORG_ALIAS: Override the default org alias (optional)

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_tooling::ToolingClient;

/// Default scratch org alias for tests.
const DEFAULT_ORG_ALIAS: &str = "roundtrip-org-a";

/// Get the org alias to use for tests.
fn org_alias() -> String {
    std::env::var("SF_TEST_ORG_ALIAS").unwrap_or_else(|_| DEFAULT_ORG_ALIAS.to_string())
}

/// Helper to get authenticated credentials from SFDX CLI.
async fn get_test_credentials() -> SalesforceCredentials {
    let alias = org_alias();
    SalesforceCredentials::from_sfdx_alias(&alias)
        .await
        .expect(&format!("Failed to get credentials for org: {}", alias))
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_sfdx_authentication() {
    let creds = get_test_credentials().await;

    assert!(creds.is_valid(), "Credentials should be valid");
    assert!(!creds.instance_url().is_empty(), "Instance URL should be set");
    assert!(!creds.access_token().is_empty(), "Access token should be set");

    // Verify the instance URL is a Salesforce URL
    let instance_url = creds.instance_url();
    assert!(
        instance_url.contains(".salesforce.com") || instance_url.contains(".my.salesforce.com"),
        "Instance URL should be a Salesforce domain: {}",
        instance_url
    );
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_credentials_debug_redaction() {
    let creds = get_test_credentials().await;

    // Ensure Debug output doesn't expose the token
    let debug_output = format!("{:?}", creds);
    assert!(debug_output.contains("[REDACTED]"), "Debug should contain [REDACTED]");
    assert!(!debug_output.contains(creds.access_token()), "Debug should not contain the actual token");
}

// ============================================================================
// REST API Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_api_versions() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    let versions = client.versions().await.expect("Failed to get API versions");

    assert!(!versions.is_empty(), "Should have at least one API version");

    // Check that we have a recent API version
    let has_v60_plus = versions.iter().any(|v| {
        let version_num: f64 = v.version.parse().unwrap_or(0.0);
        version_num >= 60.0
    });
    assert!(has_v60_plus, "Should have API version 60.0 or higher");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_api_limits() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    let limits = client.limits().await.expect("Failed to get limits");

    // Verify we got a JSON object with expected keys
    assert!(limits.is_object(), "Limits should be a JSON object");
    let limits_obj = limits.as_object().unwrap();

    // Check for common limit types
    assert!(limits_obj.contains_key("DailyApiRequests"), "Should have DailyApiRequests limit");
    assert!(limits_obj.contains_key("DailyBulkV2QueryJobs"), "Should have DailyBulkV2QueryJobs limit");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_describe_global() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    let describe = client.describe_global().await.expect("Failed to describe global");

    // Verify we got SObjects
    assert!(!describe.sobjects.is_empty(), "Should have SObjects");

    // Check for standard objects
    let sobject_names: Vec<&str> = describe.sobjects.iter().map(|s| s.name.as_str()).collect();
    assert!(sobject_names.contains(&"Account"), "Should have Account object");
    assert!(sobject_names.contains(&"Contact"), "Should have Contact object");
    assert!(sobject_names.contains(&"Lead"), "Should have Lead object");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_describe_sobject() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    let describe = client.describe_sobject("Account").await.expect("Failed to describe Account");

    assert_eq!(describe.name, "Account", "Should be describing Account");
    assert!(!describe.fields.is_empty(), "Account should have fields");

    // Check for standard fields
    let field_names: Vec<&str> = describe.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"Id"), "Account should have Id field");
    assert!(field_names.contains(&"Name"), "Account should have Name field");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_query() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    // Query for a small set of Accounts
    let result = client
        .query::<serde_json::Value>("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    // Verify query result structure
    assert!(result.done || result.next_records_url.is_some(), "Query should complete or have pagination");

    // If there are records, verify structure
    for record in &result.records {
        assert!(record.get("Id").is_some(), "Record should have Id");
        // Name might be null, so we just check Id
    }
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_rest_crud_lifecycle() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    // Create a test Account
    let test_name = format!("Test Account {}", chrono::Utc::now().timestamp_millis());
    let account_data = serde_json::json!({
        "Name": test_name,
        "Description": "Created by busbar-sf-api integration test"
    });

    // CREATE
    let id = client
        .create("Account", &account_data)
        .await
        .expect("Create should succeed");

    assert!(!id.is_empty(), "Should return an ID");
    assert!(id.starts_with("001"), "Account ID should start with 001");

    // READ
    let retrieved: serde_json::Value = client
        .get("Account", &id, Some(&["Id", "Name", "Description"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.get("Name").and_then(|v| v.as_str()), Some(test_name.as_str()));

    // UPDATE
    let update_data = serde_json::json!({
        "Description": "Updated by busbar-sf-api integration test"
    });

    client
        .update("Account", &id, &update_data)
        .await
        .expect("Update should succeed");

    // Verify update
    let updated: serde_json::Value = client
        .get("Account", &id, Some(&["Description"]))
        .await
        .expect("Get after update should succeed");

    assert_eq!(
        updated.get("Description").and_then(|v| v.as_str()),
        Some("Updated by busbar-sf-api integration test")
    );

    // DELETE
    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");

    // Verify deletion
    let delete_result = client.get::<serde_json::Value>("Account", &id, None).await;
    assert!(delete_result.is_err(), "Get deleted record should fail");
}

// ============================================================================
// Tooling API Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_tooling_query() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create Tooling client");

    // Query ApexClass
    let classes = client
        .query::<serde_json::Value>("SELECT Id, Name FROM ApexClass LIMIT 5")
        .await
        .expect("Tooling query should succeed");

    // Verify the query completed (might be empty if no Apex classes exist)
    assert!(classes.done || classes.next_records_url.is_some(), "Query should complete");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_tooling_execute_anonymous() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create Tooling client");

    // Execute simple Apex
    let result = client
        .execute_anonymous("System.debug('Hello from busbar-sf-api integration test');")
        .await
        .expect("Execute anonymous should succeed");

    assert!(result.compiled, "Apex should compile");
    assert!(result.success, "Apex should execute successfully");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_tooling_execute_anonymous_with_error() {
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create Tooling client");

    // Execute Apex with a compilation error
    let result = client
        .execute_anonymous("this is not valid apex;")
        .await;

    // Should return an error due to compilation failure
    assert!(result.is_err(), "Invalid Apex should return an error");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_invalid_query_error() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    // Execute an invalid query
    let result = client
        .query::<serde_json::Value>("SELECT InvalidField FROM Account")
        .await;

    assert!(result.is_err(), "Invalid query should return an error");
}

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_invalid_sobject_error() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    // Try to describe a non-existent SObject
    let result = client.describe_sobject("NonExistentObject__c").await;

    assert!(result.is_err(), "Describing non-existent object should return an error");
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
#[ignore = "requires scratch org"]
async fn test_client_debug_redacts_token() {
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).expect("Failed to create REST client");

    // Get the debug output
    let debug_output = format!("{:?}", client);

    // Should contain redacted marker
    assert!(debug_output.contains("[REDACTED]") || debug_output.contains("SalesforceRestClient"),
            "Debug should be safe");

    // Should not contain the actual token
    assert!(!debug_output.contains(creds.access_token()),
            "Debug output should not contain the actual access token");
}
