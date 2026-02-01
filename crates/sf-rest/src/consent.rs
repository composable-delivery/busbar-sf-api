//! Consent API types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// Response from reading consent status.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentResponse {
    #[serde(default)]
    pub results: Vec<ConsentRecord>,
}

/// A single consent record.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentRecord {
    pub result: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "objectConsulted")]
    pub object_consulted: Option<String>,
}

/// Request to write consent.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentWriteRequest {
    #[serde(default)]
    pub records: Vec<ConsentWriteRecord>,
}

/// A single consent write record.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentWriteRecord {
    pub id: String,
    pub result: String,
}

/// Response from multi-action consent read.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiConsentResponse {
    #[serde(default)]
    pub results: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_consent_response_deserialize() {
        let json = json!({
            "results": [{
                "result": "OptIn",
                "status": "Active",
                "objectConsulted": "ContactPointEmail"
            }]
        });
        let response: ConsentResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.results.len(), 1);
        assert_eq!(response.results[0].result, "OptIn");
    }

    #[test]
    fn test_consent_response_empty() {
        let json = json!({"results": []});
        let response: ConsentResponse = serde_json::from_value(json).unwrap();
        assert!(response.results.is_empty());
    }

    #[test]
    fn test_consent_write_request_serialize() {
        let request = ConsentWriteRequest {
            records: vec![ConsentWriteRecord {
                id: "001xx000003DgAAAS".to_string(),
                result: "OptIn".to_string(),
            }],
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["records"][0]["id"], "001xx000003DgAAAS");
        assert_eq!(json["records"][0]["result"], "OptIn");
    }

    #[test]
    fn test_multi_consent_response_deserialize() {
        let json = json!({
            "results": [{"action": "email", "status": "OptIn"}]
        });
        let response: MultiConsentResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.results.len(), 1);
    }
}
