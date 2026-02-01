//! Tooling API integration tests using SF_AUTH_URL.

use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_tooling::{
    ClientConfig, CompositeBatchRequest, CompositeBatchSubrequest, CompositeRequest,
    CompositeSubrequest, RunTestsAsyncRequest, ToolingClient,
};

// Global mutex to serialize Apex class creation across tests
static APEX_CLASS_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

// ============================================================================
// Tooling API Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_query_apex_classes() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, Name, Status FROM ApexClass LIMIT 10")
        .await;

    assert!(result.is_ok(), "Tooling query should succeed");

    let query_result = result.unwrap();
    assert!(
        query_result.done || query_result.next_records_url.is_some(),
        "Query should complete or have pagination"
    );
}

#[tokio::test]
async fn test_tooling_execute_anonymous_success() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let apex_code = r#"
        System.debug('Integration test from busbar-sf-api');
        Integer result = 2 + 2;
        System.debug('Result: ' + result);
    "#;

    let result = client
        .execute_anonymous(apex_code)
        .await
        .expect("Execute anonymous should succeed");

    assert!(result.compiled, "Apex should compile");
    assert!(result.success, "Apex should execute successfully");
}

#[tokio::test]
async fn test_tooling_execute_anonymous_compile_error() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let invalid_apex = "this is not valid apex code at all;";

    let result = client.execute_anonymous(invalid_apex).await;

    assert!(result.is_err(), "Invalid Apex should return error");
}

#[tokio::test]
async fn test_tooling_query_all_pagination() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let records: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM ApexClass LIMIT 50")
        .await
        .expect("query_all should succeed");

    assert!(records.len() <= 50, "Should respect LIMIT");
}

// ============================================================================
// Composite API Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_composite_api() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Create a simple composite request with Tooling API queries
    let composite_request = CompositeRequest {
        all_or_none: false,
        collate_subrequests: false,
        subrequests: vec![
            CompositeSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/tooling/query?q=SELECT+Id,Name+FROM+ApexClass+LIMIT+1",
                    creds.api_version()
                ),
                reference_id: "ApexClassQuery".to_string(),
                body: None,
            },
            CompositeSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/tooling/query?q=SELECT+Id+FROM+DebugLevel+LIMIT+1",
                    creds.api_version()
                ),
                reference_id: "DebugLevelQuery".to_string(),
                body: None,
            },
        ],
    };

    let response = client
        .composite(&composite_request)
        .await
        .expect("Tooling composite request should succeed");

    assert_eq!(response.responses.len(), 2, "Should have 2 sub-responses");

    for sub_response in &response.responses {
        assert_eq!(
            sub_response.http_status_code, 200,
            "Each sub-request should succeed"
        );
    }
}

#[tokio::test]
async fn test_tooling_composite_batch() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Create a batch request with independent queries
    let batch_request = CompositeBatchRequest {
        halt_on_error: false,
        batch_requests: vec![
            CompositeBatchSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/tooling/query?q=SELECT+Id+FROM+ApexClass+LIMIT+5",
                    creds.api_version()
                ),
                rich_input: None,
                binary_part_name: None,
                binary_part_name_alias: None,
            },
            CompositeBatchSubrequest {
                method: "GET".to_string(),
                url: format!(
                    "/services/data/v{}/tooling/query?q=SELECT+Id+FROM+ApexTrigger+LIMIT+5",
                    creds.api_version()
                ),
                rich_input: None,
                binary_part_name: None,
                binary_part_name_alias: None,
            },
        ],
    };

    let response = client
        .composite_batch(&batch_request)
        .await
        .expect("Tooling composite batch request should succeed");

    assert_eq!(response.results.len(), 2, "Should have 2 batch results");

    for result in &response.results {
        assert_eq!(result.status_code, 200, "Each batch request should succeed");
    }
}

