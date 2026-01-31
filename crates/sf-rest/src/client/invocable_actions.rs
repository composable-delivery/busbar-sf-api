use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::error::{Error, ErrorKind, Result};
use crate::invocable_actions::{
    InvocableActionCollection, InvocableActionDescribe, InvocableActionRequest,
    InvocableActionResult,
};

impl super::SalesforceRestClient {
    /// List all standard invocable actions.
    #[instrument(skip(self))]
    pub async fn list_standard_actions(&self) -> Result<InvocableActionCollection> {
        self.client
            .rest_get("actions/standard")
            .await
            .map_err(Into::into)
    }

    /// List all custom invocable actions.
    #[instrument(skip(self))]
    pub async fn list_custom_actions(&self) -> Result<InvocableActionCollection> {
        self.client
            .rest_get("actions/custom")
            .await
            .map_err(Into::into)
    }

    /// Describe a standard invocable action.
    #[instrument(skip(self))]
    pub async fn describe_standard_action(
        &self,
        action_name: &str,
    ) -> Result<InvocableActionDescribe> {
        if !soql::is_safe_field_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/standard/{}", action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Describe a custom invocable action.
    #[instrument(skip(self))]
    pub async fn describe_custom_action(
        &self,
        action_name: &str,
    ) -> Result<InvocableActionDescribe> {
        if !soql::is_safe_field_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/custom/{}", action_name);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Invoke a standard action.
    ///
    /// Returns a vector of results (one per input).
    #[instrument(skip(self, request))]
    pub async fn invoke_standard_action(
        &self,
        action_name: &str,
        request: &InvocableActionRequest,
    ) -> Result<Vec<InvocableActionResult>> {
        if !soql::is_safe_field_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/standard/{}", action_name);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }

    /// Invoke a custom action.
    ///
    /// Returns a vector of results (one per input).
    #[instrument(skip(self, request))]
    pub async fn invoke_custom_action(
        &self,
        action_name: &str,
        request: &InvocableActionRequest,
    ) -> Result<Vec<InvocableActionResult>> {
        if !soql::is_safe_field_name(action_name) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ACTION".to_string(),
                message: "Invalid action name".to_string(),
            }));
        }
        let path = format!("actions/custom/{}", action_name);
        self.client
            .rest_post(&path, request)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_describe_standard_action_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.describe_standard_action("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_describe_custom_action_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.describe_custom_action("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_invoke_standard_action_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let request = crate::invocable_actions::InvocableActionRequest {
            inputs: vec![serde_json::json!({"text": "hello"})],
        };
        let result = client
            .invoke_standard_action("Bad'; DROP--", &request)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_invoke_custom_action_invalid_name() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let request = crate::invocable_actions::InvocableActionRequest {
            inputs: vec![serde_json::json!({"text": "hello"})],
        };
        let result = client.invoke_custom_action("Bad'; DROP--", &request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ACTION"));
    }

    #[tokio::test]
    async fn test_list_standard_actions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "actions": [{
                "name": "chatterPost",
                "label": "Post to Chatter",
                "type": "APEX"
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/actions/standard$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_standard_actions()
            .await
            .expect("list_standard_actions should succeed");
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].name, "chatterPost");
    }

    #[tokio::test]
    async fn test_list_custom_actions_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "actions": [{
                "name": "myCustomAction",
                "label": "My Custom Action",
                "type": "APEX"
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/actions/custom$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_custom_actions()
            .await
            .expect("list_custom_actions should succeed");
        assert_eq!(result.actions.len(), 1);
    }

    #[tokio::test]
    async fn test_describe_standard_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "name": "chatterPost",
            "label": "Post to Chatter",
            "type": "APEX",
            "inputs": [{
                "name": "text",
                "label": "Post Text",
                "type": "STRING",
                "required": true,
                "description": "The text to post"
            }],
            "outputs": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/actions/standard/chatterPost$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_standard_action("chatterPost")
            .await
            .expect("describe_standard_action should succeed");
        assert_eq!(result.name, "chatterPost");
        assert_eq!(result.inputs.len(), 1);
    }

    #[tokio::test]
    async fn test_invoke_standard_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([{
            "actionName": "chatterPost",
            "errors": [],
            "isSuccess": true,
            "outputValues": {"feedItemId": "0D5xx0000000001"}
        }]);

        Mock::given(method("POST"))
            .and(path_regex(".*/actions/standard/chatterPost$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::invocable_actions::InvocableActionRequest {
            inputs: vec![serde_json::json!({"text": "Hello World"})],
        };
        let result = client
            .invoke_standard_action("chatterPost", &request)
            .await
            .expect("invoke_standard_action should succeed");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_success);
    }

    #[tokio::test]
    async fn test_invoke_custom_action_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([{
            "actionName": "myAction",
            "errors": [],
            "isSuccess": true,
            "outputValues": null
        }]);

        Mock::given(method("POST"))
            .and(path_regex(".*/actions/custom/myAction$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::invocable_actions::InvocableActionRequest {
            inputs: vec![serde_json::json!({"param": "value"})],
        };
        let result = client
            .invoke_custom_action("myAction", &request)
            .await
            .expect("invoke_custom_action should succeed");
        assert_eq!(result.len(), 1);
        assert!(result[0].is_success);
    }
}
