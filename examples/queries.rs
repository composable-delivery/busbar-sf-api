//! SOQL query examples with security best practices
//!
//! This example demonstrates:
//! 1. QueryBuilder - Safe by default with fluent API (RECOMMENDED)
//! 2. Type-safe structs vs dynamic JSON (both patterns shown)
//! 3. Manual escaping (last resort - shown for completeness)
//!
//! Run with: cargo run --example queries

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_client::security::soql;
use busbar_sf_rest::{QueryBuilder, SalesforceRestClient};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Account record for type-safe queries
/// 
/// Use typed structs when:
/// - Building production applications  
/// - You know the schema ahead of time
/// - You want compile-time safety and IDE support
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Account {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry", skip_serializing_if = "Option::is_none")]
    industry: Option<String>,
}

/// Contact record with relationship query support
#[derive(Debug, Deserialize)]
struct Contact {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Email")]
    email: Option<String>,
    #[serde(rename = "Account")]
    account: Option<AccountRef>,
}

/// Nested Account reference in Contact
#[derive(Debug, Deserialize)]
struct AccountRef {
    #[serde(rename = "Name")]
    name: String,
}

/// Aggregate query result
#[derive(Debug, Deserialize)]
struct IndustryCount {
    #[serde(rename = "Industry")]
    industry: String,
    #[serde(rename = "total")]
    total: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better observability
    tracing_subscriber::fmt::init();

    println!("=== Salesforce SOQL Query Examples ===\n");

    let creds = get_credentials().await?;
    let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())?;

    // RECOMMENDED: Use QueryBuilder (safe by default)
    println!("--- QueryBuilder Pattern (RECOMMENDED) ---\n");
    example_query_builder_typed(&client).await?;
    example_query_builder_dynamic(&client).await?;
    example_query_builder_advanced(&client).await?;

    // Alternative: Raw queries (less safe, but flexible)
    println!("\n--- Raw Query Patterns ---\n");
    example_basic_query_typed(&client).await?;
    example_basic_query_dynamic(&client).await?;

    // Manual escaping (NOT recommended, but shown for completeness)
    println!("\n--- Manual Escaping (NOT RECOMMENDED) ---\n");
    example_manual_escaping(&client).await?;

    // Advanced queries
    println!("\n--- Advanced Queries ---\n");
    example_relationship_query(&client).await?;
    example_aggregate_query(&client).await?;

    println!("\n✓ All query examples completed successfully!");

    Ok(())
}

/// Example 1a: QueryBuilder with type-safe results (RECOMMENDED)
async fn example_query_builder_typed(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1a: QueryBuilder with Type Safety");
    println!("------------------------------------------");

    // Simulated user input (potentially dangerous)
    let user_name = "O'Brien's Company"; // Has single quote

    // Build and execute query with automatic escaping
    let accounts: Vec<Account> = QueryBuilder::new("Account")?
        .select(&["Id", "Name", "Industry"])
        .where_eq("Name", user_name)?  // Automatically escaped!
        .limit(10)
        .execute(&client)
        .await?;

    println!("✓ Found {} accounts", accounts.len());
    println!("  Benefits: Type-safe results, automatic escaping, fluent API");
    println!();

    Ok(())
}

