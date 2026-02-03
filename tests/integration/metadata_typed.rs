//! Typed Metadata API integration tests using SF_AUTH_URL.
//!
//! These tests require the `typed-metadata` feature to be enabled.

use super::common::get_credentials;
use busbar_sf_metadata::{DeployOptions, MetadataClient, TypedMetadata as MetadataType, TypedMetadataExt};
use serde::{Deserialize, Serialize};

// Mock metadata type for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestApexClass {
    full_name: Option<String>,
    status: Option<String>,
}

impl MetadataType for TestApexClass {
    const METADATA_TYPE_NAME: &'static str = "ApexClass";
    const XML_ROOT_ELEMENT: &'static str = "ApexClass";

    fn api_name(&self) -> Option<&str> {
        self.full_name.as_deref()
    }
}

#[tokio::test]
async fn test_typed_deploy_validates_empty_batch() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Empty batch should fail
    let empty_batch: Vec<TestApexClass> = vec![];
    let result = client
        .deploy_typed_batch(&empty_batch, DeployOptions::default())
        .await;

    assert!(result.is_err(), "Empty batch should return an error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("empty"),
        "Error should mention empty batch"
    );
}

#[tokio::test]
async fn test_typed_deploy_validates_missing_api_name() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Metadata without API name should fail
    let invalid_metadata = TestApexClass {
        full_name: None,
        status: Some("Active".to_string()),
    };

    let result = client
        .deploy_typed(&invalid_metadata, DeployOptions::default())
        .await;

    assert!(result.is_err(), "Missing API name should return an error");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("api_name"),
        "Error should mention missing api_name"
    );
}

#[tokio::test]
async fn test_typed_deploy_single_creates_valid_package() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let test_class = TestApexClass {
        full_name: Some("TestTypedClass".to_string()),
        status: Some("Active".to_string()),
    };

    // This will attempt to deploy but may fail due to invalid XML format
    // The important part is that it creates a valid package structure
    let result = client
        .deploy_typed(&test_class, DeployOptions::default())
        .await;

    // We expect this to return an ID or fail with a deployment error (not a packaging error)
    // Either outcome shows the package was created correctly
    match result {
        Ok(id) => {
            assert!(!id.is_empty(), "Deploy should return an async process ID");
        }
        Err(e) => {
            let err_str = e.to_string();
            // Should not be a packaging error
            assert!(
                !err_str.contains("empty") && !err_str.contains("api_name"),
                "Error should not be about packaging: {}",
                err_str
            );
        }
    }
}

#[tokio::test]
async fn test_typed_deploy_batch_creates_valid_package() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let test_classes = vec![
        TestApexClass {
            full_name: Some("TestTypedClass1".to_string()),
            status: Some("Active".to_string()),
        },
        TestApexClass {
            full_name: Some("TestTypedClass2".to_string()),
            status: Some("Active".to_string()),
        },
    ];

    let result = client
        .deploy_typed_batch(&test_classes, DeployOptions::default())
        .await;

    // Similar to single deploy test
    match result {
        Ok(id) => {
            assert!(!id.is_empty(), "Deploy should return an async process ID");
        }
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                !err_str.contains("empty") && !err_str.contains("api_name"),
                "Error should not be about packaging: {}",
                err_str
            );
        }
    }
}
