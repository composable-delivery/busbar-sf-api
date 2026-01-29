//! Invocable Actions types and operations.

use serde::{Deserialize, Serialize};

/// An invocable action (standard or custom).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableAction {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Collection of invocable actions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionCollection {
    #[serde(default)]
    pub actions: Vec<InvocableAction>,
}

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

/// An input or output parameter for an invocable action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvocableActionParameter {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub param_type: String,
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
    #[serde(default)]
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "isSuccess")]
    pub is_success: bool,
    #[serde(rename = "outputValues")]
    pub output_values: Option<serde_json::Value>,
}
