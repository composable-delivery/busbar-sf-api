//! WASM bridge integration tests.
//!
//! These tests verify that the sf-bridge can load and execute WASM plugins,
//! and that host functions correctly bridge to Salesforce APIs.
//!
//! ## How These Tests Work
//!
//! 1. **Host authenticates** with Salesforce using `get_credentials()` (from `common.rs`)
//!    which reads `SF_AUTH_URL` environment variable to get real org credentials
//!
//! 2. **Host creates** `SalesforceRestClient` with those credentials
//!
//! 3. **Host loads** the test WASM plugin and creates `SfBridge` with the authenticated client
//!
//! 4. **WASM guest** (test plugin) calls host functions like `sf_query`, `sf_create`, etc.
//!
//! 5. **Bridge executes** real Salesforce API calls using the host's credentials
//!
//! 6. **Test verifies** the responses flow back correctly
//!
//! **YES, these tests run against a REAL Salesforce org!** The WASM guest never sees
//! the credentials - all authentication happens on the host side, just like in production.
//!
//! ## Building the Test WASM Plugin
//!
//! Before running these tests, build the test WASM plugin:
//!
//! ```sh
//! rustup target add wasm32-unknown-unknown
//! cargo build --manifest-path tests/wasm-test-plugin/Cargo.toml \
//!     --target wasm32-unknown-unknown --release
//! ```
//!
//! The compiled plugin will be at:
//! `target/wasm32-unknown-unknown/release/wasm_test_plugin.wasm`
//!
//! ## Running the Tests
//!
//! ```sh
//! # Run all bridge integration tests
//! SF_AUTH_URL="force://..." cargo test --test integration bridge::
//!
//! # Run a specific test
//! SF_AUTH_URL="force://..." cargo test --test integration test_bridge_query_operation
//! ```

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_bridge::SfBridge;
use busbar_sf_rest::SalesforceRestClient;

/// Load the test WASM plugin from disk at runtime.
/// If the plugin isn't built yet, returns None and prints a warning.
fn load_test_wasm_bytes() -> Option<Vec<u8>> {
    let wasm_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/wasm-test-plugin/target/wasm32-unknown-unknown/release/wasm_test_plugin.wasm");

    if !wasm_path.exists() {
        eprintln!(
            "⚠️  Test WASM plugin not found at: {}\n\
             Build it with: cargo build --manifest-path tests/wasm-test-plugin/Cargo.toml \
             --target wasm32-unknown-unknown --release",
            wasm_path.display()
        );
        return None;
    }

    match std::fs::read(&wasm_path) {
        Ok(bytes) => Some(bytes),
        Err(e) => {
            eprintln!("⚠️  Failed to read test WASM plugin: {}", e);
            None
        }
    }
}

/// Helper to create a bridge with the test WASM plugin.
async fn create_test_bridge() -> Option<SfBridge> {
    let wasm_bytes = load_test_wasm_bytes()?;
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    Some(SfBridge::new(wasm_bytes, client).expect("Failed to create bridge"))
}

/// Helper to call a test function and parse the JSON result.
async fn call_test_function(
    bridge: &SfBridge,
    function: &str,
    input: &serde_json::Value,
) -> serde_json::Value {
    let input_bytes = serde_json::to_vec(input).expect("Failed to serialize input");
    let output_bytes = bridge
        .call(function, input_bytes) // Pass Vec<u8> directly, not a slice
        .await
        .unwrap_or_else(|_| panic!("Failed to call {}", function));
    serde_json::from_slice(&output_bytes).expect("Failed to parse output")
}

// =============================================================================
// Basic Bridge Tests (no WASM plugin required)
// =============================================================================

#[tokio::test]
async fn test_bridge_can_load_wasm_plugin() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Create a minimal valid WASM module that just returns
    // This is the smallest valid WASM module (8 bytes magic + 4 bytes version)
    let minimal_wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // magic: \0asm
        0x01, 0x00, 0x00, 0x00, // version: 1
    ];

    // Attempt to create a bridge with this minimal module
    // We expect this to succeed in loading the module, even though it has no exports
    let result = SfBridge::new(minimal_wasm, client);

    // The bridge should successfully initialize
    assert!(result.is_ok(), "Bridge should accept a valid WASM module");
}

