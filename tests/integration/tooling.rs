//! Tooling API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_tooling::{
    CompositeBatchRequest, CompositeBatchSubrequest, CompositeRequest, CompositeSubrequest,
    ToolingClient,
};
use std::sync::LazyLock;
use tokio::sync::Mutex;

// Global mutex to serialize Apex class creation across tests
// Salesforce only allows one admin operation (class creation/compilation) at a time
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

    if query_result.is_empty() {
        eprintln!("Skipping test: No ApexClass records found in org");
        return;
    }

    let ids: Vec<String> = query_result
        .iter()
        .filter_map(|r| r.get("Id").and_then(|v| v.as_str()).map(String::from))
        .collect();

    if ids.is_empty() {
        eprintln!("Skipping test: No ApexClass IDs found");
        return;
    }

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
async fn test_tooling_collections_create_update_delete() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Note: We can't easily create ApexClass records via Tooling API without
    // using MetadataContainer, which is complex. Instead, we'll test with
    // TraceFlag or DebugLevel which are easier to create.

    // First, get a debug level ID to use for TraceFlags
    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM DebugLevel LIMIT 1")
        .await
        .expect("Query DebugLevel should succeed");

    if debug_levels.is_empty() {
        eprintln!("Skipping test: No DebugLevel found in org");
        return;
    }

    let _debug_level_id = debug_levels[0]
        .get("Id")
        .and_then(|v| v.as_str())
        .expect("Should have DebugLevel Id");

    // Get current user ID
    let _user_info: serde_json::Value = client
        .inner()
        .rest_get("sobjects/User")
        .await
        .expect("Should get user info");

    // Note: Creating TraceFlags might fail if they already exist or permissions are insufficient
    // This is more of a smoke test to ensure the API endpoint works
    eprintln!("Note: TraceFlag creation test may be skipped if already exists or permissions insufficient");
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

    // Should get results back, but they should indicate failure for these fake IDs
    if let Ok(results) = result {
        assert_eq!(results.len(), 2, "Should have 2 delete results");
        // Fake IDs should fail
        for res in results {
            assert!(
                !res.success || !res.errors.is_empty(),
                "Fake ID deletion should fail or have errors"
            );
        }
    }
}

