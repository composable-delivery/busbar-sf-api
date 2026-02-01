use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};

use crate::error::{Error, ErrorKind, Result};
use crate::search::{
    ParameterizedSearchRequest, ParameterizedSearchResponse, ScopeEntity, SearchLayoutInfo,
    SearchSuggestionResult,
};

impl super::SalesforceRestClient {
    /// Execute a SOSL search.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: If you are including user-provided values in the search term,
    /// you MUST escape them. Use `busbar_sf_client::security::soql::escape_string()`
    /// for string values in SOSL queries.
    #[instrument(skip(self))]
    pub async fn search<T: DeserializeOwned>(&self, sosl: &str) -> Result<super::SearchResult<T>> {
        let encoded = urlencoding::encode(sosl);
        let url = format!(
            "{}/services/data/v{}/search?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Execute a parameterized search request.
    ///
    /// This provides a structured alternative to raw SOSL queries,
    /// with support for filtering by SObject type, field selection,
    /// and pagination.
    #[instrument(skip(self, request))]
    pub async fn parameterized_search(
        &self,
        request: &ParameterizedSearchRequest,
    ) -> Result<ParameterizedSearchResponse> {
        // Validate SObject names if specified
        if let Some(ref sobjects) = request.sobjects {
            for spec in sobjects {
                if !soql::is_safe_sobject_name(&spec.name) {
                    return Err(Error::new(ErrorKind::Salesforce {
                        error_code: "INVALID_SOBJECT".to_string(),
                        message: format!("Invalid SObject name: {}", spec.name),
                    }));
                }
            }
        }
        self.client
            .rest_post("parameterizedSearch", request)
            .await
            .map_err(Into::into)
    }

    /// Get search suggestions (auto-complete) for a query string and SObject type.
    ///
    /// Returns suggested records matching the query prefix.
    #[instrument(skip(self))]
    pub async fn search_suggestions(
        &self,
        query: &str,
        sobject: &str,
    ) -> Result<SearchSuggestionResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let encoded_query = url_security::encode_param(query);
        let url = format!(
            "{}/services/data/v{}/search/suggestions?q={}&sobject={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded_query,
            sobject
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Get the search scope order for the current user.
    ///
    /// Returns the list of SObjects in the order they appear in the user's search scope.
    #[instrument(skip(self))]
    pub async fn search_scope_order(&self) -> Result<Vec<ScopeEntity>> {
        let url = format!(
            "{}/services/data/v{}/search/scopeOrder",
            self.client.instance_url(),
            self.client.api_version(),
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Get search result layouts for the specified SObject types.
    ///
    /// Returns the columns displayed in search results for each SObject type.
    #[instrument(skip(self))]
    pub async fn search_result_layouts(&self, sobjects: &[&str]) -> Result<Vec<SearchLayoutInfo>> {
        // Validate all SObject names
        for sobject in sobjects {
            if !soql::is_safe_sobject_name(sobject) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_SOBJECT".to_string(),
                    message: format!("Invalid SObject name: {}", sobject),
                }));
            }
        }
        let sobjects_param = sobjects.join(",");
        let url = format!(
            "{}/services/data/v{}/search/layout?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            sobjects_param
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;
    use crate::search::ParameterizedSearchRequest;

    #[tokio::test]
    async fn test_parameterized_search_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let request = ParameterizedSearchRequest {
            q: "test".to_string(),
            sobjects: Some(vec![crate::search::SearchSObjectSpec {
                name: "Bad'; DROP--".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        };
        let result = client.parameterized_search(&request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_search_suggestions_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.search_suggestions("test", "Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_search_result_layouts_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.search_result_layouts(&["Bad'; DROP--"]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_parameterized_search_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "searchRecords": [
                {
                    "attributes": {"type": "Account", "url": "/services/data/v62.0/sobjects/Account/001xx"},
                    "Id": "001xx000003Dgb2AAC",
                    "Name": "Acme"
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/parameterizedSearch$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = ParameterizedSearchRequest {
            q: "Acme".to_string(),
            ..Default::default()
        };
        let result = client
            .parameterized_search(&request)
            .await
            .expect("parameterized_search should succeed");

        assert_eq!(result.search_records.len(), 1);
    }

    #[tokio::test]
    async fn test_search_suggestions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "autoSuggestResults": [
                {
                    "attributes": {"type": "Account", "url": "/services/data/v62.0/sobjects/Account/001xx"},
                    "Id": "001xx000003Dgb2AAC",
                    "Name": "Acme Corp"
                }
            ],
            "hasMoreResults": false
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/search/suggestions.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .search_suggestions("Acme", "Account")
            .await
            .expect("search_suggestions should succeed");

        assert_eq!(result.auto_suggest_results.len(), 1);
        assert!(!result.has_more_results);
    }

    #[tokio::test]
    async fn test_search_scope_order_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {
                "name": "Account",
                "label": "Accounts",
                "inSearchScope": true,
                "searchScopeOrder": 1
            }
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/search/scopeOrder$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .search_scope_order()
            .await
            .expect("search_scope_order should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Account");
    }

    #[tokio::test]
    async fn test_search_result_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {
                "label": "Accounts",
                "searchColumns": [
                    {
                        "field": "Account.Name",
                        "label": "Account Name",
                        "format": "string",
                        "name": "Name"
                    }
                ]
            }
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/search/layout.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .search_result_layouts(&["Account"])
            .await
            .expect("search_result_layouts should succeed");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "Accounts");
        assert_eq!(result[0].columns.len(), 1);
    }
}
