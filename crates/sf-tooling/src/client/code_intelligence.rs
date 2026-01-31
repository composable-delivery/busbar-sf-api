use tracing::instrument;

use crate::error::Result;
use crate::types::CompletionsResult;

impl super::ToolingClient {
    /// Get code completions for Apex system symbols.
    ///
    /// Available since API v28.0.
    #[instrument(skip(self))]
    pub async fn completions_apex(&self) -> Result<CompletionsResult> {
        let url = self.client.tooling_url("completions?type=apex");
        self.client.get_json(&url).await.map_err(Into::into)
    }

    /// Get code completions for Visualforce components.
    ///
    /// Available since API v38.0.
    #[instrument(skip(self))]
    pub async fn completions_visualforce(&self) -> Result<CompletionsResult> {
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
                "publicDeclarations": [
                    {
                        "name": "System",
                        "type": "Class",
                        "namespace": null,
                        "signature": "System class"
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
        let result = client.completions_apex().await;

        assert!(result.is_ok());
        let completions = result.unwrap();
        assert_eq!(completions.public_declarations.public_declarations.len(), 1);
        assert_eq!(
            completions.public_declarations.public_declarations[0].name,
            "System"
        );
    }

    #[tokio::test]
    async fn test_completions_visualforce_wiremock() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "publicDeclarations": {
                "publicDeclarations": [
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
        let result = client.completions_visualforce().await;

        assert!(result.is_ok());
        let completions = result.unwrap();
        assert_eq!(completions.public_declarations.public_declarations.len(), 1);
        assert_eq!(
            completions.public_declarations.public_declarations[0].name,
            "apex:page"
        );
    }
}
