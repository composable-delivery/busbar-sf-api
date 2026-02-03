//! Error sanitization utilities for host functions.
//!
//! These functions map internal error types to stable, non-leaking error codes
//! that are safe to return to WASM guests.

#[cfg(feature = "rest")]
use busbar_sf_client::ErrorKind as ClientErrorKind;
#[cfg(feature = "rest")]
use busbar_sf_rest::ErrorKind as RestErrorKind;

/// Sanitize an error for safe return to WASM guests.
///
/// Maps internal error types to stable, non-leaking error codes.
/// The message is preserved as it typically contains user-actionable info,
/// but the code is sanitized to avoid exposing internal type names.
#[cfg(feature = "rest")]
pub(crate) fn sanitize_rest_error(err: &busbar_sf_rest::Error) -> (String, String) {
    let code = match &err.kind {
        RestErrorKind::Client(_msg) => {
            // Check if the source is a client error with more specific kind
            if let Some(source) = &err.source {
                if let Some(client_err) = source.downcast_ref::<busbar_sf_client::Error>() {
                    match &client_err.kind {
                        ClientErrorKind::Http { status, .. } => format!("HTTP_{}", status),
                        ClientErrorKind::RateLimited { .. } => "RATE_LIMITED".to_string(),
                        ClientErrorKind::Authentication(_) => "AUTH_ERROR".to_string(),
                        ClientErrorKind::Authorization(_) => "AUTHORIZATION_ERROR".to_string(),
                        ClientErrorKind::NotFound(_) => "NOT_FOUND".to_string(),
                        ClientErrorKind::PreconditionFailed(_) => "PRECONDITION_FAILED".to_string(),
                        ClientErrorKind::Timeout => "TIMEOUT".to_string(),
                        ClientErrorKind::Connection(_) => "CONNECTION_ERROR".to_string(),
                        ClientErrorKind::Json(_) => "JSON_ERROR".to_string(),
                        ClientErrorKind::InvalidUrl(_) => "INVALID_URL".to_string(),
                        ClientErrorKind::Serialization(_) => "SERIALIZATION_ERROR".to_string(),
                        ClientErrorKind::Config(_) => "CONFIG_ERROR".to_string(),
                        ClientErrorKind::SalesforceApi { error_code, .. } => error_code.clone(),
                        ClientErrorKind::RetriesExhausted { .. } => "RETRIES_EXHAUSTED".to_string(),
                        ClientErrorKind::Other(_) => "CLIENT_ERROR".to_string(),
                    }
                } else {
                    "CLIENT_ERROR".to_string()
                }
            } else {
                "CLIENT_ERROR".to_string()
            }
        }
        RestErrorKind::Auth(_) => "AUTH_ERROR".to_string(),
        RestErrorKind::Salesforce { error_code, .. } => error_code.clone(),
        RestErrorKind::Other(_) => "OTHER_ERROR".to_string(),
    };

    (code, err.to_string())
}

/// Sanitize bulk API errors.
#[cfg(feature = "bulk")]
pub(crate) fn sanitize_bulk_error(err: &busbar_sf_bulk::Error) -> (String, String) {
    // Bulk errors typically wrap client/rest errors, so try to extract those
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    // Fallback to generic bulk error code
    ("BULK_ERROR".to_string(), err.to_string())
}

/// Sanitize tooling API errors.
#[cfg(feature = "tooling")]
pub(crate) fn sanitize_tooling_error(err: &busbar_sf_tooling::Error) -> (String, String) {
    // Tooling errors typically wrap client/rest errors
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    ("TOOLING_ERROR".to_string(), err.to_string())
}

/// Sanitize metadata API errors.
#[cfg(feature = "metadata")]
pub(crate) fn sanitize_metadata_error(err: &busbar_sf_metadata::Error) -> (String, String) {
    // Metadata errors wrap various error types
    if let Some(source) = &err.source {
        if let Some(rest_err) = source.downcast_ref::<busbar_sf_rest::Error>() {
            return sanitize_rest_error(rest_err);
        }
    }
    ("METADATA_ERROR".to_string(), err.to_string())
}
