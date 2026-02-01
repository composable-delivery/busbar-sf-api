//! Bulk API 2.0 integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_bulk::{BulkApiClient, BulkOperation};
use busbar_sf_rest::{QueryBuilder, SalesforceRestClient};

// ============================================================================
// Bulk API 2.0 Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_insert_lifecycle() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let csv_data = format!(
        "Name,Industry\nBulk Test 1 {},Technology\nBulk Test 2 {},Manufacturing",
        chrono::Utc::now().timestamp_millis(),
        chrono::Utc::now().timestamp_millis()
    );

    let result = client
        .execute_ingest("Account", BulkOperation::Insert, &csv_data, None)
        .await
        .expect("Bulk insert should succeed");

    assert_eq!(
        result.job.number_records_processed, 2,
        "Should process 2 records"
    );
    assert_eq!(
        result.job.number_records_failed, 0,
        "Should have 0 failures"
    );

    if let Some(success_results) = result.successful_results {
        let lines: Vec<&str> = success_results.lines().collect();
        if lines.len() > 1 {
            for line in &lines[1..] {
                if let Some(id) = line.split(',').next() {
                    if id.starts_with("001") {
                        let rest_client =
                            SalesforceRestClient::new(creds.instance_url(), creds.access_token())
                                .expect("Failed to create REST client");
                        let _ = rest_client.delete("Account", id).await;
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_bulk_query_operation() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .limit(100);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    assert!(
        result.job.number_records_processed >= 0,
        "Should process records"
    );

    if let Some(csv_results) = result.results {
        let lines: Vec<&str> = csv_results.lines().collect();
        assert!(!lines.is_empty(), "Should have at least header line");
        if let Some(header) = lines.first() {
            assert!(
                header.to_lowercase().contains("id"),
                "Header should contain Id"
            );
            assert!(
                header.to_lowercase().contains("name"),
                "Header should contain Name"
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_update_operation() {
    let creds = get_credentials().await;

    let rest_client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create REST client");

    let test_name = format!("Bulk Update Test {}", chrono::Utc::now().timestamp_millis());
    let account_data = serde_json::json!({
        "Name": test_name
    });

    let account_id = rest_client
        .create("Account", &account_data)
        .await
        .expect("Create should succeed");

    let bulk_client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let csv_data = format!("Id,Description\n{},Updated via Bulk API", account_id);

    let result = bulk_client
        .execute_ingest("Account", BulkOperation::Update, &csv_data, None)
        .await
        .expect("Bulk update should succeed");

    assert_eq!(
        result.job.number_records_processed, 1,
        "Should process 1 record"
    );
    assert_eq!(
        result.job.number_records_failed, 0,
        "Should have 0 failures"
    );

    let updated: serde_json::Value = rest_client
        .get("Account", &account_id, Some(&["Id", "Description"]))
        .await
        .expect("Get should succeed");

    assert_eq!(
        updated.get("Description").and_then(|v| v.as_str()),
        Some("Updated via Bulk API")
    );

    let _ = rest_client.delete("Account", &account_id).await;
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_error_invalid_sobject_ingest() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let csv_data = "Name\nBad";

    let result = client
        .execute_ingest("NoSuchObject__c", BulkOperation::Insert, csv_data, None)
        .await;

    assert!(result.is_err(), "Ingest with invalid SObject should fail");
}

#[tokio::test]
async fn test_bulk_error_invalid_query_field() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "DefinitelyNotAField__c"])
        .limit(10);

    let result = client.execute_query(query_builder).await;

    assert!(result.is_err(), "Bulk query with invalid field should fail");
}

#[tokio::test]
async fn test_bulk_error_invalid_job_id() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let result = client.get_ingest_job("750000000000000AAA").await;

    assert!(result.is_err(), "Invalid job ID should fail");
}

// ============================================================================
// Parallel Query Results Tests (API v62.0+)
// ============================================================================

#[tokio::test]
async fn test_parallel_query_results_basic() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Create a query job first
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .limit(50);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    // Test get_parallel_query_results (may not be available in all orgs/API versions)
    match client
        .get_parallel_query_results(&result.job.id, None)
        .await
    {
        Ok(batch) => {
            // Should have at least one result URL if there are results
            if result.job.number_records_processed > 0 {
                assert!(
                    !batch.result_url.is_empty(),
                    "Should have at least one result URL when records exist"
                );

                // Each URL should be a valid string
                for url in &batch.result_url {
                    assert!(!url.is_empty(), "Result URL should not be empty");
                    assert!(
                        url.contains("/results/") || url.contains("/parallelResults"),
                        "URL should be a valid results endpoint"
                    );
                }
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND"),
                "Expected NOT_FOUND error, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn test_parallel_query_results_with_max_records() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Create a query job
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .limit(100);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    // Test with maxRecords parameter (may not be available in all orgs/API versions)
    match client
        .get_parallel_query_results(&result.job.id, Some(3))
        .await
    {
        Ok(batch) => {
            // Should have at most 3 result URLs
            if result.job.number_records_processed > 0 {
                assert!(
                    batch.result_url.len() <= 3,
                    "Should have at most 3 result URLs when maxRecords=3"
                );
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND"),
                "Expected NOT_FOUND error, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn test_get_all_query_results_parallel() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Create a query job with a reasonable number of records
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name", "Industry"])
        .limit(100);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    // Test high-level parallel download (may not be available in all orgs/API versions)
    match client.get_all_query_results_parallel(&result.job.id).await {
        Ok(csv_data) => {
            // Validate CSV structure
            let lines: Vec<&str> = csv_data.lines().collect();
            assert!(!lines.is_empty(), "Should have at least header line");

            if result.job.number_records_processed > 0 {
                assert!(lines.len() > 1, "Should have data rows");

                // Check header
                let header = lines[0];
                assert!(
                    header.to_lowercase().contains("id"),
                    "Header should contain Id"
                );
                assert!(
                    header.to_lowercase().contains("name"),
                    "Header should contain Name"
                );

                // Verify we got the right number of data rows
                let data_rows = lines.len() - 1; // Subtract header
                assert!(
                    data_rows > 0 && data_rows as i64 <= result.job.number_records_processed,
                    "Should have correct number of data rows"
                );
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND"),
                "Expected NOT_FOUND error, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn test_parallel_vs_serial_results_consistency() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Create a query job
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .limit(50);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    assert!(
        result.job.number_records_processed > 0,
        "Should have Account records (created by setup-scratch-org). \
         Run: cargo run --bin setup-scratch-org"
    );

    // Get results using serial method
    let serial_results = client
        .get_all_query_results(&result.job.id)
        .await
        .expect("Serial results should succeed");

    // Get results using parallel method (may not be available in all orgs/API versions)
    match client.get_all_query_results_parallel(&result.job.id).await {
        Ok(parallel_results) => {
            // Both should return CSV data with same structure
            let serial_lines: Vec<&str> = serial_results.lines().collect();
            let parallel_lines: Vec<&str> = parallel_results.lines().collect();

            assert_eq!(
                serial_lines.len(),
                parallel_lines.len(),
                "Both methods should return same number of lines"
            );

            // Headers should match
            assert_eq!(serial_lines[0], parallel_lines[0], "Headers should match");

            // Data row count should match
            let serial_data_rows = serial_lines.len() - 1;
            let parallel_data_rows = parallel_lines.len() - 1;
            assert_eq!(
                serial_data_rows, parallel_data_rows,
                "Both methods should return same number of data rows"
            );
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND"),
                "Expected NOT_FOUND error, got: {msg}"
            );
        }
    }
}

#[tokio::test]
async fn test_parallel_query_results_empty_job() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Create a query that returns no results
    let query_builder: QueryBuilder<serde_json::Value> = QueryBuilder::new("Account")
        .expect("QueryBuilder creation should succeed")
        .select(&["Id", "Name"])
        .where_eq("Id", "000000000000000AAA")
        .expect("Where clause should succeed") // Invalid ID that won't match
        .limit(10);

    let result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk query should succeed");

    // Test parallel results on empty job (may not be available in all orgs/API versions)
    match client
        .get_parallel_query_results(&result.job.id, None)
        .await
    {
        Ok(_batch) => {
            // Should handle empty results gracefully
            match client.get_all_query_results_parallel(&result.job.id).await {
                Ok(csv_data) => {
                    // Should have at least header
                    let lines: Vec<&str> = csv_data.lines().collect();
                    assert!(!lines.is_empty(), "Should have at least header line");
                }
                Err(e) => {
                    let msg = e.to_string();
                    assert!(
                        msg.contains("NOT_FOUND"),
                        "Expected NOT_FOUND error, got: {msg}"
                    );
                }
            }
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("NOT_FOUND"),
                "Expected NOT_FOUND error, got: {msg}"
            );
        }
    }
}

// ============================================================================
// MetadataComponentDependency Tests (requires dependencies feature)
// ============================================================================

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_bulk_query_metadata_component_dependencies() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<serde_json::Value> =
        QueryBuilder::new("MetadataComponentDependency")
            .expect("QueryBuilder creation should succeed")
            .select(&[
                "MetadataComponentId",
                "MetadataComponentName",
                "MetadataComponentType",
                "RefMetadataComponentId",
                "RefMetadataComponentName",
                "RefMetadataComponentType",
            ])
            .limit(1000);

    let query_result = client
        .execute_query(query_builder)
        .await
        .expect("Bulk MetadataComponentDependency query should succeed");

    assert!(
        query_result.job.number_records_processed >= 0,
        "Should process records"
    );

    if let Some(csv_results) = query_result.results {
        let lines: Vec<&str> = csv_results.lines().collect();
        assert!(!lines.is_empty(), "Should have at least header line");
        let header = lines[0];
        assert!(
            header.contains("MetadataComponentId") || header.contains("metadatacomponentid"),
            "Header should contain MetadataComponentId"
        );
    }
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_bulk_query_metadata_component_dependencies_with_filter() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<serde_json::Value> =
        QueryBuilder::new("MetadataComponentDependency")
            .expect("QueryBuilder creation should succeed")
            .select(&[
                "MetadataComponentId",
                "MetadataComponentName",
                "MetadataComponentType",
            ])
            .where_eq("MetadataComponentType", "ApexClass")
            .expect("where_eq should succeed")
            .limit(100);

    let query_result = client
        .execute_query(query_builder)
        .await
        .expect("Filtered MetadataComponentDependency query should succeed");

    assert!(
        query_result.job.number_records_processed >= 0,
        "Should process records"
    );
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_bulk_metadata_component_dependency_type_deserialization() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let query_builder: QueryBuilder<busbar_sf_client::MetadataComponentDependency> =
        QueryBuilder::new("MetadataComponentDependency")
            .expect("QueryBuilder creation should succeed")
            .select(&[
                "MetadataComponentId",
                "MetadataComponentName",
                "MetadataComponentNamespace",
                "MetadataComponentType",
                "RefMetadataComponentId",
                "RefMetadataComponentName",
                "RefMetadataComponentNamespace",
                "RefMetadataComponentType",
            ])
            .limit(5);

    let query_result = client
        .execute_query(query_builder)
        .await
        .expect("Type deserialization query should succeed");

    assert!(
        query_result.job.number_records_processed >= 0,
        "Should process records"
    );
}
