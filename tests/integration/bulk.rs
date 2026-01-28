//! Bulk API 2.0 integration tests using SF_AUTH_URL.

use busbar_sf_auth::Credentials;
use super::common::require_credentials;
use busbar_sf_bulk::{BulkApiClient, BulkOperation};
use busbar_sf_rest::{QueryBuilder, SalesforceRestClient};

// ============================================================================
// Bulk API 2.0 Tests
// ============================================================================

#[tokio::test]
async fn test_bulk_insert_lifecycle() {
    let Some(creds) = require_credentials().await else { return; };
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
    let Some(creds) = require_credentials().await else { return; };
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
            assert!(header.to_lowercase().contains("id"), "Header should contain Id");
            assert!(
                header.to_lowercase().contains("name"),
                "Header should contain Name"
            );
        }
    }
}

#[tokio::test]
async fn test_bulk_update_operation() {
    let Some(creds) = require_credentials().await else { return; };

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
    let Some(creds) = require_credentials().await else { return; };
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
    let Some(creds) = require_credentials().await else { return; };
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
    let Some(creds) = require_credentials().await else { return; };
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create Bulk client");

    let result = client.get_ingest_job("750000000000000AAA").await;

    assert!(result.is_err(), "Invalid job ID should fail");
}
