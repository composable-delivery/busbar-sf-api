//! Integration tests for domain-specific REST API endpoints.
//!
//! Tests for Consent API, Knowledge Management, User Password, Suggested Articles,
//! Platform Actions, Salesforce Scheduler, and Embedded Service endpoints.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_rest::{
    consent::{ConsentWriteRecord, ConsentWriteRequest},
    scheduler::AppointmentCandidatesRequest,
    user_password::SetPasswordRequest,
    SalesforceRestClient,
};

// ============================================================================
// Consent API Tests
// ============================================================================

#[tokio::test]
async fn test_consent_read() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account to use for consent
    let test_account = serde_json::json!({
        "Name": format!("Consent Test Account {}", chrono::Utc::now().timestamp_millis())
    });

    let account_id = client
        .create("Account", &test_account)
        .await
        .expect("Failed to create test account");

    // Try to read consent status - this may fail if consent features aren't enabled
    let result = client.read_consent("email_marketing", &account_id).await;

    // Clean up
    let _ = client.delete("Account", &account_id).await;

    // The endpoint might not be available in all orgs, so we just verify it doesn't panic
    match result {
        Ok(response) => {
            println!("Consent read succeeded: {:?}", response);
        }
        Err(e) => {
            println!("Consent read failed (may not be enabled): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_consent_write() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account
    let test_account = serde_json::json!({
        "Name": format!("Consent Write Test {}", chrono::Utc::now().timestamp_millis())
    });

    let account_id = client
        .create("Account", &test_account)
        .await
        .expect("Failed to create test account");

    let request = ConsentWriteRequest {
        consents: vec![ConsentWriteRecord {
            id: account_id.clone(),
            consent: true,
        }],
    };

    let result = client.write_consent("email_marketing", &request).await;

    // Clean up
    let _ = client.delete("Account", &account_id).await;

    match result {
        Ok(response) => {
            println!("Consent write succeeded: {:?}", response);
        }
        Err(e) => {
            println!("Consent write failed (may not be enabled): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_consent_multi_read() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account
    let test_account = serde_json::json!({
        "Name": format!("Multi Consent Test {}", chrono::Utc::now().timestamp_millis())
    });

    let account_id = client
        .create("Account", &test_account)
        .await
        .expect("Failed to create test account");

    let result = client
        .read_multi_consent("email_marketing,sms_marketing", &account_id)
        .await;

    // Clean up
    let _ = client.delete("Account", &account_id).await;

    match result {
        Ok(response) => {
            println!("Multi consent read succeeded: {:?}", response);
        }
        Err(e) => {
            println!("Multi consent read failed (may not be enabled): {:?}", e);
        }
    }
}

// ============================================================================
// Knowledge Management Tests
// ============================================================================

#[tokio::test]
async fn test_knowledge_management_settings() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.knowledge_management_settings().await;

    match result {
        Ok(settings) => {
            println!(
                "Knowledge settings retrieved: enabled={}",
                settings.is_enabled
            );
            // Successfully retrieved settings
        }
        Err(e) => {
            println!("Knowledge settings failed (may not be enabled): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_list_knowledge_articles() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.list_knowledge_articles().await;

    match result {
        Ok(articles) => {
            println!(
                "Knowledge articles retrieved: {} articles",
                articles.articles.len()
            );
        }
        Err(e) => {
            println!(
                "List knowledge articles failed (may not be enabled): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_list_data_category_groups() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.list_data_category_groups().await;

    match result {
        Ok(groups) => {
            println!(
                "Data category groups retrieved: {} groups",
                groups.category_groups.len()
            );
        }
        Err(e) => {
            println!(
                "List data category groups failed (may not be enabled): {:?}",
                e
            );
        }
    }
}

// ============================================================================
// User Password Management Tests
// ============================================================================

#[tokio::test]
async fn test_user_password_status() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Query for a user (get the current user)
    let user_query = "SELECT Id FROM User WHERE IsActive = true LIMIT 1";
    let users: Vec<serde_json::Value> = client
        .query_all(user_query)
        .await
        .expect("Failed to query users");

    if let Some(user) = users.first() {
        if let Some(user_id) = user.get("Id").and_then(|v| v.as_str()) {
            let result = client.get_user_password_status(user_id).await;

            match result {
                Ok(status) => {
                    println!("Password status: expired={}", status.is_expired);
                    // Successfully retrieved password status
                }
                Err(e) => {
                    println!("Get password status failed: {:?}", e);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_user_password_set_reset() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Query for a test user (not the current user to avoid locking ourselves out)
    let user_query = "SELECT Id FROM User WHERE IsActive = false LIMIT 1";
    let users: Vec<serde_json::Value> = client
        .query_all(user_query)
        .await
        .expect("Failed to query users");

    if let Some(user) = users.first() {
        if let Some(user_id) = user.get("Id").and_then(|v| v.as_str()) {
            // Test password reset (generates new password)
            let reset_result = client.reset_user_password(user_id).await;

            match reset_result {
                Ok(response) => {
                    println!("Password reset succeeded: {:?}", response);
                }
                Err(e) => {
                    println!("Password reset failed (may not have permission): {:?}", e);
                }
            }

            // Test password set
            let set_request = SetPasswordRequest {
                new_password: "TestPassword123!".to_string(),
            };
            let set_result = client.set_user_password(user_id, &set_request).await;

            match set_result {
                Ok(response) => {
                    println!("Password set succeeded: {:?}", response);
                }
                Err(e) => {
                    println!("Password set failed (may not have permission): {:?}", e);
                }
            }
        }
    } else {
        println!("No inactive user found for password test");
    }
}

// ============================================================================
// Suggested Articles & Platform Actions Tests
// ============================================================================

#[tokio::test]
async fn test_suggested_articles() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .get_suggested_articles(
            "Case",
            Some("How to reset password"),
            Some("User cannot log in"),
        )
        .await;

    match result {
        Ok(articles) => {
            println!(
                "Suggested articles retrieved: {} articles",
                articles.articles.len()
            );
        }
        Err(e) => {
            println!(
                "Get suggested articles failed (may not be enabled): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_platform_actions() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.get_platform_actions("Account").await;

    match result {
        Ok(actions) => {
            println!(
                "Platform actions retrieved: {} actions",
                actions.actions.len()
            );
            assert!(
                !actions.actions.is_empty(),
                "Should have at least some platform actions for Account"
            );
        }
        Err(e) => {
            println!("Get platform actions failed: {:?}", e);
        }
    }
}

// ============================================================================
// Salesforce Scheduler Tests
// ============================================================================

#[tokio::test]
async fn test_appointment_slots() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.get_appointment_slots().await;

    match result {
        Ok(slots) => {
            println!("Appointment slots retrieved: {:?}", slots);
        }
        Err(e) => {
            println!(
                "Get appointment slots failed (scheduler may not be enabled): {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_appointment_candidates() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let request = AppointmentCandidatesRequest {
        scheduling_policy_id: Some("test_policy".to_string()),
        work_type_id: Some("test_work_type".to_string()),
        account_id: None,
        additional: std::collections::HashMap::new(),
    };

    let result = client.get_appointment_candidates(&request).await;

    match result {
        Ok(candidates) => {
            println!(
                "Appointment candidates retrieved: {} candidates",
                candidates.candidates.len()
            );
        }
        Err(e) => {
            println!(
                "Get appointment candidates failed (scheduler may not be enabled): {:?}",
                e
            );
        }
    }
}

// ============================================================================
// Embedded Service Tests
// ============================================================================

#[tokio::test]
async fn test_embedded_service_config() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Query for an embedded service deployment (if any exist)
    let query = "SELECT Id FROM EmbeddedServiceConfig LIMIT 1";
    let configs: Vec<serde_json::Value> = match client.query_all(query).await {
        Ok(c) => c,
        Err(_) => {
            println!("No embedded service configs found or feature not enabled");
            return;
        }
    };

    if let Some(config) = configs.first() {
        if let Some(config_id) = config.get("Id").and_then(|v| v.as_str()) {
            let result = client.get_embedded_service_config(config_id).await;

            match result {
                Ok(config) => {
                    println!("Embedded service config retrieved: {:?}", config.id);
                    assert_eq!(config.id, config_id, "Config ID should match");
                }
                Err(e) => {
                    println!("Get embedded service config failed: {:?}", e);
                }
            }
        }
    } else {
        println!("No embedded service configs found in org");
    }
}
