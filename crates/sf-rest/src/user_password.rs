//! User password management API types.
//!
//! Provides password status, set, and reset operations for User records.
//! See: https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_sobject_user_password.htm

use serde::{Deserialize, Serialize};

/// Response from getting user password status.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserPasswordStatus {
    /// Whether the password is expired
    #[serde(rename = "isExpired")]
    pub is_expired: bool,
}

/// Request to set a user password.
#[derive(Debug, Clone, Serialize)]
pub struct SetPasswordRequest {
    /// The new password to set
    #[serde(rename = "NewPassword")]
    pub new_password: String,
}

/// Response from setting a user password.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetPasswordResponse {
    /// The new password (may be system-generated)
    #[serde(rename = "NewPassword", skip_serializing_if = "Option::is_none")]
    pub new_password: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_status_deserialization() {
        let json = r#"{"isExpired": false}"#;
        let status: UserPasswordStatus = serde_json::from_str(json).unwrap();
        assert!(!status.is_expired);
    }

    #[test]
    fn test_set_password_request() {
        let request = SetPasswordRequest {
            new_password: "NewSecurePassword123!".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("NewPassword"));
    }

    #[test]
    fn test_set_password_response() {
        let json = r#"{"NewPassword": "GeneratedPassword123"}"#;
        let response: SetPasswordResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.new_password,
            Some("GeneratedPassword123".to_string())
        );
    }
}
