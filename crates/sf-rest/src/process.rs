//! Process Rule and Approval types for the Salesforce REST API.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Deserialize `null` as the default value for the type (e.g., empty Vec).
/// Salesforce APIs often return `null` instead of `[]` for empty arrays.
fn null_as_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

/// A process rule definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Collection of process rules grouped by SObject type.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessRuleCollection {
    #[serde(default)]
    pub rules: HashMap<String, Vec<ProcessRule>>,
}

/// Request to trigger process rules for one or more records.
/// All IDs must be for records on the same SObject type.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessRuleRequest {
    #[serde(rename = "contextIds")]
    pub context_ids: Vec<String>,
}

/// Result of triggering a process rule.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessRuleResult {
    #[serde(default, deserialize_with = "null_as_default")]
    pub errors: Vec<crate::sobject::SalesforceError>,
    pub success: bool,
}

/// An approval process definition returned by GET /process/approvals.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PendingApproval {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(rename = "sortOrder", default)]
    pub sort_order: Option<i32>,
}

/// Collection of pending approvals grouped by entity type.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PendingApprovalCollection {
    #[serde(default, deserialize_with = "null_as_default")]
    pub approvals: HashMap<String, Vec<PendingApproval>>,
}

/// Request to submit, approve, or reject an approval.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApprovalRequest {
    #[serde(rename = "actionType")]
    pub action_type: ApprovalActionType,
    #[serde(rename = "contextId")]
    pub context_id: String,
    #[serde(rename = "contextActorId", skip_serializing_if = "Option::is_none")]
    pub context_actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    #[serde(rename = "nextApproverIds", skip_serializing_if = "Option::is_none")]
    pub next_approver_ids: Option<Vec<String>>,
    #[serde(
        rename = "processDefinitionNameOrId",
        skip_serializing_if = "Option::is_none"
    )]
    pub process_definition_name_or_id: Option<String>,
    #[serde(rename = "skipEntryCriteria", skip_serializing_if = "Option::is_none")]
    pub skip_entry_criteria: Option<bool>,
}

/// The type of approval action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ApprovalActionType {
    Submit,
    Approve,
    Reject,
}

/// Result of an approval action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApprovalResult {
    #[serde(rename = "actorIds", default, deserialize_with = "null_as_default")]
    pub actor_ids: Vec<String>,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    #[serde(rename = "instanceStatus")]
    pub instance_status: String,
    #[serde(
        rename = "newWorkitemIds",
        default,
        deserialize_with = "null_as_default"
    )]
    pub new_workitem_ids: Vec<String>,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_process_rule_deserialize() {
        let json = json!({
            "id": "01Qxx0000000001",
            "name": "My Rule",
            "sobjectType": "Account",
            "url": "/services/data/v62.0/process/rules/Account/01Qxx0000000001"
        });
        let rule: ProcessRule = serde_json::from_value(json).unwrap();
        assert_eq!(rule.id, "01Qxx0000000001");
        assert_eq!(rule.sobject_type, Some("Account".to_string()));
        assert!(rule.url.is_some());
    }

    #[test]
    fn test_process_rule_deserialize_without_url() {
        let json = json!({
            "id": "01Qxx0000000001",
            "name": "My Rule",
            "sobjectType": "Account"
        });
        let rule: ProcessRule = serde_json::from_value(json).unwrap();
        assert_eq!(rule.id, "01Qxx0000000001");
        assert!(rule.url.is_none());
    }

    #[test]
    fn test_process_rule_collection_deserialize() {
        let json = json!({
            "rules": {
                "Account": [{
                    "id": "01Qxx0000000001",
                    "name": "My Rule",
                    "sobjectType": "Account",
                    "url": "/rules/Account/01Qxx0000000001"
                }]
            }
        });
        let collection: ProcessRuleCollection = serde_json::from_value(json).unwrap();
        assert_eq!(collection.rules.get("Account").unwrap().len(), 1);
    }

    #[test]
    fn test_process_rule_request_serialize() {
        let request = ProcessRuleRequest {
            context_ids: vec!["001xx000003DgAAAS".to_string()],
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["contextIds"][0], "001xx000003DgAAAS");
    }

    #[test]
    fn test_process_rule_result_deserialize() {
        let json = json!({"errors": [], "success": true});
        let result: ProcessRuleResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_pending_approval_deserialize() {
        let json = json!({
            "id": "04axx0000000001",
            "name": "Account_Approval",
            "description": "Approval for accounts",
            "object": "Account",
            "sortOrder": 1
        });
        let approval: PendingApproval = serde_json::from_value(json).unwrap();
        assert_eq!(approval.object, Some("Account".to_string()));
        assert_eq!(approval.name, Some("Account_Approval".to_string()));
    }

    #[test]
    fn test_pending_approval_collection_deserialize() {
        let json = json!({
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
        let collection: PendingApprovalCollection = serde_json::from_value(json).unwrap();
        assert_eq!(collection.approvals.get("Account").unwrap().len(), 1);
    }

    #[test]
    fn test_approval_request_serialize() {
        let request = ApprovalRequest {
            action_type: ApprovalActionType::Submit,
            context_id: "001xx000003DgAAAS".to_string(),
            context_actor_id: None,
            comments: Some("Approved".to_string()),
            next_approver_ids: None,
            process_definition_name_or_id: None,
            skip_entry_criteria: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["actionType"], "Submit");
        assert_eq!(json["contextId"], "001xx000003DgAAAS");
        assert!(json.get("contextActorId").is_none());
        assert_eq!(json["comments"], "Approved");
    }

    #[test]
    fn test_approval_result_deserialize() {
        let json = json!({
            "actorIds": ["005xx000001Svf0AAC"],
            "entityId": "001xx000003DgAAAS",
            "errors": [],
            "instanceId": "04gxx0000000001",
            "instanceStatus": "Approved",
            "newWorkitemIds": [],
            "success": true
        });
        let result: ApprovalResult = serde_json::from_value(json).unwrap();
        assert!(result.success);
        assert_eq!(result.instance_status, "Approved");
        assert_eq!(result.actor_ids.len(), 1);
    }
}
