//! Typed metadata operations using `busbar-sf-types`.
//!
//! This module is only available when the `typed` feature is enabled.
//! It provides a trait extension for `MetadataClient` that allows deploy and retrieve
//! operations with fully-typed Salesforce metadata structures from `busbar-sf-types`.
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
        assert_eq!(get_file_extension("Flow"), "flow");
        assert_eq!(get_file_extension("Unknown"), "xml");
    }
}
