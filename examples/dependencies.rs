//! MetadataComponentDependency API examples
//!
//! This example demonstrates how to query metadata component dependencies
//! using both the Tooling API and Bulk API 2.0.
//!
//! The MetadataComponentDependency object (Beta) represents dependency relationships
//! between metadata components in your org.
//!
//! Limitations:
//! - Tooling API: Up to 2000 records per query
//! - Bulk API 2.0: Up to 100,000 records per query
//!
//! Run with: cargo run --example dependencies --features dependencies

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_tooling::ToolingClient;

#[cfg(feature = "dependencies")]
use busbar_sf_client::MetadataComponentDependency;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Salesforce MetadataComponentDependency Examples ===\n");

    #[cfg(not(feature = "dependencies"))]
    {
        println!("ERROR: This example requires the 'dependencies' feature!");
        println!("Run with: cargo run --example dependencies --features dependencies");
        return Ok(());
    }

    #[cfg(feature = "dependencies")]
    {
        let creds = get_credentials().await?;

        // Example 1: Query dependencies using Tooling API
        example_tooling_api(&creds).await?;

        // Example 2: Query dependencies using Bulk API
        example_bulk_api(&creds).await?;

        println!("\n✓ All MetadataComponentDependency examples completed successfully!");
    }

    Ok(())
}

#[cfg(feature = "dependencies")]
async fn example_tooling_api(
    creds: &SalesforceCredentials,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Query Dependencies with Tooling API");
    println!("------------------------------------------------");

    let client = ToolingClient::new(creds.instance_url(), creds.access_token())?;

    // Get all dependencies (limited to 2000 records)
    println!("Querying metadata component dependencies (Tooling API)...");
    let deps: Vec<MetadataComponentDependency> =
        client.get_metadata_component_dependencies(None).await?;

    println!("✓ Found {} dependency relationships", deps.len());

    if !deps.is_empty() {
        println!("\nFirst 3 dependencies:");
        for (i, dep) in deps.iter().take(3).enumerate() {
            println!(
                "  {}. {} ({}) -> {} ({})",
                i + 1,
                dep.metadata_component_name.as_deref().unwrap_or("Unknown"),
                dep.metadata_component_type.as_deref().unwrap_or("Unknown"),
                dep.ref_metadata_component_name
                    .as_deref()
                    .unwrap_or("Unknown"),
                dep.ref_metadata_component_type
                    .as_deref()
                    .unwrap_or("Unknown")
            );
        }
    }

    // Filter by component type
    println!("\nQuerying ApexClass dependencies only...");
    let apex_deps: Vec<MetadataComponentDependency> = client
        .get_metadata_component_dependencies(Some("MetadataComponentType = 'ApexClass'"))
        .await?;

    println!(
        "✓ Found {} ApexClass dependency relationships\n",
        apex_deps.len()
    );

    Ok(())
}

#[cfg(feature = "dependencies")]
async fn example_bulk_api(
    _creds: &SalesforceCredentials,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Query Dependencies with Bulk API");
    println!("--------------------------------------------");

    println!("The Bulk API 2.0 supports querying MetadataComponentDependency");
    println!("with up to 100,000 records per query (vs 2000 in Tooling API).");
    println!();
    println!("To use Bulk API for dependencies in your code:");
    println!("  1. Enable the 'dependencies' feature");
    println!("  2. Use BulkApiClient.execute_query() with a QueryBuilder");
    println!("  3. The MetadataComponentDependency type is available from both crates");
    println!();
    println!("Example code:");
    println!("  let result = bulk_client.execute_query(");
    println!("      QueryBuilder::new(\"MetadataComponentDependency\")?");
    println!("          .select(&[\"MetadataComponentId\", \"RefMetadataComponentId\"])");
    println!("          .limit(100_000)");
    println!("  ).await?;");
    println!();

    Ok(())
}

/// Get credentials from environment or interactive prompt
async fn get_credentials() -> Result<SalesforceCredentials, Box<dyn std::error::Error>> {
    // Try to get credentials from Salesforce CLI first
    if let Ok(creds) = SalesforceCredentials::from_sfdx_alias("default").await {
        println!("✓ Using credentials from Salesforce CLI\n");
        return Ok(creds);
    }

    // Try to get credentials from environment
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
