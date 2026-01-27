//! OAuth 2.0 authentication flows.
//!
//! This module provides secure OAuth 2.0 flows for Salesforce authentication:
//! - **Web Server Flow** - For web applications with user interaction
//! - **JWT Bearer Flow** - For server-to-server integration (see jwt.rs)
//! - **Refresh Token** - For refreshing expired access tokens
//!
//! Note: Device Code Flow has been intentionally excluded as it is being
//! deprecated due to security concerns.

use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::credentials::SalesforceCredentials;
use crate::error::{Error, ErrorKind, Result};

/// OAuth 2.0 configuration for a connected app.
///
/// Sensitive fields like `consumer_secret` are redacted in Debug output
/// to prevent accidental exposure in logs.
#[derive(Clone)]
pub struct OAuthConfig {
    /// Consumer key (client_id).
    pub consumer_key: String,
    /// Consumer secret (client_secret). Optional for some flows.
    consumer_secret: Option<String>,
    /// Redirect URI for web flow.
    pub redirect_uri: Option<String>,
    /// Scopes to request.
    pub scopes: Vec<String>,
}

impl std::fmt::Debug for OAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthConfig")
            .field("consumer_key", &self.consumer_key)
            .field("consumer_secret", &"[REDACTED]")
            .field("redirect_uri", &self.redirect_uri)
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl OAuthConfig {
    /// Create a new OAuth config.
    pub fn new(consumer_key: impl Into<String>) -> Self {
        Self {
            consumer_key: consumer_key.into(),
            consumer_secret: None,
            redirect_uri: None,
            scopes: vec!["api".to_string(), "refresh_token".to_string()],
        }
    }

    /// Set the consumer secret.
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.consumer_secret = Some(secret.into());
        self
    }

    /// Get the consumer secret (for internal use).
    #[allow(dead_code)]
    pub(crate) fn consumer_secret(&self) -> Option<&str> {
        self.consumer_secret.as_deref()
    }

    /// Set the redirect URI.
    pub fn with_redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.redirect_uri = Some(uri.into());
        self
    }

    /// Set the scopes.
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }
}

/// OAuth client for authenticating with Salesforce.
#[derive(Clone)]
pub struct OAuthClient {
    config: OAuthConfig,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for OAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl OAuthClient {
    /// Create a new OAuth client.
    pub fn new(config: OAuthConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get the OAuth config.
    pub fn config(&self) -> &OAuthConfig {
        &self.config
    }

    /// Refresh an access token using a refresh token.
    ///
    /// The refresh_token parameter is not logged to prevent credential exposure.
    #[instrument(skip(self, refresh_token))]
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
        login_url: &str,
    ) -> Result<TokenResponse> {
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.config.consumer_key),
        ];

        if let Some(ref secret) = self.config.consumer_secret {
            params.push(("client_secret", secret));
        }

        let body = serde_urlencoded::to_string(params)?;

