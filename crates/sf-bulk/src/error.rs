//! Error types for sf-bulk.

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
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("Client error: {0}")]
    Client(String),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Job error: {0}")]
    Job(String),
    #[error("CSV error: {0}")]
    Csv(String),
    #[error("Upload error: {0}")]
    Upload(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Timeout: {0}")]
    Timeout(String),
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

impl From<csv::Error> for Error {
    fn from(err: csv::Error) -> Self {
        Error { kind: ErrorKind::Csv(err.to_string()), source: Some(Box::new(err)) }
    }
}
