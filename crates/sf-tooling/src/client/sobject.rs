use busbar_sf_client::security::{soql, url as url_security};
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};

/// Response from create operations.
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct CreateResponse {
    pub(super) id: String,
    pub(super) success: bool,
    #[serde(default)]
    pub(super) errors: Vec<CreateError>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct CreateError {
    pub(super) message: String,
    #[serde(rename = "statusCode")]
    #[allow(dead_code)]
    pub(super) status_code: String,
}

impl super::ToolingClient {
    /// Get a Tooling API SObject by ID.
    #[instrument(skip(self))]
    pub async fn get<T: serde::de::DeserializeOwned>(&self, sobject: &str, id: &str) -> Result<T> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/{}", sobject, id);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    /// Create a Tooling API SObject.
    #[instrument(skip(self, record))]
    pub async fn create<T: serde::Serialize>(&self, sobject: &str, record: &T) -> Result<String> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        let result: CreateResponse = self.client.tooling_post(&path, record).await?;

        if result.success {
            Ok(result.id)
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "CREATE_FAILED".to_string(),
                message: result
                    .errors
                    .into_iter()
                    .map(|e| e.message)
                    .collect::<Vec<_>>()
                    .join("; "),
            }))
        }
    }

    /// Update a Tooling API SObject (partial update).
    #[instrument(skip(self, record))]
    pub async fn update<T: serde::Serialize>(
        &self,
        sobject: &str,
        id: &str,
        record: &T,
    ) -> Result<()> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/{}/{}",
            self.client.instance_url(),
            self.client.api_version(),
            sobject,
            id
        );

        self.client
            .patch_json(&url, record)
            .await
            .map_err(Into::into)
    }

    /// Delete a Tooling API SObject.
    #[instrument(skip(self))]
    pub async fn delete(&self, sobject: &str, id: &str) -> Result<()> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/{}/{}",
            self.client.instance_url(),
            self.client.api_version(),
            sobject,
            id
        );

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "DELETE_FAILED".to_string(),
                message: format!("Failed to delete {}: status {}", sobject, response.status()),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

    #[tokio::test]
    async fn test_update_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path_regex(
                ".*/tooling/sobjects/TraceFlag/7tf000000000001AAA",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let update_body = serde_json::json!({
            "ExpirationDate": "2026-12-31T23:59:59.000Z"
        });
        let result = client
            .update("TraceFlag", "7tf000000000001AAA", &update_body)
            .await;
        assert!(result.is_ok(), "update should succeed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_update_invalid_sobject() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client
            .update(
                "Robert'; DROP TABLE--",
                "7tf000000000001AAA",
                &serde_json::json!({}),
            )
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_update_invalid_id() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client
            .update("TraceFlag", "not-valid-id!", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID, got: {err}"
        );
    }
}
