//! Quick Action types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// A quick action available on an SObject.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QuickAction {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Detailed description of a quick action.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QuickActionDescribe {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(rename = "targetSobjectType")]
    pub target_sobject_type: Option<String>,
    #[serde(rename = "targetRecordTypeId")]
    pub target_record_type_id: Option<String>,
    #[serde(rename = "targetParentField")]
    pub target_parent_field: Option<String>,
    pub layout: Option<serde_json::Value>,
    #[serde(rename = "defaultValues")]
    pub default_values: Option<serde_json::Value>,
    pub icons: Vec<QuickActionIcon>,
}

/// An icon associated with a quick action.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct QuickActionIcon {
    pub url: String,
    pub theme: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

/// Result of invoking a quick action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuickActionResult {
    pub id: Option<String>,
    pub success: bool,
    #[serde(default)]
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "contextId")]
    pub context_id: Option<String>,
    #[serde(rename = "feedItemId")]
    pub feed_item_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_quick_action_deserialize() {
        let json = json!({"name": "NewCase", "label": "New Case", "type": "Create"});
        let action: QuickAction = serde_json::from_value(json).unwrap();
        assert_eq!(action.name, "NewCase");
        assert_eq!(action.label, "New Case");
        assert_eq!(action.action_type, "Create");
    }

    #[test]
    fn test_quick_action_default() {
        let action: QuickAction = serde_json::from_value(json!({})).unwrap();
        assert_eq!(action.name, "");
        assert_eq!(action.label, "");
        assert_eq!(action.action_type, "");
    }

    #[test]
    fn test_quick_action_describe_deserialize() {
        let json = json!({
            "name": "NewCase",
            "label": "New Case",
            "type": "Create",
            "targetSobjectType": "Case",
            "targetRecordTypeId": "012000000000000AAA",
            "layout": null,
            "defaultValues": null,
            "icons": [{
                "url": "https://example.com/icon.png",
                "theme": "theme4",
                "height": 32,
                "width": 32,
                "contentType": "image/png"
            }]
        });
        let describe: QuickActionDescribe = serde_json::from_value(json).unwrap();
        assert_eq!(describe.name, "NewCase");
        assert_eq!(describe.target_sobject_type.as_deref(), Some("Case"));
        assert_eq!(
            describe.target_record_type_id.as_deref(),
            Some("012000000000000AAA")
        );
        assert_eq!(describe.icons.len(), 1);
        assert_eq!(describe.icons[0].height, Some(32));
    }

    #[test]
    fn test_quick_action_describe_null_record_type() {
        let json = json!({
            "name": "LogACall",
            "label": "Log a Call",
            "type": "LogACall",
            "targetSobjectType": "Task",
            "targetRecordTypeId": null,
            "layout": null,
            "defaultValues": null,
            "icons": []
        });
        let describe: QuickActionDescribe = serde_json::from_value(json).unwrap();
        assert_eq!(describe.name, "LogACall");
        assert!(describe.target_record_type_id.is_none());
    }

    #[test]
    fn test_quick_action_result_deserialize() {
        let json = json!({
            "id": "500xx000000bZKQAA2",
            "success": true,
            "errors": [],
            "contextId": "001xx000003DgAAAS",
            "feedItemId": null
        });
        let result: QuickActionResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
        assert_eq!(result.id.unwrap(), "500xx000000bZKQAA2");
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_quick_action_result_failure() {
        let json = json!({
            "id": null,
            "success": false,
            "errors": [{
                "statusCode": "REQUIRED_FIELD_MISSING",
                "message": "Required fields are missing",
                "fields": ["Subject"]
            }],
            "contextId": null,
            "feedItemId": null
        });
        let result: QuickActionResult = serde_json::from_value(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
    }
}
