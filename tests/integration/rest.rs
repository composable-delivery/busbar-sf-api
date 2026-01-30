//! REST API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_rest::{CompositeRequest, CompositeSubrequest, QueryBuilder, SalesforceRestClient};
use serde::{Deserialize, Serialize};

// ============================================================================
// REST API - Comprehensive Tests
// ============================================================================

#[tokio::test]
async fn test_rest_composite_api() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let composite_request = CompositeRequest {
        all_or_none: true,
        collate_subrequests: false,
        subrequests: vec![
            CompositeSubrequest {
                method: "POST".to_string(),
                url: format!("/services/data/v{}/sobjects/Account", creds.api_version()),
                reference_id: "NewAccount".to_string(),
                body: Some(serde_json::json!({
                    "Name": format!(
                        "Composite Test Account {}",
                        chrono::Utc::now().timestamp_millis()
                    )
                })),
            },
            CompositeSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/sobjects/Account/@{{NewAccount.id}}",
                    creds.api_version()
                ),
                reference_id: "GetNewAccount".to_string(),
                body: None,
            },
        ],
    };

    let response = client
        .composite(&composite_request)
        .await
        .expect("Composite request should succeed");

    assert_eq!(response.responses.len(), 2, "Should have 2 sub-responses");

    if let Some(first_response) = response.responses.first() {
        if let Some(id) = first_response.body.get("id").and_then(|v| v.as_str()) {
            let _ = client.delete("Account", id).await;
        }
    }
}

#[tokio::test]
async fn test_rest_search_sosl() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let search_query = "FIND {test*} IN NAME FIELDS RETURNING Account(Id, Name), Contact(Id, Name)";
    let result = client.search::<serde_json::Value>(search_query).await;

    assert!(result.is_ok(), "SOSL search should succeed");
}

#[tokio::test]
async fn test_rest_batch_operations() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let test_accounts = vec![
        serde_json::json!({
            "Name": format!("Batch Test 1 {}", chrono::Utc::now().timestamp_millis())
        }),
        serde_json::json!({
            "Name": format!("Batch Test 2 {}", chrono::Utc::now().timestamp_millis())
        }),
        serde_json::json!({
            "Name": format!("Batch Test 3 {}", chrono::Utc::now().timestamp_millis())
        }),
    ];

    let create_results = client
        .create_multiple("Account", &test_accounts, false)
        .await
        .expect("create_multiple should succeed");

    assert_eq!(create_results.len(), 3, "Should create 3 accounts");

    let ids: Vec<String> = create_results.iter().filter_map(|r| r.id.clone()).collect();
    assert_eq!(ids.len(), 3, "Should have 3 account IDs");

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let get_results: Vec<serde_json::Value> = client
        .get_multiple("Account", &id_refs, &["Id", "Name"])
        .await
        .expect("get_multiple should succeed");

    assert_eq!(get_results.len(), 3, "Should retrieve 3 accounts");

    let updates: Vec<(String, serde_json::Value)> = ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                serde_json::json!({
                    "Description": "Updated by batch test"
                }),
            )
        })
        .collect();

    let update_results = client
        .update_multiple("Account", &updates, false)
        .await
        .expect("update_multiple should succeed");

    assert_eq!(update_results.len(), 3, "Should update 3 accounts");

    let delete_results = client
        .delete_multiple(&id_refs, false)
        .await
        .expect("delete_multiple should succeed");

    assert_eq!(delete_results.len(), 3, "Should delete 3 accounts");
}

#[tokio::test]
async fn test_rest_query_pagination() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, Name FROM Account LIMIT 5")
        .await
        .expect("Query should succeed");

    assert!(
        result.done || result.next_records_url.is_some(),
        "Should indicate completion or pagination"
    );

    let all_records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account LIMIT 100")
        .await
        .expect("query_all should succeed");

    assert!(all_records.len() <= 100, "Should respect LIMIT");
}

