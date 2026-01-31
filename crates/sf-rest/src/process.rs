//! Process Rules and Approvals types and operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A process rule.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ProcessRule {
    pub id: String,
    pub name: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: Option<String>,
    pub url: String,
}

/// Collection of process rules.
///
/// The Salesforce API returns rules as a map keyed by SObject name:
/// `{"rules": {"Account": [...], "Contact": [...]}}`
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ProcessRuleCollection {
    pub rules: HashMap<String, Vec<ProcessRule>>,
}

/// Request to trigger process rules.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProcessRuleRequest {
    #[serde(rename = "contextId")]
    pub context_id: String,
}

/// Result of triggering process rules.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ProcessRuleResult {
    pub errors: Vec<crate::sobject::SalesforceError>,
    pub success: bool,
}

/// A pending approval request.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct PendingApproval {
    pub id: String,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    #[serde(rename = "processInstanceId")]
    pub process_instance_id: String,
}

/// Collection of pending approvals.
///
/// The Salesforce API returns approvals as a map keyed by entity type:
/// `{"approvals": {"Account": [...]}}`
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct PendingApprovalCollection {
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

/// Type of approval action.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ApprovalActionType {
    Submit,
    Approve,
    Reject,
}

/// Result of an approval request.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ApprovalResult {
    #[serde(rename = "actorIds")]
    pub actor_ids: Vec<String>,
    #[serde(rename = "entityId")]
    pub entity_id: String,
    pub errors: Vec<crate::sobject::SalesforceError>,
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    #[serde(rename = "instanceStatus")]
    pub instance_status: String,
    #[serde(rename = "newWorkitemIds")]
    pub new_workitem_ids: Vec<String>,
    pub success: bool,
}
