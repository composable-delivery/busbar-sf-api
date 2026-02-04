//! Busbar authentication integration for WASM credential resolution.
//!
//! This module provides credential resolution for the `SfBridge` when used in
//! Busbar-integrated environments. Credentials are resolved transparently from:
//! 1. Environment variables (CI/CD path)
//! 2. JWT Bearer flow with auto-refresh
//! 3. (Future) OS keychain via busbar-keychain
//!
//! WASM guests never see tokens -- all credential resolution happens host-side.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use busbar_sf_auth::{JwtAuth, SalesforceCredentials};
use busbar_sf_client::DEFAULT_API_VERSION;
use tracing::{debug, instrument};

use crate::error::{Error, Result};

/// Configuration for Busbar authentication integration.
///
/// Defines how the bridge should resolve Salesforce credentials in Busbar
/// environments (local development, CI/CD, etc.).
#[derive(Debug, Clone)]
pub struct BusbarAuthConfig {
    /// Optional JWT authentication configuration for server-to-server auth.
    pub jwt_auth: Option<JwtAuthConfig>,
    /// Login URL for authentication (default: production).
    pub login_url: String,
    /// API version to use (default: from busbar_sf_client::DEFAULT_API_VERSION).
    pub api_version: String,
    /// Token cache TTL in seconds (default: 3600 = 1 hour).
    pub token_ttl_secs: u64,
}

impl Default for BusbarAuthConfig {
    fn default() -> Self {
        Self {
            jwt_auth: None,
            login_url: busbar_sf_auth::PRODUCTION_LOGIN_URL.to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            token_ttl_secs: 3600, // 1 hour
        }
    }
}

impl BusbarAuthConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set JWT authentication configuration.
    pub fn with_jwt_auth(mut self, jwt_auth: JwtAuthConfig) -> Self {
        self.jwt_auth = Some(jwt_auth);
        self
    }

    /// Set the login URL (production or sandbox).
    pub fn with_login_url(mut self, login_url: impl Into<String>) -> Self {
        self.login_url = login_url.into();
        self
    }

    /// Set the API version.
    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }

    /// Set the token TTL in seconds.
    pub fn with_token_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.token_ttl_secs = ttl_secs;
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
            .map_err(|e| Error::Auth(format!("Failed to read private key: {}", e)))?;
        Ok(Self::new(consumer_key, username, private_key))
    }
}

/// Cached credentials with expiration tracking.
#[derive(Debug, Clone)]
struct CachedCredentials {
    credentials: SalesforceCredentials,
    expires_at: Instant,
}