// ============================================================================
// SObject Collections Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_collections_get_multiple() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // First, query to get some ApexClass IDs
    let query_result: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM ApexClass LIMIT 3")
        .await
        .expect("Query should succeed");

    assert!(
        !query_result.is_empty(),
        "Org should have at least one ApexClass"
    );

    let ids: Vec<String> = query_result
        .iter()
        .filter_map(|r| r.get("Id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    assert!(!ids.is_empty(), "ApexClass records should have Id fields");

    let id_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();

    // Test get_multiple - retrieves records by ID using Tooling API SOQL query.
    // (The SObject Collections GET endpoint is documented but does not work
    // reliably on the Tooling API, so get_multiple uses SOQL internally.)
    let results: Vec<serde_json::Value> = client
        .get_multiple("ApexClass", &id_refs, &["Id", "Name"])
        .await
        .unwrap_or_else(|e| panic!("get_multiple failed for ApexClass with IDs {:?}: {e}", &ids));

    assert_eq!(
        results.len(),
        ids.len(),
        "Should return exactly as many records as IDs requested"
    );

    for result in &results {
        let id = result
            .get("Id")
            .and_then(|v| v.as_str())
            .expect("Each record should have an Id field");
        assert!(
            ids.contains(&id.to_string()),
            "Returned Id {id} should be one of the requested IDs"
        );
    }
}

#[tokio::test]
#[ignore = "incomplete test - no create/update/delete operations performed"]
async fn test_tooling_collections_create_update_delete() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM DebugLevel LIMIT 1")
        .await
        .expect("Query DebugLevel should succeed");

    assert!(
        !debug_levels.is_empty(),
        "Org should have at least one DebugLevel"
    );

    // TODO: Implement actual create/update/delete test with TraceFlag
    panic!("Test not implemented - needs actual CRUD operations");
}

#[tokio::test]
async fn test_tooling_collections_delete_multiple() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Test delete_multiple with invalid IDs to verify the endpoint works
    // (we don't want to actually delete real data in integration tests)
    let fake_ids = vec!["000000000000000AAA", "000000000000000AAB"];

    let result = client.delete_multiple(&fake_ids, false).await;

    let results = result.expect("delete_multiple should return results even for invalid IDs");
    assert_eq!(results.len(), 2, "Should have 2 delete results");
    for res in &results {
        assert!(
            !res.success || !res.errors.is_empty(),
            "Fake ID deletion should fail or have errors"
        );
    }
}

#[tokio::test]
async fn test_tooling_create_multiple_trace_flags() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM DebugLevel LIMIT 1")
        .await
        .expect("Query DebugLevel should succeed");
    assert!(
        !debug_levels.is_empty(),
        "Org should have at least one DebugLevel"
    );

    let debug_level_id = debug_levels[0]
        .get("Id")
        .and_then(|v| v.as_str())
        .expect("Should have DebugLevel Id");

    // Get an active user ID
    let user_query: Vec<serde_json::Value> = client
        .inner()
        .query_all("SELECT Id FROM User WHERE IsActive = true LIMIT 1")
        .await
        .expect("User query should succeed");
    assert!(
        !user_query.is_empty(),
        "Org should have at least one active user"
    );

    let user_id = user_query[0]
        .get("Id")
        .and_then(|v| v.as_str())
        .expect("Should have User Id");

    let now = chrono::Utc::now();
    let expiration = now + chrono::Duration::hours(1);

    let trace_flags = vec![serde_json::json!({
        "TracedEntityId": user_id,
        "DebugLevelId": debug_level_id,
        "StartDate": now.to_rfc3339(),
        "ExpirationDate": expiration.to_rfc3339(),
        "LogType": "USER_DEBUG"
    })];

    // Attempt to create - this may fail if trace flag already exists
    let result = client
        .create_multiple("TraceFlag", &trace_flags, false)
        .await;

    let results = result.expect("create_multiple TraceFlag should succeed");
    assert_eq!(results.len(), 1, "Should have 1 result");

    // Clean up if successful
    if let Some(id) = results[0].id.as_ref() {
        let _ = client.delete("TraceFlag", id).await;
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_error_invalid_query() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .query::<serde_json::Value>("SELECT Id, NotAField__c FROM ApexClass")
        .await;

    assert!(
        result.is_err(),
        "Tooling query with invalid field should fail"
    );
}

