//! Busbar capability implementation for sf-bridge.
//!
//! This module implements the `HostCapability` trait from `busbar-capability`,
//! allowing `SfBridge` to be used as a drop-in capability provider for the
//! Busbar runtime.

use crate::{registration, BridgeState, SfBridge};
use busbar_capability::{
    CapabilityError, CapabilityManifest, ConfigKeyDef, HostCapability, OperationDef,
    RiskClassification,
};
use busbar_sf_wasm_types::host_fn_names;
use extism::{PluginBuilder, UserData};

impl HostCapability for SfBridge {
    fn namespace(&self) -> &str {
        "salesforce"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn manifest(&self) -> CapabilityManifest {
        CapabilityManifest {
            namespace: "salesforce".into(),
            display_name: "Salesforce".into(),
            version: self.version().into(),
            operations: create_operation_definitions(),
            required_config: vec![ConfigKeyDef {
                key: "sf_auth_url".into(),
                description: "Salesforce authentication URL (instance_url + access_token)".into(),
                required: true,
                env_var: Some("SF_AUTH_URL".into()),
            }],
        }
    }

    fn register_host_functions<'a>(
        &'a self,
        builder: PluginBuilder<'a>,
    ) -> Result<PluginBuilder<'a>, CapabilityError> {
        // Create the bridge state that will be shared with host functions
        let state = BridgeState {
            #[cfg(feature = "rest")]
            rest_client: self.rest_client.clone(),
            #[cfg(feature = "bulk")]
            bulk_client: self.bulk_client.clone(),
            #[cfg(feature = "tooling")]
            tooling_client: self.tooling_client.clone(),
            instance_url: self.instance_url.clone(),
            access_token: self.access_token.clone(),
            handle: self.handle.clone(),
        };

        let user_data = UserData::new(state);

        // Use the existing registration logic to wire up all host functions
        let builder = registration::register_all(builder, &user_data);

        Ok(builder)
    }
}

