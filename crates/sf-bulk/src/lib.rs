//! # busbar-sf-bulk
//!
//! Salesforce Bulk API 2.0 client for large-scale data operations.
//!
//! ## Features
//!
//! - **Ingest Jobs** - Insert, Update, Upsert, Delete, Hard Delete
//! - **Query Jobs** - Query and QueryAll for large datasets
//! - **Job Management** - Create, monitor, abort, and delete jobs
//! - **CSV Support** - Native CSV data handling
//! - **Automatic Pagination** - Handle large result sets automatically
//!
//! ## Example
//!
//! ```rust,ignore
//! use busbar_sf_bulk::{BulkApiClient, BulkOperation, CreateIngestJobRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), busbar_sf_bulk::Error> {
//!     let client = BulkApiClient::new(
//!         "https://myorg.my.salesforce.com",
//!         "access_token",
//!     )?;
//!
//!     // High-level ingest operation
//!     let csv_data = "Name,Industry\nAcme Corp,Technology\nGlobal Inc,Finance";
//!     let result = client
//!         .execute_ingest("Account", BulkOperation::Insert, csv_data, None)
//!         .await?;
//!
//!     println!("Processed {} records", result.job.number_records_processed);
//!
//!     // High-level query operation
//!     let query_result = client
//!         .execute_query("SELECT Id, Name FROM Account LIMIT 10000")
//!         .await?;
//!
//!     if let Some(csv) = query_result.results {
//!         println!("Retrieved CSV:\n{}", csv);
//!     }
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
