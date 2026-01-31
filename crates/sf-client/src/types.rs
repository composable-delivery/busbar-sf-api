//! Shared types used across multiple Salesforce API surfaces.
//!
//! These types are used by both Tooling API and Bulk API to avoid duplication
//! and ensure consistency.

#[cfg(feature = "dependencies")]
use serde::{Deserialize, Serialize};

/// MetadataComponentDependency represents dependency relationships between metadata components.
///
/// This type is available in:
/// - Tooling API (API version 43.0+) - up to 2000 records per query
/// - Bulk API 2.0 (API version 49.0+) - up to 100,000 records per query
///
/// # Example
///
/// ```rust,ignore
/// use busbar_sf_client::MetadataComponentDependency;
///
/// // Query via Tooling API
/// let deps: Vec<MetadataComponentDependency> = tooling_client
///     .query_all("SELECT MetadataComponentId, MetadataComponentName, RefMetadataComponentId, RefMetadataComponentName FROM MetadataComponentDependency")
///     .await?;
///
/// // Query via Bulk API
/// let result = bulk_client
///     .execute_query(
///         QueryBuilder::new("MetadataComponentDependency")?
///             .select(&["MetadataComponentId", "MetadataComponentName"])
///     )
///     .await?;
/// ```
#[cfg(feature = "dependencies")]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetadataComponentDependency {
    /// The ID of a metadata component that depends on another component.
    ///
    /// This is usually an 18-character ID or a standard object name.
    /// Use 18-character IDs (not 15-character IDs) in queries.
    #[serde(rename = "MetadataComponentId")]
    pub metadata_component_id: Option<String>,

    /// The name of a metadata component that depends on another component.
    ///
    /// For example, "YourClass" for an Apex class or "yourField" (without the __c suffix)
    /// for a custom field.
    #[serde(rename = "MetadataComponentName")]
    pub metadata_component_name: Option<String>,

    /// The namespace of a metadata component that depends on another component.
    #[serde(rename = "MetadataComponentNamespace")]
    pub metadata_component_namespace: Option<String>,

    /// The type of a metadata component that depends on another component.
    ///
    /// Examples: "ApexClass", "CustomField", "WorkflowRule", etc.
    #[serde(rename = "MetadataComponentType")]
    pub metadata_component_type: Option<String>,

    /// The ID of a metadata component that another component depends on.
    ///
    /// This is usually an 18-character ID or a standard object name.
    /// Use 18-character IDs (not 15-character IDs) in queries.
    #[serde(rename = "RefMetadataComponentId")]
    pub ref_metadata_component_id: Option<String>,

    /// The name of a metadata component that another component depends on.
    ///
    /// For example, "YourClass" for an Apex class or "yourField" (without the __c suffix)
    /// for a custom field.
    #[serde(rename = "RefMetadataComponentName")]
    pub ref_metadata_component_name: Option<String>,

    /// The namespace of a metadata component that another component depends on.
    #[serde(rename = "RefMetadataComponentNamespace")]
    pub ref_metadata_component_namespace: Option<String>,

    /// The type of a metadata component that another component depends on.
    ///
    /// Examples: "ApexClass", "CustomField", "StandardEntity", etc.
    #[serde(rename = "RefMetadataComponentType")]
    pub ref_metadata_component_type: Option<String>,
}

#[cfg(test)]
#[cfg(feature = "dependencies")]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_component_dependency_deser() {
        let json = r#"{
            "MetadataComponentId": "01pxx00000000001AAA",
            "MetadataComponentName": "MyClass",
            "MetadataComponentNamespace": null,
            "MetadataComponentType": "ApexClass",
            "RefMetadataComponentId": "01pxx00000000002AAA",
            "RefMetadataComponentName": "ReferencedClass",
            "RefMetadataComponentNamespace": null,
            "RefMetadataComponentType": "ApexClass"
        }"#;

        let dep: MetadataComponentDependency = serde_json::from_str(json).unwrap();
        assert_eq!(
            dep.metadata_component_id,
            Some("01pxx00000000001AAA".to_string())
        );
        assert_eq!(dep.metadata_component_name, Some("MyClass".to_string()));
        assert_eq!(dep.metadata_component_type, Some("ApexClass".to_string()));
        assert_eq!(
            dep.ref_metadata_component_id,
            Some("01pxx00000000002AAA".to_string())
        );
        assert_eq!(
            dep.ref_metadata_component_name,
            Some("ReferencedClass".to_string())
        );
        assert_eq!(
            dep.ref_metadata_component_type,
            Some("ApexClass".to_string())
        );
    }

    #[test]
    fn test_metadata_component_dependency_ser() {
        let dep = MetadataComponentDependency {
            metadata_component_id: Some("01pxx00000000001AAA".to_string()),
            metadata_component_name: Some("MyClass".to_string()),
            metadata_component_namespace: None,
            metadata_component_type: Some("ApexClass".to_string()),
            ref_metadata_component_id: Some("01pxx00000000002AAA".to_string()),
            ref_metadata_component_name: Some("ReferencedClass".to_string()),
            ref_metadata_component_namespace: None,
            ref_metadata_component_type: Some("ApexClass".to_string()),
        };

        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("MetadataComponentId"));
        assert!(json.contains("01pxx00000000001AAA"));
        assert!(json.contains("RefMetadataComponentType"));
    }
}
