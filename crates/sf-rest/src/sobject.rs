//! SObject CRUD operations.

use serde::{Deserialize, Serialize};

/// Result of a create operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateResult {
    pub id: String,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of an update operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateResult {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of a delete operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteResult {
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Result of an upsert operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpsertResult {
    pub id: String,
    pub success: bool,
    pub created: bool,
    #[serde(default)]
    pub errors: Vec<SalesforceError>,
}

/// Salesforce error in operation results.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SalesforceError {
    #[serde(rename = "statusCode")]
    pub status_code: String,
    pub message: String,
    #[serde(default)]
    pub fields: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_result_success() {
        let json = json!({"id": "001xx000003DgAAAS", "success": true, "errors": []});
        let result: CreateResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.id, "001xx000003DgAAAS");
        assert!(result.success);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_create_result_failure() {
        let json = json!({
            "id": "",
            "success": false,
            "errors": [{
                "statusCode": "REQUIRED_FIELD_MISSING",
                "message": "Required fields are missing: [Name]",
                "fields": ["Name"]
            }]
        });
        let result: CreateResult = serde_json::from_value(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].status_code, "REQUIRED_FIELD_MISSING");
        assert_eq!(result.errors[0].fields, vec!["Name"]);
    }

    #[test]
    fn test_upsert_result_created() {
        let json =
            json!({"id": "001xx000003DgAAAS", "success": true, "created": true, "errors": []});
        let result: UpsertResult = serde_json::from_value(json).unwrap();
        assert!(result.created);
        assert!(result.success);
    }

    #[test]
    fn test_upsert_result_updated() {
        let json =
            json!({"id": "001xx000003DgAAAS", "success": true, "created": false, "errors": []});
        let result: UpsertResult = serde_json::from_value(json).unwrap();
        assert!(!result.created);
        assert!(result.success);
    }

    #[test]
    fn test_update_result_success() {
        let json = json!({"success": true, "errors": []});
        let result: UpdateResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_delete_result_success() {
        let json = json!({"success": true, "errors": []});
        let result: DeleteResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_salesforce_error_serialization_roundtrip() {
        let error = SalesforceError {
            status_code: "INVALID_FIELD".to_string(),
            message: "No such column 'Foo' on entity 'Account'".to_string(),
            fields: vec!["Foo".to_string()],
        };
        let json = serde_json::to_value(&error).unwrap();
        let deserialized: SalesforceError = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.status_code, "INVALID_FIELD");
        assert_eq!(deserialized.fields, vec!["Foo"]);
    }
}
