//! Platform actions and suggested articles API types.
//!
//! Provides access to suggested articles and platform actions for SObjects.
//! See: <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_sobject_suggested_articles.htm>

use serde::{Deserialize, Serialize};

/// Response containing suggested articles.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SuggestedArticlesResponse {
    /// List of suggested articles
    pub articles: Vec<SuggestedArticle>,
}

/// A suggested knowledge article.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SuggestedArticle {
    /// Article ID
    pub id: String,
    /// Article title
    pub title: String,
    /// Relevance score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Additional article metadata
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Response containing platform actions for an SObject.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformActionsResponse {
    /// List of available actions
    pub actions: Vec<PlatformAction>,
}

/// A platform action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformAction {
    /// Action type
    #[serde(rename = "actionType", skip_serializing_if = "Option::is_none")]
    pub action_type: Option<String>,
    /// Action name
    pub name: String,
    /// Action label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether the action is available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    /// Additional action metadata
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggested_article_deserialization() {
        let json = r#"{
            "id": "kA0xx0000000001",
            "title": "How to Configure",
            "score": 0.95
        }"#;
        let article: SuggestedArticle = serde_json::from_str(json).unwrap();
        assert_eq!(article.id, "kA0xx0000000001");
        assert_eq!(article.title, "How to Configure");
        assert_eq!(article.score, Some(0.95));
    }

    #[test]
    fn test_platform_action_deserialization() {
        let json = r#"{
            "actionType": "QuickAction",
            "name": "NewTask",
            "label": "New Task",
            "available": true
        }"#;
        let action: PlatformAction = serde_json::from_str(json).unwrap();
        assert_eq!(action.name, "NewTask");
        assert_eq!(action.label, Some("New Task".to_string()));
        assert_eq!(action.available, Some(true));
    }
}
