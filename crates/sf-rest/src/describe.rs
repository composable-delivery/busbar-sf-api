//! Describe operations and types.
//!
//! This module contains types for the Salesforce describe API,
//! which provides metadata about SObjects and their fields.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Describe Global Types
// ============================================================================

/// Result of the describeGlobal operation.
///
/// Contains a list of all SObjects accessible to the user.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DescribeGlobalResult {
    /// Character encoding (e.g., "UTF-8").
    pub encoding: String,

    /// Maximum batch size for composite operations.
    #[serde(rename = "maxBatchSize")]
    pub max_batch_size: u32,

    /// List of SObject descriptions.
    pub sobjects: Vec<SObjectBasicInfo>,
}

/// Basic information about an SObject from describeGlobal.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SObjectBasicInfo {
    pub name: String,
    pub label: String,
    #[serde(rename = "labelPlural")]
    pub label_plural: String,
    #[serde(rename = "keyPrefix")]
    pub key_prefix: Option<String>,
    pub custom: bool,
    pub queryable: bool,
    pub createable: bool,
    pub updateable: bool,
    pub deletable: bool,
    pub searchable: bool,
    pub retrieveable: bool,
    #[serde(rename = "customSetting")]
    pub custom_setting: Option<bool>,
    #[serde(rename = "deprecatedAndHidden")]
    pub deprecated_and_hidden: Option<bool>,
    #[serde(rename = "feedEnabled")]
    pub feed_enabled: Option<bool>,
    #[serde(rename = "mruEnabled")]
    pub mru_enabled: Option<bool>,
    pub layoutable: Option<bool>,
    pub triggerable: Option<bool>,
    pub replicateable: Option<bool>,
    pub urls: Option<HashMap<String, String>>,
}

// ============================================================================
// Describe SObject Types
// ============================================================================

/// Complete SObject describe result from Salesforce API.
///
/// Contains all metadata about an SObject including fields,
/// relationships, record types, and capabilities.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DescribeSObjectResult {
    // === Identity ===
    pub name: String,
    pub label: String,
    #[serde(rename = "labelPlural")]
    pub label_plural: Option<String>,
    #[serde(rename = "keyPrefix")]
    pub key_prefix: Option<String>,
    pub custom: bool,
    #[serde(rename = "customSetting")]
    pub custom_setting: Option<bool>,

    // === Capabilities (CRUD) ===
    #[serde(default)]
    pub createable: bool,
    #[serde(default)]
    pub deletable: bool,
    #[serde(default)]
    pub queryable: bool,
    #[serde(default)]
    pub retrieveable: bool,
    #[serde(default)]
    pub searchable: bool,
    #[serde(default)]
    pub updateable: bool,
    pub undeletable: Option<bool>,
    pub mergeable: Option<bool>,
    pub replicateable: Option<bool>,

    // === Layout & UI ===
    pub activateable: Option<bool>,
    #[serde(rename = "compactLayoutable")]
    pub compact_layoutable: Option<bool>,
    #[serde(rename = "deepCloneable")]
    pub deep_cloneable: Option<bool>,
    pub layoutable: Option<bool>,
    pub listviewable: Option<bool>,
    #[serde(rename = "lookupLayoutable")]
    pub lookup_layoutable: Option<bool>,
    #[serde(rename = "searchLayoutable")]
    pub search_layoutable: Option<bool>,
    pub triggerable: Option<bool>,
    #[serde(rename = "mruEnabled")]
    pub mru_enabled: Option<bool>,
    #[serde(rename = "feedEnabled")]
    pub feed_enabled: Option<bool>,

    // === Relationships ===
    #[serde(rename = "childRelationships", default)]
    pub child_relationships: Vec<ChildRelationship>,
    pub fields: Vec<FieldDescribe>,

    // === Record Types ===
    #[serde(rename = "recordTypeInfos", default)]
    pub record_type_infos: Vec<RecordTypeInfo>,
    #[serde(rename = "namedLayoutInfos", default)]
    pub named_layout_infos: Vec<NamedLayoutInfo>,

    // === Polymorphism & Inheritance ===
    #[serde(rename = "hasSubtypes")]
    pub has_subtypes: Option<bool>,
    #[serde(rename = "isInterface")]
    pub is_interface: Option<bool>,
    #[serde(rename = "isSubtype")]
    pub is_subtype: Option<bool>,
    #[serde(rename = "defaultImplementation")]
    pub default_implementation: Option<String>,
    #[serde(rename = "extendedBy")]
    pub extended_by: Option<String>,
    #[serde(rename = "extendsInterfaces")]
    pub extends_interfaces: Option<String>,
    #[serde(rename = "implementedBy")]
    pub implemented_by: Option<String>,
    #[serde(rename = "implementsInterfaces")]
    pub implements_interfaces: Option<String>,

    // === API Metadata ===
    #[serde(rename = "deprecatedAndHidden")]
    pub deprecated_and_hidden: Option<bool>,
    #[serde(rename = "sobjectDescribeOption")]
    pub sobject_describe_option: Option<String>,
    #[serde(rename = "networkScopeFieldName")]
    pub network_scope_field_name: Option<String>,
    #[serde(default)]
    pub urls: HashMap<String, String>,
    #[serde(rename = "supportedScopes", default)]
    pub supported_scopes: Vec<ScopeInfo>,
    #[serde(rename = "actionOverrides", default)]
    pub action_overrides: Vec<ActionOverride>,
}

