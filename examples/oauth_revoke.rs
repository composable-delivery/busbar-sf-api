//! OAuth token revocation example
//!
//! This example demonstrates how to revoke OAuth tokens in Salesforce.
//! Token revocation is essential for:
//! - Clean session termination
//! - Security-sensitive applications
//! - Logout functionality
//!
//! Run with: cargo run --example oauth_revoke

use busbar_sf_auth::{
    Credentials, OAuthClient, OAuthConfig, SalesforceCredentials, PRODUCTION_LOGIN_URL,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OAuth Token Revocation Example ===\n");

    // Example 1: Revoke using OAuthClient directly
    example_revoke_with_oauth_client().await?;

    // Example 2: Revoke using SalesforceCredentials convenience method
    example_revoke_session().await?;

    Ok(())
}

/// Example 1: Revoke a token using OAuthClient directly
///
/// This is useful when you have a standalone token to revoke
async fn example_revoke_with_oauth_client() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Revoke Token with OAuthClient");
    println!("------------------------------------------");

    // Create a minimal OAuth client (consumer_key not needed for revocation)
    let config = OAuthConfig::new("revoke_client");
    let client = OAuthClient::new(config);

    // Get token from environment (or use a test token)
    let token = std::env::var("SF_TOKEN_TO_REVOKE").unwrap_or_else(|_| {
        println!("✗ No token to revoke");
        println!("  Tip: Set SF_TOKEN_TO_REVOKE environment variable");
        println!("  This can be either an access token or refresh token\n");
        return String::new();
    });

    if token.is_empty() {
        return Ok(());
    }

    // Determine login URL (production or sandbox)
    let login_url = std::env::var("SF_LOGIN_URL").unwrap_or_else(|_| PRODUCTION_LOGIN_URL.to_string());

    match client.revoke_token(&token, &login_url).await {
        Ok(_) => {
            println!("✓ Token successfully revoked");
            println!("  Note: This is idempotent - revoking an already invalid token also succeeds");
        }
        Err(e) => {
            println!("✗ Failed to revoke token: {}", e);
        }
    }

    println!();
    Ok(())
}

/// Example 2: Revoke session using SalesforceCredentials convenience method
///
/// This is useful when you have active credentials and want to terminate the session
async fn example_revoke_session() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Revoke Session with SalesforceCredentials");
    println!("-----------------------------------------------------");

    // Try to load credentials from environment
    let creds = match SalesforceCredentials::from_env() {
        Ok(c) => c,
        Err(_) => {
            println!("✗ No credentials available");
            println!("  Tip: Set SF_INSTANCE_URL and SF_ACCESS_TOKEN");
            println!("  Optional: Set SF_REFRESH_TOKEN to demonstrate refresh token revocation\n");
            return Ok(());
        }
    };

    // Determine login URL based on instance URL
    let login_url = if creds.instance_url().contains("test.salesforce.com") {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // Example 2a: Revoke just the access token
    println!("\n2a. Revoking access token only:");
    match creds.revoke_session(false, login_url).await {
        Ok(_) => {
            println!("   ✓ Access token revoked");
            println!("   Note: Refresh token (if present) is still valid");
        }
        Err(e) => {
            println!("   ✗ Failed to revoke access token: {}", e);
        }
    }

    // Example 2b: Revoke the entire session (refresh token + all access tokens)
    if creds.refresh_token().is_some() {
        println!("\n2b. Revoking refresh token (terminates entire session):");
        match creds.revoke_session(true, login_url).await {
            Ok(_) => {
                println!("   ✓ Refresh token revoked");
                println!("   Note: This also invalidated all associated access tokens");
            }
            Err(e) => {
                println!("   ✗ Failed to revoke refresh token: {}", e);
            }
        }
    } else {
        println!("\n2b. Skipping refresh token revocation (no refresh token available)");
    }

    println!();
    Ok(())
}

/// Additional example: Best practices for logout flow
#[allow(dead_code)]
fn logout_best_practices() {
    println!("=== Token Revocation Best Practices ===\n");

    println!("1. Complete Session Termination (Logout):");
    println!("   - Revoke the refresh token (revoke_session(true, ...))");
    println!("   - This invalidates ALL access tokens associated with the refresh token");
    println!("   - Use when user explicitly logs out\n");

    println!("2. Single Token Invalidation:");
    println!("   - Revoke just the access token (revoke_session(false, ...))");
    println!("   - The refresh token remains valid");
    println!("   - Use for token rotation or when only one client needs to be logged out\n");

    println!("3. Idempotency:");
    println!("   - Revoking an already invalid token returns success");
    println!("   - This prevents information leakage about token validity");
    println!("   - Safe to call multiple times\n");

    println!("4. Security:");
    println!("   - Always revoke tokens when they're no longer needed");
    println!("   - Revoke tokens on user logout");
    println!("   - Revoke tokens when detecting suspicious activity");
    println!("   - Consider token rotation for long-lived sessions\n");
}
