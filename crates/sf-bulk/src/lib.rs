//! # busbar-sf-bulk
//!
//! Salesforce Bulk API 2.0 client for large-scale data operations.
//!
//! ## Features
//!
//! - **Ingest Jobs** - Insert, Update, Upsert, Delete, Hard Delete
//! - **Query Jobs** - Query and QueryAll for large datasets with automatic SOQL injection prevention
//! - **Job Management** - Create, monitor, abort, and delete jobs
//! - **CSV Support** - Native CSV data handling
//! - **Automatic Pagination** - Handle large result sets automatically
//! - **Security by Default** - QueryBuilder integration prevents SOQL injection
//!
//! ## Example - Safe Bulk Query
//!
//! ```rust,ignore
//! use busbar_sf_bulk::{BulkApiClient, QueryBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), busbar_sf_bulk::Error> {
//!     let client = BulkApiClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token",
//!     )?;
//!
//!     // User input is automatically escaped - safe by default!
//!     let user_input = "O'Brien's Company";
//!     let result = client.execute_query(
//!         QueryBuilder::new("Account")?
//!             .select(&["Id", "Name", "Industry"])
//!             .where_eq("Name", user_input)?  // Automatically escaped!
//!             .limit(10000)
//!     ).await?;
//!
//!     println!("Retrieved {} records", result.job.number_records_processed);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Example - Bulk Insert
//!
//! ```rust,ignore
//! use busbar_sf_bulk::{BulkApiClient, BulkOperation};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), busbar_sf_bulk::Error> {
//!     let client = BulkApiClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token",
//!     )?;
//!
//!     let csv_data = "Name,Industry\nAcme Corp,Technology\nGlobal Inc,Finance";
//!     let result = client
//!         .execute_ingest("Account", BulkOperation::Insert, csv_data, None)
//!         .await?;
//!
//!     println!("Processed {} records", result.job.number_records_processed);
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

pub use client::BulkApiClient;
pub use error::{Error, ErrorKind, Result};
pub use types::*;

// Re-export QueryBuilder when the feature is enabled for convenient access
#[cfg(feature = "query-builder")]
pub use busbar_sf_rest::QueryBuilder;
