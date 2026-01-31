//! Knowledge Management and Support API types.
//!
//! Provides access to Knowledge Management settings, knowledge articles, and data categories.
//! See: <https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_knowledge_management_settings.htm>

use serde::{Deserialize, Serialize};

/// Knowledge Management settings for the org.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeSettings {
    /// Whether Knowledge is enabled
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    /// Default language for Knowledge articles
    #[serde(rename = "defaultLanguage", skip_serializing_if = "Option::is_none")]
    pub default_language: Option<String>,
    /// Other settings as dynamic values
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Response containing a list of knowledge articles.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeArticlesResponse {
    /// List of articles
    pub articles: Vec<KnowledgeArticle>,
    /// Next page URL if available
    #[serde(rename = "nextPageUrl", skip_serializing_if = "Option::is_none")]
    pub next_page_url: Option<String>,
}

/// A Knowledge article summary.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeArticle {
    /// Article ID
    pub id: String,
    /// Article title
    pub title: String,
    /// Article URL name
    #[serde(rename = "urlName", skip_serializing_if = "Option::is_none")]
    pub url_name: Option<String>,
    /// Article type
    #[serde(rename = "articleType", skip_serializing_if = "Option::is_none")]
    pub article_type: Option<String>,
    /// Additional fields
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Response containing data category groups.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoryGroupsResponse {
    /// List of category groups
    #[serde(rename = "categoryGroups")]
    pub category_groups: Vec<DataCategoryGroup>,
}

/// A data category group.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoryGroup {
    /// Group name
    pub name: String,
    /// Group label
    pub label: String,
    /// Whether the group is active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// Response containing data categories within a group.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoriesResponse {
    /// List of categories
    pub categories: Vec<DataCategory>,
}

/// A data category.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategory {
    /// Category name
    pub name: String,
    /// Category label
    pub label: String,
    /// Parent category name
    #[serde(rename = "parentName", skip_serializing_if = "Option::is_none")]
    pub parent_name: Option<String>,
    /// Child categories
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DataCategory>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_settings_deserialization() {
        let json = r#"{"isEnabled": true, "defaultLanguage": "en_US"}"#;
        let settings: KnowledgeSettings = serde_json::from_str(json).unwrap();
        assert!(settings.is_enabled);
        assert_eq!(settings.default_language, Some("en_US".to_string()));
    }

    #[test]
    fn test_knowledge_article_deserialization() {
        let json = r#"{
            "id": "kA0xx0000000001AAA",
            "title": "Test Article",
            "urlName": "test-article"
        }"#;
        let article: KnowledgeArticle = serde_json::from_str(json).unwrap();
        assert_eq!(article.id, "kA0xx0000000001AAA");
        assert_eq!(article.title, "Test Article");
    }

    #[test]
    fn test_data_category_group() {
        let group = DataCategoryGroup {
            name: "Products".to_string(),
            label: "Products".to_string(),
            active: Some(true),
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("Products"));
    }
}
