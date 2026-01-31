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
// Standalone REST Resources - Integration Tests
// ============================================================================

#[tokio::test]
async fn test_rest_tabs() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let tabs = client.tabs().await.expect("tabs() should succeed");

    assert!(!tabs.is_empty(), "Should return at least one tab");
    // Verify structure of first tab
    if let Some(first_tab) = tabs.first() {
        assert!(first_tab.get("label").is_some(), "Tab should have a label");
        assert!(first_tab.get("url").is_some(), "Tab should have a url");
    }
}

#[tokio::test]
async fn test_rest_theme() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let theme = client.theme().await.expect("theme() should succeed");

    // Verify theme has expected structure
    assert!(
        theme.get("themeItems").is_some(),
        "Theme should have themeItems"
    );
}

#[tokio::test]
async fn test_rest_app_menu_app_switcher() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let app_menu = client
        .app_menu("AppSwitcher")
        .await
        .expect("app_menu(AppSwitcher) should succeed");

    // Verify structure
    assert!(app_menu.is_object(), "App menu should be an object");
}

#[tokio::test]
async fn test_rest_app_menu_salesforce1() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let app_menu = client
        .app_menu("Salesforce1")
        .await
        .expect("app_menu(Salesforce1) should succeed");

    // Verify structure
    assert!(app_menu.is_object(), "App menu should be an object");
}

#[tokio::test]
async fn test_rest_app_menu_invalid_type() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.app_menu("InvalidType").await;

    assert!(
        result.is_err(),
        "app_menu with invalid type should return error"
    );
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("INVALID_APP_MENU_TYPE"),
        "Error should indicate invalid app menu type"
    );
}

#[tokio::test]
async fn test_rest_recent_items() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let recent = client
        .recent_items()
        .await
        .expect("recent_items() should succeed");

    // Recent items is a Vec (could be empty if no recent items)
    // Just verify it returns successfully
    let _count = recent.len(); // Verify it's a valid Vec
}

#[tokio::test]
async fn test_rest_relevant_items() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let relevant = client
        .relevant_items()
        .await
        .expect("relevant_items() should succeed");

    // Relevant items should be an object or array
    assert!(
        relevant.is_object() || relevant.is_array(),
        "Relevant items should be an object or array"
    );
}

#[tokio::test]
async fn test_rest_compact_layouts() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let layouts = client
        .compact_layouts("Account,Contact")
        .await
        .expect("compact_layouts() should succeed");

    // Verify structure
    assert!(
        layouts.is_object() || layouts.is_array(),
        "Compact layouts should be an object or array"
    );
}

#[tokio::test]
async fn test_rest_compact_layouts_invalid_sobject() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.compact_layouts("Invalid$Object").await;

    assert!(
        result.is_err(),
        "compact_layouts with invalid sobject should return error"
    );
}

#[tokio::test]
async fn test_rest_compact_layouts_empty_input() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let result = client.compact_layouts("").await;

    assert!(
        result.is_err(),
        "compact_layouts with empty input should return error"
    );
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("cannot be empty"),
        "Error should indicate empty input"
    );
}

#[tokio::test]
async fn test_rest_platform_event_schema_invalid_name() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Test with invalid event name (contains invalid character)
    let result = client.platform_event_schema("Invalid$Event__e").await;

    assert!(
        result.is_err(),
        "platform_event_schema with invalid event name should return error"
    );
}

#[tokio::test]
async fn test_rest_lightning_toggle_metrics() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let metrics = client
        .lightning_toggle_metrics()
        .await
        .expect("lightning_toggle_metrics() should succeed");

    // Verify it returns some data
    assert!(
        metrics.is_object() || metrics.is_array(),
        "Lightning toggle metrics should be an object or array"
    );
}

#[tokio::test]
async fn test_rest_lightning_usage() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let usage = client
        .lightning_usage()
        .await
        .expect("lightning_usage() should succeed");

    // Verify it returns some data
    assert!(
        usage.is_object() || usage.is_array(),
        "Lightning usage should be an object or array"
    );
}

#[tokio::test]
async fn test_rest_deploy_returns_error() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let zip_data = vec![0u8; 100]; // Dummy zip data
    let options = serde_json::json!({
        "singlePackage": true,
        "checkOnly": false,
        "rollbackOnError": true
    });

    let result = client.rest_deploy(&zip_data, &options).await;

    // This should return an error since it's not implemented
    assert!(
        result.is_err(),
        "rest_deploy() should return error as it's not implemented"
    );
    let error = result.unwrap_err();
    let error_msg = error.to_string();
    assert!(
        error_msg.contains("multipart") || error_msg.contains("SOAP"),
        "Error should mention multipart or SOAP alternative"
    );
}