        let response = self
            .http_client
            .post(format!("{}/services/oauth2/token", login_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        self.handle_token_response(response).await
    }

    /// Validate an access token.
    ///
    /// The token parameter is not logged to prevent credential exposure.
    /// Uses POST with token in body to avoid exposing token in URL/logs.
    #[instrument(skip(self, token))]
    pub async fn validate_token(&self, token: &str, login_url: &str) -> Result<TokenInfo> {
        // Use POST with token in body instead of GET with query param
        // This prevents the token from appearing in server logs
        let form_data = [("access_token", token)];
        let body = serde_urlencoded::to_string(form_data)?;

        let response = self
            .http_client
            .post(format!("{}/services/oauth2/tokeninfo", login_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::new(ErrorKind::TokenInvalid(
                "Token validation failed".to_string(),
            )));
        }

        let info: TokenInfo = response.json().await?;
        Ok(info)
    }

    /// Revoke an access token.
    ///
    /// The token parameter is not logged to prevent credential exposure.
    #[instrument(skip(self, token))]
    pub async fn revoke_token(&self, token: &str, login_url: &str) -> Result<()> {
        let form_data = [("token", token)];
        let body = serde_urlencoded::to_string(form_data)?;

        let response = self
            .http_client
            .post(format!("{}/services/oauth2/revoke", login_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::new(ErrorKind::OAuth {
                error: "revoke_failed".to_string(),
                description: "Failed to revoke token".to_string(),
            }));
        }

        Ok(())
    }

    /// Handle a token response, checking for errors.
    async fn handle_token_response(&self, response: reqwest::Response) -> Result<TokenResponse> {
        if !response.status().is_success() {
            let error: OAuthErrorResponse = response.json().await?;
            return Err(Error::new(ErrorKind::OAuth {
                error: error.error,
                description: error.error_description,
            }));
        }

        let token: TokenResponse = response.json().await?;
        Ok(token)
    }
}

/// Web Server OAuth flow for web applications.
#[derive(Clone)]
pub struct WebFlowAuth {
    config: OAuthConfig,
    http_client: reqwest::Client,
}

impl std::fmt::Debug for WebFlowAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebFlowAuth")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl WebFlowAuth {
    /// Create a new web flow authenticator.
    pub fn new(config: OAuthConfig) -> Result<Self> {
        if config.redirect_uri.is_none() {
            return Err(Error::new(ErrorKind::Config(
                "redirect_uri is required for web flow".to_string(),
            )));
        }

        Ok(Self {
            config,
            http_client: reqwest::Client::new(),
        })
    }

    /// Generate the authorization URL to redirect users to.
    pub fn authorization_url(&self, login_url: &str, state: Option<&str>) -> String {
        let redirect_uri = self.config.redirect_uri.as_ref().unwrap();
        let scopes = self.config.scopes.join(" ");

        let mut url = format!(
            "{}/services/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}",
            login_url,
            urlencoding::encode(&self.config.consumer_key),
            urlencoding::encode(redirect_uri),
        );

        if !scopes.is_empty() {
            url.push_str(&format!("&scope={}", urlencoding::encode(&scopes)));
        }

        if let Some(state) = state {
            url.push_str(&format!("&state={}", urlencoding::encode(state)));
        }

        url
    }

    /// Exchange an authorization code for tokens.
    ///
    /// The code parameter is not logged to prevent credential exposure.
    #[instrument(skip(self, code))]
    pub async fn exchange_code(&self, code: &str, login_url: &str) -> Result<TokenResponse> {
        let redirect_uri = self.config.redirect_uri.as_ref().unwrap();

        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &self.config.consumer_key),
            ("redirect_uri", redirect_uri),
        ];

        if let Some(ref secret) = self.config.consumer_secret {
            params.push(("client_secret", secret));
        }

        let body = serde_urlencoded::to_string(params)?;

        let response = self
            .http_client
            .post(format!("{}/services/oauth2/token", login_url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error: OAuthErrorResponse = response.json().await?;
            return Err(Error::new(ErrorKind::OAuth {
                error: error.error,
                description: error.error_description,
            }));
        }

        let token: TokenResponse = response.json().await?;
        Ok(token)
    }
}

/// Token response from OAuth.
///
/// Sensitive fields like `access_token` and `refresh_token` are redacted
/// in Debug output to prevent accidental exposure in logs.
#[derive(Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    /// Access token.
    pub access_token: String,
    /// Refresh token (if requested).
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Instance URL.
    pub instance_url: String,
    /// User ID URL.
    #[serde(default)]
    pub id: Option<String>,
    /// Token type (usually "Bearer").
    #[serde(default)]
    pub token_type: Option<String>,
    /// Scopes granted.
    #[serde(default)]
    pub scope: Option<String>,
    /// Signature for verification.
    #[serde(default)]
    pub signature: Option<String>,
    /// Issued at timestamp.
    #[serde(default)]
    pub issued_at: Option<String>,
}

impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("access_token", &"[REDACTED]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("instance_url", &self.instance_url)
            .field("id", &self.id)
            .field("token_type", &self.token_type)
            .field("scope", &self.scope)
            .field("signature", &self.signature.as_ref().map(|_| "[REDACTED]"))
            .field("issued_at", &self.issued_at)
            .finish()
    }
}

impl TokenResponse {
    /// Convert to SalesforceCredentials.
    pub fn to_credentials(&self, api_version: &str) -> SalesforceCredentials {
        let mut creds =
            SalesforceCredentials::new(&self.instance_url, &self.access_token, api_version);

        if let Some(ref rt) = self.refresh_token {
            creds = creds.with_refresh_token(rt);
        }

        creds
    }
}

/// Token info from validation.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    /// Whether the token is active.
    pub active: bool,
    /// Scopes.
    #[serde(default)]
    pub scope: Option<String>,
    /// Client ID.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Username.
    #[serde(default)]
    pub username: Option<String>,
    /// Token type.
    #[serde(default)]
    pub token_type: Option<String>,
    /// Expiration time.
    #[serde(default)]
    pub exp: Option<u64>,
    /// Issued at.
    #[serde(default)]
    pub iat: Option<u64>,
    /// Subject.
    #[serde(default)]
    pub sub: Option<String>,
}

/// OAuth error response.
#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
    error_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::Credentials;

    #[test]
    fn test_oauth_config() {
        let config = OAuthConfig::new("consumer_key")
            .with_secret("secret")
            .with_redirect_uri("https://example.com/callback")
            .with_scopes(vec!["api".to_string(), "web".to_string()]);

        assert_eq!(config.consumer_key, "consumer_key");
        assert_eq!(config.consumer_secret(), Some("secret"));
        assert_eq!(
            config.redirect_uri,
            Some("https://example.com/callback".to_string())
        );
        assert_eq!(config.scopes, vec!["api", "web"]);
    }

    #[test]
    fn test_oauth_config_debug_redacts_secret() {
        let config = OAuthConfig::new("consumer_key").with_secret("super_secret_value");

        let debug_output = format!("{:?}", config);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("super_secret_value"));
    }

    #[test]
    fn test_web_flow_auth_url() {
        let config = OAuthConfig::new("my_client_id")
            .with_redirect_uri("https://localhost:8080/callback")
            .with_scopes(vec!["api".to_string()]);

        let auth = WebFlowAuth::new(config).unwrap();
        let url = auth.authorization_url("https://login.salesforce.com", Some("state123"));

        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my_client_id"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("state=state123"));
    }

    #[test]
    fn test_token_response_to_credentials() {
        let token = TokenResponse {
            access_token: "access123".to_string(),
            refresh_token: Some("refresh456".to_string()),
            instance_url: "https://na1.salesforce.com".to_string(),
            id: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            signature: None,
            issued_at: None,
        };

        let creds = token.to_credentials("62.0");
        assert_eq!(creds.instance_url(), "https://na1.salesforce.com");
        assert_eq!(creds.access_token(), "access123");
        assert_eq!(creds.refresh_token(), Some("refresh456"));
    }

    #[test]
    fn test_token_response_debug_redacts_tokens() {
        let token = TokenResponse {
            access_token: "super_secret_access_token".to_string(),
            refresh_token: Some("super_secret_refresh_token".to_string()),
            instance_url: "https://na1.salesforce.com".to_string(),
            id: None,
            token_type: Some("Bearer".to_string()),
            scope: None,
            signature: Some("signature_value".to_string()),
            issued_at: None,
        };

        let debug_output = format!("{:?}", token);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("super_secret_access_token"));
        assert!(!debug_output.contains("super_secret_refresh_token"));
        assert!(!debug_output.contains("signature_value"));
    }
}
