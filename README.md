# busbar-sf-api

[![Crates.io](https://img.shields.io/crates/v/busbar-sf-api.svg)](https://crates.io/crates/busbar-sf-api)
[![Documentation](https://docs.rs/busbar-sf-api/badge.svg)](https://docs.rs/busbar-sf-api)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A comprehensive Salesforce API client library for Rust, providing type-safe access to Salesforce APIs with built-in authentication, retry logic, and error handling.

## Features

- 🔐 **Authentication** - OAuth 2.0 flows, JWT Bearer, and credentials management
- 🚀 **REST API** - CRUD operations, queries, composite requests, and collections
- 🛡️ **QueryBuilder** - Fluent API with automatic SOQL injection prevention (secure by default)
- 📦 **Bulk API 2.0** - Large-scale data operations with efficient processing
- 🛠️ **Tooling API** - Apex operations, debug logs, and code coverage
- 📋 **Metadata API** - Deploy and retrieve Salesforce metadata
- 🔄 **Async/Await** - Built on Tokio for high-performance async operations
- 🔁 **Retry Logic** - Automatic retries with exponential backoff
- 🔒 **Security** - Sensitive data redaction in debug output and logging
- 📊 **Tracing** - Built-in tracing support for observability

## Crates

This workspace includes the following crates:

- **[busbar-sf-client](crates/sf-client)** - Core HTTP client infrastructure with retry, compression, and rate limiting
- **[busbar-sf-auth](crates/sf-auth)** - Authentication: OAuth 2.0 flows, JWT Bearer, credentials management
- **[busbar-sf-rest](crates/sf-rest)** - REST API: CRUD, Query, Describe, Composite, Collections
- **[busbar-sf-tooling](crates/sf-tooling)** - Tooling API: Apex operations, debug logs, code coverage
- **[busbar-sf-bulk](crates/sf-bulk)** - Bulk API 2.0: Large-scale data operations
- **[busbar-sf-metadata](crates/sf-metadata)** - Metadata API: Deploy and retrieve metadata

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
busbar-sf-api = "0.1"
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

Or install individual crates as needed:

```toml
[dependencies]
busbar-sf-auth = "0.1"
busbar-sf-rest = "0.1"
```

## Quick Start

### Safe Query Builder (Recommended)

The QueryBuilder provides a fluent API with automatic SOQL injection prevention:

```rust
use busbar_sf_auth::SalesforceCredentials;
use busbar_sf_rest::{QueryBuilder, SalesforceRestClient};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Account {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = SalesforceCredentials::from_sfdx_alias("my-org").await?;
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    )?;

    // User input is automatically escaped - safe by default!
    let user_input = "O'Brien's Company";
    
    let accounts: Vec<Account> = QueryBuilder::new("Account")?
        .select(&["Id", "Name", "Industry"])
        .where_eq("Name", user_input)?  // Automatically escaped!
        .limit(10)
        .execute(&client)
        .await?;

    for account in accounts {
        println!("{}: {}", account.id, account.name);
    }

    Ok(())
}
```

### Using Credentials from Salesforce CLI

```rust
use busbar_sf_auth::SalesforceCredentials;
use busbar_sf_rest::SalesforceRestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get credentials from SF CLI
    let creds = SalesforceCredentials::from_sfdx_alias("my-org").await?;

    // Create REST client
    let client = SalesforceRestClient::new(
        creds.instance_url(),
        creds.access_token(),
    )?;

    // Query accounts
    let accounts: Vec<serde_json::Value> = client
        .query_all("SELECT Id, Name FROM Account LIMIT 10")
        .await?;

    println!("Retrieved {} accounts", accounts.len());
    // Note: In production, be cautious about logging sensitive data
    // For debugging:
    // for account in &accounts {
    //     println!("Account: {}", account["Name"]);
    // }

    Ok(())
}
```

### OAuth 2.0 Authentication

```rust
use busbar_sf_auth::{OAuthConfig, OAuthFlow};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load credentials from environment variables - NEVER hardcode credentials!
    let config = OAuthConfig {
        client_id: env::var("SF_CLIENT_ID")?,
        client_secret: env::var("SF_CLIENT_SECRET").ok(),
        redirect_uri: env::var("SF_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8080/callback".to_string()),
        ..Default::default()
    };

    let flow = OAuthFlow::new(config);
    
    // Get authorization URL
    let auth_url = flow.authorization_url(&["api", "refresh_token"]);
    println!("Visit: {}", auth_url);

    // After user authorizes, exchange code for token
    let token = flow.exchange_code("authorization_code").await?;
    
    Ok(())
}
```

### Bulk API Operations

```rust
use busbar_sf_bulk::{BulkClient, BulkOperation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Account {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Industry")]
    industry: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = SalesforceCredentials::from_sfdx_alias("my-org").await?;
    let client = BulkClient::new(
        creds.instance_url(),
        creds.access_token(),
    )?;

    // Create insert job
    let accounts = vec![
        Account { name: "Acme Corp".to_string(), industry: Some("Technology".to_string()) },
        Account { name: "Global Industries".to_string(), industry: Some("Manufacturing".to_string()) },
    ];

    let job_id = client
        .create_job("Account", BulkOperation::Insert)
        .await?;

    client.upload_job_data(&job_id, &accounts).await?;
    let results = client.wait_for_job(&job_id).await?;

    println!("Processed {} records", results.number_records_processed);
    
    Ok(())
}
```

### Metadata API

```rust
use busbar_sf_metadata::MetadataClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = SalesforceCredentials::from_sfdx_alias("my-org").await?;
    let client = MetadataClient::new(
        creds.instance_url(),
        creds.access_token(),
    ).await?;

    // Retrieve metadata
    let metadata_types = vec!["ApexClass", "ApexTrigger"];
    let retrieve_id = client.retrieve(&metadata_types).await?;
    
    // Check retrieve status
    let status = client.check_retrieve_status(&retrieve_id).await?;
    
    if status.done {
        println!("Retrieve complete!");
    }

    Ok(())
}
```

## Examples

See the [examples](examples) directory for comprehensive examples:

- **[basic_auth.rs](examples/basic_auth.rs)** - Authentication methods (OAuth, JWT, SFDX, environment variables)
- **[rest_crud.rs](examples/rest_crud.rs)** - REST API CRUD operations
- **[queries.rs](examples/queries.rs)** - SOQL queries with security best practices
- **[error_handling.rs](examples/error_handling.rs)** - Error handling patterns and retry logic
- **[bulk_operations.rs](examples/bulk_operations.rs)** - Bulk API 2.0 insert, update, and query operations

Run any example with:
```bash
cargo run --example basic_auth
cargo run --example rest_crud
cargo run --example queries
```

## Security

This library is designed with security in mind. See [SECURITY.md](SECURITY.md) for full details.

**Key Security Features:**
- ✅ **QueryBuilder** - Fluent API with automatic SOQL injection prevention (RECOMMENDED)
- ✅ Automatic credential redaction in logs and debug output
- ✅ SOQL injection prevention utilities (escape_string, field validation)
- ✅ URL parameter encoding to prevent path traversal
- ✅ Secure token storage with restrictive file permissions
- ✅ Input validation for IDs, field names, and SObject names

**Security Best Practices:**
```rust
use busbar_sf_rest::QueryBuilder;

// RECOMMENDED - QueryBuilder is safe by default
let accounts: Vec<Account> = QueryBuilder::new("Account")?
    .select(&["Id", "Name"])
    .where_eq("Name", user_input)?  // Automatically escaped!
    .execute(&client)
    .await?;

// Alternative - Manual escaping (easy to forget!)
use busbar_sf_client::security::soql;
let safe_name = soql::escape_string(user_input);
let query = format!("SELECT Id FROM Account WHERE Name = '{}'", safe_name);
```

For security vulnerabilities, see our [Security Policy](SECURITY.md)

## Requirements

- Rust 1.88 or later
- Tokio runtime for async operations

## Documentation

- 📖 [API Documentation](https://docs.rs/busbar-sf-api) - Complete API reference
- 🔒 [Security Policy](SECURITY.md) - Security best practices and vulnerability reporting
- 📋 [Code Review](CODE_REVIEW.md) - Comprehensive code review for v0.1.0 release
- 📝 [Changelog](CHANGELOG.md) - Version history and release notes
- 🤝 [Contributing Guidelines](CONTRIBUTING.md) - How to contribute

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development

```bash
# Clone the repository
git clone https://github.com/composable-delivery/busbar-sf-api.git
cd busbar-sf-api

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run linter
cargo clippy --workspace -- -D warnings

# Format code
cargo fmt --workspace
```

## Community

- 💬 [GitHub Discussions](https://github.com/composable-delivery/busbar-sf-api/discussions) - Ask questions, share ideas, and discuss the project
- 🐛 [Issue Tracker](https://github.com/composable-delivery/busbar-sf-api/issues) - Report bugs and request features
- 📖 [Documentation](https://docs.rs/busbar-sf-api) - API documentation

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
