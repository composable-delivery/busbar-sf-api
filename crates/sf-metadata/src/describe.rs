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

/// Result of describe value type operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescribeValueTypeResult {
    pub value_type_fields: Vec<ValueTypeField>,
    pub parent_field: Option<ValueTypeField>,
}

/// A field definition in a metadata value type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueTypeField {
    pub name: String,
    pub soap_type: String,
    pub is_foreign_key: bool,
    pub foreign_key_domain: Option<String>,
    pub is_name_field: bool,
    pub min_occurs: u32,
    pub max_occurs: u32,
    pub fields: Vec<ValueTypeField>,
    pub picklist_values: Vec<PicklistEntry>,
}

/// A picklist entry for a metadata field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicklistEntry {
    pub active: bool,
    pub default_value: bool,
    pub label: String,
    pub value: String,
}
