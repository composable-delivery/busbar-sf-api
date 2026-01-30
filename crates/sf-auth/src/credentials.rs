//! Credentials trait and implementations.
//!
//! All credential types implement custom Debug to redact sensitive data.

use crate::error::{Error, ErrorKind, Result};

/// Trait for Salesforce credentials.
pub trait Credentials: Send + Sync {
    /// Get the Salesforce instance URL.
    fn instance_url(&self) -> &str;

    /// Get the access token.
    fn access_token(&self) -> &str;

    /// Get the API version (e.g., "62.0").
    fn api_version(&self) -> &str;

    /// Returns true if the credentials appear to be valid (non-empty).
    fn is_valid(&self) -> bool {
        !self.instance_url().is_empty() && !self.access_token().is_empty()
    }
}

/// Standard Salesforce credentials implementation.
///
/// Sensitive fields (access_token, refresh_token) are redacted in Debug output
/// to prevent accidental exposure in logs.
#[derive(Clone)]
pub struct SalesforceCredentials {
    instance_url: String,
    access_token: String,
    api_version: String,
    refresh_token: Option<String>,
}

impl std::fmt::Debug for SalesforceCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SalesforceCredentials")
            .field("instance_url", &self.instance_url)
            .field("access_token", &"[REDACTED]")
            .field("api_version", &self.api_version)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl SalesforceCredentials {
    /// Create new credentials with the given values.
    pub fn new(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        api_version: impl Into<String>,
    ) -> Self {
        Self {
            instance_url: instance_url.into(),
            access_token: access_token.into(),
            api_version: api_version.into(),
            refresh_token: None,
        }
    }

    /// Create credentials with a refresh token.
    pub fn with_refresh_token(mut self, refresh_token: impl Into<String>) -> Self {
        self.refresh_token = Some(refresh_token.into());
        self
    }

    /// Get the refresh token if available.
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// Set a new access token (e.g., after refresh).
    pub fn set_access_token(&mut self, token: impl Into<String>) {
        self.access_token = token.into();
    }

    /// Revoke the current session by invalidating the access token or refresh token.
    ///
    /// This convenience method creates an `OAuthClient` and calls `revoke_token()` with
    /// the current credentials. You can choose to revoke either the access token or the
    /// refresh token (if available).
    ///
    /// # Token Type Behavior
    ///
    /// - **Revoking refresh token** (`revoke_refresh: true`): Invalidates the refresh token
    ///   AND all associated access tokens. Use this for complete session termination.
    ///   Requires a refresh token to be present in the credentials.
    /// - **Revoking access token** (`revoke_refresh: false`): Invalidates only the current
    ///   access token. The refresh token remains valid and can be used to obtain a new
    ///   access token.
    ///
    /// # Arguments
    ///
    /// * `revoke_refresh` - If true, revokes the refresh token (and all access tokens).
    ///   If false, revokes only the access token.
    /// * `login_url` - The Salesforce login URL (e.g., <https://login.salesforce.com>
    ///   for production or <https://test.salesforce.com> for sandbox).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `revoke_refresh` is true but no refresh token is available
    /// - The HTTP request to the revocation endpoint fails
    /// - The Salesforce server returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use busbar_sf_auth::{SalesforceCredentials, PRODUCTION_LOGIN_URL};
    /// # async fn example() -> Result<(), busbar_sf_auth::Error> {
    /// let creds = SalesforceCredentials::new(
    ///     "https://na1.salesforce.com",
    ///     "access_token",
    ///     "62.0"
    /// ).with_refresh_token("refresh_token");
    ///
    /// // Revoke the entire session (refresh token + all access tokens)
    /// creds.revoke_session(true, PRODUCTION_LOGIN_URL).await?;
    ///
    /// // Or revoke just the access token
    /// creds.revoke_session(false, PRODUCTION_LOGIN_URL).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revoke_session(&self, revoke_refresh: bool, login_url: &str) -> Result<()> {
        use crate::oauth::{OAuthClient, OAuthConfig};

        // Determine which token to revoke
        let token = if revoke_refresh {
            self.refresh_token.as_ref().ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput(
                    "Cannot revoke refresh token: no refresh token available".to_string(),
                ))
            })?
        } else {
            &self.access_token
        };

        // Create a minimal OAuth client just for revocation
        let config = OAuthConfig::new("revoke_client");
        let client = OAuthClient::new(config);

        client.revoke_token(token, login_url).await
    }

    /// Load credentials from environment variables.
    ///
    /// Required environment variables:
    /// - `SF_INSTANCE_URL` or `SALESFORCE_INSTANCE_URL`
    /// - `SF_ACCESS_TOKEN` or `SALESFORCE_ACCESS_TOKEN`
    ///
    /// Optional:
    /// - `SF_API_VERSION` or `SALESFORCE_API_VERSION` (default: "62.0")
    /// - `SF_REFRESH_TOKEN` or `SALESFORCE_REFRESH_TOKEN`
    pub fn from_env() -> Result<Self> {
        let instance_url = std::env::var("SF_INSTANCE_URL")
            .or_else(|_| std::env::var("SALESFORCE_INSTANCE_URL"))
            .map_err(|_| Error::new(ErrorKind::EnvVar("SF_INSTANCE_URL".to_string())))?;

        let access_token = std::env::var("SF_ACCESS_TOKEN")
            .or_else(|_| std::env::var("SALESFORCE_ACCESS_TOKEN"))
            .map_err(|_| Error::new(ErrorKind::EnvVar("SF_ACCESS_TOKEN".to_string())))?;

        let api_version = std::env::var("SF_API_VERSION")
            .or_else(|_| std::env::var("SALESFORCE_API_VERSION"))
            .unwrap_or_else(|_| busbar_sf_client::DEFAULT_API_VERSION.to_string());

        let refresh_token = std::env::var("SF_REFRESH_TOKEN")
            .or_else(|_| std::env::var("SALESFORCE_REFRESH_TOKEN"))
            .ok();

        let mut creds = Self::new(instance_url, access_token, api_version);
        if let Some(rt) = refresh_token {
            creds = creds.with_refresh_token(rt);
        }

        Ok(creds)
    }

    /// Load credentials from SFDX CLI using an org alias or username.
    ///
    /// Requires the `sf` CLI to be installed and the org to be authenticated.
    pub async fn from_sfdx_alias(alias_or_username: &str) -> Result<Self> {
        use tokio::process::Command;

        let output = Command::new("sf")
            .args([
                "org",
                "display",
                "--target-org",
                alias_or_username,
                "--json",
            ])
            .output()
            .await
            .map_err(|e| Error::new(ErrorKind::SfdxCli(format!("Failed to run sf CLI: {}", e))))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::new(ErrorKind::SfdxCli(format!(
                "sf org display failed: {}",
                stderr
            ))));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let result = json.get("result").ok_or_else(|| {
            Error::new(ErrorKind::SfdxCli("Missing 'result' in output".to_string()))
        })?;

        let instance_url = result
            .get("instanceUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::new(ErrorKind::SfdxCli("Missing instanceUrl".to_string())))?;

        let access_token = result
            .get("accessToken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::new(ErrorKind::SfdxCli("Missing accessToken".to_string())))?;

        let api_version = result
            .get("apiVersion")
            .and_then(|v| v.as_str())
            .unwrap_or(busbar_sf_client::DEFAULT_API_VERSION);

        Ok(Self::new(instance_url, access_token, api_version))
    }

    /// Load credentials from an SFDX auth URL.
    ///
    /// The SFDX auth URL format is:
    /// - `force://<client_id>:<client_secret>:<refresh_token>@<instance_url>`
    /// - `force://<client_id>::<refresh_token>@<instance_url>` (empty client_secret)
    /// - `force://<client_id>:<client_secret>:<refresh_token>:<username>@<instance_url>` (with username)
    ///
    /// The client_secret can be empty (indicated by `::`) for the default Salesforce CLI
    /// connected app. The username field is optional.
    ///
    /// This method will parse the auth URL and use the refresh token to obtain
    /// an access token from Salesforce.
    ///
    /// # Example
    /// ```no_run
    /// # use busbar_sf_auth::SalesforceCredentials;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let auth_url = std::env::var("SF_AUTH_URL")?;
    /// let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_sfdx_auth_url(auth_url: &str) -> Result<Self> {
        use crate::oauth::{OAuthClient, OAuthConfig};

        // Parse the auth URL
        // Format: force://<client_id>:<client_secret>:<refresh_token>@<instance_url>
        // Or with username: force://<client_id>:<client_secret>:<refresh_token>:<username>@<instance_url>
        if !auth_url.starts_with("force://") {
            return Err(Error::new(ErrorKind::InvalidInput(
                "Auth URL must start with force://".to_string(),
            )));
        }

        let url = auth_url.strip_prefix("force://").unwrap();

        // Split at @ to separate credentials from instance URL
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(Error::new(ErrorKind::InvalidInput(
                "Invalid auth URL format: missing @".to_string(),
            )));
        }

        let credentials_part = parts[0];
        let instance_url = parts[1];

        // Split credentials into client_id:client_secret:refresh_token[:username]
        // Username is optional, so we accept 3 or 4 parts
        let cred_parts: Vec<&str> = credentials_part.splitn(4, ':').collect();
        if cred_parts.len() < 3 {
            return Err(Error::new(ErrorKind::InvalidInput(
                "Invalid auth URL format: expected client_id:client_secret:refresh_token[:username]"
                    .to_string(),
            )));
        }

        let client_id = cred_parts[0];
        let client_secret = if cred_parts[1].is_empty() {
            None
        } else {
            Some(cred_parts[1].to_string())
        };
        // The refresh token is in the third position
        let refresh_token = cred_parts[2];
        // Username is optional (4th position if present, not used currently)

        // Create OAuth client
        let mut config = OAuthConfig::new(client_id);
        if let Some(secret) = client_secret {
            config = config.with_secret(secret);
        }

        let oauth_client = OAuthClient::new(config);

        // Build token endpoint URL from instance URL
        // For localhost/test servers, use the instance_url directly
        // For Salesforce production/sandbox/scratch orgs, use the appropriate login URL
        let token_url = if instance_url.contains("localhost") || instance_url.contains("127.0.0.1")
        {
            instance_url
        } else if instance_url.contains("test.salesforce.com")
            || instance_url.contains("sandbox")
            || instance_url.contains(".scratch.")
        {
            "https://test.salesforce.com"
        } else {
            "https://login.salesforce.com"
        };

        // Use refresh token to get access token
        let token_response = oauth_client
            .refresh_token(refresh_token, token_url)
            .await
            .map_err(|e| {
                // Enhance error message for expired refresh tokens
                if matches!(&e.kind, ErrorKind::OAuth { error, .. } if error == "invalid_grant") {
                    Error::new(ErrorKind::OAuth {
                        error: "invalid_grant".to_string(),
                        description: format!(
                            "Refresh token expired or invalid. Generate a fresh SF_AUTH_URL using: \
                            `sf org display --verbose --json | jq -r '.result.sfdxAuthUrl'`. \
                            Original error: {}",
                            e
                        ),
                    })
                } else {
                    e
                }
            })?;

        // Create credentials from token response
        let api_version = busbar_sf_client::DEFAULT_API_VERSION.to_string();
        let mut creds = Self::new(
            token_response.instance_url,
            token_response.access_token,
            api_version,
        );
        creds = creds.with_refresh_token(refresh_token);

        Ok(creds)
    }

    /// Change the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Get the base REST API URL for this org.
    pub fn rest_api_url(&self) -> String {
        format!(
            "{}/services/data/v{}",
            self.instance_url.trim_end_matches('/'),
            self.api_version
        )
    }

    /// Get the Tooling API URL for this org.
    pub fn tooling_api_url(&self) -> String {
        format!(
            "{}/services/data/v{}/tooling",
            self.instance_url.trim_end_matches('/'),
            self.api_version
        )
    }

    /// Get the Metadata API URL for this org.
    pub fn metadata_api_url(&self) -> String {
        format!(
            "{}/services/Soap/m/{}",
            self.instance_url.trim_end_matches('/'),
            self.api_version
        )
    }

    /// Get the Bulk API 2.0 URL for this org.
    pub fn bulk_api_url(&self) -> String {
        format!(
            "{}/services/data/v{}/jobs",
            self.instance_url.trim_end_matches('/'),
            self.api_version
        )
    }
}

