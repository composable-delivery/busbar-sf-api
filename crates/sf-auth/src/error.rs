//! Error types for sf-auth.
//!
//! Error messages are designed to avoid exposing sensitive credential data.

/// Result type alias for sf-auth operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for sf-auth operations.
///
/// Error messages are sanitized to prevent accidental credential exposure.
#[derive(Debug, thiserror::Error)]
#[error("{kind}")]
pub struct Error {
    /// The kind of error that occurred.
    pub kind: ErrorKind,
    /// Optional source error.
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    /// Create a new error with the given kind.
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Create a new error with the given kind and source.
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

/// The kind of error that occurred.
///
/// Error messages avoid including credential values.
#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    /// OAuth error response from Salesforce.
    #[error("OAuth error: {error} - {description}")]
    OAuth { error: String, description: String },

    /// Token expired.
    #[error("Token expired")]
    TokenExpired,

    /// Token invalid.
    #[error("Token invalid: {0}")]
    TokenInvalid(String),

    /// JWT signing error.
    #[error("JWT error: {0}")]
    Jwt(String),

    /// Invalid credentials configuration.
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    /// HTTP error during authentication.
    #[error("HTTP error: {0}")]
    Http(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(String),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Environment variable not set.
    #[error("Environment variable not set: {0}")]
    EnvVar(String),

    /// SFDX CLI error.
    #[error("SFDX CLI error: {0}")]
    SfdxCli(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid input provided.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        // Sanitize the error message to avoid exposing URLs with tokens
        let message = err.to_string();
        let sanitized = if message.contains("access_token") || message.contains("token=") {
            "HTTP request failed (details redacted for security)".to_string()
        } else {
            message
        };
        Error::with_source(ErrorKind::Http(sanitized), err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::with_source(ErrorKind::Json(err.to_string()), err)
    }
}

impl From<serde_urlencoded::ser::Error> for Error {
    fn from(err: serde_urlencoded::ser::Error) -> Self {
        Error::with_source(ErrorKind::Serialization(err.to_string()), err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::with_source(ErrorKind::Io(err.to_string()), err)
    }
}

impl From<std::env::VarError> for Error {
    fn from(err: std::env::VarError) -> Self {
        Error::with_source(ErrorKind::EnvVar(err.to_string()), err)
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        Error::with_source(ErrorKind::Jwt(err.to_string()), err)
    }
}

impl From<busbar_sf_client::Error> for Error {
    fn from(err: busbar_sf_client::Error) -> Self {
        // Sanitize any potential credential exposure
        let message = err.to_string();
        let sanitized = if message.contains("Bearer") || message.contains("token") {
            "Client error (details redacted for security)".to_string()
        } else {
            message
        };
        Error::with_source(ErrorKind::Http(sanitized), err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_display() {
        let err = ErrorKind::TokenExpired;
        assert_eq!(err.to_string(), "Token expired");

        let err = ErrorKind::OAuth {
            error: "invalid_grant".to_string(),
            description: "expired access/refresh token".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "OAuth error: invalid_grant - expired access/refresh token"
        );
    }

    #[test]
    fn test_error_messages_dont_contain_credentials() {
        // Ensure common error patterns don't leak credentials
        let err = Error::new(ErrorKind::TokenInvalid("validation failed".to_string()));
        let msg = err.to_string();
        assert!(!msg.contains("Bearer"));
        assert!(!msg.contains("00D")); // Salesforce org ID prefix
    }
}