/// Create the complete list of operation definitions for all 98 Salesforce host functions.
///
/// Operations are classified by risk:
/// - **ReadOnly**: query, describe, list operations
/// - **WriteVisible**: create, update, upsert operations
/// - **Destructive**: delete, deploy operations
fn create_operation_definitions() -> Vec<OperationDef> {
    vec![
        // REST API: Core CRUD & Query
        op(
            "query",
            host_fn_names::QUERY,
            "Execute SOQL query",
            RiskClassification::ReadOnly,
        ),
        op(
            "query_more",
            host_fn_names::QUERY_MORE,
            "Fetch next page of query results",
            RiskClassification::ReadOnly,
        ),
        op(
            "create",
            host_fn_names::CREATE,
            "Create a new record",
            RiskClassification::WriteVisible,
        ),
        op(
            "get",
            host_fn_names::GET,
            "Get a record by ID",
            RiskClassification::ReadOnly,
        ),
        op(
            "update",
            host_fn_names::UPDATE,
            "Update a record",
            RiskClassification::WriteVisible,
        ),
        op(
            "delete",
            host_fn_names::DELETE,
            "Delete a record",
            RiskClassification::Destructive,
        ),
        op(
            "upsert",
            host_fn_names::UPSERT,
            "Upsert a record by external ID",
            RiskClassification::WriteVisible,
        ),
        op(
            "describe_global",
            host_fn_names::DESCRIBE_GLOBAL,
            "Describe all global SObjects",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_sobject",
            host_fn_names::DESCRIBE_SOBJECT,
            "Describe a specific SObject",
            RiskClassification::ReadOnly,
        ),
        op(
            "search",
            host_fn_names::SEARCH,
            "Execute SOSL search",
            RiskClassification::ReadOnly,
        ),
        op(
            "limits",
            host_fn_names::LIMITS,
            "Get org limits",
            RiskClassification::ReadOnly,
        ),
        op(
            "versions",
            host_fn_names::VERSIONS,
            "List available API versions",
            RiskClassification::ReadOnly,
        ),
        op(
            "get_deleted",
            host_fn_names::GET_DELETED,
            "Get deleted records in date range",
            RiskClassification::ReadOnly,
        ),
        op(
            "get_updated",
            host_fn_names::GET_UPDATED,
            "Get updated records in date range",
            RiskClassification::ReadOnly,
        ),
        // REST API: Composite
        op(
            "composite",
            host_fn_names::COMPOSITE,
            "Execute composite subrequests",
            RiskClassification::WriteVisible,
        ),
        op(
            "composite_batch",
            host_fn_names::COMPOSITE_BATCH,
            "Execute batch subrequests",
            RiskClassification::WriteVisible,
        ),
        op(
            "composite_tree",
            host_fn_names::COMPOSITE_TREE,
            "Create record trees",
            RiskClassification::WriteVisible,
        ),
        op(
            "composite_graph",
            host_fn_names::COMPOSITE_GRAPH,
            "Execute composite graph",
            RiskClassification::WriteVisible,
        ),
        // REST API: Collections
        op(
            "create_multiple",
            host_fn_names::CREATE_MULTIPLE,
            "Create multiple records",
            RiskClassification::WriteVisible,
        ),
        op(
            "update_multiple",
            host_fn_names::UPDATE_MULTIPLE,
            "Update multiple records",
            RiskClassification::WriteVisible,
        ),
        op(
            "get_multiple",
            host_fn_names::GET_MULTIPLE,
            "Get multiple records by IDs",
            RiskClassification::ReadOnly,
        ),
        op(
            "delete_multiple",
            host_fn_names::DELETE_MULTIPLE,
            "Delete multiple records",
            RiskClassification::Destructive,
        ),
        // REST API: Process & Approvals
        op(
            "list_process_rules",
            host_fn_names::LIST_PROCESS_RULES,
            "List all process rules",
            RiskClassification::ReadOnly,
        ),
        op(
            "list_process_rules_for_sobject",
            host_fn_names::LIST_PROCESS_RULES_FOR_SOBJECT,
            "List process rules for SObject",
            RiskClassification::ReadOnly,
        ),
        op(
            "trigger_process_rules",
            host_fn_names::TRIGGER_PROCESS_RULES,
            "Trigger process rules",
            RiskClassification::WriteVisible,
        ),
        op(
            "list_pending_approvals",
            host_fn_names::LIST_PENDING_APPROVALS,
            "List pending approvals",
            RiskClassification::ReadOnly,
        ),
        op(
            "submit_approval",
            host_fn_names::SUBMIT_APPROVAL,
            "Submit approval request",
            RiskClassification::WriteVisible,
        ),
        // REST API: List Views
        op(
            "list_views",
            host_fn_names::LIST_VIEWS,
            "List views for SObject",
            RiskClassification::ReadOnly,
        ),
        op(
            "get_list_view",
            host_fn_names::GET_LIST_VIEW,
            "Get list view metadata",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_list_view",
            host_fn_names::DESCRIBE_LIST_VIEW,
            "Describe list view",
            RiskClassification::ReadOnly,
        ),
        op(
            "execute_list_view",
            host_fn_names::EXECUTE_LIST_VIEW,
            "Execute list view query",
            RiskClassification::ReadOnly,
        ),
        // REST API: Quick Actions
        op(
            "list_global_quick_actions",
            host_fn_names::LIST_GLOBAL_QUICK_ACTIONS,
            "List global quick actions",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_global_quick_action",
            host_fn_names::DESCRIBE_GLOBAL_QUICK_ACTION,
            "Describe global quick action",
            RiskClassification::ReadOnly,
        ),
        op(
            "list_quick_actions",
            host_fn_names::LIST_QUICK_ACTIONS,
            "List quick actions for SObject",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_quick_action",
            host_fn_names::DESCRIBE_QUICK_ACTION,
            "Describe quick action",
            RiskClassification::ReadOnly,
        ),
        op(
            "invoke_quick_action",
            host_fn_names::INVOKE_QUICK_ACTION,
            "Invoke quick action",
            RiskClassification::WriteVisible,
        ),
        // REST API: Invocable Actions
        op(
            "list_standard_actions",
            host_fn_names::LIST_STANDARD_ACTIONS,
            "List standard invocable actions",
            RiskClassification::ReadOnly,
        ),
        op(
            "list_custom_action_types",
            host_fn_names::LIST_CUSTOM_ACTION_TYPES,
            "List custom action types",
            RiskClassification::ReadOnly,
        ),
        op(
            "list_custom_actions",
            host_fn_names::LIST_CUSTOM_ACTIONS,
            "List custom actions",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_standard_action",
            host_fn_names::DESCRIBE_STANDARD_ACTION,
            "Describe standard action",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_custom_action",
            host_fn_names::DESCRIBE_CUSTOM_ACTION,
            "Describe custom action",
            RiskClassification::ReadOnly,
        ),
        op(
            "invoke_standard_action",
            host_fn_names::INVOKE_STANDARD_ACTION,
            "Invoke standard action",
            RiskClassification::WriteVisible,
        ),
        op(
            "invoke_custom_action",
            host_fn_names::INVOKE_CUSTOM_ACTION,
            "Invoke custom action",
            RiskClassification::WriteVisible,
        ),
        // REST API: Layouts
        op(
            "describe_layouts",
            host_fn_names::DESCRIBE_LAYOUTS,
            "Describe layouts for SObject",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_named_layout",
            host_fn_names::DESCRIBE_NAMED_LAYOUT,
            "Describe specific layout",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_approval_layouts",
            host_fn_names::DESCRIBE_APPROVAL_LAYOUTS,
            "Describe approval layouts",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_compact_layouts",
            host_fn_names::DESCRIBE_COMPACT_LAYOUTS,
            "Describe compact layouts",
            RiskClassification::ReadOnly,
        ),
        op(
            "describe_global_publisher_layouts",
            host_fn_names::DESCRIBE_GLOBAL_PUBLISHER_LAYOUTS,
            "Describe global publisher layouts",
            RiskClassification::ReadOnly,
        ),
        op(
            "compact_layouts_multi",
            host_fn_names::COMPACT_LAYOUTS_MULTI,
            "Get compact layouts for multiple SObjects",
            RiskClassification::ReadOnly,
        ),
        // REST API: Knowledge
        op(
            "knowledge_settings",
            host_fn_names::KNOWLEDGE_SETTINGS,
            "Get knowledge settings",
            RiskClassification::ReadOnly,
        ),
        op(
            "knowledge_articles",
            host_fn_names::KNOWLEDGE_ARTICLES,
            "Query knowledge articles",
            RiskClassification::ReadOnly,
        ),
        op(
            "data_category_groups",
            host_fn_names::DATA_CATEGORY_GROUPS,
            "List data category groups",
            RiskClassification::ReadOnly,
        ),
        op(
            "data_categories",
            host_fn_names::DATA_CATEGORIES,
            "List data categories",
            RiskClassification::ReadOnly,
        ),
        // REST API: Standalone
        op(
            "tabs",
            host_fn_names::TABS,
            "List available tabs",
            RiskClassification::ReadOnly,
        ),
        op(
            "theme",
            host_fn_names::THEME,
            "Get theme info",
            RiskClassification::ReadOnly,
        ),
        op(
            "app_menu",
            host_fn_names::APP_MENU,
            "Get app menu",
            RiskClassification::ReadOnly,
        ),
        op(
            "recent_items",
            host_fn_names::RECENT_ITEMS,
            "Get recent items",
            RiskClassification::ReadOnly,
        ),
        op(
            "relevant_items",
            host_fn_names::RELEVANT_ITEMS,
            "Get relevant items",
            RiskClassification::ReadOnly,
        ),
        op(
            "platform_event_schema",
            host_fn_names::PLATFORM_EVENT_SCHEMA,
            "Get platform event schema",
            RiskClassification::ReadOnly,
        ),
        op(
            "lightning_toggle_metrics",
            host_fn_names::LIGHTNING_TOGGLE_METRICS,
            "Get Lightning toggle metrics",
            RiskClassification::ReadOnly,
        ),
        op(
            "lightning_usage",
            host_fn_names::LIGHTNING_USAGE,
            "Get Lightning usage",
            RiskClassification::ReadOnly,
        ),
        // REST API: User Password
        op(
            "get_user_password_status",
            host_fn_names::GET_USER_PASSWORD_STATUS,
            "Get user password status",
            RiskClassification::ReadOnly,
        ),
        op(
            "set_user_password",
            host_fn_names::SET_USER_PASSWORD,
            "Set user password",
            RiskClassification::WriteVisible,
        ),
        op(
            "reset_user_password",
            host_fn_names::RESET_USER_PASSWORD,
            "Reset user password",
            RiskClassification::WriteVisible,
        ),
        // REST API: Scheduler
        op(
            "appointment_candidates",
            host_fn_names::APPOINTMENT_CANDIDATES,
            "Get appointment candidates",
            RiskClassification::ReadOnly,
        ),
        op(
            "appointment_slots",
            host_fn_names::APPOINTMENT_SLOTS,
            "Get appointment slots",
            RiskClassification::ReadOnly,
        ),
        // REST API: Consent
        op(
            "read_consent",
            host_fn_names::READ_CONSENT,
            "Read consent status",
            RiskClassification::ReadOnly,
        ),
        op(
            "write_consent",
            host_fn_names::WRITE_CONSENT,
            "Write consent status",
            RiskClassification::WriteVisible,
        ),
        op(
            "read_multi_consent",
            host_fn_names::READ_MULTI_CONSENT,
            "Read multiple consent statuses",
            RiskClassification::ReadOnly,
        ),
        // REST API: Binary
        op(
            "get_blob",
            host_fn_names::GET_BLOB,
            "Get blob field data",
            RiskClassification::ReadOnly,
        ),
        op(
            "get_rich_text_image",
            host_fn_names::GET_RICH_TEXT_IMAGE,
            "Get rich text image",
            RiskClassification::ReadOnly,
        ),
        op(
            "get_relationship",
            host_fn_names::GET_RELATIONSHIP,
            "Get relationship data",
            RiskClassification::ReadOnly,
        ),
        // REST API: Embedded Service
        op(
            "get_embedded_service_config",
            host_fn_names::GET_EMBEDDED_SERVICE_CONFIG,
            "Get embedded service config",
            RiskClassification::ReadOnly,
        ),
        // REST API: Search Enhancements
        op(
            "parameterized_search",
            host_fn_names::PARAMETERIZED_SEARCH,
            "Execute parameterized search",
            RiskClassification::ReadOnly,
        ),
        op(
            "search_suggestions",
            host_fn_names::SEARCH_SUGGESTIONS,
            "Get search suggestions",
            RiskClassification::ReadOnly,
        ),
        op(
            "search_scope_order",
            host_fn_names::SEARCH_SCOPE_ORDER,
            "Get search scope order",
            RiskClassification::ReadOnly,
        ),
        op(
            "search_result_layouts",
            host_fn_names::SEARCH_RESULT_LAYOUTS,
            "Get search result layouts",
            RiskClassification::ReadOnly,
        ),
        // Bulk API 2.0
        op(
            "bulk_create_ingest_job",
            host_fn_names::BULK_CREATE_INGEST_JOB,
            "Create bulk ingest job",
            RiskClassification::WriteVisible,
        ),
        op(
            "bulk_upload_job_data",
            host_fn_names::BULK_UPLOAD_JOB_DATA,
            "Upload data to bulk job",
            RiskClassification::WriteVisible,
        ),
        op(
            "bulk_close_ingest_job",
            host_fn_names::BULK_CLOSE_INGEST_JOB,
            "Close bulk ingest job",
            RiskClassification::WriteVisible,
        ),
        op(
            "bulk_abort_ingest_job",
            host_fn_names::BULK_ABORT_INGEST_JOB,
            "Abort bulk ingest job",
            RiskClassification::WriteVisible,
        ),
        op(
            "bulk_get_ingest_job",
            host_fn_names::BULK_GET_INGEST_JOB,
            "Get bulk ingest job status",
            RiskClassification::ReadOnly,
        ),
        op(
            "bulk_get_job_results",
            host_fn_names::BULK_GET_JOB_RESULTS,
            "Get bulk job results",
            RiskClassification::ReadOnly,
        ),
        op(
            "bulk_delete_ingest_job",
            host_fn_names::BULK_DELETE_INGEST_JOB,
            "Delete bulk ingest job",
            RiskClassification::Destructive,
        ),
        op(
            "bulk_get_all_ingest_jobs",
            host_fn_names::BULK_GET_ALL_INGEST_JOBS,
            "List all bulk ingest jobs",
            RiskClassification::ReadOnly,
        ),
        op(
            "bulk_abort_query_job",
            host_fn_names::BULK_ABORT_QUERY_JOB,
            "Abort bulk query job",
            RiskClassification::WriteVisible,
        ),
        op(
            "bulk_get_query_results",
            host_fn_names::BULK_GET_QUERY_RESULTS,
            "Get bulk query results",
            RiskClassification::ReadOnly,
        ),
        // Tooling API
        op(
            "tooling_query",
            host_fn_names::TOOLING_QUERY,
            "Execute tooling query",
            RiskClassification::ReadOnly,
        ),
        op(
            "tooling_execute_anonymous",
            host_fn_names::TOOLING_EXECUTE_ANONYMOUS,
            "Execute anonymous Apex",
            RiskClassification::WriteVisible,
        ),
        op(
            "tooling_get",
            host_fn_names::TOOLING_GET,
            "Get tooling record",
            RiskClassification::ReadOnly,
        ),
        op(
            "tooling_create",
            host_fn_names::TOOLING_CREATE,
            "Create tooling record",
            RiskClassification::WriteVisible,
        ),
        op(
            "tooling_delete",
            host_fn_names::TOOLING_DELETE,
            "Delete tooling record",
            RiskClassification::Destructive,
        ),
        // Metadata API
        op(
            "metadata_deploy",
            host_fn_names::METADATA_DEPLOY,
            "Deploy metadata package",
            RiskClassification::Destructive,
        ),
        op(
            "metadata_check_deploy_status",
            host_fn_names::METADATA_CHECK_DEPLOY_STATUS,
            "Check deploy status",
            RiskClassification::ReadOnly,
        ),
        op(
            "metadata_retrieve",
            host_fn_names::METADATA_RETRIEVE,
            "Retrieve metadata package",
            RiskClassification::ReadOnly,
        ),
        op(
            "metadata_check_retrieve_status",
            host_fn_names::METADATA_CHECK_RETRIEVE_STATUS,
            "Check retrieve status",
            RiskClassification::ReadOnly,
        ),
        op(
            "metadata_list",
            host_fn_names::METADATA_LIST,
            "List metadata types",
            RiskClassification::ReadOnly,
        ),
        op(
            "metadata_describe",
            host_fn_names::METADATA_DESCRIBE,
            "Describe metadata",
            RiskClassification::ReadOnly,
        ),
    ]
}

