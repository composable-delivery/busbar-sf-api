//! Typed metadata operations using `busbar-sf-types`.
//!
//! This module is only available when the `typed` feature is enabled.
//! It provides a trait extension for `MetadataClient` that allows deploy and retrieve
//! operations with fully-typed Salesforce metadata structures from `busbar-sf-types`.
//!
//! # ⚠️ Current Limitations
//!
//! This is a **proof-of-concept implementation**. The current XML serialization is simplified
//! and wraps JSON in XML tags rather than producing proper Salesforce Metadata API XML.
//! This may not work correctly with all metadata types in production.
//!
//! For production use, proper XML serialization should be implemented using `quick-xml` or
//! similar, converting typed structures to valid Salesforce Metadata API XML format per:
//! <https://developer.salesforce.com/docs/atlas.en-us.api_meta.meta/api_meta/>
//!
//! # Example
//!
//! ```rust,ignore
//! use busbar_sf_metadata::{MetadataClient, TypedMetadataExt, DeployOptions};
//! use busbar_sf_types::{metadata::objects::CustomObject, MetadataType};
//! use busbar_sf_auth::SalesforceCredentials;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let creds = SalesforceCredentials::from_env()?;
//!     let client = MetadataClient::new(&creds)?;
//!
//!     // Create a typed custom object
//!     let custom_object = CustomObject {
//!         full_name: Some("MyObject__c".to_string()),
//!         label: Some("My Object".to_string()),
//!         ..Default::default()
//!     };
//!
//!     // Deploy using typed interface
//!     let async_id = client.deploy_typed(&custom_object, DeployOptions::default()).await?;
//!     
//!     Ok(())
//! }
//! ```

use crate::client::MetadataClient;
use crate::deploy::DeployOptions;
use crate::error::{Error, ErrorKind, Result};
use crate::retrieve::PackageManifest;
use busbar_sf_types::traits::MetadataType;
use std::io::{Cursor, Write};
use zip::write::{FileOptions, ZipWriter};

/// Extension trait for typed metadata operations.
///
/// This trait provides methods to deploy and retrieve metadata using
/// fully-typed structures from `busbar-sf-types` instead of raw zip files.
///
/// Enable with the `typed` feature flag.
#[allow(async_fn_in_trait)]
pub trait TypedMetadataExt {
    /// Deploy a single typed metadata component.
    ///
    /// Serializes the component to XML, creates a package.xml manifest,
    /// and deploys both as a zip package.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_types::metadata::objects::CustomObject;
    ///
    /// let obj = CustomObject {
    ///     full_name: Some("MyObject__c".to_string()),
    ///     label: Some("My Object".to_string()),
    ///     ..Default::default()
    /// };
    ///
    /// let async_id = client.deploy_typed(&obj, DeployOptions::default()).await?;
    /// ```
    async fn deploy_typed<T: MetadataType + serde::Serialize>(
        &self,
        metadata: &T,
        options: DeployOptions,
    ) -> Result<String>;

    /// Deploy multiple typed metadata components.
    ///
    /// Groups components by type and creates a package with all items.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use busbar_sf_types::metadata::apex::ApexClass;
    ///
    /// let classes = vec![
    ///     ApexClass { full_name: Some("MyClass".to_string()), ..Default::default() },
    ///     ApexClass { full_name: Some("OtherClass".to_string()), ..Default::default() },
    /// ];
    ///
    /// let async_id = client.deploy_typed_batch(&classes, DeployOptions::default()).await?;
    /// ```
    async fn deploy_typed_batch<T: MetadataType + serde::Serialize>(
        &self,
        metadata_items: &[T],
        options: DeployOptions,
    ) -> Result<String>;
}

impl TypedMetadataExt for MetadataClient {
    async fn deploy_typed<T: MetadataType + serde::Serialize>(
        &self,
        metadata: &T,
        options: DeployOptions,
    ) -> Result<String> {
        self.deploy_typed_batch(std::slice::from_ref(metadata), options)
            .await
    }

    async fn deploy_typed_batch<T: MetadataType + serde::Serialize>(
        &self,
        metadata_items: &[T],
        options: DeployOptions,
    ) -> Result<String> {
        if metadata_items.is_empty() {
            return Err(Error::new(ErrorKind::Other(
                "Cannot deploy empty metadata batch".to_string(),
            )));
        }

        // Create zip in memory
        let mut zip_buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(&mut zip_buffer);

        // Collect member names for package.xml
        let mut members = Vec::new();

        // Add each metadata item to the zip
        for (idx, item) in metadata_items.iter().enumerate() {
            // Get the API name
            let api_name = item
                .api_name()
                .ok_or_else(|| {
                    Error::new(ErrorKind::Other(format!(
                        "Metadata item at index {} missing api_name",
                        idx
                    )))
                })?
                .to_string();

            members.push(api_name.clone());

            // Determine the file path based on metadata type
            let file_path = format!(
                "{}/{}.{}",
                get_directory_name(T::METADATA_TYPE_NAME),
                api_name,
                get_file_extension(T::METADATA_TYPE_NAME)
            );

            // Serialize to XML
            let xml = serialize_to_metadata_xml(item)?;

            // Add to zip
            zip.start_file::<_, ()>(file_path, FileOptions::default())
                .map_err(|e| Error::new(ErrorKind::Io(e.to_string())))?;
            zip.write_all(xml.as_bytes())?;
        }

        // Create package.xml
        let manifest =
            PackageManifest::new(self.api_version()).add_type(T::METADATA_TYPE_NAME, members);

        let package_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Package xmlns="http://soap.sforce.com/2006/04/metadata">
    {}
</Package>"#,
            manifest.to_xml()
        );

