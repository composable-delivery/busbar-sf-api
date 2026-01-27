//! Error handling examples
//!
//! This example demonstrates proper error handling patterns for the Salesforce API:
//! - Error types and categorization
//! - Retry logic
//! - Rate limiting
//! - Authentication errors
//! - Transient vs permanent errors
//!
//! Run with: cargo run --example error_handling

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_client::ErrorKind;
use busbar_sf_rest::SalesforceRestClient;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging if needed

    println!("=== Salesforce Error Handling Examples ===\n");

    let creds = get_credentials().await?;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())?;

    // Error handling examples
    example_basic_error_handling(&client).await;
    example_retry_logic(&client).await;
    example_rate_limit_handling(&client).await;
    example_auth_error_detection(&client).await;
    example_error_categorization().await;

    println!("\n✓ All error handling examples completed!");

    Ok(())
}

/// Example 1: Basic error handling
async fn example_basic_error_handling(client: &SalesforceRestClient) {
    println!("Example 1: Basic Error Handling");
    println!("--------------------------------");

    // Try to get a record that might not exist
    let result: Result<serde_json::Value, _> = client
        .get("Account", "001000000000000", None) // Invalid ID
        .await;

    match result {
        Ok(account) => {
            println!("✓ Found account: {:?}", account);
        }
        Err(e) => {
            println!("✗ Error occurred: {}", e);
            println!("  Error type: {:?}", e.kind);

            // Check if it's a "not found" error
            if matches!(e.kind, busbar_sf_rest::ErrorKind::Salesforce { .. }) {
                println!("  This is a Salesforce API error");
            }
        }
    }

    println!();
}

/// Example 2: Retry logic for transient errors
async fn example_retry_logic(client: &SalesforceRestClient) {
    println!("Example 2: Retry Logic");
    println!("----------------------");

    let max_retries = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("Attempt {}/{}", attempt, max_retries);

        // Try to query accounts
        let result: Result<Vec<serde_json::Value>, _> =
            client.query_all("SELECT Id, Name FROM Account LIMIT 10").await;

        match result {
            Ok(accounts) => {
                println!("✓ Successfully retrieved {} accounts", accounts.len());
                break;
            }
            Err(e) => {
                println!("✗ Error: {}", e);

                // For demonstration, retry on any error (in production, check error type)
                if attempt < max_retries {
                    let backoff = Duration::from_secs(2u64.pow(attempt - 1));
                    println!("  Retrying after {:?}...", backoff);
                    sleep(backoff).await;
                } else {
                    println!("  Max retries reached");
                    break;
                }
            }
        }
    }

    println!();
}