impl Credentials for SalesforceCredentials {
    fn instance_url(&self) -> &str {
        &self.instance_url
    }

    fn access_token(&self) -> &str {
        &self.access_token
    }

    fn api_version(&self) -> &str {
        &self.api_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_new() {
        let creds =
            SalesforceCredentials::new("https://test.salesforce.com", "access_token_123", "62.0");

        assert_eq!(creds.instance_url(), "https://test.salesforce.com");
        assert_eq!(creds.access_token(), "access_token_123");
        assert_eq!(creds.api_version(), "62.0");
        assert!(creds.is_valid());
    }

    #[test]
    fn test_credentials_with_refresh_token() {
        let creds =
            SalesforceCredentials::new("https://test.salesforce.com", "access_token", "62.0")
                .with_refresh_token("refresh_token_123");

        assert_eq!(creds.refresh_token(), Some("refresh_token_123"));
    }

    #[test]
    fn test_api_urls() {
        let creds = SalesforceCredentials::new("https://na1.salesforce.com", "token", "62.0");

        assert_eq!(
            creds.rest_api_url(),
            "https://na1.salesforce.com/services/data/v62.0"
        );
        assert_eq!(
            creds.tooling_api_url(),
            "https://na1.salesforce.com/services/data/v62.0/tooling"
        );
        assert_eq!(
            creds.bulk_api_url(),
            "https://na1.salesforce.com/services/data/v62.0/jobs"
        );
    }

    #[test]
    fn test_invalid_credentials() {
        let creds = SalesforceCredentials::new("", "", "62.0");
        assert!(!creds.is_valid());
    }

    #[test]
    fn test_credentials_debug_redacts_tokens() {
        let creds = SalesforceCredentials::new(
            "https://test.salesforce.com",
            "super_secret_access_token_12345",
            "62.0",
        )
        .with_refresh_token("super_secret_refresh_token_67890");

        let debug_output = format!("{:?}", creds);

        // Should contain [REDACTED]
        assert!(debug_output.contains("[REDACTED]"));

        // Should NOT contain actual tokens
        assert!(!debug_output.contains("super_secret_access_token_12345"));
        assert!(!debug_output.contains("super_secret_refresh_token_67890"));

        // Should still contain non-sensitive data
        assert!(debug_output.contains("test.salesforce.com"));
        assert!(debug_output.contains("62.0"));
    }

    #[test]
    fn test_parse_auth_url_with_client_secret() {
        // Test parsing with client_secret present
        // Format: force://<client_id>:<client_secret>:<refresh_token>@<instance_url>
        let auth_url = "force://client123:secret456:refresh789@https://test.salesforce.com";

        // We can't test the full async function without mocking the OAuth server,
        // but we can test the parsing logic by extracting it
        let url = auth_url.strip_prefix("force://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        assert_eq!(parts.len(), 2);

        let cred_parts: Vec<&str> = parts[0].splitn(4, ':').collect();
        assert!(cred_parts.len() >= 3);
        assert_eq!(cred_parts[0], "client123");
        assert_eq!(cred_parts[1], "secret456");
        assert_eq!(cred_parts[2], "refresh789");
    }

    #[test]
    fn test_parse_auth_url_without_client_secret() {
        // Test parsing with empty client_secret (default Salesforce CLI connected app)
        // Format: force://<client_id>::<refresh_token>@<instance_url>
        let auth_url = "force://client123::refresh789@https://test.salesforce.com";

        let url = auth_url.strip_prefix("force://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        assert_eq!(parts.len(), 2);

        let cred_parts: Vec<&str> = parts[0].splitn(4, ':').collect();
        assert!(cred_parts.len() >= 3);
        assert_eq!(cred_parts[0], "client123");
        assert_eq!(cred_parts[1], ""); // Empty client_secret
        assert_eq!(cred_parts[2], "refresh789");
    }

    #[test]
    fn test_parse_auth_url_with_username() {
        // Test parsing with username appended (optional 4th field)
        // Format: force://<client_id>:<client_secret>:<refresh_token>:<username>@<instance_url>
        // Note: username cannot contain @ since splitn(2, '@') splits on the first @,
        // making it the delimiter between credentials and instance_url
        let auth_url = "force://client123:secret456:refresh789:user@https://test.salesforce.com";

        let url = auth_url.strip_prefix("force://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        assert_eq!(parts.len(), 2);

        let cred_parts: Vec<&str> = parts[0].splitn(4, ':').collect();
        assert_eq!(cred_parts.len(), 4);
        assert_eq!(cred_parts[0], "client123");
        assert_eq!(cred_parts[1], "secret456");
        assert_eq!(cred_parts[2], "refresh789");
        assert_eq!(cred_parts[3], "user");
    }

    #[test]
    fn test_parse_auth_url_invalid_format() {
        // Test invalid format with too few parts
        let auth_url = "force://client123:secret456@https://test.salesforce.com";

        let url = auth_url.strip_prefix("force://").unwrap();
        let parts: Vec<&str> = url.splitn(2, '@').collect();
        assert_eq!(parts.len(), 2);

        let cred_parts: Vec<&str> = parts[0].splitn(4, ':').collect();
        // Should have only 2 parts, which is less than the required 3
        assert_eq!(cred_parts.len(), 2);
        assert!(
            cred_parts.len() < 3,
            "Invalid format should have less than 3 parts"
        );
    }

    #[tokio::test]
    async fn test_from_sfdx_auth_url_with_client_secret() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Set up mock OAuth server
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/services/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_access_token",
                "instance_url": "https://na1.salesforce.com",
                "id": "https://login.salesforce.com/id/00Dxx0000000000EAA/005xx000000000QAAQ",
                "token_type": "Bearer",
                "issued_at": "1234567890"
            })))
            .mount(&mock_server)
            .await;