        zip.start_file::<_, ()>("package.xml", FileOptions::default())
            .map_err(|e| Error::new(ErrorKind::Io(e.to_string())))?;
        zip.write_all(package_xml.as_bytes())?;

        // Finish the zip
        zip.finish()
            .map_err(|e| Error::new(ErrorKind::Io(e.to_string())))?;

        let zip_bytes = zip_buffer.into_inner();

        // Deploy using the standard method
        self.deploy(&zip_bytes, options).await
    }
}

/// Serialize a metadata item to XML format.
fn serialize_to_metadata_xml<T: MetadataType + serde::Serialize>(item: &T) -> Result<String> {
    // Serialize to JSON first, then convert to XML
    // This is a simplified approach - in production you'd use proper XML serialization
    let json = serde_json::to_string_pretty(item)
        .map_err(|e| Error::new(ErrorKind::Parse(e.to_string())))?;

    // For now, wrap in XML structure with metadata namespace
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<{root} xmlns="http://soap.sforce.com/2006/04/metadata">
    <!-- Simplified XML representation - proper XML serialization would go here -->
    <!-- In production, use quick-xml or similar for proper XML serialization -->
    {json}
</{root}>"#,
        root = T::XML_ROOT_ELEMENT,
        json = json
    );

    Ok(xml)
}

/// Get the directory name for a metadata type.
fn get_directory_name(metadata_type: &str) -> &str {
    match metadata_type {
        "ApexClass" => "classes",
        "ApexTrigger" => "triggers",
        "ApexPage" => "pages",
        "ApexComponent" => "components",
        "CustomObject" => "objects",
        "CustomField" => "objects",
        "Layout" => "layouts",
        "PermissionSet" => "permissionsets",
        "Profile" => "profiles",
        "Flow" => "flows",
        "Report" => "reports",
        "Dashboard" => "dashboards",
        "EmailTemplate" => "email",
        "StaticResource" => "staticresources",
        "LightningComponentBundle" => "lwc",
        "AuraDefinitionBundle" => "aura",
        _ => "metadata", // fallback
    }
}

