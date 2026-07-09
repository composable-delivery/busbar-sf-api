//! Error types for sf-metadata.

use crate::deploy::DeployResult;
use crate::redact::redact_session_ids;

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

    pub fn with_source(
        kind: ErrorKind,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            source: Some(Box::new(source)),
        }
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
    /// A deploy that reached a terminal, unsuccessful state. Carries the
    /// FULL typed [`DeployResult`] Salesforce returned — not just a message
    /// string — so callers get programmatic access to status, every count
    /// (components/tests deployed/errored/total), and every
    /// [`crate::deploy::ComponentFailure`], not only whatever fit in a
    /// human-readable summary. `Display` renders a comprehensive message
    /// from all of it, falling back
    /// gracefully when Salesforce's own `errorMessage` is empty (which
    /// happens whenever the real errors are per-component, not top-level —
    /// previously this produced an unhelpful "Unknown error" with the actual
    /// component failures silently dropped).
    #[error("{}", fmt_deployment_failed(result))]
    DeploymentFailed { result: Box<DeployResult> },
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

/// Build a comprehensive, human-readable summary of a failed deploy from the
/// full typed [`DeployResult`] — status, counts, and up to the first 10
/// component failures (each with its component type/name/problem), not just
/// whatever Salesforce put in the top-level `errorMessage` (which is often
/// empty when the real errors are per-component). Session ids/tokens are
/// redacted defensively (see [`crate::redact`]) — Salesforce-generated
/// problem text shouldn't contain one, but a malformed/faulted response
/// occasionally echoes request content back.
fn fmt_deployment_failed(result: &DeployResult) -> String {
    let mut msg = format!(
        "deployment {:?} ({}/{} component(s) failed",
        result.status, result.number_components_errors, result.number_components_total,
    );
    if result.number_tests_errors > 0 {
        msg.push_str(&format!(", {} test(s) failed", result.number_tests_errors));
    }
    msg.push(')');
    if let Some(m) = &result.error_message {
        if !m.is_empty() {
            msg.push_str(&format!(": {m}"));
        }
    }
    if !result.component_failures.is_empty() {
        msg.push_str(" — component failures: ");
        let shown = result.component_failures.iter().take(10);
        let lines: Vec<String> = shown
            .map(|f| {
                let name = f.full_name.as_deref().unwrap_or("<unknown>");
                format!("[{name}] {}: {}", f.problem_type, f.problem)
            })
            .collect();
        msg.push_str(&lines.join("; "));
        if result.component_failures.len() > 10 {
            msg.push_str(&format!(
                " (+{} more)",
                result.component_failures.len() - 10
            ));
        }
    }
    redact_session_ids(&msg)
}

impl From<busbar_sf_client::Error> for Error {
    fn from(err: busbar_sf_client::Error) -> Self {
        Error {
            kind: ErrorKind::Client(err.to_string()),
            source: Some(Box::new(err)),
        }
    }
}

impl From<busbar_sf_auth::Error> for Error {
    fn from(err: busbar_sf_auth::Error) -> Self {
        Error {
            kind: ErrorKind::Auth(err.to_string()),
            source: Some(Box::new(err)),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error {
            kind: ErrorKind::Io(err.to_string()),
            source: Some(Box::new(err)),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error {
            kind: ErrorKind::Http(err.to_string()),
            source: Some(Box::new(err)),
        }
    }
}