        let auth_url = format!(
            "force://client123:secret456:refresh789@{}",
            mock_server.uri()
        );

        let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url).await;
        assert!(creds.is_ok(), "Failed to authenticate: {:?}", creds.err());

        let creds = creds.unwrap();
        assert_eq!(creds.instance_url(), "https://na1.salesforce.com");
        assert_eq!(creds.access_token(), "test_access_token");
        assert_eq!(creds.refresh_token(), Some("refresh789"));
    }

    #[tokio::test]
    async fn test_from_sfdx_auth_url_without_client_secret() {
        use wiremock::matchers::{method, path};
        use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

        // Custom matcher to verify client_secret is NOT in the request
        struct NoClientSecretMatcher;
        impl Match for NoClientSecretMatcher {
            fn matches(&self, request: &Request) -> bool {
                let body = String::from_utf8_lossy(&request.body);
                body.contains("client_id=client123")
                    && body.contains("refresh_token=refresh789")
                    && !body.contains("client_secret")
            }
        }

        // Set up mock OAuth server
        let mock_server = MockServer::start().await;

        // Verify that client_secret is NOT sent when empty
        Mock::given(method("POST"))
            .and(path("/services/oauth2/token"))
            .and(NoClientSecretMatcher)
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_access_token_no_secret",
                "instance_url": "https://na1.salesforce.com",
                "id": "https://login.salesforce.com/id/00Dxx0000000000EAA/005xx000000000QAAQ",
                "token_type": "Bearer",
                "issued_at": "1234567890"
            })))
            .mount(&mock_server)
            .await;

        // Auth URL with empty client_secret (double colon ::)
        let auth_url = format!("force://client123::refresh789@{}", mock_server.uri());

        let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url).await;
        assert!(creds.is_ok(), "Failed to authenticate: {:?}", creds.err());

        let creds = creds.unwrap();
        assert_eq!(creds.instance_url(), "https://na1.salesforce.com");
        assert_eq!(creds.access_token(), "test_access_token_no_secret");
        assert_eq!(creds.refresh_token(), Some("refresh789"));
    }

    #[tokio::test]
    async fn test_from_sfdx_auth_url_sandbox() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Set up mock OAuth server - note we can't actually test the sandbox URL selection
        // without mocking the actual Salesforce endpoint, but we can test that the parsing works
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/services/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_access_token_sandbox",
                "instance_url": "https://test.salesforce.com",
                "id": "https://test.salesforce.com/id/00Dxx0000000000EAA/005xx000000000QAAQ",
                "token_type": "Bearer",
                "issued_at": "1234567890"
            })))
            .mount(&mock_server)
            .await;

        // Use localhost in the auth URL so it uses the mock server
        // In production, sandbox URLs would route to test.salesforce.com
        let auth_url = format!(
            "force://client123:secret456:refresh789@{}",
            mock_server.uri()
        );

        let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url).await;
        assert!(creds.is_ok(), "Failed to authenticate: {:?}", creds.err());

        let creds = creds.unwrap();
        assert_eq!(creds.instance_url(), "https://test.salesforce.com");
        assert_eq!(creds.access_token(), "test_access_token_sandbox");
    }

    #[tokio::test]
    async fn test_from_sfdx_auth_url_with_username() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Set up mock OAuth server
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/services/oauth2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test_access_token_with_user",
                "instance_url": "https://na1.salesforce.com",
                "id": "https://login.salesforce.com/id/00Dxx0000000000EAA/005xx000000000QAAQ",
                "token_type": "Bearer",
                "issued_at": "1234567890"
            })))
            .mount(&mock_server)
            .await;

        // Auth URL with username field
        let auth_url = format!(
            "force://client123:secret456:refresh789:username@{}",
            mock_server.uri()
        );

        let creds = SalesforceCredentials::from_sfdx_auth_url(&auth_url).await;
        assert!(creds.is_ok(), "Failed to authenticate: {:?}", creds.err());

        let creds = creds.unwrap();
        assert_eq!(creds.instance_url(), "https://na1.salesforce.com");
        assert_eq!(creds.access_token(), "test_access_token_with_user");
    }

    #[tokio::test]
    async fn test_from_sfdx_auth_url_invalid_too_few_parts() {
        // Auth URL with only 2 parts (missing refresh token)
        let auth_url = "force://client123:secret456@https://test.salesforce.com";

        let creds = SalesforceCredentials::from_sfdx_auth_url(auth_url).await;
        assert!(creds.is_err());
        let err = creds.unwrap_err();
        assert!(err
            .to_string()
            .contains("expected client_id:client_secret:refresh_token"));
    }

    #[tokio::test]
    async fn test_revoke_session_access_token() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock the revoke endpoint
        Mock::given(method("POST"))
            .and(path("/services/oauth2/revoke"))
            .and(body_string_contains("token=test_access_token"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let creds =
            SalesforceCredentials::new("https://na1.salesforce.com", "test_access_token", "62.0");

        let result = creds.revoke_session(false, &mock_server.uri()).await;
        assert!(result.is_ok(), "Revoking access token should succeed");
    }

    #[tokio::test]
    async fn test_revoke_session_refresh_token() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock the revoke endpoint
        Mock::given(method("POST"))
            .and(path("/services/oauth2/revoke"))
            .and(body_string_contains("token=test_refresh_token"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let creds =
            SalesforceCredentials::new("https://na1.salesforce.com", "test_access_token", "62.0")
                .with_refresh_token("test_refresh_token");

        let result = creds.revoke_session(true, &mock_server.uri()).await;
        assert!(
            result.is_ok(),
            "Revoking refresh token should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_revoke_session_no_refresh_token() {
        let creds =
            SalesforceCredentials::new("https://na1.salesforce.com", "test_access_token", "62.0");

        // Try to revoke refresh token when none exists
        let result = creds
            .revoke_session(true, "https://login.salesforce.com")
            .await;

        assert!(result.is_err(), "Should fail when no refresh token exists");
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind, ErrorKind::InvalidInput(_)),
            "Should return InvalidInput error"
        );
    }
}
