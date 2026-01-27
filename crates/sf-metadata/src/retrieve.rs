//! Retrieve operations.

use crate::types::FileProperties;
use busbar_sf_client::security::xml;
use serde::{Deserialize, Serialize};

/// Options for retrieval.
#[derive(Debug, Clone, Default)]
pub struct RetrieveOptions {
    pub single_package: bool,
    pub unpackaged: Option<PackageManifest>,
    pub package_names: Vec<String>,
}

/// Package manifest (package.xml).
///
/// Use this structured type to safely build package manifests without
/// risk of XML injection. All values are properly escaped when converted
/// to XML.
#[derive(Debug, Clone, Default)]
pub struct PackageManifest {
    pub types: Vec<PackageTypeMembers>,
    pub version: String,
}

impl PackageManifest {
    /// Create a new package manifest with the given API version.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            types: Vec::new(),
            version: version.into(),
        }
    }

    /// Add a metadata type with its members.
    pub fn add_type(mut self, name: impl Into<String>, members: Vec<String>) -> Self {
        self.types.push(PackageTypeMembers {
            name: name.into(),
            members,
        });
        self
    }

    /// Convert to XML elements for SOAP envelope.
    /// All values are properly XML-escaped to prevent injection.
    pub(crate) fn to_xml(&self) -> String {
        let mut xml_parts = Vec::new();

        for type_member in &self.types {
            let members_xml: String = type_member
                .members
                .iter()
                .map(|m| format!("<members>{}</members>", xml::escape(m)))
                .collect::<Vec<_>>()
                .join("\n          ");

            xml_parts.push(format!(
                "<types>\n          {}\n          <name>{}</name>\n        </types>",
                members_xml,
                xml::escape(&type_member.name)
            ));
        }

        xml_parts.push(format!("<version>{}</version>", xml::escape(&self.version)));

        xml_parts.join("\n        ")
    }
}

/// Type members in a package manifest.
#[derive(Debug, Clone)]
pub struct PackageTypeMembers {
    pub name: String,
    pub members: Vec<String>,
}

/// Retrieve status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrieveStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Canceling,
    Canceled,
}

impl std::str::FromStr for RetrieveStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(RetrieveStatus::Pending),
            "InProgress" => Ok(RetrieveStatus::InProgress),
            "Succeeded" => Ok(RetrieveStatus::Succeeded),
            "Failed" => Ok(RetrieveStatus::Failed),
            "Canceling" => Ok(RetrieveStatus::Canceling),
            "Canceled" => Ok(RetrieveStatus::Canceled),
            _ => Err(format!("Unknown retrieve status: {}", s)),
        }
    }
}

/// Result of a retrieval.
#[derive(Debug, Clone)]
pub struct RetrieveResult {
    /// Async process ID.
    pub id: String,
    /// Whether the operation is complete.
    pub done: bool,
    /// Current status.
    pub status: RetrieveStatus,
    /// Whether the retrieve succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error_message: Option<String>,
    /// Error status code if failed.
    pub error_status_code: Option<String>,
    /// Base64-encoded zip file contents.
    pub zip_file: Option<String>,
    /// File properties in the retrieved package.
    pub file_properties: Vec<FileProperties>,
    /// Retrieve messages (warnings/errors).
    pub messages: Vec<RetrieveMessage>,
}

/// A message from retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieveMessage {
    pub file_name: String,
    pub problem: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieve_status_parse() {
        assert_eq!(
            "Pending".parse::<RetrieveStatus>().unwrap(),
            RetrieveStatus::Pending
        );
        assert_eq!(
            "Succeeded".parse::<RetrieveStatus>().unwrap(),
            RetrieveStatus::Succeeded
        );
        assert_eq!(
            "Failed".parse::<RetrieveStatus>().unwrap(),
            RetrieveStatus::Failed
        );
    }

    #[test]
    fn test_package_manifest_to_xml() {
        let manifest = PackageManifest::new("62.0")
            .add_type(
                "ApexClass",
                vec!["MyClass".to_string(), "OtherClass".to_string()],
            )
            .add_type("ApexTrigger", vec!["*".to_string()]);

        let xml = manifest.to_xml();
        assert!(xml.contains("<name>ApexClass</name>"));
        assert!(xml.contains("<members>MyClass</members>"));
        assert!(xml.contains("<members>OtherClass</members>"));
        assert!(xml.contains("<name>ApexTrigger</name>"));
        assert!(xml.contains("<members>*</members>"));
        assert!(xml.contains("<version>62.0</version>"));
    }

    #[test]
    fn test_package_manifest_escapes_xml_injection() {
        // Attempt XML injection via member name
        let manifest = PackageManifest::new("62.0").add_type(
            "ApexClass",
            vec!["</members><malicious>attack</malicious><members>".to_string()],
        );

        let xml = manifest.to_xml();

        // Should be escaped, not treated as XML
        assert!(xml.contains("&lt;/members&gt;"));
        assert!(xml.contains("&lt;malicious&gt;"));
        assert!(!xml.contains("<malicious>"));
    }

    #[test]
    fn test_package_manifest_escapes_type_name() {
        // Attempt XML injection via type name
        let manifest = PackageManifest::new("62.0")
            .add_type("<script>alert('xss')</script>", vec!["*".to_string()]);

        let xml = manifest.to_xml();

        // Should be escaped
        assert!(xml.contains("&lt;script&gt;"));
        assert!(!xml.contains("<script>"));
    }
}
