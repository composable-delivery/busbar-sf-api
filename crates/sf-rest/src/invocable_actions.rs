//! Invocable Action types for the Salesforce REST API.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize `null` as the default value for the type (e.g., empty Vec).
/// Salesforce APIs often return `"errors": null` instead of `"errors": []`.
fn null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

/// An invocable action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableAction {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Collection of invocable actions returned from a type-specific endpoint
/// (e.g., `/actions/standard/apex`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionCollection {
    #[serde(default)]
    pub actions: Vec<InvocableAction>,
}

/// Map of action type categories to their sub-resource URLs,
/// returned from `/actions/standard` or `/actions/custom`.
pub type InvocableActionTypeMap = std::collections::HashMap<String, String>;

/// Detailed description of an invocable action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionDescribe {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub inputs: Vec<InvocableActionParameter>,
    #[serde(default)]
    pub outputs: Vec<InvocableActionParameter>,
}

/// A parameter for an invocable action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionParameter {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    pub description: Option<String>,
}

/// Request to invoke an action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionRequest {
    pub inputs: Vec<serde_json::Value>,
}

/// Result of invoking an action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionResult {
    #[serde(rename = "actionName")]
    pub action_name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "isSuccess")]
    pub is_success: bool,
    #[serde(rename = "outputValues")]
    pub output_values: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_invocable_action_deserialize() {
        let json = json!({
            "name": "chatterPost",
            "label": "Post to Chatter",
            "type": "APEX"
        });
        let action: InvocableAction = serde_json::from_value(json).unwrap();
        assert_eq!(action.name, "chatterPost");
        assert_eq!(action.action_type, "APEX");
    }

    #[test]
    fn test_invocable_action_collection_deserialize() {
        let json = json!({
            "actions": [{
                "name": "chatterPost",
                "label": "Post to Chatter",
                "type": "APEX"
            }]
        });
        let collection: InvocableActionCollection = serde_json::from_value(json).unwrap();
        assert_eq!(collection.actions.len(), 1);
    }

    #[test]
    fn test_invocable_action_describe_deserialize() {
        let json = json!({
            "name": "chatterPost",
            "label": "Post to Chatter",
            "type": "APEX",
            "inputs": [{
                "name": "text",
                "label": "Post Text",
                "type": "STRING",
                "required": true,
                "description": "The text to post"
            }],
            "outputs": [{
                "name": "feedItemId",
                "label": "Feed Item ID",
                "type": "STRING",
                "required": false,
                "description": null
            }]
        });
        let describe: InvocableActionDescribe = serde_json::from_value(json).unwrap();
        assert_eq!(describe.inputs.len(), 1);
        assert!(describe.inputs[0].required);
        assert_eq!(describe.outputs.len(), 1);
        assert!(!describe.outputs[0].required);
    }

    #[test]
    fn test_invocable_action_request_serialize() {
        let request = InvocableActionRequest {
            inputs: vec![json!({"text": "Hello World"})],
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["inputs"][0]["text"], "Hello World");
    }

    #[test]
    fn test_invocable_action_result_deserialize() {
        let json = json!({
            "actionName": "chatterPost",
            "errors": [],
            "isSuccess": true,
            "outputValues": {"feedItemId": "0D5xx0000000001"}
        });
        let result: InvocableActionResult = serde_json::from_value(json).unwrap();
        assert!(result.is_success);
        assert_eq!(result.action_name, "chatterPost");
        assert!(result.output_values.is_some());
    }

    #[test]
    fn test_invocable_action_result_null_errors() {
        // Salesforce returns "errors": null instead of "errors": []
        let json = json!({
            "actionName": "chatterPost",
            "errors": null,
            "isSuccess": true,
            "outputValues": {"feedItemId": "0D5xx0000000001"}
        });
        let result: InvocableActionResult = serde_json::from_value(json).unwrap();
        assert!(result.is_success);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invocable_action_result_failure() {
        let json = json!({
            "actionName": "chatterPost",
            "errors": [{
                "statusCode": "INVALID_INPUT",
                "message": "Missing required input",
                "fields": []
            }],
            "isSuccess": false,
            "outputValues": null
        });
        let result: InvocableActionResult = serde_json::from_value(json).unwrap();
        assert!(!result.is_success);
        assert_eq!(result.errors.len(), 1);
    }
}
