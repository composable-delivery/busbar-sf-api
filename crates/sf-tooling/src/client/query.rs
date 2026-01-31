use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::QueryResult;

use crate::error::Result;
use crate::types::SearchResult;

impl super::ToolingClient {
    /// Execute a SOQL query against the Tooling API.
    ///
    /// Returns the first page of results. Use `query_all` for automatic pagination.
    ///
    /// # Security
    ///
    /// **IMPORTANT**: If you are including user-provided values in the WHERE clause,
    /// you MUST escape them to prevent SOQL injection attacks:
    ///
    /// ```rust,ignore
    /// use busbar_sf_client::security::soql;
    ///
    /// // CORRECT - properly escaped:
    /// let safe_name = soql::escape_string(user_input);
    /// let query = format!("SELECT Id FROM ApexClass WHERE Name = '{}'", safe_name);
    /// ```
    #[instrument(skip(self))]
    pub async fn query<T: DeserializeOwned>(&self, soql: &str) -> Result<QueryResult<T>> {
        self.client.tooling_query(soql).await.map_err(Into::into)
    }

    /// Execute a SOQL query and return all results (automatic pagination).
    ///
    /// # Security
    ///
    /// **IMPORTANT**: Escape user-provided values with `busbar_sf_client::security::soql::escape_string()`
    /// to prevent SOQL injection attacks. See `query()` for examples.
    #[instrument(skip(self))]
    pub async fn query_all<T: DeserializeOwned + Clone>(&self, soql: &str) -> Result<Vec<T>> {
        self.client
            .tooling_query_all(soql)
            .await
            .map_err(Into::into)
    }

    /// Execute a SOQL query including deleted and archived records.
    #[instrument(skip(self))]
    pub async fn query_all_records<T: DeserializeOwned>(
        &self,
        soql: &str,
    ) -> Result<QueryResult<T>> {
        let encoded = urlencoding::encode(soql);
        let url = format!(
            "{}/services/data/v{}/tooling/queryAll/?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Execute a SOSL search against Tooling API objects.
    #[instrument(skip(self))]
    pub async fn search<T: DeserializeOwned>(&self, sosl: &str) -> Result<SearchResult<T>> {
        let encoded = urlencoding::encode(sosl);
        let url = format!(
            "{}/services/data/v{}/tooling/search/?q={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

    #[tokio::test]
    async fn test_query_all_records_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "totalSize": 2,
            "done": true,
            "records": [
                {"Id": "01p000000000001AAA", "Name": "DeletedClass", "IsDeleted": true},
                {"Id": "01p000000000002AAA", "Name": "ArchivedClass", "IsDeleted": true}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/queryAll/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result: busbar_sf_client::QueryResult<serde_json::Value> = client
            .query_all_records("SELECT Id, Name FROM ApexClass WHERE IsDeleted = true")
            .await
            .expect("should succeed");
        assert_eq!(result.total_size, 2);
        assert_eq!(result.records.len(), 2);
        assert!(result.done);
    }

    #[tokio::test]
    async fn test_search_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "searchRecords": [
                {
                    "Id": "01p000000000001AAA",
                    "attributes": {
                        "type": "ApexClass",
                        "url": "/services/data/v62.0/tooling/sobjects/ApexClass/01p000000000001AAA"
                    }
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/search/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result: crate::types::SearchResult<serde_json::Value> = client
            .search("FIND {test} IN ALL FIELDS RETURNING ApexClass(Id, Name)")
            .await
            .expect("should succeed");
        assert_eq!(result.search_records.len(), 1);
    }
}