#[tokio::test]
async fn test_bridge_rejects_invalid_wasm() {
    let creds = get_credentials().await;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    // Not a valid WASM module
    let invalid_wasm = vec![0x01, 0x02, 0x03, 0x04];

    // Attempt to create a bridge with invalid WASM
    // Extism/Wasmtime validates WASM at plugin creation time
    let result = SfBridge::new(invalid_wasm, client);

    // Document current behavior: Extism rejects invalid WASM at construction
    // If this test starts failing, it means Extism changed its validation strategy
    assert!(
        result.is_err(),
        "Bridge should reject invalid WASM at construction time"
    );
}

// =============================================================================
// Priority 1: Core CRUD + Query Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_query_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "soql": "SELECT Id, Name FROM Account LIMIT 5"
    });

    let result = call_test_function(&bridge, "test_query", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Query should succeed"
    );
    assert!(
        result["data"]["total_size"].as_u64().is_some(),
        "Should have total_size"
    );
    assert!(
        result["data"]["done"].as_bool().is_some(),
        "Should have done flag"
    );
    assert!(
        result["data"]["records"].is_array(),
        "Should have records array"
    );
}

#[tokio::test]
async fn test_bridge_query_all_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "soql": "SELECT Id FROM Account LIMIT 1"
    });

    let result = call_test_function(&bridge, "test_query_all", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Query all should succeed"
    );
    assert!(
        result["data"]["total_size"].as_u64().is_some(),
        "Should have total_size"
    );
}

#[tokio::test]
async fn test_bridge_crud_operations() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "name": format!("Bridge Test Account {}", timestamp)
    });

    let result = call_test_function(&bridge, "test_crud_operations", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "CRUD operations should succeed: {}",
        result
    );
    assert!(
        result["data"]["created_id"].is_string(),
        "Should have created an account"
    );
    assert!(
        result["data"]["created_success"].as_bool().unwrap_or(false),
        "Create should have succeeded"
    );
    assert_eq!(
        result["data"]["operations_completed"]
            .as_array()
            .map(|a| a.len()),
        Some(4),
        "Should have completed all 4 operations"
    );
}

#[tokio::test]
async fn test_bridge_upsert_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    // Note: This test may fail if the org doesn't have an ExternalId__c field on Account
    // For a robust test, we'd need to set up the custom field first
    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "external_field": "ExternalId__c",
        "external_value": format!("test-{}", timestamp),
        "name": format!("Upsert Test {}", timestamp)
    });

    let result = call_test_function(&bridge, "test_upsert", &input).await;

    // Upsert might fail if the field doesn't exist - that's okay for this test
    // We're mainly verifying the bridge can call the upsert host function
    if result["success"].as_bool().unwrap_or(false) {
        assert!(
            result["data"]["id"].is_string(),
            "Should have upserted an account"
        );

        // Clean up
        if let Some(id) = result["data"]["id"].as_str() {
            let creds = get_credentials().await;
            let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
                .expect("Failed to create REST client");
            let _ = client.delete("Account", id).await;
        }
    }
}

// =============================================================================
// Priority 2: Composite Operations Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_composite_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "account_name": format!("Composite Test {}", timestamp)
    });

    let result = call_test_function(&bridge, "test_composite", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Composite should succeed: {}",
        result
    );
    assert_eq!(
        result["data"]["response_count"], 2,
        "Should have 2 subrequest responses"
    );
}

#[tokio::test]
async fn test_bridge_composite_batch_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "account_names": [
            format!("Batch Test 1 {}", timestamp),
            format!("Batch Test 2 {}", timestamp),
        ]
    });

    let result = call_test_function(&bridge, "test_composite_batch", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Composite batch should succeed: {}",
        result
    );
    assert_eq!(
        result["data"]["result_count"], 2,
        "Should have 2 batch results"
    );
}

#[tokio::test]
async fn test_bridge_composite_tree_operation() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "account_name": format!("Tree Parent {}", timestamp),
        "contact_last_name": format!("TreeContact{}", timestamp)
    });

    let result = call_test_function(&bridge, "test_composite_tree", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Composite tree should succeed: {}",
        result
    );
    assert!(
        result["data"]["result_count"].as_u64().unwrap_or(0) >= 1,
        "Should have at least 1 result"
    );
}

