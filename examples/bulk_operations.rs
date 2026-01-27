//! Bulk API 2.0 examples
//!
//! This example demonstrates large-scale data operations using the Bulk API 2.0:
//! - Bulk insert
//! - Bulk update
//! - Bulk query
//! - Job monitoring
//!
//! Run with: cargo run --example bulk_operations

use busbar_sf_auth::SalesforceCredentials;
use busbar_sf_bulk::{BulkApiClient, BulkOperation, CreateIngestJobRequest, CreateQueryJobRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Account {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry")]
    industry: Option<String>,
    #[serde(rename = "Phone")]
    phone: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== Salesforce Bulk API 2.0 Examples ===\n");

    let creds = get_credentials().await?;
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())?;

    // Bulk operations examples
    example_bulk_insert(&client).await?;
    example_bulk_query(&client).await?;
    example_manual_job_control(&client).await?;

    println!("\n✓ All Bulk API examples completed successfully!");

    Ok(())
}

/// Example 1: Bulk insert using high-level API
async fn example_bulk_insert(
    client: &BulkApiClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Bulk Insert (High-Level API)");
    println!("----------------------------------------");

    // Prepare CSV data
    let csv_data = r#"Name,Industry,Phone
Acme Corp,Technology,+1-555-0100
Global Industries,Manufacturing,+1-555-0101
Tech Startup,Technology,+1-555-0102
Retail Giant,Retail,+1-555-0103
Finance Corp,Finance,+1-555-0104"#;

    println!("Inserting {} records...", csv_data.lines().count() - 1);

    // Execute complete ingest operation
    let result = client
        .execute_ingest("Account", BulkOperation::Insert, csv_data, None)
        .await?;

    println!("\n✓ Bulk insert completed!");
    println!("  Job ID: {}", result.job.id);
    println!("  State: {:?}", result.job.state);
    println!("  Records Processed: {}", result.job.number_records_processed);
    println!("  Records Failed: {}", result.job.number_records_failed);

    if result.job.number_records_failed > 0 {
        if let Some(failed_results) = result.failed_results {
            println!("\nFailed records:");
            println!("{}", failed_results.lines().take(5).collect::<Vec<_>>().join("\n"));
        }
    }

    println!();

    Ok(())
}

/// Example 2: Bulk query
async fn example_bulk_query(client: &BulkApiClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Bulk Query");
    println!("---------------------");

    let soql = "SELECT Id, Name, Industry FROM Account WHERE Industry != null LIMIT 1000";

    println!("Executing bulk query...");

    // Execute complete query operation
    let result = client.execute_query(soql).await?;

    println!("\n✓ Bulk query completed!");
    println!("  Job ID: {}", result.job.id);
    println!("  State: {:?}", result.job.state);
    println!("  Records Processed: {}", result.job.number_records_processed);

    if let Some(csv_results) = result.results {
        let line_count = csv_results.lines().count();
        println!("  Total lines (including header): {}", line_count);

        // Show first few records
        println!("\nFirst 5 records:");
        for (i, line) in csv_results.lines().take(6).enumerate() {
            if i == 0 {
                println!("  {}", line); // Header
            } else {
                println!("  {}", line);
            }
        }
    }

    println!();

    Ok(())
}

/// Example 3: Manual job control (step-by-step)
async fn example_manual_job_control(
    client: &BulkApiClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Manual Job Control");
    println!("------------------------------");

    // Step 1: Create job
    println!("Step 1: Creating ingest job...");
    let request = CreateIngestJobRequest::new("Account", BulkOperation::Insert);
    let job = client.create_ingest_job(request).await?;
    println!("✓ Created job: {}", job.id);
    println!("  State: {:?}", job.state);

    // Step 2: Upload data
    println!("\nStep 2: Uploading data...");
    let csv_data = r#"Name,Industry
Manual Job Test 1,Technology
Manual Job Test 2,Manufacturing"#;

    client.upload_job_data(&job.id, csv_data).await?;
    println!("✓ Data uploaded");

    // Step 3: Close job (mark as UploadComplete)
    println!("\nStep 3: Closing job...");
    let closed_job = client.close_ingest_job(&job.id).await?;
    println!("✓ Job closed");
    println!("  State: {:?}", closed_job.state);

    // Step 4: Monitor job (wait for completion)
    println!("\nStep 4: Waiting for job completion...");
    let completed_job = client.wait_for_ingest_job(&job.id).await?;
    println!("✓ Job completed!");
    println!("  State: {:?}", completed_job.state);
    println!("  Records Processed: {}", completed_job.number_records_processed);
    println!("  Records Failed: {}", completed_job.number_records_failed);

    // Step 5: Get results
    if completed_job.number_records_processed > 0 {
        println!("\nStep 5: Getting successful results...");
        let results = client.get_successful_results(&job.id).await?;
        println!("✓ Retrieved results:");
        for (i, line) in results.lines().take(5).enumerate() {
            println!("  {}: {}", i, line);
        }
    }

    println!();

    Ok(())
}

