use tracing::instrument;

use busbar_sf_client::security::url as url_security;

use crate::embedded_service::EmbeddedServiceConfig;
use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Get embedded service configuration by ID.
    #[instrument(skip(self))]
    pub async fn get_embedded_service_config(
        &self,
        config_id: &str,
    ) -> Result<EmbeddedServiceConfig> {
        if !url_security::is_valid_salesforce_id(config_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("embeddedservice/configuration/{}", config_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_get_embedded_service_config_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_embedded_service_config("bad-id").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_get_embedded_service_config_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "id": "0Hsxx0000000001AAA",
            "isEnabled": true,
            "settings": {
                "chatButtonId": "573xx0000000001"
            }
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/embeddedservice/configuration/0Hsxx0000000001AAA$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_embedded_service_config("0Hsxx0000000001AAA")
            .await
            .expect("get_embedded_service_config should succeed");
        assert_eq!(result.id, "0Hsxx0000000001AAA");
        assert!(result.is_enabled);
    }
}
