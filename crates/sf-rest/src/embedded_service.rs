//! Embedded Service API types.
//!
//! Provides access to Embedded Service configurations.
//! See: https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/

use serde::{Deserialize, Serialize};

/// Embedded Service configuration.
///
/// Contains configuration details for an Embedded Service deployment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddedServiceConfig {
    /// Configuration ID
    pub id: String,
    /// Configuration name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the service is enabled
    #[serde(rename = "isEnabled", skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
    /// Site URL
    #[serde(rename = "siteUrl", skip_serializing_if = "Option::is_none")]
    pub site_url: Option<String>,
    /// Additional configuration settings
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_service_config_deserialization() {
        let json = r#"{
            "id": "0ESxx000000001",
            "name": "Chat Service",
            "isEnabled": true,
            "siteUrl": "https://example.force.com"
        }"#;
        let config: EmbeddedServiceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "0ESxx000000001");
        assert_eq!(config.name, Some("Chat Service".to_string()));
        assert_eq!(config.is_enabled, Some(true));
    }
}
