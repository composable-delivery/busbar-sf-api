//! List metadata operations.

use serde::{Deserialize, Serialize};

/// A metadata component from list metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataComponent {
    pub full_name: String,
    pub file_name: Option<String>,
    pub id: Option<String>,
    pub namespace_prefix: Option<String>,
    pub metadata_type: String,
    pub created_by_id: Option<String>,
    pub created_by_name: Option<String>,
    pub created_date: Option<String>,
    pub last_modified_by_id: Option<String>,
    pub last_modified_by_name: Option<String>,
    pub last_modified_date: Option<String>,
    pub manageable_state: Option<String>,
}
