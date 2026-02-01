//! List View types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// A list view definition.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ListView {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "developerName", default)]
    pub developer_name: String,
    #[serde(default)]
    pub label: String,
    #[serde(rename = "describeUrl", default)]
    pub describe_url: String,
    #[serde(rename = "resultsUrl", default)]
    pub results_url: String,
    #[serde(rename = "sobjectType", default)]
    pub sobject_type: String,
}

/// Collection of list views.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListViewCollection {
    pub done: bool,
    #[serde(rename = "nextRecordsUrl")]
    pub next_records_url: Option<String>,
    #[serde(alias = "listViews", default)]
    pub listviews: Vec<ListView>,
}

/// Detailed description of a list view.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListViewDescribe {
    pub id: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub label: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub columns: Vec<ListViewColumn>,
    #[serde(rename = "orderBy", default)]
    pub order_by: Vec<ListViewOrderBy>,
    #[serde(rename = "whereCondition")]
    pub where_condition: Option<serde_json::Value>,
}

/// A column in a list view.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListViewColumn {
    #[serde(rename = "fieldNameOrPath")]
    pub field_name_or_path: String,
    pub label: String,
    pub sortable: bool,
    #[serde(rename = "type")]
    pub field_type: String,
}

/// Order by clause for a list view.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListViewOrderBy {
    #[serde(rename = "fieldNameOrPath")]
    pub field_name_or_path: String,
    #[serde(rename = "sortDirection")]
    pub sort_direction: String,
    #[serde(rename = "nullsPosition")]
    pub nulls_position: Option<String>,
}

/// Result of executing a list view.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListViewResult<T> {
    pub done: bool,
    pub id: String,
    pub label: String,
    pub records: Vec<T>,
    pub size: i32,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    #[serde(rename = "nextRecordsUrl")]
    pub next_records_url: Option<String>,
}

impl<T> Default for ListViewResult<T> {
    fn default() -> Self {
        Self {
            done: true,
            id: String::new(),
            label: String::new(),
            records: Vec::new(),
            size: 0,
            developer_name: String::new(),
            next_records_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_view_deserialize() {
        let json = json!({
            "id": "00Bxx0000000001",
            "developerName": "AllAccounts",
            "label": "All Accounts",
            "describeUrl": "/services/data/v62.0/sobjects/Account/listviews/00Bxx0000000001/describe",
            "resultsUrl": "/services/data/v62.0/sobjects/Account/listviews/00Bxx0000000001/results",
            "sobjectType": "Account"
        });
        let lv: ListView = serde_json::from_value(json).unwrap();
        assert_eq!(lv.id, "00Bxx0000000001");
        assert_eq!(lv.developer_name, "AllAccounts");
        assert_eq!(lv.sobject_type, "Account");
    }

    #[test]
    fn test_list_view_collection_deserialize() {
        let json = json!({
            "done": true,
            "nextRecordsUrl": null,
            "listviews": [{
                "id": "00Bxx0000000001",
                "developerName": "AllAccounts",
                "label": "All Accounts",
                "describeUrl": "/describe",
                "resultsUrl": "/results",
                "sobjectType": "Account"
            }]
        });
        let collection: ListViewCollection = serde_json::from_value(json).unwrap();
        assert!(collection.done);
        assert_eq!(collection.listviews.len(), 1);
    }

    #[test]
    fn test_list_view_collection_camel_case_alias() {
        // Salesforce API may return "listViews" (camelCase) — verify alias works
        let json = json!({
            "done": true,
            "nextRecordsUrl": null,
            "listViews": [{
                "id": "00Bxx0000000001",
                "developerName": "AllAccounts",
                "label": "All Accounts",
                "describeUrl": "/describe",
                "resultsUrl": "/results",
                "sobjectType": "Account"
            }]
        });
        let collection: ListViewCollection = serde_json::from_value(json).unwrap();
        assert!(collection.done);
        assert_eq!(collection.listviews.len(), 1);
        assert_eq!(collection.listviews[0].developer_name, "AllAccounts");
    }

    #[test]
    fn test_list_view_collection_full_salesforce_response() {
        // Full Salesforce response includes size and sobjectType fields
        let json = json!({
            "done": true,
            "listviews": [{
                "describeUrl": "/services/data/v62.0/sobjects/Account/listviews/00Bxx0000000001/describe",
                "developerName": "AllAccounts",
                "id": "00Bxx0000000001",
                "label": "All Accounts",
                "resultsUrl": "/services/data/v62.0/sobjects/Account/listviews/00Bxx0000000001/results",
                "sobjectType": "Account",
                "soqlCompatible": true,
                "url": "/services/data/v62.0/sobjects/Account/listviews/00Bxx0000000001"
            }],
            "nextRecordsUrl": null,
            "size": 1,
            "sobjectType": "Account"
        });
        let collection: ListViewCollection = serde_json::from_value(json).unwrap();
        assert!(collection.done);
        assert_eq!(collection.listviews.len(), 1);
    }

    #[test]
    fn test_list_view_describe_deserialize() {
        let json = json!({
            "id": "00Bxx0000000001",
            "developerName": "AllAccounts",
            "label": "All Accounts",
            "sobjectType": "Account",
            "query": "SELECT Id, Name FROM Account",
            "columns": [{
                "fieldNameOrPath": "Name",
                "label": "Account Name",
                "sortable": true,
                "type": "string"
            }],
            "orderBy": [{
                "fieldNameOrPath": "Name",
                "sortDirection": "ascending",
                "nullsPosition": "first"
            }],
            "whereCondition": null
        });
        let describe: ListViewDescribe = serde_json::from_value(json).unwrap();
        assert_eq!(describe.columns.len(), 1);
        assert!(describe.columns[0].sortable);
        assert_eq!(describe.order_by.len(), 1);
    }

    #[test]
    fn test_list_view_result_deserialize() {
        let json = json!({
            "done": true,
            "id": "00Bxx0000000001",
            "label": "All Accounts",
            "records": [{"Id": "001xx", "Name": "Acme"}],
            "size": 1,
            "developerName": "AllAccounts",
            "nextRecordsUrl": null
        });
        let result: ListViewResult<serde_json::Value> = serde_json::from_value(json).unwrap();
        assert!(result.done);
        assert_eq!(result.size, 1);
        assert_eq!(result.records.len(), 1);
    }

    #[test]
    fn test_list_view_result_default() {
        let result: ListViewResult<serde_json::Value> = ListViewResult::default();
        assert!(result.done);
        assert_eq!(result.size, 0);
        assert!(result.records.is_empty());
        assert!(result.next_records_url.is_none());
    }
}