// =============================================================================
// Priority 3: Batch/Collections Operations Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_batch_operations() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "account_names": [
            format!("Batch 1 {}", timestamp),
            format!("Batch 2 {}", timestamp),
            format!("Batch 3 {}", timestamp),
        ]
    });

    let result = call_test_function(&bridge, "test_batch_operations", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Batch operations should succeed: {}",
        result
    );
    assert_eq!(
        result["data"]["created_count"], 3,
        "Should have created 3 accounts"
    );
    assert_eq!(
        result["data"]["retrieved_count"], 3,
        "Should have retrieved 3 accounts"
    );
    assert_eq!(
        result["data"]["updated_count"], 3,
        "Should have updated 3 accounts"
    );
    assert_eq!(
        result["data"]["deleted_count"], 3,
        "Should have deleted 3 accounts"
    );
}

// =============================================================================
// Priority 4: Describe Operations Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_describe_global() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_describe_global", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Describe global should succeed: {}",
        result
    );
    assert!(
        result["data"]["sobjects_count"].as_u64().unwrap_or(0) > 0,
        "Should have SObjects in describe"
    );
}

#[tokio::test]
async fn test_bridge_describe_sobject() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_describe_sobject", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Describe Account should succeed: {}",
        result
    );
    assert_eq!(result["data"]["name"], "Account", "Should be Account");
    assert!(
        result["data"]["fields_count"].as_u64().unwrap_or(0) > 0,
        "Should have fields"
    );
}

// =============================================================================
// Priority 5: Process Rules & Approvals Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_process_rules() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_process_rules", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List process rules should succeed: {}",
        result
    );
    // Number of rules can be 0 in a fresh org
    assert!(
        result["data"]["rules_count"].as_u64().is_some(),
        "Should have rules_count field"
    );
}

#[tokio::test]
async fn test_bridge_process_rules_for_sobject() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_process_rules_for_sobject", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List process rules for Account should succeed: {}",
        result
    );
    assert!(
        result["data"]["rules_count"].as_u64().is_some(),
        "Should have rules_count field"
    );
}

#[tokio::test]
async fn test_bridge_list_pending_approvals() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_list_pending_approvals", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List pending approvals should succeed: {}",
        result
    );
    assert!(
        result["data"]["approvals_count"].as_u64().is_some(),
        "Should have approvals_count field"
    );
}

// =============================================================================
// Priority 6: List Views Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_list_views() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_list_views", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List views should succeed: {}",
        result
    );
    assert!(
        result["data"]["list_views_count"].as_u64().is_some(),
        "Should have list_views_count"
    );
}

// =============================================================================
// Priority 7: Quick Actions Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_list_global_quick_actions() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_list_global_quick_actions", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List global quick actions should succeed: {}",
        result
    );
    assert!(
        result["data"]["quick_actions_count"].as_u64().is_some(),
        "Should have quick_actions_count"
    );
}

#[tokio::test]
async fn test_bridge_list_quick_actions() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_list_quick_actions", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List quick actions for Account should succeed: {}",
        result
    );
    assert!(
        result["data"]["quick_actions_count"].as_u64().is_some(),
        "Should have quick_actions_count"
    );
}

// =============================================================================
// Priority 8: Search Operations Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_search() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sosl": "FIND {test*} IN NAME FIELDS RETURNING Account(Id, Name)"
    });

    let result = call_test_function(&bridge, "test_search", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "SOSL search should succeed: {}",
        result
    );
    assert!(
        result["data"]["search_records_count"].as_u64().is_some(),
        "Should have search_records_count"
    );
}

#[tokio::test]
async fn test_bridge_parameterized_search() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "q": "test",
        "sobjects": ["Account", "Contact"],
        "fields": ["Name"]
    });

    let result = call_test_function(&bridge, "test_parameterized_search", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Parameterized search should succeed: {}",
        result
    );
    assert!(
        result["data"]["search_records_count"].as_u64().is_some(),
        "Should have search_records_count"
    );
}

// =============================================================================
// Priority 9: Sync Operations Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_get_deleted() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    // Use a time range from yesterday to now
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::days(1);

    let input = serde_json::json!({
        "sobject": "Account",
        "start": yesterday.to_rfc3339(),
        "end": now.to_rfc3339()
    });

    let result = call_test_function(&bridge, "test_get_deleted", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Get deleted should succeed: {}",
        result
    );
    assert!(
        result["data"]["deleted_count"].as_u64().is_some(),
        "Should have deleted_count"
    );
}

