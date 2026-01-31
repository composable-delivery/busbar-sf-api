//! Quick Actions types and operations.

use serde::{Deserialize, Serialize};

/// A quick action available on an SObject.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct QuickAction {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// Detailed description of a quick action.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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
    pub layout: Option<serde_json::Value>,
    #[serde(rename = "defaultValues")]
    pub default_values: Option<serde_json::Value>,
    pub icons: Vec<QuickActionIcon>,
}

/// Icon information for a quick action.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct QuickActionIcon {
    pub url: String,
    pub theme: String,
    pub height: u32,
    pub width: u32,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

/// Result of invoking a quick action.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct QuickActionResult {
    pub id: Option<String>,
    pub success: bool,
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "contextId")]
    pub context_id: Option<String>,
    #[serde(rename = "feedItemId")]
    pub feed_item_id: Option<String>,
}
