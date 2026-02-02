use busbar_sf_auth::{Credentials, SalesforceCredentials};
use std::sync::OnceLock;
use tokio::sync::OnceCell;

/// Test account names used for integration test data setup.
/// These have a unique prefix so they can be identified and cleaned up.
pub const TEST_ACCOUNT_NAMES: &[&str] = &[
    "BusbarIntTest_Alpha Corp",
    "BusbarIntTest_Beta Industries",
    "BusbarIntTest_Gamma Solutions",
];

/// Ensure test Account records exist in the org.
///
/// Creates Account records with known names. Uses upsert-like logic:
/// queries first, creates only if missing. Returns the Account IDs.
///
/// This function is safe to call from multiple tests — it creates
/// records idempotently.
pub async fn ensure_test_accounts(client: &busbar_sf_rest::SalesforceRestClient) -> Vec<String> {
    // Check if test accounts already exist
    let existing: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account WHERE Name LIKE 'BusbarIntTest_%' LIMIT 10")
        .await
        .expect("Query for test accounts should succeed");

    if existing.len() >= TEST_ACCOUNT_NAMES.len() {
        // Already have enough test accounts
        return existing
            .iter()
            .filter_map(|r| r.get("Id").and_then(|v| v.as_str()).map(String::from))
            .collect();
    }

    // Create missing test accounts
    let existing_names: Vec<String> = existing
        .iter()
        .filter_map(|r| r.get("Name").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let mut ids: Vec<String> = existing
        .iter()
        .filter_map(|r| r.get("Id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    for name in TEST_ACCOUNT_NAMES {
        if !existing_names.iter().any(|n| n == name) {
            let id = client
                .create("Account", &serde_json::json!({"Name": name}))
                .await
                .expect("Create test account should succeed");
            ids.push(id);
        }
    }

    ids
}

/// Clean up test Account records created by `ensure_test_accounts`.
#[allow(dead_code)]
pub async fn cleanup_test_accounts(client: &busbar_sf_rest::SalesforceRestClient) {
    let accounts: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM Account WHERE Name LIKE 'BusbarIntTest_%' LIMIT 100")
        .await
        .unwrap_or_default();

    for account in accounts {
        if let Some(id) = account.get("Id").and_then(|v| v.as_str()) {
            let _ = client.delete("Account", id).await;
        }
    }
}

/// Cached auth URL — parsed once from the environment, shared across all tests.
fn get_auth_url() -> &'static str {
    static AUTH_URL: OnceLock<String> = OnceLock::new();
    AUTH_URL.get_or_init(|| {
        match std::env::var("SF_AUTH_URL") {
            Ok(url) if !url.is_empty() => {
                if !url.starts_with("force://") {
                    let preview = if url.len() > 50 {
                        format!("{}...", &url[..50])
                    } else {
                        url.clone()
                    };
                    panic!(
                        "Invalid SF_AUTH_URL format! Expected 'force://...' but got: {preview}"
                    );
                }
                url
            }
            Ok(_) => panic!("SF_AUTH_URL is set but empty!"),
            Err(_) => panic!(
                "SF_AUTH_URL environment variable is NOT set! \
                 Set it with: export SF_AUTH_URL=$(sf org display --target-org busbar-test --verbose --json | jq -r '.result.sfdxAuthUrl')"
            ),
        }
    })
}

/// Shared credentials — authenticated once, reused by all tests.
///
/// This prevents 130+ tests from simultaneously exchanging the refresh token,
/// which causes Salesforce to reject concurrent token requests with
/// "token request is already being processed".
static SHARED_CREDENTIALS: OnceCell<SalesforceCredentials> = OnceCell::const_new();

/// Get authenticated credentials for integration tests.
///
/// **IMPORTANT**: Integration tests MUST run against a real Salesforce org.
/// This function will panic with a helpful error message if SF_AUTH_URL is not set
/// or is invalid. Tests should NOT skip when credentials are unavailable.
///
/// Credentials are cached: the first call authenticates, subsequent calls
/// return a clone of the cached credentials. This prevents concurrent
/// refresh token exchanges from overwhelming Salesforce's token endpoint.
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

/// Get credentials with a **fabricated** access token for destructive tests.
///
/// Use this for tests that **revoke** tokens, so they don't invalidate
/// the shared session used by all other tests. The fabricated token
/// exercises the revocation code path — Salesforce returns either 200
/// (RFC 7009) or an error for invalid tokens, both of which we accept.
///
/// **Never** call `from_sfdx_auth_url()` more than once per test run —
/// concurrent refresh token exchanges can cause Salesforce to rotate
/// the access token and invalidate the shared session.
pub async fn get_revocable_credentials() -> SalesforceCredentials {
    let shared = get_credentials().await;
    SalesforceCredentials::new(
        shared.instance_url(),
        "00D_FABRICATED_ACCESS_TOKEN_FOR_REVOCATION_TEST",
        shared.api_version(),
    )
}
