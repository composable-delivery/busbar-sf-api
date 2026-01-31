//! Metadata API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_metadata::{DeployOptions, DeployStatus, MetadataClient};
use serde_json::json;
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
// PR #60: describeValueType Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_describe_value_type() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client
        .describe_value_type("{http://soap.sforce.com/2006/04/metadata}CustomObject")
        .await
        .expect("describeValueType should succeed");

    assert!(
        !result.value_type_fields.is_empty(),
        "Should return value type fields"
    );

    let has_full_name = result
        .value_type_fields
        .iter()
        .any(|f| f.name == "fullName");
    assert!(has_full_name, "Should have a fullName field");
}

// ============================================================================
// PR #57: cancelDeploy + deployRecentValidation Tests
// ============================================================================

/// Create a minimal deploy package zip for testing.
fn create_test_package() -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("package.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    <version>62.0</version>
</Package>"#,
        )
        .unwrap();
        zip.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn test_metadata_cancel_deploy() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let package_zip = create_test_package();
    let async_id = client
        .deploy(&package_zip, DeployOptions::default())
        .await
        .expect("Deploy should succeed");

    let cancel_result = client.cancel_deploy(&async_id).await;
    assert!(
        cancel_result.is_ok(),
        "cancel_deploy should not error (got {:?})",
        cancel_result
    );

    // Poll to let SF finish cancelling
    tokio::time::sleep(Duration::from_secs(2)).await;

    let status = client
        .check_deploy_status(&async_id, false)
        .await
        .expect("check_deploy_status after cancel should succeed");

    assert!(
        status.status == DeployStatus::Canceled
            || status.status == DeployStatus::Canceling
            || status.status == DeployStatus::Succeeded
            || status.status == DeployStatus::Failed,
        "Status should be Canceled, Canceling, Succeeded, or Failed but was {:?}",
        status.status
    );
}

#[tokio::test]
async fn test_metadata_deploy_recent_validation() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let package_zip = create_test_package();
    let mut opts = DeployOptions::default();
    opts.check_only = true;

    let async_id = client
        .deploy(&package_zip, opts)
        .await
        .expect("Validation deploy should succeed");

    let result = client
        .poll_deploy_status(&async_id, Duration::from_secs(120), Duration::from_secs(3))
        .await
        .expect("Validation should complete");

    assert!(result.success, "Validation deploy should succeed");

    let quick_deploy_result = client.deploy_recent_validation(&async_id).await;
    // Quick-deploy may fail if the validation wasn't test-covered, that's OK.
    // We just verify the call round-trips without panicking.
    match quick_deploy_result {
        Ok(new_id) => {
            assert!(!new_id.is_empty(), "Should return a non-empty async ID");
        }
        Err(e) => {
            let msg = format!("{:?}", e);
            assert!(
                msg.contains("SoapFault") || msg.contains("INVALID"),
                "Error should be a SOAP fault or validation error, got: {}",
                msg
            );
        }
    }
}

#[tokio::test]
async fn test_metadata_cancel_deploy_invalid_id() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client.cancel_deploy("0Af000000000000AAA").await;
    assert!(result.is_err(), "cancel_deploy with invalid ID should fail");
}

#[tokio::test]
async fn test_metadata_deploy_recent_validation_invalid_id() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client.deploy_recent_validation("0Af000000000000AAA").await;
    assert!(
        result.is_err(),
        "deploy_recent_validation with invalid ID should fail"
    );
}

// ============================================================================
// PR #58: CRUD Sync Operations Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_crud_custom_label_lifecycle() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let label_name = format!("BusbarTest_{}", chrono::Utc::now().timestamp_millis());

    // Create
    let create_results = client
        .create_metadata(
            "CustomLabel",
            &[json!({
                "fullName": label_name,
                "language": "en_US",
                "protected": false,
                "shortDescription": label_name,
                "value": "Initial value"
            })],
        )
        .await
        .expect("create_metadata should succeed");

    assert_eq!(create_results.len(), 1);
    assert!(
        create_results[0].success,
        "Create should succeed: {:?}",
        create_results[0].errors
    );

    // Read
    let read_result = client
        .read_metadata("CustomLabel", &[&label_name])
        .await
        .expect("read_metadata should succeed");

    assert!(
        !read_result.records.is_empty(),
        "Should read back the label"
    );

    // Update
    let update_results = client
        .update_metadata(
            "CustomLabel",
            &[json!({
                "fullName": label_name,
                "language": "en_US",
                "protected": false,
                "shortDescription": label_name,
                "value": "Updated value"
            })],
        )
        .await
        .expect("update_metadata should succeed");

    assert_eq!(update_results.len(), 1);
    assert!(
        update_results[0].success,
        "Update should succeed: {:?}",
        update_results[0].errors
    );

    // Delete
    let delete_results = client
        .delete_metadata("CustomLabel", &[&label_name])
        .await
        .expect("delete_metadata should succeed");

    assert_eq!(delete_results.len(), 1);
    assert!(
        delete_results[0].success,
        "Delete should succeed: {:?}",
        delete_results[0].errors
    );
}