#[tokio::test]
async fn test_tooling_error_invalid_sobject_create_get_delete() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let create_result = client
        .create("ApexClass; DROP", &serde_json::json!({"Name": "Bad"}))
        .await;

    assert!(
        create_result.is_err(),
        "Create with invalid SObject should fail"
    );

    let get_result: Result<serde_json::Value, _> = client.get("ApexClass", "bad-id").await;

    assert!(get_result.is_err(), "Get with invalid ID should fail");

    let delete_result = client.delete("ApexClass", "bad-id").await;

    assert!(delete_result.is_err(), "Delete with invalid ID should fail");
}

#[tokio::test]
async fn test_tooling_error_invalid_log_id() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.get_apex_log_body("bad-id").await;

    assert!(result.is_err(), "Log body with invalid ID should fail");
}

// ============================================================================
// PR #59: CRUD & Discovery Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_update() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query for an existing debug level to try updating
    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id, DeveloperName FROM DebugLevel LIMIT 1")
        .await
        .expect("Query DebugLevel should succeed");

    assert!(
        !debug_levels.is_empty(),
        "Org should have at least one DebugLevel"
    );

    let debug_level_id = debug_levels[0]
        .get("Id")
        .and_then(|v| v.as_str())
        .expect("Should have DebugLevel Id");

    // Update with the same value (safe idempotent update)
    let developer_name = debug_levels[0]
        .get("DeveloperName")
        .and_then(|v| v.as_str())
        .unwrap_or("SFDC_DevConsole");

    let update_body = serde_json::json!({
        "MasterLabel": developer_name
    });

    let result = client
        .update("DebugLevel", debug_level_id, &update_body)
        .await;
    assert!(result.is_ok(), "Update should succeed: {:?}", result.err());
}

#[tokio::test]
async fn test_tooling_update_invalid_sobject() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .update(
            "Robert'; DROP TABLE--",
            "7tf000000000001AAA",
            &serde_json::json!({}),
        )
        .await;
    assert!(result.is_err(), "Update with invalid SObject should fail");
}

#[tokio::test]
async fn test_tooling_query_all_records() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result: busbar_sf_tooling::QueryResult<serde_json::Value> = client
        .query_all_records("SELECT Id, Name FROM ApexClass LIMIT 5")
        .await
        .expect("query_all_records should succeed");

    assert!(result.done, "Query should be done");
    assert!(result.records.len() <= 5, "Should respect LIMIT");
}

#[tokio::test]
async fn test_tooling_describe_global() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .describe_global()
        .await
        .expect("describe_global should succeed");

    assert!(!result.sobjects.is_empty(), "Should have SObjects");

    // ApexClass should be in the list
    let has_apex_class = result.sobjects.iter().any(|s| s.name == "ApexClass");
    assert!(has_apex_class, "Should include ApexClass");
}

#[tokio::test]
async fn test_tooling_describe_sobject() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .describe_sobject("ApexClass")
        .await
        .expect("describe_sobject should succeed");

    assert_eq!(result.name, "ApexClass");
    assert!(!result.fields.is_empty(), "Should have fields");
}

#[tokio::test]
async fn test_tooling_basic_info() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client
        .basic_info("ApexClass")
        .await
        .expect("basic_info should succeed");

    assert!(
        result.get("objectDescribe").is_some(),
        "Should have objectDescribe"
    );
}

#[tokio::test]
async fn test_tooling_resources() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.resources().await.expect("resources should succeed");

    // The resources endpoint should return something (it's a JSON map)
    assert!(result.is_object(), "Resources should return a JSON object");
}

// ============================================================================
// PR #61: Test Execution Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_run_tests_async() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let _lock = APEX_CLASS_LOCK.lock().await;

    // Create a test class to run
    let test_class_name = format!("BusbarAsyncTest_{}", chrono::Utc::now().timestamp_millis());
    let test_body = format!(
        "@IsTest\npublic class {} {{\n    @IsTest\n    static void testPass() {{\n        System.assert(true);\n    }}\n}}",
        test_class_name
    );

    let class_id = client
        .create(
            "ApexClass",
            &serde_json::json!({
                "Name": test_class_name,
                "Body": test_body
            }),
        )
        .await
        .expect("ApexClass creation should succeed");

    // Run tests async
    let request = RunTestsAsyncRequest {
        class_names: Some(vec![test_class_name.clone()]),
        test_level: Some("RunSpecifiedTests".to_string()),
        ..Default::default()
    };

    let result = client.run_tests_async(&request).await;

    // Clean up the test class regardless of result
    let _ = client.delete("ApexClass", &class_id).await;

    let job_id = result.expect("run_tests_async should succeed");
    assert!(!job_id.is_empty(), "Job ID should not be empty");
}

