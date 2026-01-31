use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::consent::{ConsentResponse, ConsentWriteRequest};
use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Read consent status for an action and a set of IDs.
    #[instrument(skip(self))]
    pub async fn read_consent(&self, action: &str, ids: &[&str]) -> Result<ConsentResponse> {
        if !soql::is_safe_field_name(action) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid consent action name".to_string(),
            }));
        }
        let ids_param = ids.join(",");
        let path = format!("consent/action/{}?ids={}", action, ids_param);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Write consent for an action (uses PATCH, not POST).
    #[instrument(skip(self, request))]
    pub async fn write_consent(&self, action: &str, request: &ConsentWriteRequest) -> Result<()> {
        if !soql::is_safe_field_name(action) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid consent action name".to_string(),
            }));
        }
        let path = format!("consent/action/{}", action);
        self.client
            .rest_patch(&path, request)
            .await
            .map_err(Into::into)
    }

    /// Read consent for multiple actions and IDs.
    #[instrument(skip(self))]
    pub async fn read_multi_consent(
        &self,
        actions: &[&str],
        ids: &[&str],
    ) -> Result<serde_json::Value> {
        let actions_param = actions.join(",");
        let ids_param = ids.join(",");
        let path = format!(
            "consent/multiaction?actions={}&ids={}",
            actions_param, ids_param
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_read_consent_invalid_action() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.read_consent("Bad'; DROP--", &["001xx"]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_write_consent_invalid_action() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let request = crate::consent::ConsentWriteRequest { records: vec![] };
        let result = client.write_consent("Bad'; DROP--", &request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_read_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "results": [{
                "result": "OptIn",
                "status": "Active",
                "objectConsulted": "ContactPointEmail"
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/consent/action/email.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .read_consent("email", &["001xx000003DgAAAS"])
            .await
            .expect("read_consent should succeed");
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].result, "OptIn");
    }

    #[tokio::test]
    async fn test_write_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path_regex(".*/consent/action/email$"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::consent::ConsentWriteRequest {
            records: vec![crate::consent::ConsentWriteRecord {
                id: "001xx000003DgAAAS".to_string(),
                result: "OptIn".to_string(),
            }],
        };
        client
            .write_consent("email", &request)
            .await
            .expect("write_consent should succeed");
    }

    #[tokio::test]
    async fn test_read_multi_consent_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "results": [
                {"action": "email", "status": "OptIn"},
                {"action": "sms", "status": "OptOut"}
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/consent/multiaction.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .read_multi_consent(&["email", "sms"], &["001xx000003DgAAAS"])
            .await
            .expect("read_multi_consent should succeed");
        assert!(result["results"].is_array());
    }
}
