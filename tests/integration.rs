//! Integration test suite (requires a real Salesforce org).
//!
//! Run all integration tests with:
//!   SF_AUTH_URL=... cargo test --test integration -- --ignored --nocapture

#[path = "integration/bulk.rs"]
mod bulk;
#[path = "integration/common.rs"]
mod common;
#[path = "integration/examples.rs"]
mod examples;
#[path = "integration/metadata.rs"]
mod metadata;
#[path = "integration/rest.rs"]
mod rest;
#[path = "integration/scratch.rs"]
mod scratch;
#[path = "integration/tooling.rs"]
mod tooling;
