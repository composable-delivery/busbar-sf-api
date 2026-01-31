//! Advanced search types and implementations.
//!
//! This module provides types for Salesforce's advanced search endpoints:
//! - Parameterized Search (structured search with filters)
//! - Search Suggestions (auto-suggest for type-ahead)
//! - Search Scope Order (user's most-searched objects)
//! - Search Result Layouts (layout metadata for displaying results)

use serde::{Deserialize, Serialize};

// =========================================================================
// Parameterized Search
// =========================================================================

/// Request for parameterized search.
///
/// Parameterized search provides structured, filtered search with pagination.
///
/// # Example
///
/// ```rust,ignore
/// use sf_rest::search::*;
///
/// let request = ParameterizedSearchRequest {
///     q: "test".to_string(),
///     fields: vec!["Id".to_string(), "Name".to_string()],
///     sobjects: vec![SearchSObjectSpec {
///         name: "Account".to_string(),
///         fields: None,
///         where_clause: None,
///         limit: Some(10),
///     }],
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParameterizedSearchRequest {
    /// The search query string (SOSL format).
    pub q: String,

    /// List of fields to return for all objects (optional).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<String>,

    /// List of SObjects to search with their specific configurations.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sobjects: Vec<SearchSObjectSpec>,

    /// Overall search configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_limit: Option<u32>,

    /// Zero-based offset for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,

    /// Spell correction mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_correction: Option<bool>,
}

/// Configuration for searching a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchSObjectSpec {
    /// SObject name (e.g., "Account", "Contact").
    pub name: String,

    /// Fields to return for this SObject (overrides top-level fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,

    /// WHERE clause to filter results (without "WHERE" keyword).
    #[serde(skip_serializing_if = "Option::is_none", rename = "where")]
    pub where_clause: Option<String>,

    /// Limit for this specific SObject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Response from parameterized search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterizedSearchResponse {
    /// Search results grouped by SObject.
    pub search_records: Vec<SearchRecordGroup>,

    /// Metadata about the search.
    pub metadata: SearchMetadata,
}

/// Search results for a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRecordGroup {
    /// Attributes containing SObject type information.
    pub attributes: SearchRecordAttributes,

    /// Additional fields returned in the search result (includes matched records and metadata).
    #[serde(flatten)]
    pub records: serde_json::Value,
}

/// Attributes identifying the SObject type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRecordAttributes {
    /// SObject type name.
    #[serde(rename = "type")]
    pub sobject_type: String,
}

/// Metadata about the search execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMetadata {
    /// Whether spell correction was applied.
    #[serde(default)]
    pub spell_correction_applied: bool,

    /// Additional metadata fields.
    #[serde(flatten)]
    pub additional: serde_json::Value,
}

// =========================================================================
// Search Suggestions
// =========================================================================

/// Response from search suggestions API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSuggestionResult {
    /// List of suggested results.
    pub auto_suggest_results: Vec<Suggestion>,

    /// Whether more results are available.
    pub has_more_results: bool,
}

/// A single search suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// SObject type attributes.
    pub attributes: SuggestionAttributes,

    /// Suggested record ID.
    #[serde(rename = "Id")]
    pub id: String,

    /// Display value for the suggestion.
    pub name: String,

    /// Additional fields returned for the suggestion.
    #[serde(flatten)]
    pub additional_fields: serde_json::Value,
}

/// Attributes for a suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionAttributes {
    /// SObject type.
    #[serde(rename = "type")]
    pub sobject_type: String,

    /// API URL for the record.
    pub url: String,
}

// =========================================================================
// Search Scope Order
// =========================================================================

/// Response from search scope order API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchScopeResult {
    /// Ordered list of SObjects the user searches most.
    pub scope_entities: Vec<ScopeEntity>,
}

/// An SObject in the user's search scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeEntity {
    /// SObject name.
    pub name: String,

    /// SObject label (display name).
    pub label: String,

    /// Whether this SObject is in the user's current search scope.
    pub in_search_scope: bool,

    /// Position in the search scope order.
    pub search_scope_order: u32,
}

// =========================================================================
// Search Result Layouts
// =========================================================================

/// Response from search result layouts API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLayoutResult {
    /// Layout information for each requested SObject.
    pub search_layout: Vec<SearchLayoutInfo>,
}

/// Layout information for a specific SObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLayoutInfo {
    /// Display label for the SObject (e.g., "Accounts", "Contacts").
    pub label: String,

    /// List of columns to display in search results.
    pub columns: Vec<SearchLayoutColumn>,

    /// Additional layout metadata.
    #[serde(flatten)]
    pub additional: serde_json::Value,
}

/// A column in the search result layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchLayoutColumn {
    /// Field name.
    pub field: String,

    /// Display label for the column.
    pub label: String,

    /// Field format/type.
    pub format: Option<String>,

    /// Column name.
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameterized_search_response_deserialization() {
        let json = r#"{
            "searchRecords": [
                {
                    "attributes": {"type": "Account"},
                    "Id": "001xx1",
                    "Name": "Acme Corp"
                }
            ],
            "metadata": {
                "spellCorrectionApplied": true
            }
        }"#;

        let result: ParameterizedSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.search_records.len(), 1);
        assert_eq!(result.search_records[0].attributes.sobject_type, "Account");
        assert!(result.metadata.spell_correction_applied);
    }

    #[test]
    fn test_search_suggestion_result_deserialization() {
        let json = r#"{
            "autoSuggestResults": [
                {
                    "attributes": {"type": "Account", "url": "/services/data/v62.0/sobjects/Account/001xx1"},
                    "Id": "001xx1",
                    "name": "Acme Corp"
                }
            ],
            "hasMoreResults": true
        }"#;

        let result: SearchSuggestionResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.auto_suggest_results.len(), 1);
        assert_eq!(result.auto_suggest_results[0].id, "001xx1");
        assert_eq!(result.auto_suggest_results[0].name, "Acme Corp");
        assert!(result.has_more_results);
    }

    #[test]
    fn test_search_scope_result_deserialization() {
        let json = r#"{
            "scopeEntities": [
                {"name": "Account", "label": "Accounts", "inSearchScope": true, "searchScopeOrder": 1},
                {"name": "Contact", "label": "Contacts", "inSearchScope": false, "searchScopeOrder": 2}
            ]
        }"#;

        let result: SearchScopeResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.scope_entities.len(), 2);
        assert_eq!(result.scope_entities[0].name, "Account");
        assert!(result.scope_entities[0].in_search_scope);
        assert!(!result.scope_entities[1].in_search_scope);
    }

    #[test]
    fn test_search_layout_result_deserialization() {
        let json = r#"{
            "searchLayout": [
                {
                    "label": "Accounts",
                    "columns": [
                        {"field": "Name", "label": "Account Name", "format": null, "name": "Name"},
                        {"field": "Phone", "label": "Phone", "format": "phone", "name": "Phone"}
                    ]
                }
            ]
        }"#;

        let result: SearchLayoutResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.search_layout.len(), 1);
        assert_eq!(result.search_layout[0].label, "Accounts");
        assert_eq!(result.search_layout[0].columns.len(), 2);
        assert_eq!(result.search_layout[0].columns[1].format, Some("phone".to_string()));
    }
}