/// Example 3: Rate limit handling
async fn example_rate_limit_handling(client: &SalesforceRestClient) {
    println!("Example 3: Rate Limit Handling");
    println!("-------------------------------");

    // Simulate checking org limits
    match client.limits().await {
        Ok(limits) => {
            println!("✓ Retrieved org limits");

            // Check API usage
            if let Some(daily_api) = limits.get("DailyApiRequests") {
                if let (Some(max), Some(remaining)) = (
                    daily_api.get("Max").and_then(|v| v.as_i64()),
                    daily_api.get("Remaining").and_then(|v| v.as_i64()),
                ) {
                    let usage_percent = ((max - remaining) as f64 / max as f64) * 100.0;
                    println!("  Daily API Usage: {:.1}% ({}/{})", usage_percent, max - remaining, max);

                    if usage_percent > 80.0 {
                        println!("  ⚠ Warning: API usage is above 80%!");
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Error retrieving limits: {}", e);
            println!("  Note: Check if rate limited or other error occurred");
        }
    }

    println!();
}

/// Example 4: Authentication error detection
async fn example_auth_error_detection(client: &SalesforceRestClient) {
    println!("Example 4: Authentication Error Detection");
    println!("------------------------------------------");

    // Try an operation (this will likely succeed with valid credentials)
    let result: Result<Vec<serde_json::Value>, _> =
        client.query_all("SELECT Id FROM Account LIMIT 1").await;

    match result {
        Ok(_) => {
            println!("✓ Authentication is valid");
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            // Check error kind
            if matches!(e.kind, busbar_sf_rest::ErrorKind::Auth(_)) {
                println!("  This is an authentication error!");
                println!("  Action: Refresh access token or re-authenticate");
            }
        }
    }

    println!();
}

/// Example 5: Error categorization
async fn example_error_categorization() {
    println!("Example 5: Error Categorization");
    println!("--------------------------------");

    // Create different error types for demonstration
    let errors = vec![
        ("Rate Limited", ErrorKind::RateLimited { retry_after: Some(Duration::from_secs(30)) }),
        ("Timeout", ErrorKind::Timeout),
        ("Authentication", ErrorKind::Authentication("Invalid token".to_string())),
        ("Not Found", ErrorKind::NotFound("Account".to_string())),
        ("Connection", ErrorKind::Connection("Network error".to_string())),
    ];

    for (name, error_kind) in errors {
        println!("\n{} Error:", name);
        println!("  Retryable: {}", error_kind.is_retryable());

        if let ErrorKind::RateLimited { retry_after } = error_kind {
            if let Some(duration) = retry_after {
                println!("  Retry after: {:?}", duration);
            }
        }
    }

    println!();
}

/// Example: Custom error handling with context
#[allow(dead_code)]
async fn create_account_with_context(
    client: &SalesforceRestClient,
    name: &str,
) -> Result<String, String> {
    let account = serde_json::json!({
        "Name": name
    });

    client
        .create("Account", &account)
        .await
        .map_err(|e| {
            // Add context to the error
            format!("Failed to create account '{}': {}", name, e)
        })
}

/// Example: Error recovery strategies
#[allow(dead_code)]
async fn query_with_fallback(
    client: &SalesforceRestClient,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    // Try primary query
    let result = client.query_all("SELECT Id, Name, CustomField__c FROM Account").await;

    match result {
        Ok(accounts) => Ok(accounts),
        Err(e) => {
            // If field doesn't exist, try without it
            if let busbar_sf_rest::ErrorKind::Salesforce { error_code, .. } = &e.kind {
                if error_code == "INVALID_FIELD" {
                    println!("  CustomField__c doesn't exist, trying without it...");
                    return client.query_all("SELECT Id, Name FROM Account").await.map_err(Into::into);
                }
            }
            Err(e.into())
        }
    }
}

/// Example: Bulk error handling
#[allow(dead_code)]
async fn process_with_partial_failures(
    client: &SalesforceRestClient,
    account_ids: Vec<String>,
) -> (Vec<serde_json::Value>, Vec<String>) {
    let mut successful = Vec::new();
    let mut failed = Vec::new();

    for id in account_ids {
        match client.get::<serde_json::Value>("Account", &id, None).await {
            Ok(account) => successful.push(account),
            Err(e) => {
                eprintln!("Failed to get account {}: {}", id, e);
                failed.push(id);
            }
        }
    }

    (successful, failed)
}

/// Helper: Exponential backoff implementation
#[allow(dead_code)]
async fn with_exponential_backoff<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(e);
                }

                let backoff = Duration::from_millis(100 * 2u64.pow(attempt - 1));
                println!("  Attempt {} failed, retrying after {:?}...", attempt, backoff);
                sleep(backoff).await;
            }
        }
    }
}

/// Helper function to get credentials
async fn get_credentials() -> Result<SalesforceCredentials, Box<dyn std::error::Error>> {
    if let Ok(creds) = SalesforceCredentials::from_sfdx_alias("default").await {
        println!("✓ Using credentials from Salesforce CLI\n");
        return Ok(creds);
    }

    match SalesforceCredentials::from_env() {
        Ok(creds) => {
            println!("✓ Using credentials from environment variables\n");
            Ok(creds)
        }
        Err(e) => {
            eprintln!("✗ Failed to load credentials: {}", e);
            eprintln!("\nPlease either:");
            eprintln!("  1. Authenticate with Salesforce CLI: sf org login web");
            eprintln!("  2. Set environment variables: SF_INSTANCE_URL, SF_ACCESS_TOKEN");
            Err(e.into())
        }
    }
}
