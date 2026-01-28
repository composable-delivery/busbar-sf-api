//! Metadata API integration tests using SF_AUTH_URL.

use super::common::require_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_metadata::MetadataClient;

// ============================================================================
// Metadata API Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_describe_types() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client
        .describe_metadata()
        .await
        .expect("Describe metadata should succeed");

    assert!(
        !result.metadata_objects.is_empty(),
        "Should return metadata objects"
    );
}

#[tokio::test]
async fn test_metadata_list_types() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let types = client
        .list_metadata_types()
        .await
        .expect("List metadata types should succeed");

    assert!(!types.is_empty(), "Should return metadata type names");
    assert!(
        types
            .iter()
            .any(|t| t == "ApexClass" || t == "CustomObject"),
        "Should include ApexClass or CustomObject metadata types"
    );
}

#[tokio::test]
async fn test_metadata_list_custom_objects() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client
        .list_metadata("CustomObject", None)
        .await
        .expect("List metadata should succeed");

    for component in &result {
        assert_eq!(component.metadata_type, "CustomObject");
        assert!(!component.full_name.is_empty());
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_error_invalid_token() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::from_parts(creds.instance_url(), "invalid-token");

    let result = client.describe_metadata().await;

    assert!(result.is_err(), "Describe with invalid token should fail");
}