/// Example 1b: QueryBuilder with dynamic JSON results
async fn example_query_builder_dynamic(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1b: QueryBuilder with Dynamic JSON");
    println!("-------------------------------------------");

    let user_pattern = "tech%value"; // Has SQL wildcards

    // Works with HashMap for ergonomic access
    let accounts: Vec<HashMap<String, serde_json::Value>> = QueryBuilder::new("Account")?
        .select(&["Id", "Name", "Industry"])
        .where_like("Name", user_pattern)?  // Wildcards automatically escaped!
        .limit(5)
        .execute(&client)
        .await?;

    println!("✓ Found {} accounts", accounts.len());
    for account in accounts.iter().take(3) {
        let name = account.get("Name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        println!("  - {}", name);
    }
    println!("  Benefits: Flexible, ergonomic HashMap access, no struct needed");
    println!();

    Ok(())
}

/// Example 1c: QueryBuilder with advanced features
async fn example_query_builder_advanced(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1c: QueryBuilder Advanced Features");
    println!("-------------------------------------------");

    let industries = vec!["Technology", "Finance"];

    let accounts: Vec<Account> = QueryBuilder::new("Account")?
        .select(&["Id", "Name", "Industry"])
        .where_in("Industry", &industries)?  // Multiple values
        .order_by("Name", true)?  // Sort ascending
        .limit(20)
        .execute(&client)
        .await?;

    println!("✓ Found {} accounts in specified industries", accounts.len());
    println!("  Features: WHERE IN, ORDER BY, fluent chaining");
    println!();

    Ok(())
}

/// Example 1a: Basic type-safe SOQL query (RECOMMENDED for production)
async fn example_basic_query_typed(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1a: Type-Safe Query");
    println!("----------------------------");

    let query = "SELECT Id, Name, Industry FROM Account LIMIT 5";
    let result: busbar_sf_client::QueryResult<Account> = client.query(query).await?;

    println!("✓ Found {} accounts (total: {})", result.records.len(), result.total_size);
    for account in &result.records {
        println!("  - {} (Industry: {:?})", account.name, account.industry);
    }
    println!("  Benefits: Type safety, field access without unwrapping");
    println!();

    Ok(())
}

/// Example 1b: Dynamic JSON query with proper serde_json patterns
async fn example_basic_query_dynamic(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1b: Dynamic JSON Query");
    println!("-------------------------------");

    let query = "SELECT Id, Name, Industry FROM Account LIMIT 5";
    
    // Use HashMap for more ergonomic access than raw Value
    let result: busbar_sf_client::QueryResult<HashMap<String, serde_json::Value>> = 
        client.query(query).await?;

    println!("✓ Found {} accounts", result.records.len());
    for account in &result.records {
        // Much more ergonomic than account["Name"].as_str().unwrap_or()
        let name = account.get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let industry = account.get("Industry")
            .and_then(|v| v.as_str())
            .unwrap_or("None");
        println!("  - {} (Industry: {})", name, industry);
    }
    println!("  Benefits: HashMap provides .get() method, no indexing panics");
    println!();

    Ok(())
}

/// Example 2: Automatic pagination with type safety
async fn example_query_pagination_typed(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Pagination with Type Safety");
    println!("---------------------------------------");

    // query_all automatically handles pagination
    let query = "SELECT Id, Name FROM Account LIMIT 100";
    let accounts: Vec<Account> = client.query_all(query).await?;

    println!("✓ Retrieved {} accounts (automatic pagination)", accounts.len());
    println!();

    Ok(())
}

/// Example 3: Manual escaping - NOT RECOMMENDED but shown for completeness
///
/// WARNING: This approach is error-prone! 
/// - Easy to forget to escape
/// - Easy to use wrong escape function (escape_string vs escape_like)
/// - Not safe by default
///
/// Prefer the SafeQueryBuilder pattern shown above!
async fn example_manual_escaping(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Manual Escaping (NOT RECOMMENDED)");
    println!("---------------------------------------------");
    println!("⚠️  WARNING: Easy to forget! Use QueryBuilder instead.");
    println!();

    let user_input = "O'Brien's Company";
    let malicious_input = "'; DELETE FROM Account--";

    // Manual escaping - requires developer to remember!
    let safe_name = soql::escape_string(user_input);
    let query = format!("SELECT Id, Name FROM Account WHERE Name = '{}'", safe_name);

    let accounts: Vec<Account> = client.query_all(&query).await?;
    println!("  Found {} accounts", accounts.len());

    // Show what happens if you forget to escape (DON'T DO THIS!)
    let safe_malicious = soql::escape_string(malicious_input);
    println!("\n  Injection attempt:");
    println!("  Raw input:      {}", malicious_input);
    println!("  After escaping: {}", safe_malicious);
    println!("\n  ❌ Problem: Relies on developer remembering to escape");
    println!("  ✅ Solution: Use QueryBuilder that escapes automatically");
    println!();

    Ok(())
}

/// Example 4: Field validation
async fn example_field_validation(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Field Validation");
    println!("---------------------------");

    // User-provided field names (could be malicious)
    let user_fields = vec![
        "Id",
        "Name",
        "Industry",
        "Bad'; DROP TABLE--", // Injection attempt
        "CustomField__c",
    ];

    // Filter to only safe field names
    let safe_fields: Vec<&str> = soql::filter_safe_fields(user_fields.iter().copied()).collect();

    println!("  Original fields: {:?}", user_fields);
    println!("  Safe fields:     {:?}", safe_fields);

    // Build SELECT clause with safe fields
    if let Some(select_clause) = soql::build_safe_select(&safe_fields) {
        let query = format!("SELECT {} FROM Account LIMIT 5", select_clause);
        
        // Use HashMap for dynamic field access
        let result: busbar_sf_client::QueryResult<HashMap<String, serde_json::Value>> =
            client.query(&query).await?;
        println!("✓ Retrieved {} records", result.records.len());
    } else {
        println!("✗ No safe fields to query");
    }

    println!();

    Ok(())
}

/// Example 7: Relationship query
async fn example_relationship_query(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 7: Relationship Query");
    println!("-----------------------------");

    let query = "SELECT Id, Name, Email, Account.Name FROM Contact WHERE Account.Name != null LIMIT 5";

    let contacts: Vec<serde_json::Value> = client.query_all(query).await?;

    println!("✓ Found {} contacts with accounts", contacts.len());
    for contact in &contacts {
        let name = contact.get("Name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let id = contact.get("Id").and_then(|v| v.as_str()).unwrap_or("Unknown");
        if let Some(account) = contact.get("Account") {
            if let Some(account_name) = account.get("Name").and_then(|v| v.as_str()) {
                println!("  - {} ({}) @ {}", name, id, account_name);
            }
        }
    }
    println!();

    Ok(())
}

/// Example 8: Aggregate query
async fn example_aggregate_query(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 8: Aggregate Query");
    println!("--------------------------");

    let query = "SELECT Industry, COUNT(Id) total FROM Account WHERE Industry != null GROUP BY Industry ORDER BY COUNT(Id) DESC LIMIT 5";

    let results: Vec<serde_json::Value> = client.query_all(query).await?;

    println!("✓ Top {} industries:", results.len());
    for result in &results {
        let industry = result.get("Industry").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let total = result.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
        println!("  - {}: {} accounts", industry, total);
    }
    println!();

    Ok(())
}

/// Helper function to build secure queries
#[allow(dead_code)]
fn build_secure_query(
    sobject: &str,
    fields: &[&str],
    where_field: &str,
    user_value: &str,
) -> Option<String> {
    // Validate SObject name
    if !soql::is_safe_sobject_name(sobject) {
        return None;
    }

    // Validate and build field list
    let select_clause = soql::build_safe_select(fields)?;

    // Validate WHERE field
    if !soql::is_safe_field_name(where_field) {
        return None;
    }

    // Escape user value
    let safe_value = soql::escape_string(user_value);

    Some(format!(
        "SELECT {} FROM {} WHERE {} = '{}'",
        select_clause, sobject, where_field, safe_value
    ))
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
