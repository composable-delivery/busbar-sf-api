//! Describe metadata operations.

use serde::{Deserialize, Serialize};

/// Result of describe metadata.
#[derive(Debug, Clone)]
pub struct DescribeMetadataResult {
    pub metadata_objects: Vec<MetadataType>,
    pub organization_namespace: Option<String>,
    pub partial_save_allowed: bool,
    pub test_required: bool,
}

/// A metadata type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataType {
    pub xml_name: String,
    pub directory_name: Option<String>,
    pub suffix: Option<String>,
    pub meta_file: bool,
    pub in_folder: bool,
    pub child_xml_names: Vec<String>,
}