/// Example: Bulk update with external ID
#[allow(dead_code)]
async fn example_bulk_upsert(client: &BulkApiClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example: Bulk Upsert");
    println!("--------------------");

    // CSV data with external ID
    let csv_data = r#"ExternalId__c,Name,Industry
EXT-001,Updated Account 1,Technology
EXT-002,Updated Account 2,Manufacturing
EXT-003,New Account 3,Retail"#;

    println!("Upserting records with external ID...");

    // Create upsert job with external ID field
    let request = CreateIngestJobRequest::new("Account", BulkOperation::Upsert)
        .with_external_id_field("ExternalId__c");

    let job = client.create_ingest_job(request).await?;
    client.upload_job_data(&job.id, csv_data).await?;
    client.close_ingest_job(&job.id).await?;

    let completed_job = client.wait_for_ingest_job(&job.id).await?;

    println!("✓ Upsert completed!");
    println!("  Records Processed: {}", completed_job.number_records_processed);
    println!("  Records Failed: {}", completed_job.number_records_failed);

    println!();

    Ok(())
}

/// Example: Polling with custom intervals
#[allow(dead_code)]
async fn example_custom_polling(
    creds: &SalesforceCredentials,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;

    println!("Example: Custom Polling Configuration");
    println!("--------------------------------------");

    // Create client with custom polling settings
    let client = BulkApiClient::new(creds.instance_url(), creds.access_token())?
        .with_poll_interval(Duration::from_secs(10)) // Poll every 10 seconds
        .with_max_wait(Duration::from_secs(600)); // Wait max 10 minutes

    let csv_data = "Name,Industry\nTest Account,Technology";

    let result = client
        .execute_ingest("Account", BulkOperation::Insert, csv_data, None)
        .await?;

    println!("✓ Job completed with custom polling");
    println!("  Records Processed: {}", result.job.number_records_processed);

    println!();

    Ok(())
}

/// Example: Abort a job
#[allow(dead_code)]
async fn example_abort_job(client: &BulkApiClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example: Abort Job");
    println!("------------------");

    // Create a job
    let request = CreateIngestJobRequest::new("Account", BulkOperation::Insert);
    let job = client.create_ingest_job(request).await?;
    println!("Created job: {}", job.id);

    // Abort the job
    let aborted_job = client.abort_ingest_job(&job.id).await?;
    println!("✓ Job aborted");
    println!("  State: {:?}", aborted_job.state);

    println!();

    Ok(())
}

/// Example: Query job with pagination
#[allow(dead_code)]
async fn example_query_pagination(
    client: &BulkApiClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example: Query with Pagination");
    println!("-------------------------------");

    // Create query job
    let request = CreateQueryJobRequest::new("SELECT Id, Name FROM Account LIMIT 10000");
    let job = client.create_query_job(request).await?;

    // Wait for completion
    let completed_job = client.wait_for_query_job(&job.id).await?;
    println!("✓ Query completed");
    println!("  Records: {}", completed_job.number_records_processed);

    // Get results with pagination
    let mut locator: Option<String> = None;
    let mut page = 1;

    loop {
        println!("\nFetching page {}...", page);
        let results = client
            .get_query_results(&job.id, locator.as_deref(), Some(1000))
            .await?;

        println!("  Retrieved {} bytes", results.csv_data.len());

        match results.locator {
            Some(next_locator) => {
                locator = Some(next_locator);
                page += 1;
            }
            None => break,
        }
    }

    println!("\n✓ All pages retrieved");

    println!();

    Ok(())
}

/// Helper function to get credentials
async fn get_credentials() -> Result<SalesforceCredentials, Box<dyn std::error::Error>> {
    if let Ok(creds) = SalesforceCredentials::from_sfdx_alias("default").await {
        println!("✓ Using credentials from Salesforce CLI\n");
        return Ok(creds);
    }

    match SalesforceCredentials::from_env() {
        Ok(creds) => {
            println!("✓ Using credentials from environment variables\n");
            Ok(creds)
        }
        Err(e) => {
            eprintln!("✗ Failed to load credentials: {}", e);
            eprintln!("\nPlease either:");
            eprintln!("  1. Authenticate with Salesforce CLI: sf org login web");
            eprintln!("  2. Set environment variables: SF_INSTANCE_URL, SF_ACCESS_TOKEN");
            Err(e.into())
        }
    }
}
