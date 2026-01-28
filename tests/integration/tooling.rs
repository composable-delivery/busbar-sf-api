//! Tooling API integration tests using SF_AUTH_URL.

use busbar_sf_auth::Credentials;
use super::common::require_credentials;
use busbar_sf_tooling::ToolingClient;

// ============================================================================
// Tooling API Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_query_apex_classes() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

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
async fn test_tooling_execute_anonymous_success() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

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
async fn test_tooling_execute_anonymous_compile_error() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let invalid_apex = "this is not valid apex code at all;";

    let result = client.execute_anonymous(invalid_apex).await;

    assert!(result.is_err(), "Invalid Apex should return error");
}

#[tokio::test]
async fn test_tooling_query_all_pagination() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM ApexClass LIMIT 50")
        .await
        .expect("query_all should succeed");

    assert!(records.len() <= 50, "Should respect LIMIT");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_error_invalid_query() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, NotAField__c FROM ApexClass")
        .await;

    assert!(result.is_err(), "Tooling query with invalid field should fail");
}

#[tokio::test]
async fn test_tooling_error_invalid_sobject_create_get_delete() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let create_result = client
        .create("ApexClass; DROP", &serde_json::json!({"Name": "Bad"}))
        .await;

    assert!(create_result.is_err(), "Create with invalid SObject should fail");

    let get_result: Result<serde_json::Value, _> = client.get("ApexClass", "bad-id").await;

    assert!(get_result.is_err(), "Get with invalid ID should fail");

    let delete_result = client.delete("ApexClass", "bad-id").await;

    assert!(delete_result.is_err(), "Delete with invalid ID should fail");
}

#[tokio::test]
async fn test_tooling_error_invalid_log_id() {
    let Some(creds) = require_credentials().await else { return; };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.get_apex_log_body("bad-id").await;

    assert!(result.is_err(), "Log body with invalid ID should fail");
}
