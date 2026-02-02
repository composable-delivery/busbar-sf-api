//! Process & Approvals API host function handlers.
use super::error::*;
use busbar_sf_rest::SalesforceRestClient;
use busbar_sf_rest::{
    ApprovalActionType, ApprovalRequest as RestApprovalRequest,
    ProcessRuleRequest as RestProcessRuleRequest,
};
use busbar_sf_wasm_types::*;

/// List all process rules.
pub(crate) async fn handle_list_process_rules(
    client: &SalesforceRestClient,
) -> BridgeResult<ProcessRuleCollection> {
    match client.list_process_rules().await {
        Ok(result) => BridgeResult::ok(ProcessRuleCollection {
            rules: result
                .rules
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.into_iter()
                            .map(|r| ProcessRule {
                                id: r.id,
                                name: r.name,
                                sobject_type: r.sobject_type,
                                url: r.url,
                            })
                            .collect(),
                    )
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List process rules for a specific SObject.
pub(crate) async fn handle_list_process_rules_for_sobject(
    client: &SalesforceRestClient,
    request: ListProcessRulesForSObjectRequest,
) -> BridgeResult<Vec<ProcessRule>> {
    match client
        .list_process_rules_for_sobject(&request.sobject)
        .await
    {
        Ok(rules) => BridgeResult::ok(
            rules
                .into_iter()
                .map(|r| ProcessRule {
                    id: r.id,
                    name: r.name,
                    sobject_type: r.sobject_type,
                    url: r.url,
                })
                .collect(),
        ),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Trigger process rules.
pub(crate) async fn handle_trigger_process_rules(
    client: &SalesforceRestClient,
    request: ProcessRuleRequest,
) -> BridgeResult<ProcessRuleResult> {
    let rest_request = RestProcessRuleRequest {
        context_ids: request.context_ids,
    };
    match client.trigger_process_rules(&rest_request).await {
        Ok(result) => BridgeResult::ok(ProcessRuleResult {
            errors: result
                .errors
                .into_iter()
                .map(|e| SalesforceApiError {
                    status_code: e.status_code,
                    message: e.message,
                    fields: e.fields,
                })
                .collect(),
            success: result.success,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// List pending approvals.
pub(crate) async fn handle_list_pending_approvals(
    client: &SalesforceRestClient,
) -> BridgeResult<PendingApprovalCollection> {
    match client.list_pending_approvals().await {
        Ok(result) => BridgeResult::ok(PendingApprovalCollection {
            approvals: result
                .approvals
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.into_iter()
                            .map(|a| PendingApproval {
                                id: a.id,
                                name: a.name,
                                description: a.description,
                                object: a.object,
                                sort_order: a.sort_order,
                            })
                            .collect(),
                    )
                })
                .collect(),
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}

/// Submit an approval.
pub(crate) async fn handle_submit_approval(
    client: &SalesforceRestClient,
    request: ApprovalRequest,
) -> BridgeResult<ApprovalResult> {
    let action_type = match request.action_type.as_str() {
        "Submit" => ApprovalActionType::Submit,
        "Approve" => ApprovalActionType::Approve,
        "Reject" => ApprovalActionType::Reject,
        _ => return BridgeResult::err("INVALID_ACTION_TYPE", "Invalid approval action type"),
    };

    let rest_request = RestApprovalRequest {
        action_type,
        context_id: request.context_id,
        context_actor_id: request.context_actor_id,
        comments: request.comments,
        next_approver_ids: request.next_approver_ids,
        process_definition_name_or_id: request.process_definition_name_or_id,
        skip_entry_criteria: request.skip_entry_criteria,
    };

    match client.submit_approval(&rest_request).await {
        Ok(result) => BridgeResult::ok(ApprovalResult {
            actor_ids: result.actor_ids,
            entity_id: result.entity_id,
            errors: result
                .errors
                .into_iter()
                .map(|e| SalesforceApiError {
                    status_code: e.status_code,
                    message: e.message,
                    fields: e.fields,
                })
                .collect(),
            instance_id: result.instance_id,
            instance_status: result.instance_status,
            new_workitem_ids: result.new_workitem_ids,
            success: result.success,
        }),
        Err(e) => {
            let (code, message) = sanitize_rest_error(&e);
            BridgeResult::err(code, message)
        }
    }
}