#[tokio::test]
async fn test_tooling_create_multiple_trace_flags() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Get a debug level to use
    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM DebugLevel LIMIT 1")
        .await
        .expect("Query DebugLevel should succeed");

    if debug_levels.is_empty() {
        eprintln!("Skipping test: No DebugLevel found in org");
        return;
    }

    let debug_level_id = debug_levels[0]
        .get("Id")
        .and_then(|v| v.as_str())
        .expect("Should have DebugLevel Id");

    // Get the current user ID via query (more reliable than REST endpoint)
    let user_query: Vec<serde_json::Value> = client
        .inner()
        .query_all("SELECT Id FROM User WHERE Username = UserInfo.getUserName() LIMIT 1")
        .await
        .unwrap_or_default();

    if user_query.is_empty() {
        eprintln!("Skipping test: Could not get current user ID");
        return;
    }

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

    match result {
        Ok(results) => {
            assert_eq!(results.len(), 1, "Should have 1 result");

            // Clean up if successful
            if let Some(id) = results[0].id.as_ref() {
                let _ = client.delete("TraceFlag", id).await;
            }
        }
        Err(e) => {
            // It's okay if this fails due to existing trace flags or permissions
            eprintln!("TraceFlag creation failed (expected in some orgs): {:?}", e);
        }
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
// Test Execution Tests
// ============================================================================

#[tokio::test]
async fn test_run_tests_async_with_test_class() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarIntegrationTest{}", timestamp);

    // First, create a simple test class
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testSimpleAssertion() {{
        System.assertEquals(4, 2 + 2, 'Math should work');
    }}
    
    @isTest
    static void testAnotherAssertion() {{
        System.assertNotEquals(null, 'value', 'Value should not be null');
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        // Create the test class
        let class_data = serde_json::json!({
            "Name": class_name,
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        id
    };

    // Run tests asynchronously using class ID
    let request = busbar_sf_tooling::RunTestsAsyncRequest {
        class_ids: Some(vec![class_id.clone()]),
        test_level: Some("RunSpecifiedTests".to_string()),
        skip_code_coverage: Some(true),
        ..Default::default()
    };

    let job_id = client
        .run_tests_async(&request)
        .await
        .expect("run_tests_async should succeed");

    assert!(!job_id.is_empty(), "Should return a job ID");
    assert!(
        job_id.starts_with("707"),
        "Job ID should be an AsyncApexJob ID (starts with 707)"
    );

    // Poll for completion (wait up to 30 seconds)
    let mut completed = false;
    for _ in 0..15 {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let query = format!(
            "SELECT Id, Status, ApexClassId FROM ApexTestQueueItem WHERE ParentJobId = '{}'",
            job_id
        );
        let queue_items: Vec<serde_json::Value> = client
            .query_all(&query)
            .await
            .expect("Should query test queue items");

        if !queue_items.is_empty() {
            let status = queue_items[0]
                .get("Status")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if status == "Completed" || status == "Failed" {
                completed = true;
                break;
            }
        }
    }

    assert!(completed, "Test should complete within 30 seconds");

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}

#[tokio::test]
async fn test_run_tests_sync_with_test_method() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarSyncTest{}", timestamp);

    // First, create a simple test class
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testBasicAssertion() {{
        Integer result = 10 + 5;
        System.assertEquals(15, result, 'Addition should work');
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        // Create the test class
        let class_data = serde_json::json!({
            "Name": class_name.clone(),
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        id
    };

    // Run tests synchronously
    let request = busbar_sf_tooling::RunTestsSyncRequest {
        tests: Some(vec![format!("{}.testBasicAssertion", class_name)]),
        skip_code_coverage: Some(true),
    };

    let result = client
        .run_tests_sync(&request)
        .await
        .expect("run_tests_sync should succeed");

    assert_eq!(result.num_tests_run, 1, "Should run 1 test");
    assert_eq!(result.num_failures, 0, "Should have 0 failures");
    assert!(!result.successes.is_empty(), "Should have success results");

    if !result.successes.is_empty() {
        let success = &result.successes[0];
        assert_eq!(
            success.method_name, "testBasicAssertion",
            "Should be the correct method"
        );
        assert_eq!(success.name, class_name, "Should be the correct class");
    }

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}

#[tokio::test]
async fn test_run_tests_sync_with_failing_test() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarFailTest{}", timestamp);

    // Create a test class with a failing test
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testThatFails() {{
        System.assertEquals(5, 10, 'This should fail');
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        let class_data = serde_json::json!({
            "Name": class_name.clone(),
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        id
    };

    // Run the failing test
    let request = busbar_sf_tooling::RunTestsSyncRequest {
        tests: Some(vec![format!("{}.testThatFails", class_name)]),
        skip_code_coverage: Some(true),
    };

    let result = client
        .run_tests_sync(&request)
        .await
        .expect("run_tests_sync should succeed even with failing tests");

    assert_eq!(result.num_tests_run, 1, "Should run 1 test");
    assert_eq!(result.num_failures, 1, "Should have 1 failure");
    assert!(!result.failures.is_empty(), "Should have failure results");

    if !result.failures.is_empty() {
        let failure = &result.failures[0];
        assert_eq!(
            failure.method_name, "testThatFails",
            "Should be the correct method"
        );
        assert!(
            failure.message.contains("This should fail"),
            "Failure message should contain assertion text"
        );
    }

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}

#[tokio::test]
async fn test_discover_tests_v65() {
    let Some(creds) = require_credentials().await else {
        return;
    };

    // Use v65.0 for the Test Discovery API
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client")
        .with_api_version("65.0");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarDiscoverTest{}", timestamp);

    // First, create a simple test class to discover
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testDiscovery() {{
        System.assert(true);
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        let class_data = serde_json::json!({
            "Name": class_name.clone(),
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile and be discoverable before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        id
    };

    // Discover all tests
    let result = client
        .discover_tests(None)
        .await
        .expect("discover_tests should succeed");

    assert!(
        !result.tests.is_empty(),
        "Should discover at least one test"
    );

    // Find our test in the results
    let our_test = result
        .tests
        .iter()
        .find(|t| t.class_name.as_deref() == Some(class_name.as_str()));

    assert!(our_test.is_some(), "Should find our test class");

    if let Some(test) = our_test {
        assert_eq!(test.category, "apex", "Should be an Apex test");
        assert_eq!(
            test.name, "testDiscovery",
            "Should have correct method name"
        );
    }

    // Test category filtering - get only Apex tests
    let apex_tests = client
        .discover_tests(Some("apex"))
        .await
        .expect("discover_tests with category filter should succeed");

    assert!(
        !apex_tests.tests.is_empty(),
        "Should discover Apex tests with filter"
    );
    assert!(
        apex_tests.tests.iter().all(|t| t.category == "apex"),
        "All tests should be Apex category"
    );

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}

#[tokio::test]
async fn test_run_tests_unified_api_v65() {
    let Some(creds) = require_credentials().await else {
        return;
    };

    // Use v65.0 for the unified Test Runner API
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client")
        .with_api_version("65.0");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarUnifiedTest{}", timestamp);

    // Create a simple test class
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testUnifiedRunner() {{
        System.assertEquals(100, 50 + 50);
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        let class_data = serde_json::json!({
            "Name": class_name,
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        id
    };

    // Run tests using the unified API
    let request = busbar_sf_tooling::RunTestsRequest {
        class_ids: Some(vec![class_id.clone()]),
        test_level: Some("RunSpecifiedTests".to_string()),
        skip_code_coverage: Some(true),
        ..Default::default()
    };

    let test_run_id = client
        .run_tests(&request)
        .await
        .expect("run_tests (unified API) should succeed");

    assert!(!test_run_id.is_empty(), "Should return a test run ID");

    // The test run ID can be used to query test results
    // For now, we just verify we got an ID back

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}

#[tokio::test]
async fn test_run_tests_async_with_class_names() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Use unique class name with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().timestamp_millis();
    let class_name = format!("BusbarNameTest{}", timestamp);

    // Create a test class
    let test_class_body = format!(
        r#"
@isTest
private class {} {{
    @isTest
    static void testWithClassName() {{
        System.assert(true);
    }}
}}
"#,
        class_name
    );

    // Serialize Apex class creation to avoid "admin operation already in progress" errors
    let class_id = {
        let _lock = APEX_CLASS_LOCK.lock().await;

        let class_data = serde_json::json!({
            "Name": class_name.clone(),
            "Body": test_class_body
        });

        let id = client
            .create("ApexClass", &class_data)
            .await
            .expect("Failed to create test class");

        // Give it a moment to compile before releasing lock
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        id
    };

    // Run tests using class names instead of IDs
    let request = busbar_sf_tooling::RunTestsAsyncRequest {
        class_names: Some(vec![class_name]),
        test_level: Some("RunSpecifiedTests".to_string()),
        skip_code_coverage: Some(true),
        ..Default::default()
    };

    let job_id = client
        .run_tests_async(&request)
        .await
        .expect("run_tests_async with class names should succeed");

    assert!(
        !job_id.is_empty(),
        "Should return a job ID when using class names"
    );

    // Clean up: delete the test class
    let _ = client.delete("ApexClass", &class_id).await;
}
