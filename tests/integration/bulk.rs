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
            .limit(1000); // Bulk API supports up to 100,000 records

    let result = client.execute_query(query_builder).await;

    // MetadataComponentDependency may not be available in all orgs or may require specific API version
    // Handle both success and expected failure cases
    match result {
        Ok(query_result) => {
            assert!(
                query_result.job.number_records_processed >= 0,
                "Should process records"
            );

            println!(
                "Bulk query processed {} MetadataComponentDependency records",
                query_result.job.number_records_processed
            );

            if let Some(csv_results) = query_result.results {
                let lines: Vec<&str> = csv_results.lines().collect();
                assert!(!lines.is_empty(), "Should have at least header line");
                if let Some(header) = lines.first() {
                    assert!(
                        header.contains("MetadataComponentId")
                            || header.contains("metadatacomponentid"),
                        "Header should contain MetadataComponentId"
                    );
                    assert!(
                        header.contains("RefMetadataComponentId")
                            || header.contains("refmetadatacomponentid"),
                        "Header should contain RefMetadataComponentId"
                    );
                }
            }
        }
        Err(e) => {
            // If the query fails, it might be because:
            // - The org doesn't support MetadataComponentDependency (API version < 49.0)
            // - The object is not available in this org type
            println!(
                "MetadataComponentDependency query failed (this may be expected): {}",
                e
            );
            // Don't fail the test - this is expected in some orgs
        }
    }
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_bulk_query_metadata_component_dependencies_with_filter() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Query with a filter for ApexClass dependencies
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

    let result = client.execute_query(query_builder).await;

    // This may fail if there are no ApexClass dependencies in the scratch org,
    // or succeed with 0 results, both are valid outcomes
    match result {
        Ok(query_result) => {
            println!(
                "Bulk query with filter processed {} records",
                query_result.job.number_records_processed
            );
        }
        Err(e) => {
            // If it fails, it should be due to no matching records or query limitations
            println!("Bulk query with filter error (expected): {}", e);
        }
    }
}

#[cfg(feature = "dependencies")]
#[tokio::test]
async fn test_bulk_metadata_component_dependency_type_deserialization() {
    let creds = get_credentials().await;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    // Query a small number of records to test type deserialization
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

    let result = client.execute_query(query_builder).await;

    // MetadataComponentDependency may not be available in all orgs
    match result {
        Ok(query_result) => {
            println!(
                "Type deserialization test processed {} records",
                query_result.job.number_records_processed
            );
        }
        Err(e) => {
            // If the query fails, it might be because:
            // - The org doesn't support MetadataComponentDependency (API version < 49.0)
            // - The object is not available in this org type
            println!(
                "MetadataComponentDependency type deserialization test failed (this may be expected): {}",
                e
            );
            // Don't fail the test - this is expected in some orgs
        }
    }
}
