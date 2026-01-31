//! List Views types and operations.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// A list view for an SObject.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ListView {
    pub id: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub label: String,
    #[serde(rename = "describeUrl")]
    pub describe_url: String,
    #[serde(rename = "resultsUrl")]
    pub results_url: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: String,
}

/// List of list views.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ListViewCollection {
    pub done: bool,
    #[serde(rename = "nextRecordsUrl")]
    pub next_records_url: Option<String>,
    pub listviews: Vec<ListView>,
}

/// Detailed description of a list view.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ListViewDescribe {
    pub id: String,
    #[serde(rename = "developerName")]
    pub developer_name: String,
    pub label: String,
    #[serde(rename = "sobjectType")]
    pub sobject_type: String,
    pub query: String,
    pub columns: Vec<ListViewColumn>,
    #[serde(rename = "orderBy")]
    pub order_by: Vec<ListViewOrderBy>,
    #[serde(rename = "whereCondition")]
    pub where_condition: Option<serde_json::Value>,
}

/// A column in a list view.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ListViewColumn {
    #[serde(rename = "fieldNameOrPath")]
    pub field_name_or_path: String,
    pub label: String,
    #[serde(rename = "sortable")]
    pub sortable: bool,
    #[serde(rename = "type")]
    pub field_type: String,
}

/// Sort order for a list view.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
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
#[serde(bound(deserialize = "T: DeserializeOwned"), default)]
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
            done: false,
            id: String::new(),
            label: String::new(),
            records: Vec::new(),
            size: 0,
            developer_name: String::new(),
            next_records_url: None,
        }
    }
}