#[tokio::test]
async fn test_tooling_run_tests_sync() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let _lock = APEX_CLASS_LOCK.lock().await;

    // Create a test class to run
    let test_class_name = format!("BusbarSyncTest_{}", chrono::Utc::now().timestamp_millis());
    let test_body = format!(
        "@IsTest\npublic class {} {{\n    @IsTest\n    static void testPass() {{\n        System.assert(true);\n    }}\n}}",
        test_class_name
    );

    let class_id = client
        .create(
            "ApexClass",
            &serde_json::json!({
                "Name": test_class_name,
                "Body": test_body
            }),
        )
        .await
        .expect("ApexClass creation should succeed");

    // Run tests synchronously
    let request = busbar_sf_tooling::RunTestsSyncRequest {
        tests: Some(vec![test_class_name.clone()]),
        ..Default::default()
    };

    let result = client.run_tests_sync(&request).await;

    // Clean up the test class regardless of result
    let _ = client.delete("ApexClass", &class_id).await;

    let sync_result = result.expect("run_tests_sync should succeed");
    assert!(
        sync_result.num_tests_run > 0,
        "Should have run at least one test"
    );
}

// ============================================================================
// PR #62: Code Intelligence Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_completions_apex() {
    let creds = get_credentials().await;
    // Apex completions returns the entire Apex standard library — response is very large.
    // Use a longer timeout than the default 30s to avoid retry exhaustion.
    let config = ClientConfig::builder()
        .with_timeout(Duration::from_secs(120))
        .build();
    let client = ToolingClient::with_config(creds.instance_url(), creds.access_token(), config)
        .expect("Failed to create Tooling client");

    let completions = client
        .completions_apex()
        .await
        .expect("completions_apex should succeed");
    let obj = completions
        .as_object()
        .expect("completions response should be a JSON object");
    assert!(!obj.is_empty(), "Should have Apex completions data");
}

#[tokio::test]
async fn test_tooling_completions_visualforce() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let completions = client
        .completions_visualforce()
        .await
        .expect("completions_visualforce should succeed");
    let obj = completions
        .as_object()
        .expect("completions response should be a JSON object");
    assert!(!obj.is_empty(), "Should have Visualforce completions data");
}

// ============================================================================
// MetadataComponentDependency Tests (requires dependencies feature)
// ============================================================================

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_tooling_query_metadata_component_dependencies() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query all dependencies (limited to 2000 records)
    let result = client.get_metadata_component_dependencies(None).await;

    assert!(
        result.is_ok(),
        "MetadataComponentDependency query should succeed"
    );

    let deps = result.unwrap();
    // The scratch org may or may not have dependencies, so we just verify the query succeeds
    println!("Found {} metadata component dependencies", deps.len());
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_tooling_query_metadata_component_dependencies_with_filter() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query with a filter for ApexClass dependencies
    let result = client
        .get_metadata_component_dependencies(Some("MetadataComponentType = 'ApexClass'"))
        .await;

    assert!(
        result.is_ok(),
        "Filtered MetadataComponentDependency query should succeed"
    );

    let deps = result.unwrap();
    // Verify that if there are results, they match the filter
    for dep in &deps {
        if let Some(component_type) = &dep.metadata_component_type {
            assert_eq!(
                component_type, "ApexClass",
                "Filtered results should only contain ApexClass"
            );
        }
    }

    println!(
        "Found {} ApexClass metadata component dependencies",
        deps.len()
    );
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_tooling_query_metadata_component_dependencies_raw() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query using raw SOQL to test the type deserialization
    let result: Result<Vec<busbar_sf_client::MetadataComponentDependency>, _> = client
        .query_all(
            "SELECT MetadataComponentId, MetadataComponentName, MetadataComponentType, \
             RefMetadataComponentId, RefMetadataComponentName, RefMetadataComponentType \
             FROM MetadataComponentDependency LIMIT 10",
        )
        .await;

    assert!(
        result.is_ok(),
        "Raw MetadataComponentDependency query should succeed"
    );
}
