//! Shared test credential helper.
//!
//! A trimmed copy of the busbar-sf-api root crate's
//! `tests/integration/common.rs::get_credentials` — duplicated here rather
//! than shared because sf-bridge is intentionally excluded from that
//! workspace (see crates/sf-bridge/Cargo.toml).

use busbar_sf_auth::SalesforceCredentials;
use std::sync::OnceLock;
use tokio::sync::OnceCell;

fn get_auth_url() -> &'static str {
    static AUTH_URL: OnceLock<String> = OnceLock::new();
    AUTH_URL.get_or_init(|| match std::env::var("SF_AUTH_URL") {
        Ok(url) if !url.is_empty() => {
            if !url.starts_with("force://") {
                let preview = if url.len() > 50 {
                    format!("{}...", &url[..50])
                } else {
                    url.clone()
                };
                panic!("Invalid SF_AUTH_URL format! Expected 'force://...' but got: {preview}");
            }
            url
        }
        Ok(_) => panic!("SF_AUTH_URL is set but empty!"),
        Err(_) => panic!(
            "SF_AUTH_URL environment variable is NOT set! \
             Set it with: export SF_AUTH_URL=$(sf org display --target-org busbar-test --verbose --json | jq -r '.result.sfdxAuthUrl')"
        ),
    })
}

/// Shared credentials — authenticated once, reused by all tests in this binary.
static SHARED_CREDENTIALS: OnceCell<SalesforceCredentials> = OnceCell::const_new();

/// Get authenticated credentials for integration tests.
///
/// **IMPORTANT**: Integration tests MUST run against a real Salesforce org.
/// This function will panic with a helpful error message if SF_AUTH_URL is
/// not set or is invalid.
pub async fn get_credentials() -> SalesforceCredentials {
    SHARED_CREDENTIALS
        .get_or_init(|| async {
            let auth_url = get_auth_url();
            match SalesforceCredentials::from_sfdx_auth_url(auth_url).await {
                Ok(creds) => creds,
                Err(e) => {
                    panic!(
                        "Failed to authenticate with SF_AUTH_URL: {e}\n\
                         Ensure the org exists and the auth URL is fresh."
                    );
                }
            }
        })
        .await
        .clone()
}
