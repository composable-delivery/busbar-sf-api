//! Search API types for parameterized search, suggestions, scope, and layouts.

use serde::{Deserialize, Serialize};

/// A parameterized search request for the REST API.
///
/// Provides a structured alternative to raw SOSL queries.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParameterizedSearchRequest {
    /// The search query string (required).
    pub q: String,
    /// List of fields to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// List of SObject-specific search specifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sobjects: Option<Vec<SearchSObjectSpec>>,
    /// Maximum number of results to return overall.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_limit: Option<u32>,
    /// Offset into the results for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Whether to apply spell correction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_correction: Option<bool>,
}

/// Specification for searching a specific SObject type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchSObjectSpec {
    /// The SObject API name (e.g., "Account").
    pub name: String,
    /// List of fields to return for this SObject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// WHERE clause to filter results for this SObject.
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<String>,
    /// Maximum number of results for this SObject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Response from a parameterized search request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ParameterizedSearchResponse {
    /// The search result records grouped by SObject type.
    pub search_records: Vec<SearchRecordGroup>,
    /// Optional metadata about the search execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SearchMetadata>,
}

/// A group of search records (flattened from the API response).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchRecordGroup {
    /// Record attributes (type, url).
    #[serde(default)]
    pub attributes: SearchRecordAttributes,
    /// All additional fields are flattened into this map.
    #[serde(flatten)]
    pub records: serde_json::Map<String, serde_json::Value>,
}

/// Attributes of a search record.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SearchRecordAttributes {
    /// The SObject type name.
    #[serde(rename = "type")]
    pub sobject_type: String,
    /// The record URL.
    #[serde(default)]
    pub url: String,
}

/// Metadata about the search execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchMetadata {
    /// Whether spell correction was applied.
    #[serde(default)]
    pub spell_correction_applied: bool,
    /// Additional metadata fields.
    #[serde(flatten)]
    pub additional: serde_json::Map<String, serde_json::Value>,
}

/// Result from a search suggestion (auto-complete) request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchSuggestionResult {
    /// The suggested records.
    pub auto_suggest_results: Vec<Suggestion>,
    /// Whether there are more results available.
    #[serde(default)]
    pub has_more_results: bool,
}

/// A single suggestion from auto-complete.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Suggestion {
    /// Attributes of the suggested record.
    #[serde(default)]
    pub attributes: SuggestionAttributes,
    /// The record ID.
    #[serde(default, rename = "Id")]
    pub id: String,
    /// The record Name.
    #[serde(default, rename = "Name")]
    pub name: String,
    /// Additional fields returned.
    #[serde(flatten)]
    pub additional_fields: serde_json::Map<String, serde_json::Value>,
}

/// Attributes of a suggestion record.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct SuggestionAttributes {
    /// The SObject type name.
    #[serde(rename = "type")]
    pub sobject_type: String,
    /// The record URL.
    #[serde(default)]
    pub url: String,
}

/// An entity in the search scope order.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ScopeEntity {
    /// The SObject API name.
    pub name: String,
    /// The SObject display label.
    #[serde(default)]
    pub label: String,
    /// Whether this SObject is in the user's search scope.
    #[serde(default)]
    pub in_search_scope: bool,
    /// The order position in the search scope.
    #[serde(default)]
    pub search_scope_order: u32,
}

/// Search layout information for an SObject.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchLayoutInfo {
    /// The SObject display label.
    #[serde(default)]
    pub label: String,
    /// Columns in the search result layout.
    #[serde(default, rename = "searchColumns")]
    pub columns: Vec<SearchLayoutColumn>,
    /// Additional layout fields.
    #[serde(flatten)]
    pub additional: serde_json::Map<String, serde_json::Value>,
}

/// A column in a search result layout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchLayoutColumn {
    /// The field API name (may be `fieldNameOrPath` in the response).
    #[serde(default, alias = "fieldNameOrPath")]
    pub field: Option<String>,
    /// The column display label.
    #[serde(default)]
    pub label: Option<String>,
    /// The field format type (nullable for some column types).
    #[serde(default)]
    pub format: Option<String>,
    /// The column name.
    #[serde(default)]
    pub name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parameterized_search_response_deserialization() {
        let json = json!({
            "searchRecords": [
                {
                    "attributes": {"type": "Account", "url": "/services/data/v62.0/sobjects/Account/001xx"},
                    "Id": "001xx000003Dgb2AAC",
                    "Name": "Acme Corp"
                }
            ],
            "metadata": {
                "spellCorrectionApplied": true
            }
        });

        let response: ParameterizedSearchResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.search_records.len(), 1);
        assert_eq!(
            response.search_records[0].attributes.sobject_type,
            "Account"
        );
        assert!(response.metadata.is_some());
        assert!(response.metadata.unwrap().spell_correction_applied);
    }

    #[test]
    fn test_parameterized_search_response_no_metadata() {
        let json = json!({
            "searchRecords": []
        });

        let response: ParameterizedSearchResponse = serde_json::from_value(json).unwrap();
        assert!(response.search_records.is_empty());
        assert!(response.metadata.is_none());
    }

    #[test]
    fn test_search_suggestion_result_deserialization() {
        let json = json!({
            "autoSuggestResults": [
                {
                    "attributes": {"type": "Account", "url": "/services/data/v62.0/sobjects/Account/001xx"},
                    "Id": "001xx000003Dgb2AAC",
                    "Name": "Acme Corp"
                }
            ],
            "hasMoreResults": true
        });

        let result: SearchSuggestionResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.auto_suggest_results.len(), 1);
        assert!(result.has_more_results);
        assert_eq!(result.auto_suggest_results[0].id, "001xx000003Dgb2AAC");
        assert_eq!(result.auto_suggest_results[0].name, "Acme Corp");
        assert_eq!(
            result.auto_suggest_results[0].attributes.sobject_type,
            "Account"
        );
    }

    #[test]
    fn test_search_scope_result_deserialization() {
        let json = json!([
            {
                "name": "Account",
                "label": "Accounts",
                "inSearchScope": true,
                "searchScopeOrder": 1
            },
            {
                "name": "Contact",
                "label": "Contacts",
                "inSearchScope": true,
                "searchScopeOrder": 2
            }
        ]);

        let results: Vec<ScopeEntity> = serde_json::from_value(json).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Account");
        assert!(results[0].in_search_scope);
        assert_eq!(results[0].search_scope_order, 1);
        assert_eq!(results[1].name, "Contact");
    }

    #[test]
    fn test_search_scope_empty_array() {
        let json = json!([]);
        let results: Vec<ScopeEntity> = serde_json::from_value(json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_layout_result_deserialization() {
        let json = json!([
            {
                "label": "Accounts",
                "searchColumns": [
                    {
                        "field": "Account.Name",
                        "label": "Account Name",
                        "format": "string",
                        "name": "Name"
                    }
                ]
            }
        ]);

        let results: Vec<SearchLayoutInfo> = serde_json::from_value(json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].label, "Accounts");
        assert_eq!(results[0].columns.len(), 1);
        assert_eq!(results[0].columns[0].field.as_deref(), Some("Account.Name"));
        assert_eq!(results[0].columns[0].label.as_deref(), Some("Account Name"));
        assert_eq!(results[0].columns[0].name.as_deref(), Some("Name"));
    }

    #[test]
    fn test_search_layout_result_empty() {
        let json = json!([]);
        let results: Vec<SearchLayoutInfo> = serde_json::from_value(json).unwrap();
        assert!(results.is_empty());
    }
}
