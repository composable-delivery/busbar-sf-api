//! Consent API types and responses.
//!
//! The Consent API enables reading and writing user consent status for data privacy compliance.
//! See: <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_consent.htm>

use serde::{Deserialize, Serialize};

/// Response from a consent read operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentResponse {
    /// List of consent statuses for records
    pub consents: Vec<ConsentRecord>,
}

/// Consent status for a single record.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsentRecord {
    /// Record ID
    pub id: String,
    /// Whether consent is granted
    pub consent: bool,
    /// Optional error if there was a problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request body for writing consent.
#[derive(Debug, Clone, Serialize)]
pub struct ConsentWriteRequest {
    /// List of records to update consent for
    pub consents: Vec<ConsentWriteRecord>,
}

/// Individual consent write record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentWriteRecord {
    /// Record ID
    pub id: String,
    /// Consent status to set
    pub consent: bool,
}

/// Response from a multi-action consent read.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiConsentResponse {
    /// Map of action names to consent responses
    #[serde(flatten)]
    pub actions: std::collections::HashMap<String, ConsentResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_record_serialization() {
        let record = ConsentRecord {
            id: "001xx000003DHP0AAO".to_string(),
            consent: true,
            error: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"consent\":true"));
    }

    #[test]
    fn test_consent_write_request() {
        let request = ConsentWriteRequest {
            consents: vec![ConsentWriteRecord {
                id: "001xx000003DHP0AAO".to_string(),
                consent: true,
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("consents"));
        assert!(json.contains("001xx000003DHP0AAO"));
    }
}
