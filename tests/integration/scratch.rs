//! Integration tests for busbar-sf-api using a scratch org.
//!
//! These tests require a Salesforce scratch org to be authenticated.
//! Run with: `cargo test --test integration -- --ignored --nocapture`

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_tooling::ToolingClient;

/// Default scratch org alias for tests.
const DEFAULT_ORG_ALIAS: &str = "roundtrip-org-a";

/// Get the org alias to use for tests.
fn org_alias() -> String {
    std::env::var("SF_TEST_ORG_ALIAS").unwrap_or_else(|_| DEFAULT_ORG_ALIAS.to_string())
}

fn require_scratch_credentials() -> bool {
    if std::env::var("SF_AUTH_URL").is_ok() || std::env::var("SF_TEST_ORG_ALIAS").is_ok() {
        true
    } else {
        eprintln!("skipping: SF_AUTH_URL or SF_TEST_ORG_ALIAS not set");
        false
    }
}

/// Helper to get authenticated credentials from SFDX CLI or SF_AUTH_URL.
async fn get_test_credentials() -> SalesforceCredentials {
    if let Ok(auth_url) = std::env::var("SF_AUTH_URL") {
        return SalesforceCredentials::from_sfdx_auth_url(&auth_url)
            .await
            .unwrap_or_else(|_| panic!("Failed to authenticate from SF_AUTH_URL"));
    }

    let alias = org_alias();
    SalesforceCredentials::from_sfdx_alias(&alias)
        .await
        .unwrap_or_else(|_| panic!("Failed to get credentials for org: {}", alias))
}

// ============================================================================
// Authentication Tests
// ============================================================================

#[tokio::test]
async fn test_sfdx_authentication() {
    if !require_scratch_credentials() {
        return;
    }
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

    let instance_url = creds.instance_url();
    assert!(
        instance_url.contains(".salesforce.com") || instance_url.contains(".my.salesforce.com"),
        "Instance URL should be a Salesforce domain: {}",
        instance_url
    );
}

#[tokio::test]
async fn test_credentials_debug_redaction() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;

    let debug_output = format!("{:?}", creds);
    assert!(
        debug_output.contains("[REDACTED]"),
        "Debug should contain [REDACTED]"
    );
    assert!(
        !debug_output.contains(creds.access_token()),
        "Debug should not contain the actual token"
    );
}

// ============================================================================
// REST API Tests
// ============================================================================

#[tokio::test]
async fn test_rest_api_versions() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let versions = client.versions().await.expect("Failed to get API versions");

    assert!(!versions.is_empty(), "Should have at least one API version");

    let has_v60_plus = versions.iter().any(|v| {
        let version_num: f64 = v.version.parse().unwrap_or(0.0);
        version_num >= 60.0
    });
    assert!(has_v60_plus, "Should have API version 60.0 or higher");
}

#[tokio::test]
async fn test_rest_api_limits() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let limits = client.limits().await.expect("Failed to get limits");

    assert!(limits.is_object(), "Limits should be a JSON object");
    let limits_obj = limits.as_object().unwrap();

    assert!(
        limits_obj.contains_key("DailyApiRequests"),
        "Should have DailyApiRequests limit"
    );
    assert!(
        limits_obj.contains_key("DailyBulkV2QueryJobs"),
        "Should have DailyBulkV2QueryJobs limit"
    );
}

#[tokio::test]
async fn test_rest_describe_global() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let describe = client
        .describe_global()
        .await
        .expect("Failed to describe global");

    assert!(!describe.sobjects.is_empty(), "Should have SObjects");

    let sobject_names: Vec<&str> = describe.sobjects.iter().map(|s| s.name.as_str()).collect();
    assert!(
        sobject_names.contains(&"Account"),
        "Should have Account object"
    );
    assert!(
        sobject_names.contains(&"Contact"),
        "Should have Contact object"
    );
    assert!(sobject_names.contains(&"Lead"), "Should have Lead object");
}

#[tokio::test]
async fn test_rest_describe_sobject() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let describe = client
        .describe_sobject("Account")
        .await
        .expect("Failed to describe Account");

    assert_eq!(describe.name, "Account", "Should be describing Account");
    assert!(!describe.fields.is_empty(), "Account should have fields");

    let field_names: Vec<&str> = describe.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"Id"), "Account should have Id field");
    assert!(field_names.contains(&"Name"), "Account should have Name field");
}

#[tokio::test]
async fn test_rest_query() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    assert!(
        result.done || result.next_records_url.is_some(),
        "Query should complete or have pagination"
    );

    for record in &result.records {
        assert!(record.get("Id").is_some(), "Record should have Id");
    }
}

#[tokio::test]
async fn test_rest_crud_lifecycle() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let test_name = format!("Test Account {}", chrono::Utc::now().timestamp_millis());
    let account_data = serde_json::json!({
        "Name": test_name,
        "Description": "Created by busbar-sf-api integration test"
    });

    let id = client
        .create("Account", &account_data)
        .await
        .expect("Create should succeed");

    assert!(!id.is_empty(), "Should return an ID");
    assert!(id.starts_with("001"), "Account ID should start with 001");

    let retrieved: serde_json::Value = client
        .get("Account", &id, Some(&["Id", "Name", "Description"]))
        .await
        .expect("Get should succeed");

    assert_eq!(
        retrieved.get("Name").and_then(|v| v.as_str()),
        Some(test_name.as_str())
    );

    let update_data = serde_json::json!({
        "Description": "Updated by busbar-sf-api integration test"
    });

    client
        .update("Account", &id, &update_data)
        .await
        .expect("Update should succeed");

    let updated: serde_json::Value = client
        .get("Account", &id, Some(&["Description"]))
        .await
        .expect("Get after update should succeed");

    assert_eq!(
        updated.get("Description").and_then(|v| v.as_str()),
        Some("Updated by busbar-sf-api integration test")
    );

    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");

    let delete_result = client.get::<serde_json::Value>("Account", &id, None).await;
    assert!(delete_result.is_err(), "Get deleted record should fail");
}

// ============================================================================
// Tooling API Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_query() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let classes = client
        .query::<serde_json::Value>("SELECT Id, Name FROM ApexClass LIMIT 5")
        .await
        .expect("Tooling query should succeed");

    assert!(
        classes.done || classes.next_records_url.is_some(),
        "Query should complete"
    );
}

#[tokio::test]
async fn test_tooling_execute_anonymous() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .execute_anonymous("System.debug('Hello from busbar-sf-api integration test');")
        .await
        .expect("Execute anonymous should succeed");

    assert!(result.compiled, "Apex should compile");
    assert!(result.success, "Apex should execute successfully");
}

#[tokio::test]
async fn test_tooling_execute_anonymous_with_error() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.execute_anonymous("this is not valid apex;").await;

    assert!(result.is_err(), "Invalid Apex should return an error");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_query_error() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT InvalidField FROM Account")
        .await;

    assert!(result.is_err(), "Invalid query should return an error");
}

#[tokio::test]
async fn test_invalid_sobject_error() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.describe_sobject("NonExistentObject__c").await;

    assert!(
        result.is_err(),
        "Describing non-existent object should return an error"
    );
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
async fn test_client_debug_redacts_token() {
    if !require_scratch_credentials() {
        return;
    }
    let creds = get_test_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let debug_output = format!("{:?}", client);

    assert!(
        debug_output.contains("[REDACTED]") || debug_output.contains("SalesforceRestClient"),
        "Debug should be safe"
    );

    assert!(
        !debug_output.contains(creds.access_token()),
        "Debug output should not contain the actual access token"
    );
}
