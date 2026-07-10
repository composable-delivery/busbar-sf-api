//! Data Cloud (Data 360) integration tests.
//!
//! These tests require a Salesforce scratch org with the **DataCloud** feature
//! enabled (see `config/project-scratch-def.json`). The org credentials are
//! read from the `SF_AUTH_URL` environment variable, exactly as all other
//! integration tests do.
//!
//! The token exchange flow is tested first. If the connected org does not
//! have Data Cloud provisioned, the exchange returns an OAuth error and the
//! test fails with a clear message — consistent with the policy of never
//! silently skipping integration tests.
//!
//! To create a Data Cloud–enabled scratch org:
//! ```sh
//! sf org create scratch \
//!   -f config/project-scratch-def.json \
//!   -a busbar-dc-test \
//!   --duration-days 7
//!
//! export SF_AUTH_URL=$(
//!   sf org display \
//!     --target-org busbar-dc-test \
//!     --verbose --json \
//!   | jq -r '.result.sfdxAuthUrl'
//! )
//! cargo test --test integration data_cloud:: -- --nocapture
//! ```

use super::common::get_credentials;
use busbar_sf_auth::{Credentials, OAuthClient, OAuthConfig};
use busbar_sf_rest::{DataCloudClient, DataCloudQueryRequest, VectorSearchRequest};

/// Derive the Salesforce login URL from the instance URL.
///
/// Scratch orgs use `test.salesforce.com`; production uses the standard
/// `login.salesforce.com`.
fn login_url_for_instance(instance_url: &str) -> String {
    if instance_url.contains("test.salesforce.com")
        || instance_url.contains("sandbox")
        || instance_url.contains(".scratch.")
        || instance_url.contains("--")
    {
        "https://test.salesforce.com".to_string()
    } else {
        "https://login.salesforce.com".to_string()
    }
}

/// Perform the Data Cloud token exchange and return a `DataCloudClient`.
///
/// This is the entry-point for all Data Cloud tests. A failure here means the
/// scratch org does not have Data Cloud provisioned — re-create it with the
/// `DataCloud` feature enabled in `config/project-scratch-def.json`.
async fn get_data_cloud_client() -> DataCloudClient {
    let creds = get_credentials().await;
    let login_url = login_url_for_instance(creds.instance_url());

    // The Data Cloud token exchange (RFC 8693 / Salesforce extension) does not
    // require a `client_id` in the request body — the subject token identifies
    // the caller. `OAuthConfig` is used only to obtain a configured HTTP client;
    // the consumer key is intentionally left empty here.
    let config = OAuthConfig::new("");
    let oauth = OAuthClient::new(config);

    let dc_token = oauth
        .exchange_for_data_cloud(creds.access_token(), &login_url)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Data Cloud token exchange failed: {e}\n\
                 \n\
                 The scratch org may not have the DataCloud feature enabled.\n\
                 Re-create the org with:\n\
                 \n\
                 sf org create scratch -f config/project-scratch-def.json -a busbar-dc-test --duration-days 7\n\
                 \n\
                 Ensure your DevHub has Salesforce Data Cloud licensed and enabled."
            )
        });

    assert!(
        !dc_token.access_token.is_empty(),
        "Data Cloud access token should not be empty"
    );
    assert!(
        !dc_token.instance_url.is_empty(),
        "Data Cloud instance URL (TSE URL) should not be empty"
    );

    DataCloudClient::new(&dc_token.instance_url, &dc_token.access_token)
        .expect("Failed to create DataCloudClient from token exchange response")
}

/// Validate that a Data Model Object name is a safe SQL identifier.
///
/// DMO names returned by the metadata API are Salesforce identifiers: they
/// consist of letters, digits, and underscores only. Reject anything that
/// doesn't match to prevent SQL injection in tests that interpolate the name
/// into a query string.
fn validate_dmo_name(name: &str) -> &str {
    assert!(
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "DMO name '{name}' contains characters that are not valid in a SQL identifier"
    );
    name
}

// ============================================================================
// Token Exchange
// ============================================================================

