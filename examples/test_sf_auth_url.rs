//! Test SF_AUTH_URL authentication
//!
//! Run with: `SF_AUTH_URL=<auth_url> cargo run --example test_sf_auth_url`

use busbar_sf_auth::{Credentials, SalesforceCredentials};

#[tokio::main]
async fn main() {
    let auth_url = std::env::var("SF_AUTH_URL").expect("SF_AUTH_URL must be set");

    println!("Testing SF_AUTH_URL authentication...");

    match SalesforceCredentials::from_sfdx_auth_url(&auth_url).await {
        Ok(creds) => {
            println!("✓ Successfully authenticated!");
            println!("Instance URL: {}", creds.instance_url());
            println!("API Version: {}", creds.api_version());
            println!("Access token length: {}", creds.access_token().len());
        }
        Err(e) => {
            eprintln!("✗ Authentication failed: {:?}", e);
            eprintln!("\nError details: {}", e);
            std::process::exit(1);
        }
    }
}
