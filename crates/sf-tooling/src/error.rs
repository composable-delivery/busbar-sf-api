//! Error types for sf-tooling.

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

    #[error("Salesforce error: {error_code} - {message}")]
    Salesforce { error_code: String, message: String },

    #[error("Apex compilation error: {0}")]
    ApexCompilation(String),

    #[error("Apex execution error: {0}")]
    ApexExecution(String),

    #[error("{0}")]
    Other(String),
}

impl From<busbar_sf_client::Error> for Error {
    fn from(err: busbar_sf_client::Error) -> Self {
        Error {
            kind: ErrorKind::Client(err.to_string()),
            source: Some(Box::new(err)),
        }
    }
}