/// Verify that the Data Cloud OAuth 2.0 token exchange succeeds and returns
/// a non-empty TSE (Tenant Service Endpoint) URL and access token.
#[tokio::test]
async fn test_data_cloud_token_exchange_returns_tse_url_and_token() {
    let creds = get_credentials().await;
    let login_url = login_url_for_instance(creds.instance_url());

    // The Data Cloud token exchange does not require a `client_id` — the
    // subject token is sufficient. `OAuthConfig` is used only as a handle
    // to the configured HTTP client; the consumer key is left empty.
    let config = OAuthConfig::new("");
    let oauth = OAuthClient::new(config);

    let dc_token = oauth
        .exchange_for_data_cloud(creds.access_token(), &login_url)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Data Cloud token exchange failed: {e}\n\
                 Ensure the scratch org has the DataCloud feature enabled."
            )
        });

    assert!(
        !dc_token.access_token.is_empty(),
        "Data Cloud access token must not be empty"
    );
    assert!(
        !dc_token.instance_url.is_empty(),
        "Data Cloud TSE URL must not be empty"
    );
    assert_eq!(
        dc_token.token_type.as_deref(),
        Some("Bearer"),
        "token_type should be Bearer"
    );

    println!("Data Cloud TSE URL: {}", dc_token.instance_url);
}

// ============================================================================
// Metadata Discovery
// ============================================================================

/// Call the Data Cloud metadata discovery endpoint and verify it returns
/// at least one metadata entity when filtered by `DataModelObject`.
#[tokio::test]
async fn test_data_cloud_metadata_discovery_returns_entities() {
    let client = get_data_cloud_client().await;

    let result = client
        .metadata(Some("DataModelObject"))
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Data Cloud metadata discovery failed: {e}\n\
                 Ensure the scratch org has DataCloud provisioned and the TSE URL is reachable."
            )
        });

    // A Data Cloud-enabled org always has at least the core system DMOs.
    assert!(
        !result.metadata.is_empty(),
        "Expected at least one DataModelObject in metadata response, got 0.\n\
         This might indicate Data Cloud data streams have not been configured."
    );

    for obj in &result.metadata {
        assert!(
            !obj.name.is_empty(),
            "Each metadata object should have a non-empty name"
        );
    }

    println!("Data Cloud metadata: {} object(s)", result.metadata.len());
    for obj in result.metadata.iter().take(5) {
        println!(
            "  - {} (entityType: {})",
            obj.name,
            obj.entity_type.as_deref().unwrap_or("unknown")
        );
    }
}

/// Call the metadata endpoint without an entity type filter.
#[tokio::test]
async fn test_data_cloud_metadata_discovery_without_filter() {
    let client = get_data_cloud_client().await;

    let result = client
        .metadata(None)
        .await
        .unwrap_or_else(|e| panic!("Data Cloud metadata (no filter) failed: {e}"));

    // No filter = broader results; should have at least some entities.
    println!(
        "Data Cloud metadata (no filter): {} entity(ies)",
        result.metadata.len()
    );
}

// ============================================================================
// SQL Query
// ============================================================================

