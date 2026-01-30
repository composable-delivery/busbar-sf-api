use busbar_sf_auth::SalesforceCredentials;

/// Get authenticated credentials for integration tests.
/// 
/// **IMPORTANT**: Integration tests MUST run against a real Salesforce org.
/// This function will panic with a helpful error message if SF_AUTH_URL is not set
/// or is invalid. Tests should NOT skip when credentials are unavailable.
pub async fn get_credentials() -> SalesforceCredentials {
    // Check if SF_AUTH_URL is set
    let auth_url = match std::env::var("SF_AUTH_URL") {
        Ok(url) if !url.is_empty() => url,
        Ok(_) => {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════════════╗\n\
                ║ INTEGRATION TEST CONFIGURATION ERROR                                 ║\n\
                ╠══════════════════════════════════════════════════════════════════════╣\n\
                ║ SF_AUTH_URL is set but empty!                                        ║\n\
                ║                                                                      ║\n\
                ║ Integration tests require a valid Salesforce org authentication URL. ║\n\
                ║                                                                      ║\n\
                ║ To fix:                                                              ║\n\
                ║   1. Authenticate to a Salesforce org using sfdx/sf CLI             ║\n\
                ║   2. Get auth URL: sf org display --verbose                          ║\n\
                ║   3. Export: export SF_AUTH_URL='force://...'                        ║\n\
                ║                                                                      ║\n\
                ║ Or run from a GitHub Actions workflow with SF_AUTH_URL secret set.   ║\n\
                ╚══════════════════════════════════════════════════════════════════════╝\n\n"
            );
        }
        Err(_) => {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════════════╗\n\
                ║ INTEGRATION TEST CONFIGURATION ERROR                                 ║\n\
                ╠══════════════════════════════════════════════════════════════════════╣\n\
                ║ SF_AUTH_URL environment variable is NOT set!                         ║\n\
                ║                                                                      ║\n\
                ║ Integration tests require a valid Salesforce org authentication URL. ║\n\
                ║ These tests CANNOT run without a real Salesforce org.                ║\n\
                ║                                                                      ║\n\
                ║ To fix:                                                              ║\n\
                ║   1. Authenticate to a Salesforce org using sfdx/sf CLI             ║\n\
                ║   2. Get auth URL: sf org display --verbose                          ║\n\
                ║   3. Export: export SF_AUTH_URL='force://...'                        ║\n\
                ║                                                                      ║\n\
                ║ Or run from a GitHub Actions workflow with SF_AUTH_URL secret set.   ║\n\
                ╚══════════════════════════════════════════════════════════════════════╝\n\n"
            );
        }
    };

    // Validate the auth URL format before attempting to use it
    if !auth_url.starts_with("force://") {
        let preview = if auth_url.len() > 50 {
            format!("{}...", &auth_url[..50])
        } else {
            auth_url.clone()
        };
        
        panic!(
            "\n\n\
            ╔══════════════════════════════════════════════════════════════════════╗\n\
            ║ INTEGRATION TEST CONFIGURATION ERROR                                 ║\n\
            ╠══════════════════════════════════════════════════════════════════════╣\n\
            ║ Invalid SF_AUTH_URL format!                                          ║\n\
            ║                                                                      ║\n\
            ║ Expected format: force://PlatformCLI::...                            ║\n\
            ║ Actual value:    {}                             ║\n\
            ║                                                                      ║\n\
            ║ Common issues:                                                       ║\n\
            ║   - URL is truncated or corrupted                                    ║\n\
            ║   - Missing 'force://' prefix                                        ║\n\
            ║   - Wrong format (should be SFDX auth URL)                           ║\n\
            ║                                                                      ║\n\
            ║ To fix:                                                              ║\n\
            ║   1. Get a fresh auth URL: sf org display --verbose                  ║\n\
            ║   2. Copy the full 'Sfdx Auth Url' value                             ║\n\
            ║   3. Export: export SF_AUTH_URL='<paste full URL here>'              ║\n\
            ╚══════════════════════════════════════════════════════════════════════╝\n\n",
            preview
        );
    }

    // Attempt to authenticate
    match SalesforceCredentials::from_sfdx_auth_url(&auth_url).await {
        Ok(creds) => creds,
        Err(e) => {
            panic!(
                "\n\n\
                ╔══════════════════════════════════════════════════════════════════════╗\n\
                ║ INTEGRATION TEST AUTHENTICATION FAILED                               ║\n\
                ╠══════════════════════════════════════════════════════════════════════╣\n\
                ║ Failed to authenticate with the provided SF_AUTH_URL                 ║\n\
                ║                                                                      ║\n\
                ║ Error: {:60} ║\n\
                ║                                                                      ║\n\
                ║ This usually means:                                                  ║\n\
                ║   - The auth URL is expired or invalid                               ║\n\
                ║   - The org no longer exists (scratch orgs expire)                   ║\n\
                ║   - Network connectivity issues                                      ║\n\
                ║                                                                      ║\n\
                ║ To fix:                                                              ║\n\
                ║   1. Create/authenticate to a Salesforce org                         ║\n\
                ║   2. Get a fresh auth URL: sf org display --verbose                  ║\n\
                ║   3. Export: export SF_AUTH_URL='<paste full URL here>'              ║\n\
                ╚══════════════════════════════════════════════════════════════════════╝\n\n",
                format!("{}", e)
            );
        }
    }
}

// ============================================================================
// DEPRECATED: Legacy functions for backward compatibility
// These will be removed in a future version. Use get_credentials() instead.
// ============================================================================

/// DEPRECATED: Use get_credentials() instead.
/// This function silently skips tests, which is NOT the desired behavior.
#[deprecated(
    since = "0.0.3",
    note = "Use get_credentials() instead. Integration tests should fail, not skip."
)]
pub fn require_sf_auth_url() -> bool {
    std::env::var("SF_AUTH_URL").is_ok()
}

/// DEPRECATED: Use get_credentials() instead.
/// This function silently skips tests, which is NOT the desired behavior.
#[deprecated(
    since = "0.0.3",
    note = "Use get_credentials() instead. Integration tests should fail, not skip."
)]
pub async fn require_credentials() -> Option<SalesforceCredentials> {
    if std::env::var("SF_AUTH_URL").is_ok() {
        Some(get_credentials().await)
    } else {
        None
    }
}
