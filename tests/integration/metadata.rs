//! Metadata API integration tests using SF_AUTH_URL.

use super::common::get_credentials;
use busbar_sf_auth::Credentials;
use busbar_sf_metadata::MetadataClient;
use serde_json::json;

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
// CRUD Operations Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_crud_custom_label_lifecycle() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Use a unique name with timestamp to avoid conflicts
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let label_name = format!("TestLabel_{}", timestamp);

    // 1. Create a CustomLabel
    let metadata = vec![json!({
        "fullName": label_name.clone(),
        "value": "Original Value",
        "language": "en_US",
        "protected": false,
        "categories": "Test"
    })];

    let create_results = client
        .create_metadata("CustomLabel", &metadata)
        .await
        .expect("Create metadata should succeed");

    assert_eq!(create_results.len(), 1, "Should return one result");
    assert!(
        create_results[0].success,
        "Create should succeed: {:?}",
        create_results[0].errors
    );
    assert_eq!(create_results[0].full_name, label_name);

    // 2. Read the created label
    let read_result = client
        .read_metadata("CustomLabel", &[&label_name])
        .await
        .expect("Read metadata should succeed");

    assert_eq!(read_result.records.len(), 1, "Should return one record");
    let record = &read_result.records[0];
    assert_eq!(
        record.get("fullName").and_then(|v| v.as_str()),
        Some(label_name.as_str())
    );

    // 3. Update the label
    let updated_metadata = vec![json!({
        "fullName": label_name.clone(),
        "value": "Updated Value",
        "language": "en_US",
        "protected": false,
        "categories": "Test"
    })];

    let update_results = client
        .update_metadata("CustomLabel", &updated_metadata)
        .await
        .expect("Update metadata should succeed");

    assert_eq!(update_results.len(), 1, "Should return one result");
    assert!(
        update_results[0].success,
        "Update should succeed: {:?}",
        update_results[0].errors
    );

    // 4. Delete the label
    let delete_results = client
        .delete_metadata("CustomLabel", &[&label_name])
        .await
        .expect("Delete metadata should succeed");

    assert_eq!(delete_results.len(), 1, "Should return one result");
    assert!(
        delete_results[0].success,
        "Delete should succeed: {:?}",
        delete_results[0].errors
    );
    assert_eq!(delete_results[0].full_name, label_name);
}

#[tokio::test]
async fn test_metadata_upsert_custom_label() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let label_name = format!("TestUpsertLabel_{}", timestamp);

    // First upsert (should create)
    let metadata = vec![json!({
        "fullName": label_name.clone(),
        "value": "Initial Value",
        "language": "en_US",
        "protected": false,
        "categories": "Test"
    })];

    let upsert_results = client
        .upsert_metadata("CustomLabel", &metadata)
        .await
        .expect("Upsert metadata should succeed");

    assert_eq!(upsert_results.len(), 1, "Should return one result");
    assert!(
        upsert_results[0].success,
        "Upsert should succeed: {:?}",
        upsert_results[0].errors
    );
    assert!(
        upsert_results[0].created,
        "First upsert should create the label"
    );

    // Second upsert (should update)
    let updated_metadata = vec![json!({
        "fullName": label_name.clone(),
        "value": "Updated Value",
        "language": "en_US",
        "protected": false,
        "categories": "Test"
    })];

    let upsert_results2 = client
        .upsert_metadata("CustomLabel", &updated_metadata)
        .await
        .expect("Second upsert should succeed");

    assert_eq!(upsert_results2.len(), 1, "Should return one result");
    assert!(
        upsert_results2[0].success,
        "Second upsert should succeed: {:?}",
        upsert_results2[0].errors
    );
    assert!(
        !upsert_results2[0].created,
        "Second upsert should update, not create"
    );

    // Cleanup
    let _ = client.delete_metadata("CustomLabel", &[&label_name]).await;
}

#[tokio::test]
async fn test_metadata_rename_custom_label() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let old_name = format!("TestRenameOld_{}", timestamp);
    let new_name = format!("TestRenameNew_{}", timestamp);

    // Create a label
    let metadata = vec![json!({
        "fullName": old_name.clone(),
        "value": "Test Value",
        "language": "en_US",
        "protected": false,
        "categories": "Test"
    })];

    let create_results = client
        .create_metadata("CustomLabel", &metadata)
        .await
        .expect("Create metadata should succeed");

    assert!(
        create_results[0].success,
        "Create should succeed: {:?}",
        create_results[0].errors
    );

    // Rename it
    let rename_result = client
        .rename_metadata("CustomLabel", &old_name, &new_name)
        .await
        .expect("Rename metadata should succeed");

    assert!(
        rename_result.success,
        "Rename should succeed: {:?}",
        rename_result.errors
    );

    // Cleanup - delete the renamed label
    let _ = client.delete_metadata("CustomLabel", &[&new_name]).await;
}

