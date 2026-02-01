use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Get deleted records for an SObject type within a date range.
    ///
    /// The start and end parameters should be ISO 8601 date-time strings
    /// (e.g., "2024-01-01T00:00:00Z").
    #[instrument(skip(self))]
    pub async fn get_deleted(
        &self,
        sobject: &str,
        start: &str,
        end: &str,
    ) -> Result<super::GetDeletedResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/deleted/?start={}&end={}",
            sobject,
            urlencoding::encode(start),
            urlencoding::encode(end)
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get updated record IDs for an SObject type within a date range.
    ///
    /// The start and end parameters should be ISO 8601 date-time strings
    /// (e.g., "2024-01-01T00:00:00Z").
    #[instrument(skip(self))]
    pub async fn get_updated(
        &self,
        sobject: &str,
        start: &str,
        end: &str,
    ) -> Result<super::GetUpdatedResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!(
            "sobjects/{}/updated/?start={}&end={}",
            sobject,
            urlencoding::encode(start),
            urlencoding::encode(end)
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_get_deleted_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_deleted(
                "Bad'; DROP--",
                "2024-01-01T00:00:00Z",
                "2024-01-15T00:00:00Z",
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_updated_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .get_updated(
                "Bad'; DROP--",
                "2024-01-01T00:00:00Z",
                "2024-01-15T00:00:00Z",
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_deleted_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "deletedRecords": [
                {"id": "001xx000003DgAAAS", "deletedDate": "2024-01-15T10:30:00.000Z"}
            ],
            "earliestDateAvailable": "2024-01-01T00:00:00.000Z",
            "latestDateCovered": "2024-01-15T23:59:59.000Z"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/deleted/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_deleted("Account", "2024-01-01T00:00:00Z", "2024-01-15T00:00:00Z")
            .await
            .expect("get_deleted should succeed");
        assert_eq!(result.deleted_records.len(), 1);
        assert_eq!(result.deleted_records[0].id, "001xx000003DgAAAS");
    }

    #[tokio::test]
    async fn test_get_updated_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "ids": ["001xx000003DgAAAS", "001xx000003DgBBAS"],
            "latestDateCovered": "2024-01-15T23:59:59.000Z"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/updated/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_updated("Account", "2024-01-01T00:00:00Z", "2024-01-15T00:00:00Z")
            .await
            .expect("get_updated should succeed");
        assert_eq!(result.ids.len(), 2);
    }
}
