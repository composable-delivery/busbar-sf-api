//! Integration test suite (requires a real Salesforce org).
//!
//! Run all integration tests with:
//!   SF_AUTH_URL=... cargo test --test integration -- --nocapture

#[path = "integration/auth.rs"]
mod auth;
#[path = "integration/bulk.rs"]
mod bulk;
#[path = "integration/common.rs"]
mod common;
#[path = "integration/data_cloud.rs"]
mod data_cloud;
#[path = "integration/metadata.rs"]
mod metadata;
#[path = "integration/rest.rs"]
mod rest;
#[path = "integration/tooling.rs"]
mod tooling;
