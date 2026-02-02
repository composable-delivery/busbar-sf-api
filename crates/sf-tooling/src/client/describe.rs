use busbar_sf_client::security::soql;
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};

impl super::ToolingClient {
    /// Get a list of all Tooling API SObjects available in the org.
    #[instrument(skip(self))]
    pub async fn describe_global(&self) -> Result<busbar_sf_rest::DescribeGlobalResult> {
        self.client
            .tooling_get("sobjects")
            .await
            .map_err(Into::into)
    }

    /// Get detailed metadata for a specific Tooling API SObject.
    #[instrument(skip(self))]
    pub async fn describe_sobject(
        &self,
        sobject: &str,
    ) -> Result<busbar_sf_rest::DescribeSObjectResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe", sobject);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    /// Get basic information about a Tooling API SObject.
    #[instrument(skip(self))]
    pub async fn basic_info(&self, sobject: &str) -> Result<serde_json::Value> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}", sobject);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    /// Get a list of all available Tooling API resources.
    #[instrument(skip(self))]
    pub async fn resources(&self) -> Result<serde_json::Value> {
        let url = format!(
            "{}/services/data/v{}/tooling/",
            self.client.instance_url(),
            self.client.api_version()
        );
        self.client.get_json(&url).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

    #[tokio::test]
    async fn test_describe_global_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "encoding": "UTF-8",
            "maxBatchSize": 200,
            "sobjects": [
                {
                    "name": "ApexClass",
                    "label": "Apex Class",
                    "labelPlural": "Apex Classes",
                    "keyPrefix": "01p",
                    "custom": false,
                    "queryable": true,
                    "createable": true,
                    "updateable": true,
                    "deletable": true,
                    "searchable": true,
                    "retrieveable": true
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/sobjects$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client.describe_global().await.expect("should succeed");
        assert_eq!(result.encoding, "UTF-8");
        assert_eq!(result.max_batch_size, 200);
        assert_eq!(result.sobjects.len(), 1);
        assert_eq!(result.sobjects[0].name, "ApexClass");
    }

    #[tokio::test]
    async fn test_describe_sobject_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "name": "ApexClass",
            "label": "Apex Class",
            "labelPlural": "Apex Classes",
            "keyPrefix": "01p",
            "custom": false,
            "createable": true,
            "updateable": true,
            "deletable": true,
            "queryable": true,
            "searchable": true,
            "retrieveable": true,
            "fields": [
                {
                    "name": "Id",
                    "label": "Apex Class ID",
                    "type": "id"
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/sobjects/ApexClass/describe"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_sobject("ApexClass")
            .await
            .expect("should succeed");
        assert_eq!(result.name, "ApexClass");
        assert_eq!(result.label, "Apex Class");
        assert!(result.createable);
        assert!(!result.fields.is_empty());
        assert_eq!(result.fields[0].name, "Id");
    }

    #[tokio::test]
    async fn test_describe_sobject_invalid_name() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.describe_sobject("Robert'; DROP TABLE--").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_basic_info_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "objectDescribe": {
                "name": "ApexClass",
                "label": "Apex Class",
                "keyPrefix": "01p"
            },
            "recentItems": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/sobjects/ApexClass$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .basic_info("ApexClass")
            .await
            .expect("should succeed");
        let describe = result
            .get("objectDescribe")
            .expect("should have objectDescribe");
        assert_eq!(describe.get("name").unwrap().as_str().unwrap(), "ApexClass");
    }

    #[tokio::test]
    async fn test_basic_info_invalid_sobject() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result = client.basic_info("Robert'; DROP TABLE--").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_resources_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let body = serde_json::json!({
            "tooling": "/services/data/v62.0/tooling",
            "query": "/services/data/v62.0/tooling/query",
            "search": "/services/data/v62.0/tooling/search",
            "sobjects": "/services/data/v62.0/tooling/sobjects"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/tooling/$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = ToolingClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client.resources().await.expect("should succeed");
        assert!(result.get("query").is_some());
        assert!(result.get("search").is_some());
        assert!(result.get("sobjects").is_some());
    }
}