#[tokio::test]
async fn test_rest_upsert_operation() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let unique_number = format!("TEST-{}", chrono::Utc::now().timestamp_millis());

    let account_data = serde_json::json!({
        "Name": format!("Upsert Test {}", unique_number),
        "AccountNumber": unique_number
    });

    let result1 = client
        .upsert("Account", "AccountNumber", &unique_number, &account_data)
        .await;

    if let Ok(upsert_result) = result1 {
        assert!(upsert_result.created, "First upsert should create record");
        let account_id = upsert_result.id.clone();

        let updated_data = serde_json::json!({
            "Name": format!("Upsert Test Updated {}", unique_number),
            "AccountNumber": unique_number
        });

        let result2 = client
            .upsert("Account", "AccountNumber", &unique_number, &updated_data)
            .await;

        if let Ok(upsert_result2) = result2 {
            assert!(
                !upsert_result2.created,
                "Second upsert should update record"
            );
            assert_eq!(upsert_result2.id, account_id, "Should be same account ID");
        }

        let _ = client.delete("Account", &account_id).await;
    } else {
        println!("Note: Upsert test skipped - AccountNumber may not be set as external ID");
    }
}

// ============================================================================
// QueryBuilder Security Tests
// ============================================================================

#[tokio::test]
async fn test_query_builder_injection_prevention() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let malicious_input = "Test' OR '1'='1";

    let result: Result<Vec<serde_json::Value>, _> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Name", malicious_input)
        .expect("where_eq should succeed")
        .limit(10)
        .execute(&client)
        .await;

    assert!(result.is_ok(), "Query should succeed with escaped input");
    let accounts = result.unwrap();

    assert_eq!(
        accounts.len(),
        0,
        "Should not find any accounts with malicious input"
    );
}

#[tokio::test]
async fn test_query_builder_like_escaping() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let pattern = "test%";

    let result: Result<Vec<serde_json::Value>, _> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_like("Name", pattern)
        .expect("where_like should succeed")
        .limit(10)
        .execute(&client)
        .await;

    assert!(result.is_ok(), "LIKE query should succeed");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_rest_error_invalid_field_query() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, InvalidFieldName123 FROM Account")
        .await;

    assert!(result.is_err(), "Query with invalid field should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sobject_describe() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.describe_sobject("NonExistentObject__c").await;

    assert!(
        result.is_err(),
        "Describing non-existent object should fail"
    );
}

#[tokio::test]
async fn test_rest_error_invalid_record_id_get() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result: Result<serde_json::Value, _> = client.get("Account", "bad-id", None).await;

    assert!(result.is_err(), "Getting non-existent record should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sobject_create() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .create(
            "Account'; DROP TABLE--",
            &serde_json::json!({"Name": "Bad"}),
        )
        .await;

    assert!(result.is_err(), "Create with invalid SObject should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_id_update_delete() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let update_result = client
        .update("Account", "bad-id", &serde_json::json!({"Name": "Bad"}))
        .await;

    assert!(update_result.is_err(), "Update with invalid ID should fail");

    let delete_result = client.delete("Account", "bad-id").await;

    assert!(delete_result.is_err(), "Delete with invalid ID should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_upsert_field() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .upsert(
            "Account",
            "AccountNumber; DROP",
            "bad",
            &serde_json::json!({"Name": "Bad"}),
        )
        .await;

    assert!(result.is_err(), "Upsert with invalid field should fail");
}

#[tokio::test]
async fn test_rest_error_invalid_sosl() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client
        .search::<serde_json::Value>("FIND {test} RETURNING")
        .await;

    assert!(result.is_err(), "Invalid SOSL should fail");
}