/// Child relationship metadata for an SObject.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChildRelationship {
    #[serde(rename = "childSObject")]
    pub child_sobject: String,
    pub field: String,
    #[serde(rename = "relationshipName")]
    pub relationship_name: Option<String>,
    #[serde(rename = "deprecatedAndHidden")]
    pub deprecated_and_hidden: Option<bool>,
    #[serde(rename = "cascadeDelete")]
    pub cascade_delete: Option<bool>,
    #[serde(rename = "restrictedDelete")]
    pub restricted_delete: Option<bool>,
    #[serde(rename = "junctionIdListNames", default)]
    pub junction_id_list_names: Vec<String>,
    #[serde(rename = "junctionReferenceTo", default)]
    pub junction_reference_to: Vec<String>,
}

/// Record type information for an SObject.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordTypeInfo {
    pub name: String,
    #[serde(rename = "recordTypeId")]
    pub record_type_id: String,
    #[serde(rename = "developerName")]
    pub developer_name: Option<String>,
    pub active: bool,
    pub available: bool,
    #[serde(rename = "defaultRecordTypeMapping")]
    pub default_record_type_mapping: bool,
    pub master: Option<bool>,
}

/// Named layout info for an SObject.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NamedLayoutInfo {
    pub name: String,
}

/// Scope info for SOQL queries.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScopeInfo {
    pub name: String,
    pub label: String,
}

/// Action override for UI customization.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionOverride {
    #[serde(rename = "formFactor")]
    pub form_factor: Option<String>,
    #[serde(rename = "isAvailableInTouch")]
    pub is_available_in_touch: Option<bool>,
    pub name: String,
    #[serde(rename = "pageId")]
    pub page_id: Option<String>,
    pub url: Option<String>,
}

// ============================================================================
// Field Describe Types
// ============================================================================

