//! SObject Collections for batch operations.

use crate::sobject::SalesforceError;
use serde::{Deserialize, Serialize};

/// Request for SObject Collections operations.
#[derive(Debug, Clone, Serialize)]
pub struct CollectionRequest {
    #[serde(rename = "allOrNone")]
    pub all_or_none: bool,
    pub records: Vec<serde_json::Value>,
}

/// Result of a collection operation.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionResult {
    pub id: Option<String>,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
    pub created: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_collection_request_serialization() {
        let request = CollectionRequest {
            all_or_none: true,
            records: vec![
                json!({"attributes": {"type": "Account"}, "Name": "Acme"}),
                json!({"attributes": {"type": "Account"}, "Name": "Widget Co"}),
            ],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["allOrNone"], true);
        assert_eq!(json["records"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_collection_result_success() {
        let json =
            json!({"id": "001xx000003DgAAAS", "success": true, "errors": [], "created": true});
        let result: CollectionResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.id, Some("001xx000003DgAAAS".to_string()));
        assert!(result.success);
        assert_eq!(result.created, Some(true));
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_collection_result_failure() {
        let json = json!({
            "id": null,
            "success": false,
            "errors": [{"statusCode": "DUPLICATES_DETECTED", "message": "Duplicate found", "fields": []}],
            "created": null
        });
        let result: CollectionResult = serde_json::from_value(json).unwrap();
        assert!(result.id.is_none());
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].status_code, "DUPLICATES_DETECTED");
    }

    #[test]
    fn test_collection_result_batch_response() {
        // Salesforce returns an array of CollectionResults for batch ops
        let json = json!([
            {"id": "001xx000003Dg1", "success": true, "errors": [], "created": true},
            {"id": null, "success": false, "errors": [{"statusCode": "INVALID_FIELD", "message": "bad field", "fields": ["Foo"]}], "created": null},
            {"id": "001xx000003Dg3", "success": true, "errors": [], "created": true}
        ]);
        let results: Vec<CollectionResult> = serde_json::from_value(json).unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0].success);
        assert!(!results[1].success);
        assert!(results[2].success);
    }
}
