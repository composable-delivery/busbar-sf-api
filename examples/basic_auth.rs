//! Basic authentication examples for Salesforce API
//!
//! This example demonstrates various authentication methods:
//! - Environment variables
//! - Salesforce CLI (SFDX)
//! - OAuth 2.0 flows
//! - JWT Bearer flow
//!
//! Run with: cargo run --example basic_auth

use busbar_sf_auth::{JwtAuth, OAuthClient, OAuthConfig, SalesforceCredentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("=== Salesforce Authentication Examples ===\n");

    // Example 1: Load from environment variables
    example_from_env().await?;

    // Example 2: Load from Salesforce CLI
    example_from_sfdx().await?;

    // Example 3: OAuth 2.0 Refresh Token
    example_oauth_refresh().await?;

    // Example 4: JWT Bearer Flow
    example_jwt_auth().await?;

    Ok(())
}

/// Example 1: Load credentials from environment variables
///
/// Required environment variables:
/// - SF_INSTANCE_URL or SALESFORCE_INSTANCE_URL
/// - SF_ACCESS_TOKEN or SALESFORCE_ACCESS_TOKEN
/// Optional:
/// - SF_API_VERSION or SALESFORCE_API_VERSION
/// - SF_REFRESH_TOKEN or SALESFORCE_REFRESH_TOKEN
async fn example_from_env() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Load from Environment Variables");
    println!("-------------------------------------------");

    match SalesforceCredentials::from_env() {
        Ok(creds) => {
            println!("✓ Loaded credentials from environment");
            println!("  Instance URL: {}", creds.instance_url());
            println!("  API Version: {}", creds.api_version());
            println!("  Has Refresh Token: {}", creds.refresh_token().is_some());
        }
        Err(e) => {
            println!("✗ Failed to load from environment: {}", e);
            println!("  Tip: Set SF_INSTANCE_URL and SF_ACCESS_TOKEN");
        }
    }

    println!();
    Ok(())
}

/// Example 2: Load credentials from Salesforce CLI (SFDX)
///
/// Requires 'sf' CLI to be installed and an org to be authenticated
async fn example_from_sfdx() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Load from Salesforce CLI");
    println!("------------------------------------");

    // Try to load from default org
    match SalesforceCredentials::from_sfdx_alias("default").await {
        Ok(creds) => {
            println!("✓ Loaded credentials from SFDX CLI");
            println!("  Instance URL: {}", creds.instance_url());
            println!("  API Version: {}", creds.api_version());
        }
        Err(e) => {
            println!("✗ Failed to load from SFDX: {}", e);
            println!("  Tip: Run 'sf org login web' or 'sf org login jwt'");
            println!("  Or specify an alias: SalesforceCredentials::from_sfdx_alias(\"my-org\")");
        }
    }

    println!();
    Ok(())
}

/// Example 3: OAuth 2.0 Refresh Token
///
/// This example shows how to refresh an access token using a refresh token
async fn example_oauth_refresh() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: OAuth 2.0 Refresh Token");
    println!("-----------------------------------");

    // Configuration from environment or hardcoded for example
    let consumer_key = std::env::var("SF_CONSUMER_KEY")
        .unwrap_or_else(|_| "your_consumer_key".to_string());
    let consumer_secret = std::env::var("SF_CONSUMER_SECRET").ok();
    let refresh_token = std::env::var("SF_REFRESH_TOKEN")
        .unwrap_or_else(|_| "your_refresh_token".to_string());

    if consumer_key == "your_consumer_key" {
        println!("✗ OAuth not configured");
        println!("  Tip: Set SF_CONSUMER_KEY and SF_REFRESH_TOKEN");
        println!();
        return Ok(());
    }

    let mut config = OAuthConfig::new(consumer_key);
    if let Some(secret) = consumer_secret {
        config = config.with_secret(secret);
    }

    let oauth = OAuthClient::new(config);

    match oauth
        .refresh_token(&refresh_token, "https://login.salesforce.com")
        .await
    {
        Ok(token_response) => {
            println!("✓ Successfully refreshed access token");
            println!("  Instance URL: {}", token_response.instance_url);
            println!("  Token Type: {:?}", token_response.token_type);

            // Convert to credentials
            let creds = token_response.to_credentials("62.0");
            println!("  API URL: {}", creds.rest_api_url());
        }
        Err(e) => {
            println!("✗ Failed to refresh token: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 4: JWT Bearer Flow
///
/// This is ideal for server-to-server integration without user interaction
async fn example_jwt_auth() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: JWT Bearer Flow");
    println!("---------------------------");

    let consumer_key = std::env::var("SF_JWT_CONSUMER_KEY")
        .unwrap_or_else(|_| "your_consumer_key".to_string());
    let username = std::env::var("SF_JWT_USERNAME")
        .unwrap_or_else(|_| "user@example.com".to_string());
    let key_path = std::env::var("SF_JWT_KEY_PATH")
        .unwrap_or_else(|_| "/path/to/server.key".to_string());

    if consumer_key == "your_consumer_key" {
        println!("✗ JWT not configured");
        println!("  Tip: Set SF_JWT_CONSUMER_KEY, SF_JWT_USERNAME, and SF_JWT_KEY_PATH");
        println!("  The private key should be in PEM format (RSA)");
        println!();
        return Ok(());
    }

    match JwtAuth::from_key_file(consumer_key, username, &key_path) {
        Ok(jwt_auth) => {
            // Authenticate with production
            match jwt_auth.authenticate_production().await {
                Ok(creds) => {
                    println!("✓ Successfully authenticated with JWT");
                    println!("  Instance URL: {}", creds.instance_url());
                    println!("  API Version: {}", creds.api_version());
                }
                Err(e) => {
                    println!("✗ JWT authentication failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load JWT key: {}", e);
        }
    }

    println!();
    Ok(())
}

// Additional helper: Generate OAuth authorization URL
#[allow(dead_code)]
fn generate_oauth_url() {
    use busbar_sf_auth::WebFlowAuth;

    let config = OAuthConfig::new("your_consumer_key")
        .with_redirect_uri("http://localhost:8080/callback")
        .with_scopes(vec!["api".to_string(), "refresh_token".to_string()]);

    let web_flow = WebFlowAuth::new(config).unwrap();
    let auth_url = web_flow.authorization_url("https://login.salesforce.com", Some("random_state_123"));

    println!("OAuth Authorization URL:");
    println!("{}", auth_url);
    println!("\nVisit this URL in a browser to authorize the application");
}
