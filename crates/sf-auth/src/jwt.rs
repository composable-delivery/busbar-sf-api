//! JWT Bearer authentication flow.

use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::credentials::SalesforceCredentials;
use crate::error::{Error, ErrorKind, Result};

/// JWT Bearer authentication for server-to-server integration.
///
/// This flow is ideal for automated processes that don't require user interaction.
/// Requires a connected app with a certificate configured.
#[derive(Debug, Clone)]
pub struct JwtAuth {
    /// Consumer key (client_id) from the connected app.
    consumer_key: String,
    /// Username of the Salesforce user to authenticate as.
    username: String,
    /// Private key for signing the JWT (PEM format).
    private_key: Vec<u8>,
    /// Token expiration duration (default: 3 minutes).
    expiration: Duration,
}

impl JwtAuth {
    /// Create a new JWT authenticator.
    ///
    /// # Arguments
    ///
    /// * `consumer_key` - The consumer key from the connected app
    /// * `username` - The Salesforce username to authenticate as
    /// * `private_key` - The private key in PEM format (RSA)
    pub fn new(
        consumer_key: impl Into<String>,
        username: impl Into<String>,
        private_key: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            consumer_key: consumer_key.into(),
            username: username.into(),
            private_key: private_key.into(),
            expiration: Duration::minutes(3),
        }
    }

    /// Load the private key from a file.
    pub fn from_key_file(
        consumer_key: impl Into<String>,
        username: impl Into<String>,
        key_path: impl AsRef<std::path::Path>,
    ) -> Result<Self> {
        let private_key = std::fs::read(key_path.as_ref())?;
        Ok(Self::new(consumer_key, username, private_key))
    }

    /// Set the JWT expiration duration.
    pub fn with_expiration(mut self, expiration: Duration) -> Self {
        self.expiration = expiration;
        self
    }

    /// Generate a signed JWT assertion.
    fn generate_assertion(&self, audience: &str) -> Result<String> {
        let now = Utc::now();
        let exp = now + self.expiration;

        let claims = JwtClaims {
            iss: self.consumer_key.clone(),
            sub: self.username.clone(),
            aud: audience.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(&self.private_key)?;

        let token = encode(&header, &claims, &key)?;
        Ok(token)
    }

    /// Authenticate using the JWT Bearer flow.
    ///
    /// # Arguments
    ///
    /// * `login_url` - The Salesforce login URL (e.g., "<https://login.salesforce.com>")
    ///
    /// # Returns
    ///
    /// Credentials containing the access token and instance URL.
    pub async fn authenticate(&self, login_url: &str) -> Result<SalesforceCredentials> {
        let assertion = self.generate_assertion(login_url)?;

        debug!(login_url, "Authenticating with JWT Bearer flow");

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/services/oauth2/token", login_url))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &assertion),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let error: OAuthErrorResponse = response.json().await?;
            return Err(Error::new(ErrorKind::OAuth {
                error: error.error,
                description: error.error_description,
            }));
        }

        let token_response: JwtTokenResponse = response.json().await?;

        Ok(SalesforceCredentials::new(
            token_response.instance_url,
            token_response.access_token,
            busbar_sf_client::DEFAULT_API_VERSION,
        ))
    }

    /// Authenticate using the JWT Bearer flow for production.
    pub async fn authenticate_production(&self) -> Result<SalesforceCredentials> {
        self.authenticate(crate::PRODUCTION_LOGIN_URL).await
    }

    /// Authenticate using the JWT Bearer flow for sandbox.
    pub async fn authenticate_sandbox(&self) -> Result<SalesforceCredentials> {
        self.authenticate(crate::SANDBOX_LOGIN_URL).await
    }
}

/// JWT claims for Salesforce OAuth.
#[derive(Debug, Serialize)]
struct JwtClaims {
    /// Issuer (consumer key).
    iss: String,
    /// Subject (username).
    sub: String,
    /// Audience (login URL).
    aud: String,
    /// Expiration time (Unix timestamp).
    exp: i64,
    /// Issued at time (Unix timestamp).
    iat: i64,
}

/// Token response from JWT authentication.
#[derive(Debug, Deserialize)]
struct JwtTokenResponse {
    access_token: String,
    instance_url: String,
    #[serde(default)]
    #[allow(dead_code)]
    token_type: String,
    #[serde(default)]
    #[allow(dead_code)]
    scope: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<String>,
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

    // Note: Full JWT tests require a valid private key
    // These are basic structure tests

    #[test]
    fn test_jwt_auth_creation() {
        let auth = JwtAuth::new(
            "consumer_key",
            "user@example.com",
            b"fake_private_key".to_vec(),
        );

        assert_eq!(auth.consumer_key, "consumer_key");
        assert_eq!(auth.username, "user@example.com");
    }

    #[test]
    fn test_jwt_auth_with_expiration() {
        let auth =
            JwtAuth::new("key", "user", b"key".to_vec()).with_expiration(Duration::minutes(5));

        assert_eq!(auth.expiration, Duration::minutes(5));
    }
}