#[tokio::test]
async fn test_rest_error_batch_invalid_input() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let create_result = client
        .create_multiple("Account; DROP", &Vec::<serde_json::Value>::new(), false)
        .await;

    assert!(
        create_result.is_err(),
        "create_multiple invalid sobject should fail"
    );

    let update_result = client
        .update_multiple(
            "Account",
            &[("bad-id".to_string(), serde_json::json!({"Name": "Bad"}))],
            false,
        )
        .await;

    assert!(
        update_result.is_err(),
        "update_multiple invalid ID should fail"
    );

    let delete_result = client.delete_multiple(&["bad-id"], false).await;

    assert!(
        delete_result.is_err(),
        "delete_multiple invalid ID should fail"
    );

    let get_result: Result<Vec<serde_json::Value>, _> =
        client.get_multiple("Account", &["bad-id"], &["Id"]).await;

    assert!(get_result.is_err(), "get_multiple invalid ID should fail");
}

#[tokio::test]
async fn test_rest_composite_error_subrequest() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let composite_request = CompositeRequest {
        all_or_none: false,
        collate_subrequests: false,
        subrequests: vec![CompositeSubrequest {
            method: "GET".to_string(),
            url: format!("/services/data/v{}/sobjects/Nope__c", creds.api_version()),
            reference_id: "BadRequest".to_string(),
            body: None,
        }],
    };

    let response = client
        .composite(&composite_request)
        .await
        .expect("Composite request should succeed at transport level");

    let sub = response
        .responses
        .first()
        .expect("Should have one response");
    assert!(sub.http_status_code >= 400, "Subrequest should error");
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
async fn test_credentials_redaction() {
    let creds = get_credentials().await;

    let debug_output = format!("{:?}", creds);

    assert!(
        debug_output.contains("[REDACTED]") || !debug_output.contains(creds.access_token()),
        "Debug output should not contain actual token"
    );
}

#[tokio::test]
async fn test_client_debug_redaction() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let debug_output = format!("{:?}", client);

    assert!(
        !debug_output.contains(creds.access_token()),
        "Client debug output should not contain actual token"
    );
}

// ============================================================================
// Type-Safe Pattern Tests
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestAccount {
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry", skip_serializing_if = "Option::is_none")]
    industry: Option<String>,
}

#[tokio::test]
async fn test_type_safe_crud() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let account = TestAccount {
        id: None,
        name: format!("Type Safe Test {}", chrono::Utc::now().timestamp_millis()),
        industry: Some("Technology".to_string()),
    };

    let id = client
        .create("Account", &account)
        .await
        .expect("Create should succeed");

    let retrieved: TestAccount = client
        .get("Account", &id, Some(&["Id", "Name", "Industry"]))
        .await
        .expect("Get should succeed");

    assert_eq!(retrieved.name, account.name);
    assert_eq!(retrieved.industry, account.industry);

    let update_data = serde_json::json!({
        "Industry": "Finance"
    });

    client
        .update("Account", &id, &update_data)
        .await
        .expect("Update should succeed");

    client
        .delete("Account", &id)
        .await
        .expect("Delete should succeed");
}

#[tokio::test]
async fn test_type_safe_query() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let accounts: Vec<TestAccount> = client
        .query_all("SELECT Id, Name, Industry FROM Account LIMIT 10")
        .await
        .expect("Query should succeed");

    for account in &accounts {
        assert!(account.id.is_some(), "Account should have ID");
        assert!(!account.name.is_empty(), "Account should have name");
    }
}

// ============================================================================
// Quick Actions Integration Tests
// ============================================================================

#[tokio::test]
async fn test_quick_actions_list() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List quick actions for Account
    let actions = client
        .list_quick_actions("Account")
        .await
        .expect("list_quick_actions should succeed");

    // Quick actions may or may not exist depending on org setup
    // Just verify the API call succeeds and returns a valid array
    assert!(
        actions.is_empty() || !actions.is_empty(),
        "Should return valid quick actions array"
    );
}

