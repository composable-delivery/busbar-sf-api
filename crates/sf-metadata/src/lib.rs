//! # busbar-sf-metadata
//!
//! Salesforce Metadata API client for deploying and retrieving metadata.
//!
//! ## Features
//!
//! - **Deploy** - Deploy metadata packages via SOAP API
//! - **Retrieve** - Retrieve metadata from an org
//! - **List Metadata** - List metadata components by type
//! - **Describe Metadata** - Get available metadata types
//! - **Status Polling** - Automatic polling for async operations
//!
//! ## Example
//!
//! ```rust,ignore
//! use busbar_sf_metadata::{MetadataClient, DeployOptions};
//! use busbar_sf_auth::SalesforceCredentials;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), busbar_sf_metadata::Error> {
//!     let creds = SalesforceCredentials::from_env()?;
//!     let client = MetadataClient::new(&creds)?;
//!
//!     // Deploy a package
//!     let zip_bytes = std::fs::read("package.zip")?;
//!     let async_id = client.deploy(&zip_bytes, DeployOptions::default()).await?;
//!
//!     // Poll for completion
//!     let result = client.poll_deploy_status(
//!         &async_id,
//!         Duration::from_secs(600),
//!         Duration::from_secs(5),
//!     ).await?;
//!
//!     println!("Deploy status: {:?}", result.status);
//!
//!     // Retrieve metadata (with secure XML escaping)
//!     let manifest = PackageManifest::new("62.0")
//!         .add_type("ApexClass", vec!["*".to_string()])
//!         .add_type("ApexTrigger", vec!["*".to_string()]);
//!     let retrieve_id = client.retrieve_unpackaged(&manifest).await?;
//!
//!     // List metadata
//!     let apex_classes = client.list_metadata("ApexClass", None).await?;
//!     for class in apex_classes {
//!         println!("  {}", class.full_name);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod client;
mod deploy;
mod describe;
mod error;
mod list;
mod retrieve;
mod types;

pub use client::MetadataClient;
pub use deploy::{ComponentFailure, DeployOptions, DeployResult, DeployStatus};
pub use describe::{
    DescribeMetadataResult, DescribeValueTypeResult, MetadataType, PicklistEntry, ValueTypeField,
};
pub use error::{Error, ErrorKind, Result};
pub use list::MetadataComponent;
pub use retrieve::{
    PackageManifest, PackageTypeMembers, RetrieveMessage, RetrieveOptions, RetrieveResult,
    RetrieveStatus,
};
pub use types::{
    ComponentSuccess, FileProperties, SoapFault, TestFailure, TestLevel, DEFAULT_API_VERSION,
};
