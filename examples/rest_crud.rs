//! REST API CRUD operations example
//!
//! This example demonstrates basic Create, Read, Update, Delete operations
//! using the Salesforce REST API.
//!
//! Run with: cargo run --example rest_crud

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_rest::SalesforceRestClient;

// For examples, we'll use serde_json::Value for simplicity
// In real code, you'd define your own structs with serde derives

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging if needed

    println!("=== Salesforce REST API CRUD Examples ===\n");

    // Get credentials (try SFDX first, then environment)
    let creds = get_credentials().await?;

    // Create REST client
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())?;

    // Run CRUD examples
    let account_id = example_create(&client).await?;
    example_read(&client, &account_id).await?;
    example_update(&client, &account_id).await?;
    example_upsert(&client).await?;
    example_delete(&client, &account_id).await?;

    // Collection operations
    example_create_multiple(&client).await?;

    println!("\n✓ All CRUD examples completed successfully!");

    Ok(())
}

/// Example 1: Create a record
async fn example_create(client: &SalesforceRestClient) -> Result<String, Box<dyn std::error::Error>> {
    println!("Example 1: Create Record");
    println!("------------------------");

    let account = serde_json::json!({
        "Name": "Acme Corporation",
        "Industry": "Technology",
        "Phone": "+1-555-0100",
        "Website": "https://acme.example.com"
    });

    let id = client.create("Account", &account).await?;
    println!("✓ Created account with ID: {}", id);
    println!();

    Ok(id)
}

/// Example 2: Read a record
async fn example_read(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Read Record");
    println!("----------------------");

    // Read with specific fields
    let account: serde_json::Value = client
        .get("Account", account_id, Some(&["Id", "Name", "Industry", "Phone", "Website"]))
        .await?;

    println!("✓ Retrieved account:");
    println!("  ID: {:?}", account["Id"]);
    println!("  Name: {}", account["Name"]);
    println!("  Industry: {:?}", account["Industry"]);
    println!("  Phone: {:?}", account["Phone"]);
    println!("  Website: {:?}", account["Website"]);
    println!();

    Ok(())
}

/// Example 3: Update a record
async fn example_update(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Update Record");
    println!("------------------------");

    let updates = serde_json::json!({
        "Name": "Acme Corporation (Updated)",
        "Phone": "+1-555-0101",
        "Website": "https://www.acme.example.com"
    });

    client.update("Account", account_id, &updates).await?;
    println!("✓ Updated account {}", account_id);
    println!();

    Ok(())
}

/// Example 4: Upsert (create or update based on external ID)
async fn example_upsert(client: &SalesforceRestClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Upsert Record");
    println!("------------------------");

    let account = serde_json::json!({
        "Name": "Global Industries",
        "Industry": "Manufacturing"
    });

    // Use a custom external ID field (must exist in your org)
    // This example uses AccountNumber as an example
    let external_id = "EXT-12345";

    match client
        .upsert("Account", "AccountNumber", external_id, &account)
        .await
    {
        Ok(result) => {
            if result.created {
                println!("✓ Created new account: {}", result.id);
            } else {
                println!("✓ Updated existing account: {}", result.id);
            }
        }
        Err(e) => {
            println!("Note: Upsert requires an external ID field in your org");
            println!("  Error: {}", e);
        }
    }
    println!();

    Ok(())
}

/// Example 5: Delete a record
async fn example_delete(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Delete Record");
    println!("------------------------");

    client.delete("Account", account_id).await?;
    println!("✓ Deleted account {}", account_id);
    println!();

    Ok(())
}

/// Example 6: Create multiple records at once
async fn example_create_multiple(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 6: Create Multiple Records");
    println!("-----------------------------------");

    let accounts = vec![
        serde_json::json!({
            "Name": "Tech Startup Inc",
            "Industry": "Technology",
            "Website": "https://techstartup.example"
        }),
        serde_json::json!({
            "Name": "Retail Giant LLC",
            "Industry": "Retail",
            "Website": "https://retailgiant.example"
        }),
        serde_json::json!({
            "Name": "Finance Corp",
            "Industry": "Finance"
        }),
    ];

    // Create up to 200 records at once
    // all_or_none: true means either all succeed or all fail
    let results = client.create_multiple("Account", &accounts, true).await?;

    println!("✓ Created {} accounts", results.len());
    for (i, result) in results.iter().enumerate() {
        if result.success {
            let name = accounts[i].get("Name").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let id = result.id.as_ref().map(|s| s.as_str()).unwrap_or("Unknown");
            println!("  Account {}: {} - ID: {}", i + 1, name, id);
        } else {
            println!("  Account {}: Failed - {:?}", i + 1, result.errors);
        }
    }

    // Clean up
    let ids: Vec<&str> = results.iter().filter_map(|r| r.id.as_deref()).collect();
    if !ids.is_empty() {
        let _ = client.delete_multiple(&ids, false).await;
        println!("✓ Cleaned up {} test accounts", ids.len());
    }

    println!();

    Ok(())
}

/// Helper function to get credentials
async fn get_credentials() -> Result<SalesforceCredentials, Box<dyn std::error::Error>> {
    // Try SFDX first
    if let Ok(creds) = SalesforceCredentials::from_sfdx_alias("default").await {
        println!("✓ Using credentials from Salesforce CLI\n");
        return Ok(creds);
    }

    // Fall back to environment variables
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