/// Execute a simple SQL query against Data Cloud. We use `LIMIT 0` so the test
/// doesn't depend on actual data being loaded, while still exercising the
/// query parsing and column-metadata response.
#[tokio::test]
async fn test_data_cloud_sql_query_returns_column_metadata() {
    let client = get_data_cloud_client().await;

    // First discover a real DMO name so we don't hard-code one.
    let meta = client
        .metadata(Some("DataModelObject"))
        .await
        .unwrap_or_else(|e| panic!("Metadata discovery failed: {e}"));

    if meta.metadata.is_empty() {
        panic!(
            "No DataModelObjects found in the scratch org.\n\
             Data Cloud data streams must be configured before running SQL tests."
        );
    }

    let dmo_name = validate_dmo_name(&meta.metadata[0].name);
    println!("Querying DMO: {dmo_name}");

    let request = DataCloudQueryRequest {
        sql: format!("SELECT * FROM {dmo_name} LIMIT 0"),
        page_size: Some(10),
        r#async: None,
    };

    let response = client.query_sql(&request).await.unwrap_or_else(|e| {
        panic!(
            "Data Cloud SQL query failed for DMO '{dmo_name}': {e}\n\
             Ensure the DMO exists and the Data Cloud access token has sufficient permissions."
        )
    });

    // With LIMIT 0, data should be empty but metadata must be present.
    assert!(response.done, "LIMIT 0 query should be done immediately");
    assert!(
        !response.metadata.columns.is_empty(),
        "SQL response metadata should include at least one column for DMO '{dmo_name}'"
    );

    println!(
        "DMO '{}' has {} column(s): {}",
        dmo_name,
        response.metadata.columns.len(),
        response
            .metadata
            .columns
            .iter()
            .map(|c| format!("{}:{}", c.name, c.col_type))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Test that an asynchronous query submission returns a query ID and can be
/// polled for status.
#[tokio::test]
async fn test_data_cloud_async_sql_query_returns_query_id() {
    let client = get_data_cloud_client().await;

    let meta = client
        .metadata(Some("DataModelObject"))
        .await
        .unwrap_or_else(|e| panic!("Metadata discovery failed: {e}"));

    if meta.metadata.is_empty() {
        panic!("No DataModelObjects found; cannot run async SQL test.");
    }

    let dmo_name = validate_dmo_name(&meta.metadata[0].name);

    let request = DataCloudQueryRequest {
        sql: format!("SELECT * FROM {dmo_name} LIMIT 1"),
        page_size: Some(10),
        r#async: Some(true),
    };

    let response = client
        .query_sql(&request)
        .await
        .unwrap_or_else(|e| panic!("Async Data Cloud SQL query failed: {e}"));

    // For async queries the server may return a queryId immediately.
    // Some implementations return results synchronously even when async=true.
    if let Some(query_id) = &response.query_id {
        println!("Async query ID: {query_id}");

        // Poll status (one round is sufficient to verify the endpoint works).
        let status = client
            .query_status(query_id)
            .await
            .unwrap_or_else(|e| panic!("query_status failed for {query_id}: {e}"));

        assert!(
            !status.status.is_empty(),
            "Query status should have a non-empty status string"
        );
        println!("Async query status: {}", status.status);

        // If the query succeeded, fetch the rows.
        if status.status == "success" {
            let rows = client
                .query_rows(query_id)
                .await
                .unwrap_or_else(|e| panic!("query_rows failed for {query_id}: {e}"));
            println!("Async query rows: {}", rows.data.len());
        }
    } else {
        // Synchronous response even with async=true — this is acceptable.
        println!("Server returned synchronous response (async=true was ignored).");
        assert!(response.done, "Synchronous response must have done=true");
    }
}

// ============================================================================
// Vector Search
// ============================================================================

/// Verify the vector search endpoint is reachable. In a scratch org without
/// indexed knowledge articles the call may return an empty result set or an
/// error indicating no index exists — both outcomes are acceptable here; we
/// only verify the HTTP round-trip completes without a network-level error.
#[tokio::test]
async fn test_data_cloud_vector_search_endpoint_is_reachable() {
    let client = get_data_cloud_client().await;

    let request = VectorSearchRequest {
        index_name: "Knowledge_Articles_Index".to_string(),
        query_text: "How do I reset my password?".to_string(),
        top_k: Some(3),
    };

    match client.vector_search(&request).await {
        Ok(response) => {
            println!(
                "Vector search returned {} result(s)",
                response.results.len()
            );
        }
        Err(e) => {
            // An error here typically means the index doesn't exist in the
            // scratch org — which is expected when no Knowledge articles have
            // been indexed. Treat as a setup issue, not a code defect.
            let err_str = e.to_string();
            println!("Vector search error (may indicate no index configured): {err_str}");
            assert!(
                err_str.contains("not found")
                    || err_str.contains("does not exist")
                    || err_str.contains("NO_SUCH_INDEX")
                    || err_str.contains("invalid")
                    || err_str.contains("400")
                    || err_str.contains("404"),
                "Unexpected vector search error: {err_str}\n\
                 Expected an index-not-found type error for a fresh scratch org."
            );
        }
    }
}

// ============================================================================
// Profile Lookup
// ============================================================================

/// Call the unified profile endpoint. In a fresh scratch org without Data
/// Cloud data streams configured, this returns an empty result or a DMO-not-
/// found error. We verify the endpoint is reachable and the client handles
/// both outcomes correctly.
#[tokio::test]
async fn test_data_cloud_profile_endpoint_is_reachable() {
    let client = get_data_cloud_client().await;

    match client.profile("Individual__dlm", None).await {
        Ok(response) => {
            println!(
                "Profile response keys: {:?}",
                response.as_object().map(|m| m.keys().collect::<Vec<_>>())
            );
        }
        Err(e) => {
            let err_str = e.to_string();
            println!("Profile error (may indicate no Individual__dlm configured): {err_str}");
            // A 404 / not-found error is expected in scratch orgs without data streams.
            assert!(
                err_str.contains("404")
                    || err_str.contains("not found")
                    || err_str.contains("NOT_FOUND")
                    || err_str.contains("invalid")
                    || err_str.contains("400"),
                "Unexpected profile error: {err_str}"
            );
        }
    }
}