#[tokio::test]
async fn test_metadata_upsert_custom_label() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let label_name = format!("BusbarUpsert_{}", chrono::Utc::now().timestamp_millis());

    // Upsert (create)
    let upsert_results = client
        .upsert_metadata(
            "CustomLabel",
            &[json!({
                "fullName": label_name,
                "language": "en_US",
                "protected": false,
                "shortDescription": label_name,
                "value": "Upserted value"
            })],
        )
        .await
        .expect("upsert_metadata should succeed");

    assert_eq!(upsert_results.len(), 1);
    assert!(
        upsert_results[0].success,
        "Upsert should succeed: {:?}",
        upsert_results[0].errors
    );
    assert!(
        upsert_results[0].created,
        "Should be created on first upsert"
    );

    // Cleanup
    let _ = client.delete_metadata("CustomLabel", &[&label_name]).await;
}

#[tokio::test]
async fn test_metadata_rename_custom_label() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let ts = chrono::Utc::now().timestamp_millis();
    let old_name = format!("BusbarRenOld_{}", ts);
    let new_name = format!("BusbarRenNew_{}", ts);

    // Create
    let _ = client
        .create_metadata(
            "CustomLabel",
            &[json!({
                "fullName": old_name,
                "language": "en_US",
                "protected": false,
                "shortDescription": old_name,
                "value": "Rename test"
            })],
        )
        .await
        .expect("create should succeed");

    // Rename
    let rename_result = client
        .rename_metadata("CustomLabel", &old_name, &new_name)
        .await
        .expect("rename_metadata should succeed");

    assert!(
        rename_result.success,
        "Rename should succeed: {:?}",
        rename_result.errors
    );

    // Cleanup
    let _ = client.delete_metadata("CustomLabel", &[&new_name]).await;
}

#[tokio::test]
async fn test_metadata_create_multiple_labels() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let ts = chrono::Utc::now().timestamp_millis();
    let names: Vec<String> = (0..3)
        .map(|i| format!("BusbarMulti_{}_{}", ts, i))
        .collect();

    let metadata_objects: Vec<serde_json::Value> = names
        .iter()
        .map(|name| {
            json!({
                "fullName": name,
                "language": "en_US",
                "protected": false,
                "shortDescription": name,
                "value": format!("Value for {}", name)
            })
        })
        .collect();

    let create_results = client
        .create_metadata("CustomLabel", &metadata_objects)
        .await
        .expect("create_metadata should succeed");

    assert_eq!(create_results.len(), 3);
    for result in &create_results {
        assert!(result.success, "Create should succeed: {:?}", result.errors);
    }

    // Cleanup
    let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let _ = client.delete_metadata("CustomLabel", &name_refs).await;
}

#[tokio::test]
async fn test_metadata_crud_validation_max_limit() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let objects: Vec<serde_json::Value> = (0..11)
        .map(|i| json!({"fullName": format!("Obj{}", i)}))
        .collect();

    let result = client.create_metadata("CustomLabel", &objects).await;
    assert!(result.is_err(), "Should reject more than 10 components");

    let names: Vec<&str> = (0..11).map(|_| "x").collect();
    let result = client.delete_metadata("CustomLabel", &names).await;
    assert!(result.is_err(), "Should reject more than 10 components");
}

#[tokio::test]
async fn test_metadata_read_nonexistent() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client
        .read_metadata("CustomLabel", &["NonExistentLabel_xyz_123"])
        .await
        .expect("read_metadata should not error for nonexistent");

    // Salesforce returns an empty or partial result for missing components
    // Just verify we get a response without error
    let _ = result.records;
}

#[tokio::test]
async fn test_metadata_delete_nonexistent() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let result = client
        .delete_metadata("CustomLabel", &["NonExistentLabel_xyz_123"])
        .await
        .expect("delete_metadata should not panic");

    assert_eq!(result.len(), 1);
    // SF may return success=false for nonexistent, that's fine
}

#[tokio::test]
async fn test_metadata_create_with_xml_escaping() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let label_name = format!("BusbarEsc_{}", chrono::Utc::now().timestamp_millis());

    let create_results = client
        .create_metadata(
            "CustomLabel",
            &[json!({
                "fullName": label_name,
                "language": "en_US",
                "protected": false,
                "shortDescription": label_name,
                "value": "Value with <special> & \"characters\""
            })],
        )
        .await
        .expect("create_metadata with special chars should succeed");

    assert_eq!(create_results.len(), 1);
    assert!(
        create_results[0].success,
        "Create with special chars should succeed: {:?}",
        create_results[0].errors
    );

    // Cleanup
    let _ = client.delete_metadata("CustomLabel", &[&label_name]).await;
}
