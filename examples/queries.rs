//! SOQL query examples with security best practices
//!
//! This example demonstrates:
//! - Type-safe SOQL queries
//! - Query with pagination
//! - Secure query building with user input
//! - LIKE queries with wildcards
//! - Field validation
//!
//! Run with: cargo run --example queries

use busbar_sf_auth::{Credentials, SalesforceCredentials};
use busbar_sf_client::security::soql;
use busbar_sf_rest::SalesforceRestClient;
use serde::{Deserialize, Serialize};

/// Account record for type-safe queries
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

    // Basic queries
    example_basic_query(&client).await?;
    example_query_with_limit(&client).await?;
    example_query_pagination(&client).await?;

    // Secure queries with user input (CRITICAL for production)
    example_secure_query_user_input(&client).await?;
    example_secure_like_query(&client).await?;
    example_field_validation(&client).await?;

    // Advanced queries
    example_relationship_query(&client).await?;
    example_aggregate_query(&client).await?;

    println!("\n✓ All query examples completed successfully!");

    Ok(())
}

/// Example 1: Basic type-safe SOQL query
async fn example_basic_query(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: Basic SOQL Query");
    println!("----------------------------");

    let query = "SELECT Id, Name, Industry FROM Account LIMIT 5";
    let result: busbar_sf_client::QueryResult<Account> = client.query(query).await?;

    println!("✓ Found {} accounts (total: {})", result.records.len(), result.total_size);
    for account in &result.records {
        println!("  - {} ({})", account.name, account.id);
    }
    println!();

    Ok(())
}

/// Example 2: Query with LIMIT and type safety
async fn example_query_with_limit(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: Query with LIMIT");
    println!("---------------------------");

    // Type-safe query that returns strongly-typed Account records
    let query = "SELECT Id, Name FROM Account WHERE Industry != null LIMIT 100";
    let accounts: Vec<Account> = client.query_all(query).await?;

    println!("✓ Retrieved {} accounts with industry", accounts.len());
    println!();

    Ok(())
}

/// Example 3: Automatic pagination with type safety
async fn example_query_pagination(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: Automatic Pagination");
    println!("--------------------------------");

    // query_all automatically handles pagination and returns strongly-typed results
    let query = "SELECT Id, Name FROM Account";
    let all_accounts: Vec<Account> = client.query_all(query).await?;

    println!("✓ Retrieved all {} accounts (with automatic pagination)", all_accounts.len());
    println!();

    Ok(())
}

/// Example 4: Secure query with user input - PRODUCTION PATTERN
///
/// CRITICAL: This shows the correct way to handle user input in SOQL queries.
/// Always escape user input to prevent SOQL injection attacks!
async fn example_secure_query_user_input(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: Secure Query with User Input");
    println!("----------------------------------------");

    // Simulate user input (potentially malicious)
    let user_input = "O'Brien's Company"; // Contains single quote
    let malicious_input = "'; DELETE FROM Account--"; // SQL injection attempt

    // CORRECT: Always escape user input
    let safe_name = soql::escape_string(user_input);
    let query = format!("SELECT Id, Name FROM Account WHERE Name = '{}'", safe_name);
    println!("Safe query: {}", query);

    let accounts: Vec<Account> = client.query_all(&query).await?;
    println!("✓ Found {} accounts", accounts.len());

    // Show how injection attempt is prevented
    let safe_malicious = soql::escape_string(malicious_input);
    println!("\nInjection prevention:");
    println!("  Input: {}", malicious_input);
    println!("  Escaped: {}", safe_malicious);
    println!("  Result: The injection attempt is safely escaped");
    println!();

    Ok(())
}

/// Example 5: Secure LIKE query with wildcard escaping
async fn example_secure_like_query(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 5: Secure LIKE Query");
    println!("----------------------------");

    // User input for pattern matching
    let user_pattern = "tech%"; // User might try to inject wildcards

    // CORRECT: Use escape_like for LIKE patterns (escapes %, _ and other special chars)
    let safe_pattern = soql::escape_like(user_pattern);
    let query = format!(
        "SELECT Id, Name FROM Account WHERE Name LIKE '%{}%'",
        safe_pattern
    );

    println!("Query: {}", query);

    let accounts: Vec<Account> = client.query_all(&query).await?;
    println!("✓ Found {} accounts", accounts.len());
    println!();

    Ok(())
}

/// Example 6: Field validation
async fn example_field_validation(
    client: &SalesforceRestClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 6: Field Validation");
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

    println!("Original fields: {:?}", user_fields);
    println!("Safe fields: {:?}", safe_fields);

    // Build SELECT clause with safe fields
    if let Some(select_clause) = soql::build_safe_select(&safe_fields) {
        let query = format!("SELECT {} FROM Account LIMIT 5", select_clause);
        println!("Query: {}", query);

        let result: busbar_sf_client::QueryResult<serde_json::Value> =
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
