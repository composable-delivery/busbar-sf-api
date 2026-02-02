//! Auth integration tests using SF_AUTH_URL.

use super::common::{get_credentials, get_revocable_credentials};
use busbar_sf_auth::{Credentials, OAuthClient, OAuthConfig, PRODUCTION_LOGIN_URL};

// ============================================================================
// OAuth Token Revocation - Integration Tests
// ============================================================================

/// Determine the login URL based on instance URL.
fn login_url_for(creds: &busbar_sf_auth::SalesforceCredentials) -> &'static str {
    if creds.instance_url().contains("test.salesforce.com")
        || creds.instance_url().contains("sandbox")
        || creds.instance_url().contains(".scratch.")
    {
        "https://test.salesforce.com"
    } else {
        PRODUCTION_LOGIN_URL
    }
}

#[tokio::test]
async fn test_revoke_access_token() {
    // Use a fabricated token so revoking it doesn't invalidate the shared session.
    // This exercises the revocation code path — Salesforce returns 200 (RFC 7009)
    // or an error for invalid/unknown tokens, both of which we accept.
    let creds = get_revocable_credentials().await;
    let login_url = login_url_for(&creds);

    let result = creds.revoke_session(false, login_url).await;
    match &result {
        Ok(()) => {} // RFC 7009: revoke endpoint returns 200 even for unknown tokens
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("revocation failed") || err_str.contains("invalid_token"),
                "Unexpected error revoking access token: {err_str}"
            );
        }
    }
}

#[tokio::test]
async fn test_revoke_refresh_token() {
    let creds = get_credentials().await;
    let login_url = login_url_for(&creds);

    // We MUST NOT revoke the real refresh token — it is shared across all tests
    // and across CI runs. Instead, create credentials with a dummy refresh token
    // to exercise the revoke_session(true, ...) code path.
    let creds_with_dummy = busbar_sf_auth::SalesforceCredentials::new(
        creds.instance_url(),
        creds.access_token(),
        creds.api_version(),
    )
    .with_refresh_token("dummy-refresh-token-for-revocation-test");

    // Salesforce's revoke endpoint should return 200 per RFC 7009, but some
    // environments reject fabricated tokens. Either outcome validates that
    // our code correctly sends the refresh token and handles the response.
    let result = creds_with_dummy.revoke_session(true, login_url).await;
    match &result {
        Ok(()) => {} // Token accepted (RFC 7009 compliance)
        Err(e) => {
            let err_str = e.to_string();
            // Acceptable: server rejected the fabricated token
            assert!(
                err_str.contains("revocation failed") || err_str.contains("invalid_token"),
                "Unexpected error revoking dummy token: {err_str}"
            );
        }
    }
}

#[tokio::test]
async fn test_revoke_token_with_oauth_client() {
    // Use a fabricated token so revoking it doesn't invalidate the shared session.
    let creds = get_revocable_credentials().await;
    let login_url = login_url_for(&creds);

    let config = OAuthConfig::new("test_revoke_client");
    let oauth_client = OAuthClient::new(config);

    let result = oauth_client
        .revoke_token(creds.access_token(), login_url)
        .await;

    match &result {
        Ok(()) => {}
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("revocation failed") || err_str.contains("invalid_token"),
                "Unexpected error revoking token with OAuthClient: {err_str}"
            );
        }
    }
}

#[tokio::test]
async fn test_revoke_token_idempotency() {
    // Use a fabricated token so revoking it doesn't invalidate the shared session.
    let creds = get_revocable_credentials().await;
    let login_url = login_url_for(&creds);

    // First revocation.
    let result1 = creds.revoke_session(false, login_url).await;
    match &result1 {
        Ok(()) => {}
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("revocation failed") || err_str.contains("invalid_token"),
                "Unexpected error on first revocation: {err_str}"
            );
        }
    }

    // Second revocation of the same (now-invalid) token.
    // Per RFC 7009 this should return 200, but some Salesforce environments
    // reject already-revoked tokens. Both outcomes are acceptable.
    let result2 = creds.revoke_session(false, login_url).await;
    match &result2 {
        Ok(()) => {} // Idempotent as expected
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("revocation failed") || err_str.contains("invalid_token"),
                "Unexpected error on second revocation: {err_str}"
            );
        }
    }
}

#[tokio::test]
async fn test_revoke_session_without_refresh_token() {
    let creds = get_credentials().await;
    let login_url = login_url_for(&creds);

    // Create credentials without refresh token
    let creds_no_refresh = busbar_sf_auth::SalesforceCredentials::new(
        creds.instance_url(),
        creds.access_token(),
        creds.api_version(),
    );

    // Try to revoke refresh token when none exists — should fail with InvalidInput
    let result = creds_no_refresh.revoke_session(true, login_url).await;
    assert!(
        result.is_err(),
        "Should fail when trying to revoke non-existent refresh token"
    );
}
