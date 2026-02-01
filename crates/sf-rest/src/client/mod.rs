//! Salesforce REST API client.
//!
//! This client wraps `SalesforceClient` from `sf-client` and provides
//! typed methods for REST API operations including CRUD, Query, Describe,
//! Composite, and Collections.

use busbar_sf_client::{ClientConfig, SalesforceClient};

use crate::error::Result;

mod binary;
mod collections;
mod composite;
mod consent;
mod crud;
mod describe;
mod embedded_service;
mod invocable_actions;
mod knowledge;
mod layout;
mod limits;
mod list_views;
mod process;
mod query;
mod quick_actions;
mod scheduler;
mod search;
mod standalone;
mod sync;
mod user_password;

/// Salesforce REST API client.
///
/// Provides typed methods for all REST API operations:
/// - CRUD operations on SObjects
/// - SOQL queries with automatic pagination
/// - SOSL search
/// - Describe operations
/// - Composite API
/// - SObject Collections
///
/// # Example
///
/// ```rust,ignore
/// use sf_rest::SalesforceRestClient;
///
/// let client = SalesforceRestClient::new(
///     "https://myorg.my.salesforce.com",
///     "access_token_here",
/// )?;
///
/// // Query
/// let accounts: Vec<Account> = client.query_all("SELECT Id, Name FROM Account").await?;
///
/// // Create
/// let id = client.create("Account", &json!({"Name": "New Account"})).await?;
///
/// // Update
/// client.update("Account", &id, &json!({"Name": "Updated"})).await?;
///
/// // Delete
/// client.delete("Account", &id).await?;
/// ```
#[derive(Debug, Clone)]
pub struct SalesforceRestClient {
    client: SalesforceClient,
}

impl SalesforceRestClient {
    /// Create a new REST client with the given instance URL and access token.
    pub fn new(instance_url: impl Into<String>, access_token: impl Into<String>) -> Result<Self> {
        let client = SalesforceClient::new(instance_url, access_token)?;
        Ok(Self { client })
    }

    /// Create a new REST client with custom HTTP configuration.
    pub fn with_config(
        instance_url: impl Into<String>,
        access_token: impl Into<String>,
        config: ClientConfig,
    ) -> Result<Self> {
        let client = SalesforceClient::with_config(instance_url, access_token, config)?;
        Ok(Self { client })
    }

    /// Create a REST client from an existing SalesforceClient.
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

/// Result of a SOSL search.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SearchResult<T> {
    #[serde(rename = "searchRecords")]
    pub search_records: Vec<T>,
}

/// API version information.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApiVersion {
    pub version: String,
    pub label: String,
    pub url: String,
}

/// Result of a getDeleted request.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GetDeletedResult {
    #[serde(rename = "deletedRecords")]
    pub deleted_records: Vec<DeletedRecord>,
    #[serde(rename = "earliestDateAvailable")]
    pub earliest_date_available: String,
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

/// A deleted record.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DeletedRecord {
    pub id: String,
    #[serde(rename = "deletedDate")]
    pub deleted_date: String,
}

/// Result of a getUpdated request.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct GetUpdatedResult {
    pub ids: Vec<String>,
    #[serde(rename = "latestDateCovered")]
    pub latest_date_covered: String,
}

/// Basic information about an SObject.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SObjectInfo {
    #[serde(rename = "objectDescribe")]
    pub object_describe: SObjectInfoDescribe,
    #[serde(rename = "recentItems")]
    pub recent_items: Vec<serde_json::Value>,
}

/// Basic describe information from SObject info endpoint.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SObjectInfoDescribe {
    pub name: String,
    pub label: String,
    #[serde(rename = "keyPrefix")]
    pub key_prefix: Option<String>,
    pub urls: std::collections::HashMap<String, String>,
    pub custom: bool,
    pub createable: bool,
    pub updateable: bool,
    pub deletable: bool,
    pub queryable: bool,
    pub searchable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token123").unwrap();

        assert_eq!(client.instance_url(), "https://na1.salesforce.com");
        assert_eq!(client.api_version(), "62.0");
    }

    #[test]
    fn test_api_version_override() {
        let client = SalesforceRestClient::new("https://na1.salesforce.com", "token")
            .unwrap()
            .with_api_version("60.0");

        assert_eq!(client.api_version(), "60.0");
    }

    #[test]
    fn test_get_deleted_result_deserialize() {
        let json = serde_json::json!({
            "deletedRecords": [
                {"id": "001xx000003DgAAAS", "deletedDate": "2024-01-15T10:30:00.000Z"}
            ],
            "earliestDateAvailable": "2024-01-01T00:00:00.000Z",
            "latestDateCovered": "2024-01-15T23:59:59.000Z"
        });
        let result: GetDeletedResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.deleted_records.len(), 1);
        assert_eq!(result.deleted_records[0].id, "001xx000003DgAAAS");
        assert_eq!(result.earliest_date_available, "2024-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_get_updated_result_deserialize() {
        let json = serde_json::json!({
            "ids": ["001xx000003DgAAAS", "001xx000003DgBBAS"],
            "latestDateCovered": "2024-01-15T23:59:59.000Z"
        });
        let result: GetUpdatedResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.ids.len(), 2);
        assert_eq!(result.latest_date_covered, "2024-01-15T23:59:59.000Z");
    }

    #[test]
    fn test_sobject_info_deserialize() {
        let json = serde_json::json!({
            "objectDescribe": {
                "name": "Account",
                "label": "Account",
                "keyPrefix": "001",
                "urls": {
                    "sobject": "/services/data/v62.0/sobjects/Account",
                    "describe": "/services/data/v62.0/sobjects/Account/describe"
                },
                "custom": false,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "queryable": true,
                "searchable": true
            },
            "recentItems": [
                {"Id": "001xx000003DgAAAS", "Name": "Acme Corp"}
            ]
        });
        let info: SObjectInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.object_describe.name, "Account");
        assert_eq!(info.object_describe.key_prefix, Some("001".to_string()));
        assert!(!info.object_describe.custom);
        assert!(info.object_describe.createable);
        assert_eq!(info.recent_items.len(), 1);
    }

    #[test]
    fn test_sobject_info_describe_custom() {
        let json = serde_json::json!({
            "objectDescribe": {
                "name": "MyObject__c",
                "label": "My Object",
                "keyPrefix": "a00",
                "urls": {},
                "custom": true,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "queryable": true,
                "searchable": false
            },
            "recentItems": []
        });
        let info: SObjectInfo = serde_json::from_value(json).unwrap();
        assert!(info.object_describe.custom);
        assert!(!info.object_describe.searchable);
        assert!(info.recent_items.is_empty());
    }
}
