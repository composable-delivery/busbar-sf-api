//! REST API CRUD operations example
//!
//! This example demonstrates TWO approaches to working with Salesforce data:
//! 1. Type-safe structs (recommended for production)
//! 2. Dynamic serde_json::Value (useful for exploration/prototyping)
//!
//! Run with: cargo run --example rest_crud

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_rest::SalesforceRestClient;
use serde::{Deserialize, Serialize};

/// Account record with proper type safety
///
/// Use this approach when:
/// - Building production applications
/// - You know the schema ahead of time
/// - You want compile-time safety and IDE support
#[derive(Debug, Serialize, Deserialize)]
struct Account {
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry", skip_serializing_if = "Option::is_none")]
    industry: Option<String>,
    #[serde(rename = "Phone", skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
    #[serde(rename = "Website", skip_serializing_if = "Option::is_none")]
    website: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging and debugging
    tracing_subscriber::fmt::init();

    println!("=== Salesforce REST API CRUD Examples ===\n");

    // Get credentials (try SFDX first, then environment)
    let creds = get_credentials().await?;

    // Create REST client
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())?;

    // Run CRUD examples - showing BOTH patterns
    println!("--- Type-Safe Struct Pattern ---\n");
    let account_id = example_create_typed(&client).await?;
    example_read_typed(&client, &account_id).await?;
    example_update_typed(&client, &account_id).await?;

    println!("\n--- Dynamic JSON Pattern ---\n");
    let dynamic_id = example_create_dynamic(&client).await?;
    example_read_dynamic(&client, &dynamic_id).await?;

    // Clean up
    example_delete(&client, &account_id).await?;
    example_delete(&client, &dynamic_id).await?;

    // Advanced operations
    example_upsert(&client).await?;
    example_create_multiple(&client).await?;

    println!("\n✓ All CRUD examples completed successfully!");

    Ok(())
}

/// Example 1a: Create with type-safe struct (RECOMMENDED)
async fn example_create_typed(
    client: &SalesforceRestClient,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Example 1a: Create with Type-Safe Struct");
    println!("------------------------------------------");

    let account = Account {
        id: None,
        name: "Acme Corporation".to_string(),
        industry: Some("Technology".to_string()),
        phone: Some("+1-555-0100".to_string()),
        website: Some("https://acme.example.com".to_string()),
    };

    let id = client.create("Account", &account).await?;
    println!("✓ Created account with ID: {}", id);
    println!("  Benefits: Type safety, IDE support, compile-time checking");
    println!();

    Ok(id)
}

/// Example 1b: Create with dynamic JSON
async fn example_create_dynamic(
    client: &SalesforceRestClient,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Example 1b: Create with Dynamic JSON");
    println!("--------------------------------------");

    let account = serde_json::json!({
        "Name": "Dynamic Industries",
        "Industry": "Technology",
        "Phone": "+1-555-0200"
    });

    let id = client.create("Account", &account).await?;
    println!("✓ Created account with ID: {}", id);
    println!("  Benefits: Flexible, good for exploration/prototyping");
    println!();

    Ok(id)
}

/// Example 2a: Read with type-safe deserialization
async fn example_read_typed(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2a: Read with Type-Safe Struct");
    println!("----------------------------------------");

    let account: Account = client
        .get(
            "Account",
            account_id,
            Some(&["Id", "Name", "Industry", "Phone", "Website"]),
        )
        .await?;

    println!("✓ Retrieved account:");
    println!("  ID: {:?}", account.id);
    println!("  Name: {}", account.name);
    println!("  Industry: {:?}", account.industry);
    println!("  Phone: {:?}", account.phone);
    println!("  Website: {:?}", account.website);
    println!();

    Ok(())
}

/// Example 2b: Read with dynamic JSON
async fn example_read_dynamic(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2b: Read with Dynamic JSON");
    println!("------------------------------------");

    let account: serde_json::Value = client
        .get(
            "Account",
            account_id,
            Some(&["Id", "Name", "Industry", "Phone"]),
        )
        .await?;

    println!("✓ Retrieved account:");
    println!("  ID: {}", account["Id"]);
    println!("  Name: {}", account["Name"]);
    println!("  Industry: {}", account["Industry"]);
    println!("  Phone: {}", account["Phone"]);
    println!();

    Ok(())
}

/// Example 3: Update with partial data (works with either pattern)
async fn example_update_typed(
    client: &SalesforceRestClient,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Update Record");
    println!("------------------------");

    // For updates, dynamic JSON is often more convenient
    let updates = serde_json::json!({
        "Name": "Acme Corporation (Updated)",
        "Phone": "+1-555-0101"
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

/// Example 6: Create multiple records at once with type safety
async fn example_create_multiple(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 6: Create Multiple Records");
    println!("-----------------------------------");

    let accounts = vec![
        Account {
            id: None,
            name: "Tech Startup Inc".to_string(),
            industry: Some("Technology".to_string()),
            phone: None,
            website: Some("https://techstartup.example".to_string()),
        },
        Account {
            id: None,
            name: "Retail Giant LLC".to_string(),
            industry: Some("Retail".to_string()),
            phone: None,
            website: Some("https://retailgiant.example".to_string()),
        },
        Account {
            id: None,
            name: "Finance Corp".to_string(),
            industry: Some("Finance".to_string()),
            phone: None,
            website: None,
        },
    ];

    // Create up to 200 records at once
    // all_or_none: true means either all succeed or all fail
    let results = client.create_multiple("Account", &accounts, true).await?;

    println!("✓ Created {} accounts", results.len());
    for (i, result) in results.iter().enumerate() {
        if result.success {
            let id = result.id.as_deref().unwrap_or("Unknown");
            println!("  Account {}: {} - ID: {}", i + 1, accounts[i].name, id);
        } else {
            println!("  Account {}: Failed - {:?}", i + 1, result.errors);
        }
    }

    // Clean up test data
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
