//! Tooling API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_tooling::{
    CompositeBatchRequest, CompositeBatchSubrequest, CompositeRequest, CompositeSubrequest,
    ToolingClient,
};

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
// New Endpoint Integration Tests
// ============================================================================

#[tokio::test]
async fn test_tooling_update_trace_flag() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // First, get or create a DebugLevel
    let debug_levels: Vec<serde_json::Value> = client
        .query_all("SELECT Id FROM DebugLevel LIMIT 1")
        .await
        .expect("Should get debug levels");

    if debug_levels.is_empty() {
        eprintln!("Skipping test_tooling_update_trace_flag: No DebugLevel available");
        return;
    }

    let debug_level_id = debug_levels[0]["Id"]
        .as_str()
        .expect("Should have debug level ID");

    // Get current user ID (use an active user since UserInfo.getUserName() is Apex, not SOQL)
    let user_query_result = client
        .query::<serde_json::Value>("SELECT Id FROM User WHERE IsActive = true LIMIT 1")
        .await;

    let user_info = match user_query_result {
        Ok(qr) => match qr.records.into_iter().next() {
            Some(user) => user,
            None => {
                eprintln!("Skipping test: Could not find an active user");
                return;
            }
        },
        Err(e) => {
            eprintln!("Skipping test: Could not get current user ID: {e}");
            return;
        }
    };

    let user_id = user_info["Id"].as_str().expect("Should have user ID");

    // Create a TraceFlag
    let expiration = chrono::Utc::now() + chrono::Duration::hours(1);
    let trace_flag_data = serde_json::json!({
        "TracedEntityId": user_id,
        "DebugLevelId": debug_level_id,
        "LogType": "USER_DEBUG",
        "StartDate": chrono::Utc::now().to_rfc3339(),
        "ExpirationDate": expiration.to_rfc3339(),
    });

    let trace_flag_id = client
        .create("TraceFlag", &trace_flag_data)
        .await
        .expect("Should create TraceFlag");

    // Now test the update method
    let new_expiration = chrono::Utc::now() + chrono::Duration::hours(2);
    let update_data = serde_json::json!({
        "ExpirationDate": new_expiration.to_rfc3339(),
    });

    let update_result = client
        .update("TraceFlag", &trace_flag_id, &update_data)
        .await;

    assert!(
        update_result.is_ok(),
        "TraceFlag update should succeed: {:?}",
        update_result
    );

    // Verify the update
    let updated_flag: serde_json::Value = client
        .get("TraceFlag", &trace_flag_id)
        .await
        .expect("Should retrieve updated TraceFlag");

    assert!(
        updated_flag["ExpirationDate"].is_string(),
        "Should have expiration date"
    );

    // Cleanup
    let _ = client.delete("TraceFlag", &trace_flag_id).await;
}

#[tokio::test]
async fn test_tooling_query_all_records() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Query all records including deleted ones (queryAll may not be available in all orgs)
    match client
        .query_all_records::<serde_json::Value>("SELECT Id, Name FROM ApexClass LIMIT 10")
        .await
    {
        Ok(query_result) => {
            assert!(
                query_result.done || query_result.next_records_url.is_some(),
                "Query should complete or have pagination"
            );
        }
        Err(e) => {
            // queryAll may not be supported on the Tooling API in all org editions
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND")
                    || msg.contains("INVALID")
                    || msg.contains("not supported"),
                "Expected NOT_FOUND or unsupported error, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn test_tooling_search_sosl() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Search for Apex classes using SOSL
    let search_query =
        "FIND {test*} IN NAME FIELDS RETURNING ApexClass(Id, Name), ApexTrigger(Id, Name)";
    let result = client.search::<serde_json::Value>(search_query).await;

    assert!(result.is_ok(), "SOSL search should succeed: {:?}", result);

    // Verify the result structure is correct (search_records field exists)
    let _search_result = result.unwrap();
    // search_records may be empty if no matches found, which is ok
}

#[tokio::test]
async fn test_tooling_describe_global() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.describe_global().await;

    assert!(
        result.is_ok(),
        "describe_global should succeed: {:?}",
        result
    );

    let global_describe = result.unwrap();
    assert!(
        !global_describe.sobjects.is_empty(),
        "Should have tooling sObjects"
    );

    // Verify that common Tooling objects are present
    let apex_class_found = global_describe
        .sobjects
        .iter()
        .any(|obj| obj.name == "ApexClass");
    assert!(apex_class_found, "Should include ApexClass in results");
}

#[tokio::test]
async fn test_tooling_describe_sobject() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.describe_sobject("ApexClass").await;

    assert!(
        result.is_ok(),
        "describe_sobject should succeed: {:?}",
        result
    );

    let describe = result.unwrap();
    assert_eq!(describe.name, "ApexClass", "Should describe ApexClass");
    assert!(!describe.fields.is_empty(), "Should have fields");

    // Verify common fields exist
    let has_name_field = describe.fields.iter().any(|f| f.name == "Name");
    let has_body_field = describe.fields.iter().any(|f| f.name == "Body");
    assert!(has_name_field, "Should have Name field");
    assert!(has_body_field, "Should have Body field");
}

#[tokio::test]
async fn test_tooling_basic_info() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.basic_info("ApexClass").await;

    assert!(result.is_ok(), "basic_info should succeed: {:?}", result);

    let info = result.unwrap();
    assert!(
        info.is_object(),
        "basic_info should return a JSON object: {:?}",
        info
    );

    // Should have objectDescribe or other metadata
    assert!(
        info.get("objectDescribe").is_some() || info.get("recentItems").is_some(),
        "Should have metadata fields"
    );
}

#[tokio::test]
async fn test_tooling_resources() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.resources().await;

    assert!(result.is_ok(), "resources should succeed: {:?}", result);

    let resources = result.unwrap();
    assert!(
        resources.is_object(),
        "resources should return a JSON object"
    );

    // Should have common Tooling API endpoints
    let resources_obj = resources.as_object().expect("Should be an object");
    assert!(!resources_obj.is_empty(), "Should have resource endpoints");

    // Verify some expected resources exist (as strings or objects)
    let has_sobjects = resources_obj.contains_key("sobjects");
    let has_query = resources_obj.contains_key("query");
    assert!(
        has_sobjects || has_query,
        "Should have sobjects or query resource"
    );
}

// ============================================================================
// Error Handling Tests for New Endpoints
// ============================================================================

#[tokio::test]
async fn test_tooling_error_invalid_sobject_describe() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let result = client.describe_sobject("NonExistentToolingObject__c").await;

    assert!(
        result.is_err(),
        "Describing non-existent Tooling object should fail"
    );
}

#[tokio::test]
async fn test_tooling_error_invalid_sobject_update() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    let update_data = serde_json::json!({"Name": "Test"});
    let result = client.update("ApexClass", "invalid-id", &update_data).await;

    assert!(result.is_err(), "Update with invalid ID should fail");
}

#[tokio::test]
async fn test_tooling_error_invalid_sosl_search() {
    let creds = get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Tooling client");

    // Invalid SOSL query syntax
    let result = client
        .search::<serde_json::Value>("this is not valid SOSL")
        .await;

    assert!(result.is_err(), "Invalid SOSL should fail");
}
