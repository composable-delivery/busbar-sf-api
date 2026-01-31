//! Knowledge Management types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// Knowledge management settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeSettings {
    #[serde(rename = "defaultLanguage", default)]
    pub default_language: String,
    #[serde(rename = "knowledgeEnabled", default)]
    pub knowledge_enabled: bool,
    #[serde(default)]
    pub languages: Vec<serde_json::Value>,
}

/// Response from knowledge articles endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeArticlesResponse {
    #[serde(default)]
    pub articles: Vec<KnowledgeArticle>,
    #[serde(rename = "currentPageUrl")]
    pub current_page_url: Option<String>,
    #[serde(rename = "nextPageUrl")]
    pub next_page_url: Option<String>,
    #[serde(rename = "pageNumber", default)]
    pub page_number: i32,
}

/// A knowledge article.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeArticle {
    pub id: String,
    #[serde(rename = "articleNumber", default)]
    pub article_number: String,
    pub title: Option<String>,
    #[serde(rename = "urlName")]
    pub url_name: Option<String>,
    pub summary: Option<String>,
}

/// Response from data category groups endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoryGroupsResponse {
    #[serde(rename = "categoryGroups", default)]
    pub category_groups: Vec<DataCategoryGroup>,
}

/// A data category group.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoryGroup {
    pub name: String,
    pub label: String,
    #[serde(rename = "objectUsage")]
    pub object_usage: Option<String>,
    #[serde(rename = "topCategoriesUrl")]
    pub top_categories_url: Option<String>,
}

/// Response from data categories endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategoriesResponse {
    #[serde(default)]
    pub categories: Vec<DataCategory>,
}

/// A data category.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DataCategory {
    pub name: String,
    pub label: String,
    pub url: Option<String>,
    #[serde(rename = "childCategories", default)]
    pub child_categories: Vec<DataCategory>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_knowledge_settings_deserialize() {
        let json = json!({
            "defaultLanguage": "en_US",
            "knowledgeEnabled": true,
            "languages": [{"name": "en_US", "label": "English"}]
        });
        let settings: KnowledgeSettings = serde_json::from_value(json).unwrap();
        assert!(settings.knowledge_enabled);
        assert_eq!(settings.default_language, "en_US");
    }

    #[test]
    fn test_knowledge_articles_response_deserialize() {
        let json = json!({
            "articles": [{
                "id": "kA0xx0000000001",
                "articleNumber": "000001",
                "title": "How to Reset Password",
                "urlName": "how-to-reset-password",
                "summary": "Instructions for resetting your password"
            }],
            "currentPageUrl": "/articles?pageNumber=1",
            "nextPageUrl": null,
            "pageNumber": 1
        });
        let response: KnowledgeArticlesResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.articles.len(), 1);
        assert_eq!(response.articles[0].article_number, "000001");
    }

    #[test]
    fn test_data_category_groups_response_deserialize() {
        let json = json!({
            "categoryGroups": [{
                "name": "Products",
                "label": "Products",
                "objectUsage": "KnowledgeArticle",
                "topCategoriesUrl": "/support/dataCategoryGroups/Products/dataCategories"
            }]
        });
        let response: DataCategoryGroupsResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.category_groups.len(), 1);
        assert_eq!(response.category_groups[0].name, "Products");
    }

    #[test]
    fn test_data_categories_response_deserialize() {
        let json = json!({
            "categories": [{
                "name": "Software",
                "label": "Software",
                "url": "/support/dataCategoryGroups/Products/dataCategories/Software",
                "childCategories": [{
                    "name": "CRM",
                    "label": "CRM",
                    "url": null,
                    "childCategories": []
                }]
            }]
        });
        let response: DataCategoriesResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.categories.len(), 1);
        assert_eq!(response.categories[0].child_categories.len(), 1);
    }
}
