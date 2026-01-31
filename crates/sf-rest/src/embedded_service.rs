//! Embedded Service types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// Configuration for an embedded service deployment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddedServiceConfig {
    #[serde(rename = "id", default)]
    pub id: String,
    #[serde(rename = "isEnabled", default)]
    pub is_enabled: bool,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_embedded_service_config_deserialize() {
        let json = json!({
            "id": "0Hsxx0000000001",
            "isEnabled": true,
            "settings": {
                "chatButtonId": "573xx0000000001",
                "deploymentId": "572xx0000000001"
            }
        });
        let config: EmbeddedServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.id, "0Hsxx0000000001");
        assert!(config.is_enabled);
    }
}
