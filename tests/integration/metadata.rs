//! Metadata API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_metadata::{DeployOptions, DeployStatus, MetadataClient};
use std::io::Write;
use std::time::Duration;

// ============================================================================
// Metadata API Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_describe_types() {
    let creds = get_credentials().await;
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
    let creds = get_credentials().await;
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
    let creds = get_credentials().await;
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
    let creds = get_credentials().await;
    let client = MetadataClient::from_parts(creds.instance_url(), "invalid-token");

    let result = client.describe_metadata().await;

    assert!(result.is_err(), "Describe with invalid token should fail");
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a minimal metadata package for testing deployments.
/// Returns a zip file containing a simple Apex class.
fn create_test_package() -> Vec<u8> {
    let mut zip_buffer = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));

    // Create package.xml
    let package_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    <types>
        <members>TestDeployClass</members>
        <name>ApexClass</name>
    </types>
    <version>62.0</version>
</Package>"#;

    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::DEFLATE);

    zip.start_file("package.xml", options).unwrap();
    zip.write_all(package_xml.as_bytes()).unwrap();

    // Create a simple Apex class
    let apex_class = r#"public class TestDeployClass {
    public static String getMessage() {
        return 'Hello from TestDeployClass';
    }
}"#;

    zip.start_file("classes/TestDeployClass.cls", options)
        .unwrap();
    zip.write_all(apex_class.as_bytes()).unwrap();

    // Create meta.xml for the class
    let meta_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ApexClass xmlns="http://soap.sforce.com/2006/04/metadata">
    <apiVersion>62.0</apiVersion>
    <status>Active</status>
</ApexClass>"#;

    zip.start_file("classes/TestDeployClass.cls-meta.xml", options)
        .unwrap();
    zip.write_all(meta_xml.as_bytes()).unwrap();

    let _ = zip.finish().unwrap();

    zip_buffer
}

// ============================================================================
// Deploy and Cancel Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_cancel_deploy() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Create a test package
    let package = create_test_package();

    // Start a deployment
    let options = DeployOptions {
        check_only: true, // Use validation mode to avoid actual changes
        ..Default::default()
    };

    let deploy_id = client
        .deploy(&package, options)
        .await
        .expect("Deploy should start successfully");

    assert!(!deploy_id.is_empty(), "Should return a deploy ID");

    // Immediately try to cancel the deployment
    let cancel_result = client
        .cancel_deploy(&deploy_id)
        .await
        .expect("Cancel deploy should succeed");

    assert_eq!(
        cancel_result.id, deploy_id,
        "Cancel result should reference the same deploy ID"
    );

    // Wait a moment and check status to see if cancellation took effect
    tokio::time::sleep(Duration::from_secs(2)).await;

    let status = client
        .check_deploy_status(&deploy_id, false)
        .await
        .expect("Check deploy status should succeed");

    // The deployment should be either Canceling or Canceled
    assert!(
        matches!(
            status.status,
            DeployStatus::Canceling | DeployStatus::Canceled | DeployStatus::Succeeded
        ),
        "Deploy should be in canceling, canceled, or already succeeded state, got: {:?}",
        status.status
    );
}

#[tokio::test]
async fn test_metadata_deploy_recent_validation() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Create a test package
    let package = create_test_package();

    // First, validate the deployment (check_only=true)
    let validate_options = DeployOptions {
        check_only: true,
        run_all_tests: false, // Don't run tests in scratch orgs to save time
        ..Default::default()
    };

    let validation_id = client
        .deploy(&package, validate_options)
        .await
        .expect("Validation deploy should start successfully");

    assert!(!validation_id.is_empty(), "Should return a validation ID");

    // Poll for validation completion (with timeout)
    let validation_result = client
        .poll_deploy_status(
            &validation_id,
            Duration::from_secs(300),
            Duration::from_secs(5),
        )
        .await
        .expect("Validation should complete successfully");

    assert!(
        validation_result.success,
        "Validation should succeed, error: {:?}",
        validation_result.error_message
    );
    assert_eq!(
        validation_result.status,
        DeployStatus::Succeeded,
        "Validation status should be Succeeded"
    );

    // Now quick-deploy using the validated deployment ID
    let quick_deploy_id = client
        .deploy_recent_validation(&validation_id)
        .await
        .expect("Deploy recent validation should succeed");

    assert!(
        !quick_deploy_id.is_empty(),
        "Should return a quick-deploy ID"
    );
    assert_ne!(
        quick_deploy_id, validation_id,
        "Quick-deploy ID should be different from validation ID"
    );

    // Poll for quick-deploy completion
    let deploy_result = client
        .poll_deploy_status(
            &quick_deploy_id,
            Duration::from_secs(300),
            Duration::from_secs(5),
        )
        .await
        .expect("Quick-deploy should complete successfully");

    assert!(
        deploy_result.success,
        "Quick-deploy should succeed, error: {:?}",
        deploy_result.error_message
    );
    assert_eq!(
        deploy_result.status,
        DeployStatus::Succeeded,
        "Quick-deploy status should be Succeeded"
    );
}

// ============================================================================
// Error Handling Tests for New Operations
// ============================================================================

#[tokio::test]
async fn test_metadata_cancel_deploy_invalid_id() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Try to cancel a non-existent deployment
    let result = client.cancel_deploy("0Af000000000000AAA").await;

    assert!(
        result.is_err(),
        "Canceling a non-existent deploy should fail"
    );
}

#[tokio::test]
async fn test_metadata_deploy_recent_validation_invalid_id() {
    let Some(creds) = require_credentials().await else {
        return;
    };
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Try to quick-deploy with a non-existent validation ID
    let result = client.deploy_recent_validation("0Af000000000000AAA").await;

    assert!(
        result.is_err(),
        "Quick-deploying a non-existent validation should fail"
    );
}