impl CachedCredentials {
    fn new(credentials: SalesforceCredentials, ttl: Duration) -> Self {
        Self {
            credentials,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Credential resolver with caching and auto-refresh support.
///
/// Resolves credentials in priority order:
/// 1. Environment variables (SF_ACCESS_TOKEN, SF_INSTANCE_URL)
/// 2. JWT Bearer auth (if configured)
/// 3. (Future) OS keychain via busbar-keychain
///
/// Caches credentials with configurable TTL and auto-refreshes on expiry.
pub struct BusbarAuthResolver {
    config: BusbarAuthConfig,
    cache: Arc<Mutex<Option<CachedCredentials>>>,
}

impl BusbarAuthResolver {
    /// Create a new resolver with the given configuration.
    pub fn new(config: BusbarAuthConfig) -> Self {
        Self {
            config,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Resolve credentials using the configured resolution chain.
    ///
    /// Checks cache first, then tries resolution methods in priority order.
    /// Transparently handles token refresh when cached credentials expire.
    #[instrument(skip(self))]
    pub async fn resolve(&self) -> Result<SalesforceCredentials> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(ref cached) = *cache {
                if !cached.is_expired() {
                    debug!("Using cached credentials");
                    return Ok(cached.credentials.clone());
                }
                debug!("Cached credentials expired, refreshing");
            }
        }

        // Try resolution chain
        let credentials = self.resolve_fresh().await?;

        // Cache the new credentials
        let ttl = Duration::from_secs(self.config.token_ttl_secs);
        let cached = CachedCredentials::new(credentials.clone(), ttl);
        *self.cache.lock().unwrap() = Some(cached);

        Ok(credentials)
    }

    /// Resolve credentials without checking cache (forces fresh resolution).
    #[instrument(skip(self))]
    async fn resolve_fresh(&self) -> Result<SalesforceCredentials> {
        // 1. Try environment variables first (CI/CD path)
        if let Ok(creds) = self.try_from_env() {
            debug!("Resolved credentials from environment variables");
            return Ok(creds);
        }

        // 2. Try JWT bearer auth if configured
        if let Some(ref jwt_config) = self.config.jwt_auth {
            debug!("Attempting JWT bearer authentication");
            return self.try_jwt_auth(jwt_config).await;
        }

        // 3. Future: Try keychain (busbar-keychain integration)
        // if let Some(keychain_path) = &self.config.keychain_path {
        //     debug!("Attempting keychain resolution");
        //     return self.try_keychain(keychain_path).await;
        // }

        Err(Error::Auth(
            "No credentials found. Set SF_ACCESS_TOKEN and SF_INSTANCE_URL environment variables, \
             or configure JWT authentication."
                .to_string(),
        ))
    }

    /// Try to resolve credentials from environment variables.
    fn try_from_env(&self) -> Result<SalesforceCredentials> {
        let instance_url = std::env::var("SF_INSTANCE_URL")
            .or_else(|_| std::env::var("SALESFORCE_INSTANCE_URL"))
            .map_err(|_| Error::Auth("SF_INSTANCE_URL not set".to_string()))?;

        let access_token = std::env::var("SF_ACCESS_TOKEN")
            .or_else(|_| std::env::var("SALESFORCE_ACCESS_TOKEN"))
            .map_err(|_| Error::Auth("SF_ACCESS_TOKEN not set".to_string()))?;

        let api_version = std::env::var("SF_API_VERSION")
            .or_else(|_| std::env::var("SALESFORCE_API_VERSION"))
            .unwrap_or_else(|_| self.config.api_version.clone());

        Ok(SalesforceCredentials::new(
            instance_url,
            access_token,
            api_version,
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
            .map_err(|e| Error::Auth(format!("JWT authentication failed: {}", e)))
    }

    /// Clear the cached credentials, forcing a fresh resolution on next call.
    pub fn clear_cache(&self) {
        *self.cache.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use busbar_sf_auth::Credentials;

    #[tokio::test]
    async fn test_resolve_from_env() {
        // Set test environment variables
        std::env::set_var("SF_INSTANCE_URL", "https://test.salesforce.com");
        std::env::set_var("SF_ACCESS_TOKEN", "test_token_12345");
        std::env::set_var("SF_API_VERSION", "62.0");

        let config = BusbarAuthConfig::new();
        let resolver = BusbarAuthResolver::new(config);

        let creds = resolver.resolve().await.unwrap();
        assert_eq!(creds.instance_url(), "https://test.salesforce.com");
        assert_eq!(creds.access_token(), "test_token_12345");
        assert_eq!(creds.api_version(), "62.0");
    }

    #[tokio::test]
    #[ignore] // Run with --ignored to test cache behavior in isolation
    async fn test_cache_behavior() {
        // Set unique test environment variables
        std::env::set_var("SF_INSTANCE_URL", "https://cache-test.salesforce.com");
        std::env::set_var("SF_ACCESS_TOKEN", "cache_test_token_12345");

        let config = BusbarAuthConfig::new().with_token_ttl_secs(2);
        let resolver = BusbarAuthResolver::new(config);

        // First resolve
        let creds1 = resolver.resolve().await.unwrap();
        assert_eq!(creds1.access_token(), "cache_test_token_12345");

        // Change env var
        std::env::set_var("SF_ACCESS_TOKEN", "cache_different_token");

        // Second resolve should use cache
        let creds2 = resolver.resolve().await.unwrap();
        assert_eq!(creds2.access_token(), "cache_test_token_12345"); // Still cached

        // Wait for cache expiry
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Third resolve should get new value
        let creds3 = resolver.resolve().await.unwrap();
        assert_eq!(creds3.access_token(), "cache_different_token");
    }

    #[tokio::test]
    #[ignore] // Run with --ignored to test cache clearing in isolation
    async fn test_clear_cache() {
        // Set test environment variables
        std::env::set_var("SF_INSTANCE_URL", "https://clear-test.salesforce.com");
        std::env::set_var("SF_ACCESS_TOKEN", "clear_test_token_12345");

        let config = BusbarAuthConfig::new();
        let resolver = BusbarAuthResolver::new(config);

        // First resolve
        let creds1 = resolver.resolve().await.unwrap();
        assert_eq!(creds1.access_token(), "clear_test_token_12345");

        // Change env var and clear cache
        std::env::set_var("SF_ACCESS_TOKEN", "clear_different_token");
        resolver.clear_cache();

        // Should get new value immediately
        let creds2 = resolver.resolve().await.unwrap();
        assert_eq!(creds2.access_token(), "clear_different_token");
    }

    #[tokio::test]
    #[ignore] // Run with --ignored to avoid interfering with other tests
    async fn test_missing_env_vars() {
        // Ensure env vars are not set
        std::env::remove_var("SF_INSTANCE_URL");
        std::env::remove_var("SF_ACCESS_TOKEN");
        std::env::remove_var("SALESFORCE_INSTANCE_URL");
        std::env::remove_var("SALESFORCE_ACCESS_TOKEN");

        let config = BusbarAuthConfig::new();
        let resolver = BusbarAuthResolver::new(config);

        let result = resolver.resolve().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No credentials found"));
    }
}