#[tokio::test]
async fn test_metadata_create_multiple_labels() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let label_names: Vec<String> = (1..=3)
        .map(|i| format!("TestMultiLabel_{}_{}", timestamp, i))
        .collect();

    // Create multiple labels
    let metadata: Vec<_> = label_names
        .iter()
        .map(|name| {
            json!({
                "fullName": name,
                "value": format!("Value for {}", name),
                "language": "en_US",
                "protected": false,
                "categories": "Test"
            })
        })
        .collect();

    let create_results = client
        .create_metadata("CustomLabel", &metadata)
        .await
        .expect("Create multiple metadata should succeed");

    assert_eq!(create_results.len(), 3, "Should return three results");
    for result in &create_results {
        assert!(
            result.success,
            "All creates should succeed: {:?}",
            result.errors
        );
    }

    // Read all created labels
    let label_name_refs: Vec<&str> = label_names.iter().map(|s| s.as_str()).collect();
    let read_result = client
        .read_metadata("CustomLabel", &label_name_refs)
        .await
        .expect("Read multiple metadata should succeed");

    assert_eq!(read_result.records.len(), 3, "Should return three records");

    // Cleanup - delete all labels
    let delete_results = client
        .delete_metadata("CustomLabel", &label_name_refs)
        .await
        .expect("Delete multiple metadata should succeed");

    assert_eq!(delete_results.len(), 3, "Should return three results");
    for result in &delete_results {
        assert!(
            result.success,
            "All deletes should succeed: {:?}",
            result.errors
        );
    }
}

#[tokio::test]
async fn test_metadata_crud_validation_max_limit() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Try to create more than 10 components (should fail)
    let metadata: Vec<_> = (1..=11)
        .map(|i| {
            json!({
                "fullName": format!("TestLabel_{}", i),
                "value": "Test",
                "language": "en_US",
                "protected": false
            })
        })
        .collect();

    let result = client.create_metadata("CustomLabel", &metadata).await;

    assert!(
        result.is_err(),
        "Creating more than 10 components should fail"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("10") || err_msg.contains("Maximum"),
        "Error should mention the 10 component limit"
    );
}

#[tokio::test]
async fn test_metadata_read_nonexistent() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Try to read a label that doesn't exist
    let result = client
        .read_metadata("CustomLabel", &["NonExistentLabel_12345"])
        .await
        .expect("Read should succeed even for nonexistent metadata");

    // The API returns a record with null/empty values for nonexistent metadata
    assert_eq!(result.records.len(), 1, "Should return one record");
}

#[tokio::test]
async fn test_metadata_delete_nonexistent() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    // Try to delete a label that doesn't exist
    let delete_results = client
        .delete_metadata("CustomLabel", &["NonExistentLabel_12345"])
        .await
        .expect("Delete should return results even for nonexistent metadata");

    assert_eq!(delete_results.len(), 1, "Should return one result");
    // Deleting nonexistent metadata typically fails with an error
    if !delete_results[0].success {
        assert!(
            !delete_results[0].errors.is_empty(),
            "Should have error details"
        );
    }
}

#[tokio::test]
async fn test_metadata_create_with_xml_escaping() {
    let creds = get_credentials().await;
    let client = MetadataClient::new(&creds).expect("Failed to create Metadata client");

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let label_name = format!("TestXMLLabel_{}", timestamp);

    // Create a label with special XML characters
    let metadata = vec![json!({
        "fullName": label_name.clone(),
        "value": "Test <value> & \"special\" 'chars'",
        "language": "en_US",
        "protected": false,
        "categories": "Test & Category"
    })];

    let create_results = client
        .create_metadata("CustomLabel", &metadata)
        .await
        .expect("Create with XML characters should succeed");

    assert!(
        create_results[0].success,
        "Create should succeed: {:?}",
        create_results[0].errors
    );

    // Cleanup
    let _ = client.delete_metadata("CustomLabel", &[&label_name]).await;
}