/// Complete field describe result from Salesforce API.
///
/// Contains all metadata about a field including type, size,
/// capabilities, relationships, and picklist values.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldDescribe {
    // === Identity ===
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(rename = "soapType")]
    pub soap_type: Option<String>,
    pub custom: Option<bool>,

    // === Size & Precision ===
    pub length: Option<i32>,
    #[serde(rename = "byteLength")]
    pub byte_length: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    pub digits: Option<i32>,

    // === Capabilities ===
    #[serde(default)]
    pub createable: bool,
    #[serde(default)]
    pub updateable: bool,
    #[serde(default)]
    pub nillable: bool,
    #[serde(default)]
    pub filterable: bool,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub groupable: bool,
    pub aggregatable: Option<bool>,
    pub searchable: Option<bool>,
    #[serde(default)]
    pub unique: bool,
    pub permissionable: Option<bool>,

    // === Field Characteristics ===
    #[serde(rename = "externalId", default)]
    pub external_id: bool,
    #[serde(rename = "idLookup", default)]
    pub id_lookup: bool,
    #[serde(default)]
    pub calculated: bool,
    #[serde(rename = "calculatedFormula")]
    pub calculated_formula: Option<String>,
    #[serde(rename = "autoNumber", default)]
    pub auto_number: bool,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(rename = "nameField")]
    pub name_field: Option<bool>,
    #[serde(rename = "namePointing")]
    pub name_pointing: Option<bool>,
    #[serde(rename = "caseSensitive")]
    pub case_sensitive: Option<bool>,
    #[serde(rename = "htmlFormatted")]
    pub html_formatted: Option<bool>,
    #[serde(rename = "highScaleNumber")]
    pub high_scale_number: Option<bool>,
    #[serde(rename = "displayLocationInDecimal")]
    pub display_location_in_decimal: Option<bool>,
    #[serde(rename = "queryByDistance")]
    pub query_by_distance: Option<bool>,

    // === Defaults ===
    #[serde(rename = "defaultValue")]
    pub default_value: Option<serde_json::Value>,
    #[serde(rename = "defaultValueFormula")]
    pub default_value_formula: Option<String>,
    #[serde(rename = "defaultedOnCreate")]
    pub defaulted_on_create: Option<bool>,
    #[serde(rename = "formulaTreatNullNumberAsZero")]
    pub formula_treat_null_number_as_zero: Option<bool>,

    // === Relationships ===
    #[serde(rename = "referenceTo", default)]
    pub reference_to: Option<Vec<String>>,
    #[serde(rename = "relationshipName")]
    pub relationship_name: Option<String>,
    #[serde(rename = "relationshipOrder")]
    pub relationship_order: Option<i32>,
    #[serde(rename = "referenceTargetField")]
    pub reference_target_field: Option<String>,
    #[serde(rename = "polymorphicForeignKey")]
    pub polymorphic_foreign_key: Option<bool>,
    #[serde(rename = "cascadeDelete")]
    pub cascade_delete: Option<bool>,
    #[serde(rename = "restrictedDelete")]
    pub restricted_delete: Option<bool>,
    #[serde(rename = "writeRequiresMasterRead")]
    pub write_requires_master_read: Option<bool>,

    // === Compound Fields ===
    #[serde(rename = "compoundFieldName")]
    pub compound_field_name: Option<String>,
    #[serde(rename = "extraTypeInfo")]
    pub extra_type_info: Option<String>,

    // === Picklist ===
    #[serde(rename = "picklistValues", default)]
    pub picklist_values: Option<Vec<PicklistValue>>,
    #[serde(rename = "dependentPicklist")]
    pub dependent_picklist: Option<bool>,
    #[serde(rename = "controllerName")]
    pub controller_name: Option<String>,
    #[serde(rename = "restrictedPicklist")]
    pub restricted_picklist: Option<bool>,

    // === Lookup Filters ===
    #[serde(rename = "filteredLookupInfo")]
    pub filtered_lookup_info: Option<FilteredLookupInfo>,
    #[serde(rename = "searchPrefilterable")]
    pub search_prefilterable: Option<bool>,

    // === Masked Fields ===
    pub mask: Option<String>,
    #[serde(rename = "maskType")]
    pub mask_type: Option<String>,

    // === AI ===
    #[serde(rename = "aiPredictionField")]
    pub ai_prediction_field: Option<bool>,

    // === API Metadata ===
    #[serde(rename = "deprecatedAndHidden")]
    pub deprecated_and_hidden: Option<bool>,
    #[serde(rename = "inlineHelpText")]
    pub inline_help_text: Option<String>,
}

/// Filtered lookup info for lookup fields.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilteredLookupInfo {
    #[serde(rename = "controllingFields", default)]
    pub controlling_fields: Vec<String>,
    pub dependent: Option<bool>,
    #[serde(rename = "optionalFilter")]
    pub optional_filter: Option<bool>,
}

/// Picklist value for picklist fields.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PicklistValue {
    pub value: String,
    pub label: String,
    pub active: bool,
    #[serde(rename = "defaultValue")]
    pub default_value: bool,
    #[serde(rename = "validFor")]
    pub valid_for: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_global_result_deser() {
        let json = r#"{
            "encoding": "UTF-8",
            "maxBatchSize": 200,
            "sobjects": [{
                "name": "Account",
                "label": "Account",
                "labelPlural": "Accounts",
                "keyPrefix": "001",
                "custom": false,
                "queryable": true,
                "createable": true,
                "updateable": true,
                "deletable": true,
                "searchable": true,
                "retrieveable": true
            }]
        }"#;

        let result: DescribeGlobalResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.encoding, "UTF-8");
        assert_eq!(result.max_batch_size, 200);
        assert_eq!(result.sobjects.len(), 1);
        assert_eq!(result.sobjects[0].name, "Account");
    }

    #[test]
    fn test_field_describe_deser() {
        let json = r#"{
            "name": "Name",
            "label": "Account Name",
            "type": "string",
            "length": 255,
            "createable": true,
            "updateable": true,
            "nillable": false,
            "filterable": true,
            "sortable": true,
            "groupable": true,
            "unique": false
        }"#;

        let field: FieldDescribe = serde_json::from_str(json).unwrap();
        assert_eq!(field.name, "Name");
        assert_eq!(field.field_type, "string");
        assert_eq!(field.length, Some(255));
        assert!(field.createable);
        assert!(!field.nillable);
    }

    #[test]
    fn test_picklist_value_deser() {
        let json = r#"{
            "value": "Hot",
            "label": "Hot",
            "active": true,
            "defaultValue": false
        }"#;

        let pv: PicklistValue = serde_json::from_str(json).unwrap();
        assert_eq!(pv.value, "Hot");
        assert_eq!(pv.label, "Hot");
        assert!(pv.active);
        assert!(!pv.default_value);
    }
}
