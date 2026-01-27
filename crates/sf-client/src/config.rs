//! Client configuration.

use crate::retry::RetryConfig;
use std::time::Duration;

/// Configuration for the HTTP client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Retry configuration.
    pub retry: Option<RetryConfig>,
    /// Compression configuration.
    pub compression: CompressionConfig,
    /// Request timeout.
    pub timeout: Duration,
    /// Connection timeout.
    pub connect_timeout: Duration,
    /// Pool idle timeout.
    pub pool_idle_timeout: Duration,
    /// Maximum idle connections per host.
    pub pool_max_idle_per_host: usize,
    /// User-Agent header value.
    pub user_agent: String,
    /// Whether to enable request/response tracing.
    pub enable_tracing: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            retry: Some(RetryConfig::default()),
            compression: CompressionConfig::default(),
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            pool_idle_timeout: Duration::from_secs(90),
            pool_max_idle_per_host: 10,
            user_agent: crate::USER_AGENT.to_string(),
            enable_tracing: true,
        }
    }
}

impl ClientConfig {
    /// Create a new client config builder.
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::default()
    }
}

/// Builder for ClientConfig.
#[derive(Debug, Default)]
pub struct ClientConfigBuilder {
    config: ClientConfig,
}

impl ClientConfigBuilder {
    /// Set the retry configuration.
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.config.retry = Some(retry);
        self
    }

    /// Disable retries.
    pub fn without_retry(mut self) -> Self {
        self.config.retry = None;
        self
    }

    /// Enable compression for requests and responses.
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.config.compression.enabled = enabled;
        self
    }

    /// Set compression configuration.
    pub fn with_compression_config(mut self, config: CompressionConfig) -> Self {
        self.config.compression = config;
        self
    }

    /// Set request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set pool idle timeout.
    pub fn with_pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.config.pool_idle_timeout = timeout;
        self
    }

    /// Set maximum idle connections per host.
    pub fn with_pool_max_idle(mut self, max: usize) -> Self {
        self.config.pool_max_idle_per_host = max;
        self
    }

    /// Set custom User-Agent.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = user_agent.into();
        self
    }

    /// Enable or disable request/response tracing.
    pub fn with_tracing(mut self, enabled: bool) -> Self {
        self.config.enable_tracing = enabled;
        self
    }

    /// Build the client configuration.
    pub fn build(self) -> ClientConfig {
        self.config
    }
}

/// Configuration for request/response compression.
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Whether compression is enabled.
    pub enabled: bool,
    /// Whether to compress request bodies.
    pub compress_requests: bool,
    /// Accept compressed responses.
    pub accept_compressed: bool,
    /// Minimum body size to compress (bytes).
    pub min_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            compress_requests: false, // Generally not worth it for small API payloads
            accept_compressed: true,  // Always accept compressed responses
            min_size: 1024,           // Only compress bodies > 1KB
        }
    }
}

impl CompressionConfig {
    /// Disable all compression.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            compress_requests: false,
            accept_compressed: false,
            min_size: 0,
        }
    }

    /// Full compression (both requests and responses).
    pub fn full() -> Self {
        Self {
            enabled: true,
            compress_requests: true,
            accept_compressed: true,
            min_size: 512,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ClientConfig::default();
        assert!(config.retry.is_some());
        assert!(config.compression.enabled);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.user_agent.contains("busbar-sf-api"));
    }

    #[test]
    fn test_builder() {
        let config = ClientConfig::builder()
            .with_timeout(Duration::from_secs(60))
            .without_retry()
            .with_compression(false)
            .with_user_agent("custom-agent/1.0")
            .build();

        assert!(config.retry.is_none());
        assert!(!config.compression.enabled);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.user_agent, "custom-agent/1.0");
    }

    #[test]
    fn test_compression_config() {
        let disabled = CompressionConfig::disabled();
        assert!(!disabled.enabled);
        assert!(!disabled.compress_requests);
        assert!(!disabled.accept_compressed);

        let full = CompressionConfig::full();
        assert!(full.enabled);
        assert!(full.compress_requests);
        assert!(full.accept_compressed);
    }
}
