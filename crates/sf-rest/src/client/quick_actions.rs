use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::error::{Error, ErrorKind, Result};
use crate::quick_actions::{QuickAction, QuickActionDescribe, QuickActionResult};

impl super::SalesforceRestClient {
    /// List all global quick actions.
    #[instrument(skip(self))]
    pub async fn list_global_quick_actions(&self) -> Result<Vec<QuickAction>> {
        self.client
            .rest_get("quickActions")
            .await
            .map_err(Into::into)
    }

    /// Describe a global quick action.
    #[instrument(skip(self))]
    pub async fn describe_global_quick_action(
        &self,
        action_name: &str,
    ) -> Result<QuickActionDescribe> {
        if !soql::is_safe_action_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("quickActions/{}", action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// List all quick actions available for an SObject.
    #[instrument(skip(self))]
    pub async fn list_quick_actions(&self, sobject: &str) -> Result<Vec<QuickAction>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Describe a specific quick action.
    ///
    /// Action names can contain dots for SObject-scoped actions
    /// (e.g., `FeedItem.TextPost`, `FeedItem.ContentPost`).
    #[instrument(skip(self))]
    pub async fn describe_quick_action(
        &self,
        sobject: &str,
        action_name: &str,
    ) -> Result<QuickActionDescribe> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !soql::is_safe_action_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions/{}", sobject, action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Invoke a quick action on an SObject.
    ///
    /// Action names can contain dots for SObject-scoped actions
    /// (e.g., `FeedItem.TextPost`, `FeedItem.ContentPost`).
    #[instrument(skip(self, body))]
    pub async fn invoke_quick_action(
        &self,
        sobject: &str,
        action_name: &str,
        body: &serde_json::Value,
    ) -> Result<QuickActionResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !soql::is_safe_action_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/quickActions/{}", sobject, action_name);
        self.client.rest_post(&path, body).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_list_quick_actions_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.list_quick_actions("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_describe_quick_action_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .describe_quick_action("Bad'; DROP--", "NewCase")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_describe_quick_action_invalid_action() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .describe_quick_action("Account", "Bad'; DROP--")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_describe_quick_action_dotted_name_allowed() {
        // Salesforce quick action names can contain dots (e.g., FeedItem.TextPost)
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "name": "FeedItem.TextPost",
            "label": "Post",
            "type": "Create",
            "targetSobjectType": "FeedItem",
            "targetRecordTypeId": null,
            "layout": null,
            "defaultValues": null,
            "icons": []
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/quickActions/FeedItem.TextPost$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_quick_action("Account", "FeedItem.TextPost")
            .await
            .expect("describe_quick_action should accept dotted action names");
        assert_eq!(result.name, "FeedItem.TextPost");
    }

    #[tokio::test]
    async fn test_describe_global_quick_action_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.describe_global_quick_action("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_invoke_quick_action_invalid_action() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .invoke_quick_action("Account", "Bad'; DROP--", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_invoke_quick_action_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .invoke_quick_action("Bad'; DROP--", "NewCase", &serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_list_global_quick_actions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {"name": "NewCase", "label": "New Case", "type": "Create"},
            {"name": "LogACall", "label": "Log a Call", "type": "LogACall"}
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/quickActions$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_global_quick_actions()
            .await
            .expect("list_global_quick_actions should succeed");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "NewCase");
    }

    #[tokio::test]
    async fn test_describe_global_quick_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "name": "LogACall",
            "label": "Log a Call",
            "type": "LogACall",
            "targetSobjectType": "Task",
            "targetRecordTypeId": null,
            "layout": null,
            "defaultValues": null,
            "icons": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/quickActions/LogACall$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_global_quick_action("LogACall")
            .await
            .expect("describe_global_quick_action should succeed");
        assert_eq!(result.name, "LogACall");
        assert_eq!(result.target_sobject_type.as_deref(), Some("Task"));
    }

    #[tokio::test]
    async fn test_list_quick_actions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([
            {"name": "NewCase", "label": "New Case", "type": "Create"}
        ]);

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/quickActions$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_quick_actions("Account")
            .await
            .expect("list_quick_actions should succeed");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "NewCase");
    }

    #[tokio::test]
    async fn test_describe_quick_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "name": "NewCase",
            "label": "New Case",
            "type": "Create",
            "targetSobjectType": "Case",
            "targetRecordTypeId": "012000000000000AAA",
            "layout": null,
            "defaultValues": null,
            "icons": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/quickActions/NewCase$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_quick_action("Account", "NewCase")
            .await
            .expect("describe_quick_action should succeed");
        assert_eq!(result.name, "NewCase");
        assert_eq!(result.target_sobject_type.as_deref(), Some("Case"));
    }

    #[tokio::test]
    async fn test_invoke_quick_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "id": "500xx000000bZKQAA2",
            "success": true,
            "errors": [],
            "contextId": "001xx000003DgAAAS",
            "feedItemId": null
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/sobjects/Account/quickActions/NewCase$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .invoke_quick_action(
                "Account",
                "NewCase",
                &serde_json::json!({"Subject": "Test"}),
            )
            .await
            .expect("invoke_quick_action should succeed");
        assert!(result.success);
        assert_eq!(result.id.unwrap(), "500xx000000bZKQAA2");
    }
}
