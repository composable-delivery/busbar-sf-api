use busbar_sf_auth::SalesforceCredentials;

/// Returns true if SF_AUTH_URL is present, otherwise prints a skip message.
pub fn require_sf_auth_url() -> bool {
    if std::env::var("SF_AUTH_URL").is_ok() {
        true
    } else {
        eprintln!("skipping: SF_AUTH_URL not set");
        false
    }
}

/// Returns credentials if SF_AUTH_URL is present; otherwise returns None.
pub async fn require_credentials() -> Option<SalesforceCredentials> {
    if !require_sf_auth_url() {
        return None;
    }

    Some(get_test_credentials().await)
}

/// Helper to get authenticated credentials from SF_AUTH_URL.
pub async fn get_test_credentials() -> SalesforceCredentials {
    let auth_url =
        std::env::var("SF_AUTH_URL").expect("SF_AUTH_URL environment variable must be set");

    SalesforceCredentials::from_sfdx_auth_url(&auth_url)
        .await
        .expect("Failed to authenticate from SF_AUTH_URL")
}