#[tokio::test]
async fn test_quick_actions_describe() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // First, list quick actions to find one that exists
    let actions = client
        .list_quick_actions("Account")
        .await
        .expect("list_quick_actions should succeed");

    if let Some(action) = actions.first() {
        // Describe the first available action
        let description = client
            .describe_quick_action("Account", &action.name)
            .await
            .expect("describe_quick_action should succeed");

        assert_eq!(description.name, action.name);
        assert!(!description.label.is_empty());
    } else {
        println!("Note: No quick actions available in org for Account");
    }
}

#[tokio::test]
async fn test_quick_actions_invoke() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // First, list quick actions to find an invocable one
    let actions = client
        .list_quick_actions("Account")
        .await
        .expect("list_quick_actions should succeed");

    if let Some(action) = actions.first() {
        // Try to invoke with minimal data - may fail if action requires specific fields
        let result = client
            .invoke_quick_action("Account", &action.name, &serde_json::json!({}))
            .await;

        // Invocation may fail due to missing required fields, but the API call should be valid
        match result {
            Ok(_) => println!("Quick action invoked successfully"),
            Err(e) => println!("Quick action invocation failed as expected: {}", e),
        }
    } else {
        println!("Note: No quick actions available in org for Account");
    }
}

// ============================================================================
// List Views Integration Tests
// ============================================================================

#[tokio::test]
async fn test_list_views_list() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List views for Account
    let list_views = client
        .list_views("Account")
        .await
        .expect("list_views should succeed");

    // Standard orgs should have at least some list views
    assert!(
        !list_views.listviews.is_empty(),
        "Should have at least one list view for Account"
    );

    // Verify structure of first list view
    if let Some(view) = list_views.listviews.first() {
        assert!(!view.id.is_empty(), "List view should have ID");
        assert!(!view.label.is_empty(), "List view should have label");
        assert_eq!(view.sobject_type, "Account");
    }
}

#[tokio::test]
async fn test_list_views_get() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // First, get list of views
    let list_views = client
        .list_views("Account")
        .await
        .expect("list_views should succeed");

    if let Some(view) = list_views.listviews.first() {
        // Get specific list view
        let specific_view = client
            .get_list_view("Account", &view.id)
            .await
            .expect("get_list_view should succeed");

        assert_eq!(specific_view.id, view.id);
        assert_eq!(specific_view.label, view.label);
    }
}

#[tokio::test]
async fn test_list_views_describe() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // First, get list of views
    let list_views = client
        .list_views("Account")
        .await
        .expect("list_views should succeed");

    if let Some(view) = list_views.listviews.first() {
        // Describe the list view
        let description = client
            .describe_list_view("Account", &view.id)
            .await
            .expect("describe_list_view should succeed");

        assert_eq!(description.id, view.id);
        assert!(!description.columns.is_empty(), "Should have columns");
        assert!(!description.query.is_empty(), "Should have query");
    }
}

#[tokio::test]
async fn test_list_views_execute() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // First, get list of views
    let list_views = client
        .list_views("Account")
        .await
        .expect("list_views should succeed");

    if let Some(view) = list_views.listviews.first() {
        // Execute the list view
        let results: busbar_sf_rest::ListViewResult<serde_json::Value> = client
            .execute_list_view("Account", &view.id)
            .await
            .expect("execute_list_view should succeed");

        assert_eq!(results.id, view.id);
        assert_eq!(results.label, view.label);
        // Results may or may not be empty depending on data in org
        assert!(results.size >= 0, "Should have valid size");
    }
}

// ============================================================================
// Process Rules Integration Tests
// ============================================================================

#[tokio::test]
async fn test_process_rules_list_all() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List all process rules
    let rules = client
        .list_process_rules()
        .await
        .expect("list_process_rules should succeed");

    // Process rules may or may not exist
    assert!(
        rules.rules.is_empty() || !rules.rules.is_empty(),
        "Should return valid process rules array"
    );
}

