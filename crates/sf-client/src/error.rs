//! Error types for sf-client.

use std::time::Duration;

/// Result type alias for sf-client operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for sf-client operations.
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

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }

    /// Returns true if this is a rate limit error.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self.kind, ErrorKind::RateLimited { .. })
    }

    /// Returns true if this is an authentication error.
    pub fn is_auth_error(&self) -> bool {
        matches!(self.kind, ErrorKind::Authentication(_))
    }

    /// Returns the retry-after duration if this is a rate limit error.
    pub fn retry_after(&self) -> Option<Duration> {
        match &self.kind {
            ErrorKind::RateLimited { retry_after } => *retry_after,
            _ => None,
        }
    }
}

/// The kind of error that occurred.
#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    /// HTTP request failed.
    #[error("HTTP error: {status} {message}")]
    Http { status: u16, message: String },

    /// Rate limit exceeded (HTTP 429).
    #[error("Rate limited{}", retry_after.map(|d| format!(", retry after {:?}", d)).unwrap_or_default())]
    RateLimited { retry_after: Option<Duration> },

    /// Authentication error (HTTP 401).
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Authorization error (HTTP 403).
    #[error("Authorization error: {0}")]
    Authorization(String),

    /// Resource not found (HTTP 404).
    #[error("Not found: {0}")]
    NotFound(String),

    /// Precondition failed (HTTP 412) - ETag mismatch.
    #[error("Precondition failed: {0}")]
    PreconditionFailed(String),

    /// Request timeout.
    #[error("Request timeout")]
    Timeout,

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(String),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Salesforce API error response.
    #[error("Salesforce API error: {error_code} - {message}")]
    SalesforceApi {
        error_code: String,
        message: String,
        fields: Vec<String>,
    },

    /// All retries exhausted.
    #[error("All {attempts} retry attempts exhausted")]
    RetriesExhausted { attempts: u32 },

    /// Other error.
    #[error("{0}")]
    Other(String),
}

impl ErrorKind {
    /// Returns true if this error kind is retryable.
    pub fn is_retryable(&self) -> bool {
        match self {
            ErrorKind::RateLimited { .. } => true,
            ErrorKind::Timeout => true,
            ErrorKind::Connection(_) => true,
            ErrorKind::Http { status, .. } => is_retryable_status(*status),
            _ => false,
        }
    }
}

/// Check if an HTTP status code is typically retryable.
fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        let kind = if err.is_timeout() {
            ErrorKind::Timeout
        } else if err.is_connect() {
            ErrorKind::Connection(err.to_string())
        } else if let Some(status) = err.status() {
            ErrorKind::Http {
                status: status.as_u16(),
                message: err.to_string(),
            }
        } else {
            ErrorKind::Other(err.to_string())
        };

        Error::with_source(kind, err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::with_source(ErrorKind::Json(err.to_string()), err)
    }
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::with_source(ErrorKind::Config(format!("Invalid URL: {}", err)), err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        let err = Error::new(ErrorKind::RateLimited { retry_after: None });
        assert!(err.is_retryable());

        let err = Error::new(ErrorKind::Timeout);
        assert!(err.is_retryable());

        let err = Error::new(ErrorKind::Http {
            status: 503,
            message: "Service unavailable".to_string(),
        });
        assert!(err.is_retryable());

        let err = Error::new(ErrorKind::NotFound("resource".to_string()));
        assert!(!err.is_retryable());

        let err = Error::new(ErrorKind::Authentication("invalid".to_string()));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_is_rate_limited() {
        let err = Error::new(ErrorKind::RateLimited {
            retry_after: Some(Duration::from_secs(30)),
        });
        assert!(err.is_rate_limited());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(30)));

        let err = Error::new(ErrorKind::Timeout);
        assert!(!err.is_rate_limited());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn test_error_is_auth_error() {
        let err = Error::new(ErrorKind::Authentication("expired".to_string()));
        assert!(err.is_auth_error());

        let err = Error::new(ErrorKind::Authorization("forbidden".to_string()));
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_salesforce_api_error() {
        let err = Error::new(ErrorKind::SalesforceApi {
            error_code: "INVALID_FIELD".to_string(),
            message: "No such column 'foo' on entity 'Account'".to_string(),
            fields: vec!["foo".to_string()],
        });

        assert!(!err.is_retryable());
        assert!(err.to_string().contains("INVALID_FIELD"));
    }

    #[test]
    fn test_error_kind_display_messages() {
        // Verify each ErrorKind variant formats its Display message correctly
        let cases: Vec<(ErrorKind, &str)> = vec![
            (
                ErrorKind::Http {
                    status: 500,
                    message: "Internal Server Error".into(),
                },
                "HTTP error: 500 Internal Server Error",
            ),
            (
                ErrorKind::RateLimited {
                    retry_after: Some(Duration::from_secs(30)),
                },
                "retry after",
            ),
            (ErrorKind::RateLimited { retry_after: None }, "Rate limited"),
            (
                ErrorKind::Authentication("expired token".into()),
                "Authentication error: expired token",
            ),
            (
                ErrorKind::Authorization("insufficient privileges".into()),
                "Authorization error: insufficient privileges",
            ),
            (
                ErrorKind::NotFound("Account/001".into()),
                "Not found: Account/001",
            ),
            (
                ErrorKind::PreconditionFailed("ETag mismatch".into()),
                "Precondition failed: ETag mismatch",
            ),
            (ErrorKind::Timeout, "Request timeout"),
            (
                ErrorKind::Connection("refused".into()),
                "Connection error: refused",
            ),
            (
                ErrorKind::Json("unexpected EOF".into()),
                "JSON error: unexpected EOF",
            ),
            (
                ErrorKind::InvalidUrl("no scheme".into()),
                "Invalid URL: no scheme",
            ),
            (
                ErrorKind::Serialization("not a map".into()),
                "Serialization error: not a map",
            ),
            (
                ErrorKind::Config("missing field".into()),
                "Configuration error: missing field",
            ),
            (
                ErrorKind::RetriesExhausted { attempts: 3 },
                "All 3 retry attempts exhausted",
            ),
            (ErrorKind::Other("something else".into()), "something else"),
        ];

        for (kind, expected_substring) in cases {
            let display = kind.to_string();
            assert!(
                display.contains(expected_substring),
                "Expected '{display}' to contain '{expected_substring}'"
            );
        }
    }

    #[test]
    fn test_retryable_http_status_codes() {
        let retryable = [429, 500, 502, 503, 504];
        for status in retryable {
            let err = Error::new(ErrorKind::Http {
                status,
                message: "error".into(),
            });
            assert!(err.is_retryable(), "HTTP {status} should be retryable");
        }

        let non_retryable = [400, 401, 403, 404, 405, 409, 422];
        for status in non_retryable {
            let err = Error::new(ErrorKind::Http {
                status,
                message: "error".into(),
            });
            assert!(!err.is_retryable(), "HTTP {status} should NOT be retryable");
        }
    }

    #[test]
    fn test_error_with_source() {
        let source_err = std::io::Error::other("disk full");
        let err = Error::with_source(ErrorKind::Other("write failed".into()), source_err);

        assert!(err.source.is_some());
        assert_eq!(err.to_string(), "write failed");
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<String>("not valid json").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err.kind, ErrorKind::Json(_)));
        assert!(err.source.is_some());
    }

    #[test]
    fn test_from_url_parse_error() {
        let url_err = url::Url::parse("not a url").unwrap_err();
        let err: Error = url_err.into();
        assert!(matches!(err.kind, ErrorKind::Config(_)));
        assert!(err.to_string().contains("Invalid URL"));
    }
}
