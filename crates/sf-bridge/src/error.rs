//! Error types for the bridge crate.

/// Bridge-level errors (host-side only, not crossing the WASM boundary).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from the Extism runtime.
    #[error("extism error: {0}")]
    Extism(#[from] extism::Error),

    /// Error serializing/deserializing data at the ABI boundary.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Error from the Salesforce REST client.
    #[error("salesforce REST error: {0}")]
    SalesforceRest(#[from] busbar_sf_rest::Error),

    /// Error from the Salesforce Bulk API client.
    #[error("salesforce bulk error: {0}")]
    SalesforceBulk(#[from] busbar_sf_bulk::Error),

    /// Error from the Salesforce Tooling API client.
    #[error("salesforce tooling error: {0}")]
    SalesforceTooling(#[from] busbar_sf_tooling::Error),

    /// Error from the Salesforce Metadata API client.
    #[error("salesforce metadata error: {0}")]
    SalesforceMetadata(#[from] busbar_sf_metadata::Error),

    /// Error from the Salesforce client.
    #[error("salesforce client error: {0}")]
    SalesforceClient(#[from] busbar_sf_client::Error),

    /// Error from the Salesforce auth client.
    #[error("salesforce auth error: {0}")]
    SalesforceAuth(#[from] busbar_sf_auth::Error),

    /// Error from a tokio join handle.
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, Error>;
