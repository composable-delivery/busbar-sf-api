use busbar_sf_auth::SalesforceCredentials;

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
