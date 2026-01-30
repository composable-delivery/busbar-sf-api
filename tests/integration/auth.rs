//! Auth integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::{Credentials, OAuthClient, OAuthConfig, PRODUCTION_LOGIN_URL};

// ============================================================================
// OAuth Token Revocation - Integration Tests
// ============================================================================

#[tokio::test]
#[ignore] // Requires SF_AUTH_URL to be set
async fn test_revoke_access_token() {
    let creds = get_credentials().await;

    // Determine login URL based on instance URL
    let login_url = if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // Test revoking access token using revoke_session convenience method
    let result = creds.revoke_session(false, login_url).await;

    // Should succeed (returns 200 even if token is already invalid)
    assert!(
        result.is_ok(),
        "Failed to revoke access token: {:?}",
        result.err()
    );

    println!("✓ Successfully revoked access token");
}

#[tokio::test]
#[ignore] // Requires SF_AUTH_URL to be set with refresh token
async fn test_revoke_refresh_token() {
    let creds = get_credentials().await;

    // Check if refresh token is available
    if creds.refresh_token().is_none() {
        eprintln!("skipping: No refresh token available in credentials");
        return;
    }

    // Determine login URL based on instance URL
    let login_url = if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // Test revoking refresh token (also invalidates all access tokens)
    let result = creds.revoke_session(true, login_url).await;

    // Should succeed (returns 200 even if token is already invalid)
    assert!(
        result.is_ok(),
        "Failed to revoke refresh token: {:?}",
        result.err()
    );

    println!("✓ Successfully revoked refresh token");
}

#[tokio::test]
#[ignore] // Requires SF_AUTH_URL to be set
async fn test_revoke_token_with_oauth_client() {
    let creds = get_credentials().await;

    // Create OAuth client
    let config = OAuthConfig::new("test_revoke_client");
    let oauth_client = OAuthClient::new(config);

    // Determine login URL based on instance URL
    let login_url = if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // Test revoking access token directly with OAuthClient
    let result = oauth_client
        .revoke_token(creds.access_token(), login_url)
        .await;

    // Should succeed (returns 200 even if token is already invalid)
    assert!(
        result.is_ok(),
        "Failed to revoke token with OAuthClient: {:?}",
        result.err()
    );

    println!("✓ Successfully revoked token using OAuthClient");
}

#[tokio::test]
#[ignore] // Requires SF_AUTH_URL to be set
async fn test_revoke_token_idempotency() {
    let creds = get_credentials().await;

    // Determine login URL based on instance URL
    let login_url = if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // First revocation
    let result1 = creds.revoke_session(false, login_url).await;
    assert!(
        result1.is_ok(),
        "First revocation failed: {:?}",
        result1.err()
    );

    // Second revocation of the same token (idempotent)
    let result2 = creds.revoke_session(false, login_url).await;
    assert!(
        result2.is_ok(),
        "Second revocation failed (idempotency): {:?}",
        result2.err()
    );

    println!("✓ Successfully verified idempotent token revocation");
}

#[tokio::test]
#[ignore] // Requires SF_AUTH_URL to be set
async fn test_revoke_session_without_refresh_token() {
    let creds = get_credentials().await;

    // Create credentials without refresh token
    let creds_no_refresh = busbar_sf_auth::SalesforceCredentials::new(
        creds.instance_url(),
        creds.access_token(),
        creds.api_version(),
    );

    let login_url = if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    };

    // Try to revoke refresh token when none exists
    let result = creds_no_refresh.revoke_session(true, login_url).await;

    // Should fail with InvalidInput error
    assert!(
        result.is_err(),
        "Should fail when trying to revoke non-existent refresh token"
    );

    println!("✓ Correctly handles missing refresh token error");
}
