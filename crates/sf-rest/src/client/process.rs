use tracing::instrument;

use busbar_sf_client::security::soql;

use crate::error::{Error, ErrorKind, Result};
use crate::process::{
    ApprovalRequest, ApprovalResult, PendingApprovalCollection, ProcessRuleCollection,
    ProcessRuleRequest, ProcessRuleResult,
};

impl super::SalesforceRestClient {
    /// List all process rules.
    #[instrument(skip(self))]
    pub async fn list_process_rules(&self) -> Result<ProcessRuleCollection> {
        self.client
            .rest_get("process/rules")
            .await
            .map_err(Into::into)
    }

    /// List process rules for a specific SObject type.
    ///
    /// Returns the array of rules for that SObject.
    #[instrument(skip(self))]
    pub async fn list_process_rules_for_sobject(
        &self,
        sobject: &str,
    ) -> Result<Vec<crate::process::ProcessRule>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("process/rules/{}", sobject);
        // Per-SObject endpoint returns {"rules": [...]} wrapper
        let collection: crate::process::ProcessRuleCollection = self.client.rest_get(&path).await?;
        // Extract rules for this SObject, or return all rules flattened
        if let Some(rules) = collection.rules.get(sobject) {
            Ok(rules.clone())
        } else {
            // Flatten all rules from the response
            Ok(collection.rules.into_values().flatten().collect())
        }
    }

    /// Trigger process rules for a record.
    #[instrument(skip(self, request))]
    pub async fn trigger_process_rules(
        &self,
        request: &ProcessRuleRequest,
    ) -> Result<ProcessRuleResult> {
        self.client
            .rest_post("process/rules", request)
            .await
            .map_err(Into::into)
    }

    /// List pending approval work items.
    #[instrument(skip(self))]
    pub async fn list_pending_approvals(&self) -> Result<PendingApprovalCollection> {
        self.client
            .rest_get("process/approvals")
            .await
            .map_err(Into::into)
    }

    /// Submit, approve, or reject an approval request.
    ///
    /// The response is an array; this method returns the first element.
    #[instrument(skip(self, request))]
    pub async fn submit_approval(&self, request: &ApprovalRequest) -> Result<ApprovalResult> {
        let wrapper = serde_json::json!({ "requests": [request] });
        let results: Vec<ApprovalResult> =
            self.client.rest_post("process/approvals", &wrapper).await?;
        results.into_iter().next().ok_or_else(|| {
            Error::new(ErrorKind::Salesforce {
                error_code: "EMPTY_RESPONSE".to_string(),
                message: "No approval result returned".to_string(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_list_process_rules_for_sobject_invalid() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.list_process_rules_for_sobject("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_list_process_rules_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "rules": {
                "Account": [{
                    "id": "01Qxx0000000001",
                    "name": "My Rule",
                    "sobjectType": "Account",
                    "url": "/rules/Account/01Qxx0000000001"
                }]
            }
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/process/rules$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_process_rules()
            .await
            .expect("list_process_rules should succeed");
        assert_eq!(result.rules.get("Account").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_trigger_process_rules_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({"errors": [], "success": true});

        Mock::given(method("POST"))
            .and(path_regex(".*/process/rules$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::process::ProcessRuleRequest {
            context_ids: vec!["001xx000003DgAAAS".to_string()],
        };
        let result = client
            .trigger_process_rules(&request)
            .await
            .expect("trigger_process_rules should succeed");
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_list_pending_approvals_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "approvals": {
                "Account": [{
                    "id": "04axx0000000001",
                    "name": "Account_Approval",
                    "description": null,
                    "object": "Account",
                    "sortOrder": 1
                }]
            }
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/process/approvals$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_pending_approvals()
            .await
            .expect("list_pending_approvals should succeed");
        assert_eq!(result.approvals.get("Account").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_submit_approval_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!([{
            "actorIds": ["005xx000001Svf0AAC"],
            "entityId": "001xx000003DgAAAS",
            "errors": [],
            "instanceId": "04gxx0000000001",
            "instanceStatus": "Pending",
            "newWorkitemIds": ["04ixx0000000002"],
            "success": true
        }]);

        Mock::given(method("POST"))
            .and(path_regex(".*/process/approvals$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::process::ApprovalRequest {
            action_type: crate::process::ApprovalActionType::Submit,
            context_id: "001xx000003DgAAAS".to_string(),
            context_actor_id: None,
            comments: Some("Please approve".to_string()),
            next_approver_ids: None,
            process_definition_name_or_id: None,
            skip_entry_criteria: None,
        };
        let result = client
            .submit_approval(&request)
            .await
            .expect("submit_approval should succeed");
        assert!(result.success);
        assert_eq!(result.instance_status, "Pending");
    }
}