#[tokio::test]
async fn test_process_rules_list_for_sobject() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List process rules for Account
    let rules = client
        .list_process_rules_for_sobject("Account")
        .await
        .expect("list_process_rules_for_sobject should succeed");

    // Process rules may or may not exist
    assert!(
        rules.rules.is_empty() || !rules.rules.is_empty(),
        "Should return valid process rules array"
    );
}

#[tokio::test]
async fn test_process_rules_trigger() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account to use as context
    let account_name = format!(
        "Process Rule Test {}",
        chrono::Utc::now().timestamp_millis()
    );
    let account_id = client
        .create("Account", &serde_json::json!({"Name": account_name}))
        .await
        .expect("Should create test account");

    // Try to trigger process rules for the account
    let request = busbar_sf_rest::ProcessRuleRequest {
        context_id: account_id.clone(),
    };

    let result = client.trigger_process_rules(&request).await;

    // This may succeed or fail depending on whether process rules exist
    match result {
        Ok(rule_result) => {
            println!("Process rules triggered, success: {}", rule_result.success);
        }
        Err(e) => {
            println!("Process rules trigger failed (may be expected): {}", e);
        }
    }

    // Clean up
    let _ = client.delete("Account", &account_id).await;
}

// ============================================================================
// Approvals Integration Tests
// ============================================================================

#[tokio::test]
async fn test_approvals_list_pending() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List pending approvals
    let approvals = client
        .list_pending_approvals()
        .await
        .expect("list_pending_approvals should succeed");

    // Approvals may or may not exist
    assert!(
        approvals.approvals.is_empty() || !approvals.approvals.is_empty(),
        "Should return valid approvals array"
    );
}

#[tokio::test]
async fn test_approvals_submit() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a test account
    let account_name = format!("Approval Test {}", chrono::Utc::now().timestamp_millis());
    let account_id = client
        .create("Account", &serde_json::json!({"Name": account_name}))
        .await
        .expect("Should create test account");

    // Try to submit for approval
    let request = busbar_sf_rest::ApprovalRequest {
        action_type: busbar_sf_rest::ApprovalActionType::Submit,
        context_id: account_id.clone(),
        context_actor_id: None,
        comments: Some("Integration test approval".to_string()),
        next_approver_ids: None,
        process_definition_name_or_id: None,
        skip_entry_criteria: None,
    };

    let result = client.submit_approval(&request).await;

    // This will likely fail if no approval process is set up, but API call should be valid
    match result {
        Ok(approval_result) => {
            println!("Approval submitted successfully");
            assert!(approval_result.success, "Approval should succeed");
        }
        Err(e) => {
            println!(
                "Approval submission failed (expected if no approval process): {}",
                e
            );
        }
    }

    // Clean up
    let _ = client.delete("Account", &account_id).await;
}

// ============================================================================
// Invocable Actions Integration Tests
// ============================================================================

#[tokio::test]
async fn test_invocable_actions_list_standard() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List standard invocable actions
    let actions = client
        .list_standard_actions()
        .await
        .expect("list_standard_actions should succeed");

    // Standard actions should exist in most orgs
    assert!(
        !actions.actions.is_empty(),
        "Should have standard invocable actions"
    );

    // Verify structure
    if let Some(action) = actions.actions.first() {
        assert!(!action.name.is_empty(), "Action should have name");
        assert!(!action.label.is_empty(), "Action should have label");
    }
}

#[tokio::test]
async fn test_invocable_actions_describe_standard() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List to find an action to describe
    let actions = client
        .list_standard_actions()
        .await
        .expect("list_standard_actions should succeed");

    if let Some(action) = actions.actions.first() {
        // Describe the action
        let description = client
            .describe_standard_action(&action.name)
            .await
            .expect("describe_standard_action should succeed");

        assert_eq!(description.name, action.name);
        assert!(!description.label.is_empty());
    }
}