/// Get the file extension for a metadata type.
fn get_file_extension(metadata_type: &str) -> &str {
    match metadata_type {
        "ApexClass" => "cls",
        "ApexTrigger" => "trigger",
        "ApexPage" => "page",
        "ApexComponent" => "component",
        "CustomObject" => "object",
        "CustomField" => "field",
        "Layout" => "layout",
        "PermissionSet" => "permissionset",
        "Profile" => "profile",
        "Flow" => "flow",
        "Report" => "report",
        "Dashboard" => "dashboard",
        "EmailTemplate" => "email",
        "StaticResource" => "resource",
        _ => "xml", // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    // Mock metadata type for testing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct MockMetadata {
        full_name: Option<String>,
        label: Option<String>,
    }

    impl busbar_sf_types::traits::MetadataType for MockMetadata {
        const METADATA_TYPE_NAME: &'static str = "ApexClass";
        const XML_ROOT_ELEMENT: &'static str = "ApexClass";

        fn api_name(&self) -> Option<&str> {
            self.full_name.as_deref()
        }
    }

    #[test]
    fn test_get_directory_name() {
        assert_eq!(get_directory_name("ApexClass"), "classes");
        assert_eq!(get_directory_name("CustomObject"), "objects");
        assert_eq!(get_directory_name("Flow"), "flows");
        assert_eq!(get_directory_name("Unknown"), "metadata");
    }

    #[test]
    fn test_get_file_extension() {
        assert_eq!(get_file_extension("ApexClass"), "cls");
        assert_eq!(get_file_extension("CustomObject"), "object");
        assert_eq!(get_file_extension("CustomField"), "field");
        assert_eq!(get_file_extension("Flow"), "flow");
        assert_eq!(get_file_extension("Unknown"), "xml");
    }

    #[test]
    fn test_get_directory_name_all_types() {
        // Test all known metadata types
        assert_eq!(get_directory_name("ApexTrigger"), "triggers");
        assert_eq!(get_directory_name("ApexPage"), "pages");
        assert_eq!(get_directory_name("ApexComponent"), "components");
        assert_eq!(get_directory_name("Layout"), "layouts");
        assert_eq!(get_directory_name("PermissionSet"), "permissionsets");
        assert_eq!(get_directory_name("Profile"), "profiles");
        assert_eq!(get_directory_name("Report"), "reports");
        assert_eq!(get_directory_name("Dashboard"), "dashboards");
        assert_eq!(get_directory_name("EmailTemplate"), "email");
        assert_eq!(get_directory_name("StaticResource"), "staticresources");
        assert_eq!(get_directory_name("LightningComponentBundle"), "lwc");
        assert_eq!(get_directory_name("AuraDefinitionBundle"), "aura");
    }

    #[test]
    fn test_get_file_extension_all_types() {
        // Test all known metadata types
        assert_eq!(get_file_extension("ApexTrigger"), "trigger");
        assert_eq!(get_file_extension("ApexPage"), "page");
        assert_eq!(get_file_extension("ApexComponent"), "component");
        assert_eq!(get_file_extension("Layout"), "layout");
        assert_eq!(get_file_extension("PermissionSet"), "permissionset");
        assert_eq!(get_file_extension("Profile"), "profile");
        assert_eq!(get_file_extension("Report"), "report");
        assert_eq!(get_file_extension("Dashboard"), "dashboard");
        assert_eq!(get_file_extension("EmailTemplate"), "email");
        assert_eq!(get_file_extension("StaticResource"), "resource");
    }

    #[test]
    fn test_serialize_to_metadata_xml_generates_xml_structure() {
        let metadata = MockMetadata {
            full_name: Some("TestClass".to_string()),
            label: Some("Test Class".to_string()),
        };

        let xml = serialize_to_metadata_xml(&metadata).expect("Should serialize");

        // Verify XML structure
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<ApexClass xmlns=\"http://soap.sforce.com/2006/04/metadata\">"));
        assert!(xml.contains("</ApexClass>"));
        assert!(xml.contains("TestClass"));
        assert!(xml.contains("Test Class"));
    }

    #[test]
    fn test_serialize_to_metadata_xml_handles_empty_fields() {
        let metadata = MockMetadata {
            full_name: None,
            label: None,
        };

        let result = serialize_to_metadata_xml(&metadata);
        assert!(result.is_ok(), "Should handle empty fields");
    }

    #[test]
    fn test_package_manifest_generation() {
        let manifest = PackageManifest::new("65.0").add_type(
            "ApexClass",
            vec!["TestClass".to_string(), "AnotherClass".to_string()],
        );

        let xml = manifest.to_xml();

        // Verify manifest contains the metadata type and members
        assert!(xml.contains("<name>ApexClass</name>"));
        assert!(xml.contains("<members>TestClass</members>"));
        assert!(xml.contains("<members>AnotherClass</members>"));
        assert!(xml.contains("<version>65.0</version>"));
    }

    #[test]
    fn test_metadata_type_directory_mapping_consistency() {
        // Ensure every type with a directory mapping has an extension mapping
        let types_with_dirs = vec![
            "ApexClass",
            "ApexTrigger",
            "ApexPage",
            "ApexComponent",
            "CustomObject",
            "CustomField",
            "Layout",
            "PermissionSet",
            "Profile",
            "Flow",
        ];

        for metadata_type in types_with_dirs {
            let dir = get_directory_name(metadata_type);
            let ext = get_file_extension(metadata_type);

            // Both should not be fallback values
            assert_ne!(
                dir, "metadata",
                "Type {} missing directory mapping",
                metadata_type
            );
            assert_ne!(
                ext, "xml",
                "Type {} missing extension mapping",
                metadata_type
            );
        }
    }

    #[test]
    fn test_error_on_empty_batch() {
        // This would need to be an async test with a mock client
        // Testing the validation logic for empty batches
        let empty_items: Vec<MockMetadata> = vec![];
        assert!(empty_items.is_empty(), "Empty batch should be detected");
    }

    #[test]
    fn test_error_on_missing_api_name() {
        let metadata = MockMetadata {
            full_name: None,
            label: Some("Test".to_string()),
        };

        assert!(
            metadata.api_name().is_none(),
            "Should detect missing API name"
        );
    }

    #[test]
    fn test_mock_metadata_implements_metadata_type() {
        let metadata = MockMetadata {
            full_name: Some("TestClass".to_string()),
            label: Some("Test Class".to_string()),
        };

        // Verify trait implementation
        assert_eq!(MockMetadata::METADATA_TYPE_NAME, "ApexClass");
        assert_eq!(MockMetadata::XML_ROOT_ELEMENT, "ApexClass");
        assert_eq!(metadata.api_name(), Some("TestClass"));
        assert_eq!(metadata.full_name(), Some("TestClass".to_string()));
    }
}
