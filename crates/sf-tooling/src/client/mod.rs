//! Salesforce Tooling API client.
//!
//! This client wraps `SalesforceClient` from `sf-client` and provides
//! typed methods for Tooling API operations.

use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::error::Result;

mod apex;
mod code_intelligence;
mod collections;
mod composite;
mod coverage;
mod describe;
mod execute;
mod logs;
mod query;
mod sobject;
mod test_execution;
mod trace_flags;

/// Salesforce Tooling API client.
///
/// Provides typed methods for Tooling API operations:
/// - Execute anonymous Apex
/// - Query Apex classes, triggers, and logs
/// - Manage debug logs and trace flags
/// - Code coverage information
///
/// # Example
///
/// ```rust,ignore
/// use sf_tooling::ToolingClient;
///
/// let client = ToolingClient::new(
///     "https://myorg.my.salesforce.com",
///     "access_token_here",
/// )?;
///
/// // Execute anonymous Apex
/// let result = client.execute_anonymous("System.debug('Hello');").await?;
///
/// // Query Apex classes
/// let classes: Vec<ApexClass> = client
///     .query_all("SELECT Id, Name FROM ApexClass")
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct ToolingClient {
    client: SalesforceClient,
}

impl ToolingClient {
    /// Create a new Tooling API client with the given instance URL and access token.
    pub fn new(instance_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
        let client = SalesforceClient::new(instance_url, access_token)?;
        Ok(Self { client })
    }

    /// Create a new Tooling API client with custom HTTP configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(instance_url, access_token, config)?;
        Ok(Self { client })
    }

    /// Create a Tooling client from an existing SalesforceClient.
    pub fn from_client(client: SalesforceClient) -> Self {
        Self { client }
    }

    /// Get the underlying SalesforceClient.
    pub fn inner(&self) -> &SalesforceClient {
        &self.client
    }

    /// Get the instance URL.
    pub fn instance_url(&self) -> &str {
        self.client.instance_url()
    }

    /// Get the API version.
    pub fn api_version(&self) -> &str {
        self.client.api_version()
    }

    /// Set the API version.
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.client = self.client.with_api_version(version);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token123").unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
    }
}