/// Helper to create an `OperationDef` with consistent tagging.
fn op(name: &str, host_fn_name: &str, description: &str, risk: RiskClassification) -> OperationDef {
    OperationDef {
        name: name.into(),
        host_fn_name: host_fn_name.into(),
        description: description.into(),
        risk,
        requires_auth: true,
        tags: vec!["salesforce".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_count() {
        let ops = create_operation_definitions();
        assert_eq!(ops.len(), 98, "Expected 98 operations");
    }

    #[test]
    fn test_manifest_structure() {
        // Create a mock SfBridge for testing (using mock data)
        // Since we can't easily construct a real SfBridge without auth,
        // we'll just test the standalone functions
        let ops = create_operation_definitions();
        assert_eq!(ops.len(), 98);

        // Verify all operations have non-empty names
        for op in &ops {
            assert!(!op.name.is_empty(), "Operation name should not be empty");
            assert!(
                !op.host_fn_name.is_empty(),
                "Host function name should not be empty"
            );
            assert!(
                !op.description.is_empty(),
                "Operation description should not be empty"
            );
            assert!(op.requires_auth, "All operations should require auth");
        }
    }

    #[test]
    fn test_risk_classifications() {
        let ops = create_operation_definitions();

        // Count operations by risk level
        let read_only_count = ops
            .iter()
            .filter(|op| op.risk == RiskClassification::ReadOnly)
            .count();
        let write_visible_count = ops
            .iter()
            .filter(|op| op.risk == RiskClassification::WriteVisible)
            .count();
        let destructive_count = ops
            .iter()
            .filter(|op| op.risk == RiskClassification::Destructive)
            .count();

        // We should have operations in all three categories
        assert!(
            read_only_count > 0,
            "Should have read-only operations (got {read_only_count})"
        );
        assert!(
            write_visible_count > 0,
            "Should have write-visible operations (got {write_visible_count})"
        );
        assert!(
            destructive_count > 0,
            "Should have destructive operations (got {destructive_count})"
        );

        // Total should equal 98
        assert_eq!(
            read_only_count + write_visible_count + destructive_count,
            98
        );

        // Verify specific high-risk operations
        let delete_op = ops.iter().find(|op| op.name == "delete").unwrap();
        assert_eq!(delete_op.risk, RiskClassification::Destructive);

        let query_op = ops.iter().find(|op| op.name == "query").unwrap();
        assert_eq!(query_op.risk, RiskClassification::ReadOnly);

        let create_op = ops.iter().find(|op| op.name == "create").unwrap();
        assert_eq!(create_op.risk, RiskClassification::WriteVisible);
    }

    #[test]
    fn test_unique_operation_names() {
        let ops = create_operation_definitions();
        let mut names = std::collections::HashSet::new();

        for op in &ops {
            assert!(
                names.insert(&op.name),
                "Duplicate operation name: {}",
                op.name
            );
        }
    }

    #[test]
    fn test_unique_host_function_names() {
        let ops = create_operation_definitions();
        let mut names = std::collections::HashSet::new();

        for op in &ops {
            assert!(
                names.insert(&op.host_fn_name),
                "Duplicate host function name: {}",
                op.host_fn_name
            );
        }
    }
}
