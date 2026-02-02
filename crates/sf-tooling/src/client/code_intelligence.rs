use tracing::instrument;

use crate::error::Result;

impl super::ToolingClient {
    /// Get code completions for Apex system symbols.
    ///
    /// Returns the raw JSON response because the Salesforce completions
    /// response format varies between Apex and Visualforce types and
    /// across API versions.
    ///
    /// Available since API v28.0.
    #[instrument(skip(self))]
    pub async fn completions_apex(&self) -> Result<serde_json::Value> {
        let url = self.client.tooling_url("completions?type=apex");
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Get code completions for Visualforce components.
    ///
    /// Returns the raw JSON response because the Salesforce completions
    /// response format varies between Apex and Visualforce types and
    /// across API versions.
    ///
    /// Available since API v38.0.
    #[instrument(skip(self))]
    pub async fn completions_visualforce(&self) -> Result<serde_json::Value> {
        let url = self.client.tooling_url("completions?type=visualforce");
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_completions_apex_wiremock() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "publicDeclarations": {
                "System": [
                    {
                        "name": "debug",
                        "type": "Method",
                        "namespace": "System",
                        "signature": "void debug(Object)"
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .and(path("/services/data/v62.0/tooling/completions"))
            .and(query_param("type", "apex"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let completions = client
            .completions_apex()
            .await
            .expect("completions_apex should succeed");
        let pd = completions["publicDeclarations"]
            .as_object()
            .expect("should have publicDeclarations object");
        assert!(pd.contains_key("System"));
    }

    #[tokio::test]
    async fn test_completions_visualforce_wiremock() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "publicDeclarations": {
                "apex": [
                    {
                        "name": "apex:page",
                        "type": "Component",
                        "namespace": "apex"
                    }
                ]
            }
        });

        Mock::given(method("GET"))
            .and(path("/services/data/v62.0/tooling/completions"))
            .and(query_param("type", "visualforce"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let completions = client
            .completions_visualforce()
            .await
            .expect("completions_visualforce should succeed");
        let pd = completions["publicDeclarations"]
            .as_object()
            .expect("should have publicDeclarations object");
        assert!(pd.contains_key("apex"));
    }
}
