//! Busbar keychain integration for credential resolution.
//!
//! This module integrates with the Busbar keychain system from the
//! composable-delivery/busbar repository to resolve Salesforce credentials
//! transparently without requiring pre-authenticated clients.
//!
//! Credentials are resolved from:
//! 1. Environment variables (SF_ACCESS_TOKEN, SF_INSTANCE_URL)
//! 2. Busbar keychain (via busbar-keychain::SecretStore)
//! 3. JWT bearer auth (if configured)
//!
//! WASM guests never see tokens - all credential resolution happens host-side.

use busbar_keychain::SecretStore;
use busbar_sf_auth::{JwtAuth, SalesforceCredentials};
use busbar_sf_client::DEFAULT_API_VERSION;
use std::sync::Arc;
use tracing::{debug, instrument};

use crate::error::{Error, Result};

/// Configuration for Busbar keychain-based credential resolution.
///
/// This configuration allows the bridge to resolve Salesforce credentials
/// from the Busbar keychain system without requiring pre-authenticated clients.
#[derive(Debug, Clone)]
pub struct KeychainAuthConfig {
    /// Path prefix in the keychain for Salesforce credentials.
    /// Example: "sf/production" will look for "sf_production_access_token"
    pub keychain_prefix: Option<String>,

    /// Optional JWT authentication for server-to-server flows.
    pub jwt_auth: Option<JwtAuthConfig>,

    /// Salesforce login URL (default: production).
    pub login_url: String,

    /// API version to use (default: from busbar_sf_client::DEFAULT_API_VERSION).
    pub api_version: String,
}

impl Default for KeychainAuthConfig {
    fn default() -> Self {
        Self {
            keychain_prefix: None,
            jwt_auth: None,
            login_url: busbar_sf_auth::PRODUCTION_LOGIN_URL.to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
        }
    }
}

impl KeychainAuthConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the keychain prefix for credential lookups.
    ///
    /// Example: `with_keychain_prefix("sf/production")` will look for
    /// credentials at "sf_production_access_token" and "sf_production_instance_url".
    pub fn with_keychain_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.keychain_prefix = Some(prefix.into());
        self
    }

    /// Set JWT authentication configuration.
    pub fn with_jwt_auth(mut self, jwt_auth: JwtAuthConfig) -> Self {
        self.jwt_auth = Some(jwt_auth);
        self
    }

    /// Set the Salesforce login URL.
    pub fn with_login_url(mut self, login_url: impl Into<String>) -> Self {
        self.login_url = login_url.into();
        self
    }

    /// Set the API version.
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }
}

/// JWT authentication configuration.
#[derive(Debug, Clone)]
pub struct JwtAuthConfig {
    /// Consumer key (client_id) from the connected app.
    pub consumer_key: String,
    /// Username to authenticate as.
    pub username: String,
    /// Private key for JWT signing (PEM format).
    pub private_key: Vec<u8>,
}

impl JwtAuthConfig {
    /// Create a new JWT auth configuration.
    pub fn new(
        consumer_key: impl Into<String>,
        username: impl Into<String>,
        private_key: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            consumer_key: consumer_key.into(),
            username: username.into(),
            private_key: private_key.into(),
        }
    }

    /// Load private key from a file.
    pub fn with_key_file(
        consumer_key: impl Into<String>,
        username: impl Into<String>,
        key_path: impl AsRef<std::path::Path>,
    ) -> Result<Self> {
        let private_key = std::fs::read(key_path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read private key: {}", e)))?;
        Ok(Self::new(consumer_key, username, private_key))
    }
}

/// Credential resolver that integrates with Busbar keychain.
///
/// Resolves credentials in priority order:
/// 1. Environment variables (SF_ACCESS_TOKEN, SF_INSTANCE_URL)
/// 2. Busbar keychain (via SecretStore)
/// 3. JWT bearer auth (if configured)
pub struct KeychainAuthResolver {
    config: KeychainAuthConfig,
    store: Arc<SecretStore>,
}

impl KeychainAuthResolver {
    /// Create a new resolver with the given configuration.
    pub async fn new(config: KeychainAuthConfig) -> Result<Self> {
        let store = SecretStore::new()
            .await
            .map_err(|e| Error::Config(format!("Failed to initialize secret store: {}", e)))?;

        Ok(Self {
            config,
            store: Arc::new(store),
        })
    }