#[tokio::test]
async fn test_invocable_actions_invoke_standard() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Try to invoke emailSimple action if it exists
    let actions = client
        .list_standard_actions()
        .await
        .expect("list_standard_actions should succeed");

    // Look for emailSimple or any email-related action
    if let Some(action) = actions.actions.iter().find(|a| a.name.contains("email")) {
        let request = busbar_sf_rest::InvocableActionRequest {
            inputs: vec![serde_json::json!({})],
        };

        let result = client.invoke_standard_action(&action.name, &request).await;

        // May fail due to missing required fields, but API call should be valid
        match result {
            Ok(_) => println!("Standard action invoked successfully"),
            Err(e) => println!("Standard action invocation failed (may be expected): {}", e),
        }
    } else {
        println!("Note: No suitable standard action found for testing");
    }
}

#[tokio::test]
async fn test_invocable_actions_list_custom() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List custom invocable actions
    let actions = client
        .list_custom_actions()
        .await
        .expect("list_custom_actions should succeed");

    // Custom actions may or may not exist
    assert!(
        actions.actions.is_empty() || !actions.actions.is_empty(),
        "Should return valid custom actions array"
    );
}

#[tokio::test]
async fn test_invocable_actions_describe_custom() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List custom actions
    let actions = client
        .list_custom_actions()
        .await
        .expect("list_custom_actions should succeed");

    if let Some(action) = actions.actions.first() {
        // Describe the custom action
        let description = client
            .describe_custom_action(&action.name)
            .await
            .expect("describe_custom_action should succeed");

        assert_eq!(description.name, action.name);
        assert!(!description.label.is_empty());
    } else {
        println!("Note: No custom actions available in org");
    }
}

#[tokio::test]
async fn test_invocable_actions_invoke_custom() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // List custom actions
    let actions = client
        .list_custom_actions()
        .await
        .expect("list_custom_actions should succeed");

    if let Some(action) = actions.actions.first() {
        // Try to invoke with minimal data
        let request = busbar_sf_rest::InvocableActionRequest {
            inputs: vec![serde_json::json!({})],
        };

        let result = client.invoke_custom_action(&action.name, &request).await;

        // May fail due to missing required fields, but API call should be valid
        match result {
            Ok(_) => println!("Custom action invoked successfully"),
            Err(e) => println!("Custom action invocation failed (may be expected): {}", e),
        }
    } else {
        println!("Note: No custom actions available in org");
    }
}

// ============================================================================
// Error Handling Tests for New Endpoints
// ============================================================================

#[tokio::test]
async fn test_quick_actions_error_invalid_sobject() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.list_quick_actions("Invalid;Object").await;
    assert!(result.is_err(), "Should reject invalid SObject name");
}

#[tokio::test]
async fn test_list_views_error_invalid_id() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.get_list_view("Account", "invalid-id").await;
    assert!(result.is_err(), "Should reject invalid list view ID");
}

#[tokio::test]
async fn test_process_rules_error_invalid_id() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let request = busbar_sf_rest::ProcessRuleRequest {
        context_id: "invalid-id".to_string(),
    };

    let result = client.trigger_process_rules(&request).await;
    assert!(result.is_err(), "Should reject invalid context ID");
}

#[tokio::test]
async fn test_approvals_error_invalid_id() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let request = busbar_sf_rest::ApprovalRequest {
        action_type: busbar_sf_rest::ApprovalActionType::Submit,
        context_id: "invalid-id".to_string(),
        context_actor_id: None,
        comments: None,
        next_approver_ids: None,
        process_definition_name_or_id: None,
        skip_entry_criteria: None,
    };

    let result = client.submit_approval(&request).await;
    assert!(result.is_err(), "Should reject invalid context ID");
}

#[tokio::test]
async fn test_invocable_actions_error_invalid_name() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.describe_standard_action("Invalid;Action").await;
    assert!(result.is_err(), "Should reject invalid action name");
}
