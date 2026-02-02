use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::error::{Error, ErrorKind, Result};
use crate::knowledge::{
    DataCategoriesResponse, DataCategoryGroupsResponse, KnowledgeArticlesResponse,
    KnowledgeSettings,
};

impl super::SalesforceRestClient {
    /// Get knowledge management settings.
    #[instrument(skip(self))]
    pub async fn knowledge_settings(&self) -> Result<KnowledgeSettings> {
        self.client
            .rest_get("knowledgeManagement/settings")
            .await
            .map_err(Into::into)
    }

    /// List knowledge articles, optionally filtering by query string and channel.
    #[instrument(skip(self))]
    pub async fn knowledge_articles(
        &self,
        query: Option<&str>,
        channel: Option<&str>,
    ) -> Result<KnowledgeArticlesResponse> {
        let mut path = "support/knowledgeArticles".to_string();
        let mut params = Vec::new();
        if let Some(q) = query {
            params.push(format!("q={}", urlencoding::encode(q)));
        }
        if let Some(ch) = channel {
            params.push(format!("channel={}", urlencoding::encode(ch)));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get data category groups, optionally filtered by SObject type.
    #[instrument(skip(self))]
    pub async fn data_category_groups(
        &self,
        sobject: Option<&str>,
    ) -> Result<DataCategoryGroupsResponse> {
        if let Some(s) = sobject {
            if !soql::is_safe_sobject_name(s) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_SOBJECT".to_string(),
                    message: "Invalid SObject name".to_string(),
                }));
            }
        }
        let path = match sobject {
            Some(s) => format!("support/dataCategoryGroups?sObjectType={}", s),
            None => "support/dataCategoryGroups".to_string(),
        };
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get data categories within a group, optionally filtered by SObject type.
    #[instrument(skip(self))]
    pub async fn data_categories(
        &self,
        group: &str,
        sobject: Option<&str>,
    ) -> Result<DataCategoriesResponse> {
        if !soql::is_safe_field_name(group) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_GROUP".to_string(),
                message: "Invalid data category group name".to_string(),
            }));
        }
        if let Some(s) = sobject {
            if !soql::is_safe_sobject_name(s) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_SOBJECT".to_string(),
                    message: "Invalid SObject name".to_string(),
                }));
            }
        }
        let path = match sobject {
            Some(s) => format!(
                "support/dataCategoryGroups/{}/dataCategories?sObjectType={}",
                group, s
            ),
            None => format!("support/dataCategoryGroups/{}/dataCategories", group),
        };
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_data_category_groups_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.data_category_groups(Some("Bad'; DROP--")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_data_categories_invalid_group() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.data_categories("Bad'; DROP--", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_GROUP"));
    }

    #[tokio::test]
    async fn test_knowledge_settings_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "defaultLanguage": "en_US",
            "knowledgeEnabled": true,
            "languages": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/knowledgeManagement/settings$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .knowledge_settings()
            .await
            .expect("knowledge_settings should succeed");
        assert!(result.knowledge_enabled);
    }

    #[tokio::test]
    async fn test_knowledge_articles_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "articles": [{
                "id": "kA0xx0000000001",
                "articleNumber": "000001",
                "title": "Test Article",
                "urlName": "test-article",
                "summary": "A test article"
            }],
            "currentPageUrl": null,
            "nextPageUrl": null,
            "pageNumber": 1
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/support/knowledgeArticles.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .knowledge_articles(Some("test"), None)
            .await
            .expect("knowledge_articles should succeed");
        assert_eq!(result.articles.len(), 1);
    }

    #[tokio::test]
    async fn test_data_category_groups_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "categoryGroups": [{
                "name": "Products",
                "label": "Products",
                "objectUsage": null,
                "topCategoriesUrl": null
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/support/dataCategoryGroups$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .data_category_groups(None)
            .await
            .expect("data_category_groups should succeed");
        assert_eq!(result.category_groups.len(), 1);
    }

    #[tokio::test]
    async fn test_data_categories_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "categories": [{
                "name": "Software",
                "label": "Software",
                "url": null,
                "childCategories": []
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/support/dataCategoryGroups/Products/dataCategories.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .data_categories("Products", None)
            .await
            .expect("data_categories should succeed");
        assert_eq!(result.categories.len(), 1);
    }
}
