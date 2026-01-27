//! Error types for sf-metadata.

use crate::deploy::ComponentFailure;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub fn with_source(kind: ErrorKind, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self { kind, source: Some(Box::new(source)) }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("Client error: {0}")]
    Client(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Deploy error: {0}")]
    Deploy(String),
    #[error("Deployment failed: {message}")]
    DeploymentFailed { message: String, failures: Vec<ComponentFailure> },
    #[error("Retrieve error: {0}")]
    Retrieve(String),
    #[error("Retrieve failed: {0}")]
    RetrieveFailed(String),
    #[error("SOAP fault: {0}")]
    SoapFault(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("XML parse error: {0}")]
    Parse(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Timeout")]
    Timeout,
    #[error("IO error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

impl From<busbar_sf_client::Error> for Error {
    fn from(err: busbar_sf_client::Error) -> Self {
        Error { kind: ErrorKind::Client(err.to_string()), source: Some(Box::new(err)) }
    }
}

impl From<busbar_sf_auth::Error> for Error {
    fn from(err: busbar_sf_auth::Error) -> Self {
        Error { kind: ErrorKind::Auth(err.to_string()), source: Some(Box::new(err)) }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error { kind: ErrorKind::Io(err.to_string()), source: Some(Box::new(err)) }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error { kind: ErrorKind::Http(err.to_string()), source: Some(Box::new(err)) }
    }
}