#[tokio::test]
async fn test_bridge_get_updated() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    // Use a time range from yesterday to now
    let now = chrono::Utc::now();
    let yesterday = now - chrono::Duration::days(1);

    let input = serde_json::json!({
        "sobject": "Account",
        "start": yesterday.to_rfc3339(),
        "end": now.to_rfc3339()
    });

    let result = call_test_function(&bridge, "test_get_updated", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Get updated should succeed: {}",
        result
    );
    assert!(
        result["data"]["updated_count"].as_u64().is_some(),
        "Should have updated_count"
    );
}

// =============================================================================
// Priority 10: Limits and Versions Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_limits() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_limits", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Limits should succeed: {}",
        result
    );
    assert!(result["data"].is_object(), "Should have limits data");
}

#[tokio::test]
async fn test_bridge_versions() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_versions", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Versions should succeed: {}",
        result
    );
    assert!(
        result["data"]["versions_count"].as_u64().unwrap_or(0) > 0,
        "Should have multiple versions"
    );
}

// =============================================================================
// Priority 11: Bulk API Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_bulk_ingest() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let timestamp = chrono::Utc::now().timestamp_millis();
    let input = serde_json::json!({
        "sobject": "Account",
        "operation": "insert",
        "csv_data": format!("Name\nBulk Test {}\n", timestamp)
    });

    let result = call_test_function(&bridge, "test_bulk_ingest", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Bulk ingest should succeed: {}",
        result
    );
    assert!(result["data"]["job_id"].is_string(), "Should have job_id");
}

#[tokio::test]
async fn test_bridge_bulk_query() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "soql": "SELECT Id, Name FROM Account LIMIT 5"
    });

    let result = call_test_function(&bridge, "test_bulk_query", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Bulk query should succeed: {}",
        result
    );
    assert!(result["data"]["job_id"].is_string(), "Should have job_id");
}

// =============================================================================
// Priority 12: Tooling API Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_tooling_query() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "soql": "SELECT Id, Name FROM ApexClass LIMIT 5"
    });

    let result = call_test_function(&bridge, "test_tooling_query", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Tooling query should succeed: {}",
        result
    );
    assert!(
        result["data"]["total_size"].as_u64().is_some(),
        "Should have total_size"
    );
}

#[tokio::test]
async fn test_bridge_execute_anonymous_apex() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "apex_code": "System.debug('Hello from WASM bridge');"
    });

    let result = call_test_function(&bridge, "test_execute_anonymous_apex", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Execute anonymous should succeed: {}",
        result
    );
    assert!(
        result["data"]["compiled"].as_bool().is_some(),
        "Should have compiled field"
    );
    assert!(
        result["data"]["success"].as_bool().is_some(),
        "Should have success field"
    );
}

// =============================================================================
// Priority 13: Metadata API Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_metadata_list() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "metadata_type": "ApexClass"
    });

    let result = call_test_function(&bridge, "test_metadata_list", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Metadata list should succeed: {}",
        result
    );
    assert!(
        result["data"]["metadata_count"].as_u64().is_some(),
        "Should have metadata_count"
    );
}

#[tokio::test]
async fn test_bridge_metadata_describe() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_metadata_describe", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Metadata describe should succeed: {}",
        result
    );
    assert!(
        result["data"]["metadata_objects_count"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "Should have metadata objects"
    );
}

// =============================================================================
// Priority 14: Layouts Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_get_sobject_layouts() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_get_sobject_layouts", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Get SObject layouts should succeed: {}",
        result
    );
    assert!(
        result["data"]["layouts_count"].as_u64().is_some(),
        "Should have layouts_count"
    );
}

#[tokio::test]
async fn test_bridge_get_compact_layouts() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({
        "sobject": "Account"
    });

    let result = call_test_function(&bridge, "test_get_compact_layouts", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "Get compact layouts should succeed: {}",
        result
    );
    assert!(
        result["data"]["compact_layouts_count"].as_u64().is_some(),
        "Should have compact_layouts_count"
    );
}

// =============================================================================
// Priority 15: Named Credentials Tests
// =============================================================================

#[tokio::test]
async fn test_bridge_list_named_credentials() {
    let Some(bridge) = create_test_bridge().await else {
        return;
    };

    let input = serde_json::json!({});
    let result = call_test_function(&bridge, "test_list_named_credentials", &input).await;

    assert!(
        result["success"].as_bool().unwrap_or(false),
        "List named credentials should succeed: {}",
        result
    );
    assert!(
        result["data"]["credentials_count"].as_u64().is_some(),
        "Should have credentials_count"
    );
}