    /// Create a resolver with an existing SecretStore.
    pub fn with_store(config: KeychainAuthConfig, store: Arc<SecretStore>) -> Self {
        Self { config, store }
    }

    /// Resolve Salesforce credentials using the configured resolution chain.
    ///
    /// This method tries:
    /// 1. Environment variables (SF_ACCESS_TOKEN, SF_INSTANCE_URL)
    /// 2. Busbar keychain
    /// 3. JWT bearer auth (if configured)
    #[instrument(skip(self))]
    pub async fn resolve(&self) -> Result<SalesforceCredentials> {
        // 1. Try environment variables first (CI/CD path)
        if let Ok(creds) = self.try_from_env() {
            debug!("Resolved credentials from environment variables");
            return Ok(creds);
        }

        // 2. Try Busbar keychain
        if let Ok(creds) = self.try_from_keychain().await {
            debug!("Resolved credentials from Busbar keychain");
            return Ok(creds);
        }

        // 3. Try JWT bearer auth if configured
        if let Some(ref jwt_config) = self.config.jwt_auth {
            debug!("Attempting JWT bearer authentication");
            return self.try_jwt_auth(jwt_config).await;
        }

        Err(Error::Config(
            "No credentials found. Set SF_ACCESS_TOKEN and SF_INSTANCE_URL environment variables, \
             configure credentials in Busbar keychain, or configure JWT authentication."
                .to_string(),
        ))
    }

    /// Try to resolve credentials from environment variables.
    fn try_from_env(&self) -> Result<SalesforceCredentials> {
        let instance_url = std::env::var("SF_INSTANCE_URL")
            .or_else(|_| std::env::var("SALESFORCE_INSTANCE_URL"))
            .map_err(|_| Error::Config("SF_INSTANCE_URL not set".to_string()))?;

        let access_token = std::env::var("SF_ACCESS_TOKEN")
            .or_else(|_| std::env::var("SALESFORCE_ACCESS_TOKEN"))
            .map_err(|_| Error::Config("SF_ACCESS_TOKEN not set".to_string()))?;

        let api_version = std::env::var("SF_API_VERSION")
            .or_else(|_| std::env::var("SALESFORCE_API_VERSION"))
            .unwrap_or_else(|_| self.config.api_version.clone());

        Ok(SalesforceCredentials::new(
            instance_url,
            access_token,
            api_version,
        ))
    }

    /// Try to resolve credentials from the Busbar keychain.
    async fn try_from_keychain(&self) -> Result<SalesforceCredentials> {
        let prefix = self.config.keychain_prefix.as_deref().unwrap_or("sf");

        let access_token_key = format!("{}/access_token", prefix);
        let instance_url_key = format!("{}/instance_url", prefix);

        let access_token = self.store.get(&access_token_key).await.map_err(|e| {
            Error::Config(format!("Failed to get access token from keychain: {}", e))
        })?;

        let instance_url = self.store.get(&instance_url_key).await.map_err(|e| {
            Error::Config(format!("Failed to get instance URL from keychain: {}", e))
        })?;

        Ok(SalesforceCredentials::new(
            instance_url,
            access_token,
            self.config.api_version.clone(),
        ))
    }

    /// Try to authenticate using JWT bearer flow.
    async fn try_jwt_auth(&self, jwt_config: &JwtAuthConfig) -> Result<SalesforceCredentials> {
        let jwt_auth = JwtAuth::new(
            jwt_config.consumer_key.clone(),
            jwt_config.username.clone(),
            jwt_config.private_key.clone(),
        );

        jwt_auth
            .authenticate(&self.config.login_url)
            .await
            .map_err(|e| Error::Config(format!("JWT authentication failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires environment setup
    async fn test_resolve_from_env() {
        std::env::set_var("SF_INSTANCE_URL", "https://test.salesforce.com");
        std::env::set_var("SF_ACCESS_TOKEN", "test_token_12345");

        let config = KeychainAuthConfig::new();
        let resolver = KeychainAuthResolver::new(config).await.unwrap();

        let creds = resolver.resolve().await.unwrap();
        assert_eq!(creds.instance_url(), "https://test.salesforce.com");
        assert_eq!(creds.access_token(), "test_token_12345");

        std::env::remove_var("SF_INSTANCE_URL");
        std::env::remove_var("SF_ACCESS_TOKEN");
    }
}
