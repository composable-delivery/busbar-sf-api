//! SOQL query operations and types.
//!
//! This module re-exports the QueryResult from sf_client and provides
//! additional query-related types.

// Re-export QueryResult from busbar_sf_client to ensure type compatibility
pub use busbar_sf_client::QueryResult;

/// Options for query execution.
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Batch size for results (Sforce-Query-Options header).
    pub batch_size: Option<u32>,
    /// Include deleted records (QueryAll endpoint).
    pub include_deleted: bool,
}
