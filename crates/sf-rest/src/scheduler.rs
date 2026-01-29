//! Salesforce Scheduler API types.
//!
//! Provides access to appointment scheduling functionality.
//! See: https://developer.salesforce.com/docs/atlas.en-us.api_rest.meta/api_rest/resources_ls_intro.htm

use serde::{Deserialize, Serialize};

/// Request to get appointment candidates.
#[derive(Debug, Clone, Serialize)]
pub struct AppointmentCandidatesRequest {
    /// Scheduling policy ID
    #[serde(rename = "schedulingPolicyId", skip_serializing_if = "Option::is_none")]
    pub scheduling_policy_id: Option<String>,
    /// Work type ID
    #[serde(rename = "workTypeId", skip_serializing_if = "Option::is_none")]
    pub work_type_id: Option<String>,
    /// Account ID
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Additional request parameters
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

/// Response containing appointment candidates.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppointmentCandidatesResponse {
    /// List of candidate time slots
    pub candidates: Vec<AppointmentCandidate>,
}

/// An appointment candidate time slot.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppointmentCandidate {
    /// Start time
    #[serde(rename = "startTime", skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// End time
    #[serde(rename = "endTime", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Resources assigned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Vec<serde_json::Value>>,
    /// Additional candidate data
    #[serde(flatten)]
    pub additional: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_appointment_candidates_request() {
        let request = AppointmentCandidatesRequest {
            scheduling_policy_id: Some("0VsB000000001".to_string()),
            work_type_id: Some("08qB000000001".to_string()),
            account_id: Some("001B000000001".to_string()),
            additional: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("schedulingPolicyId"));
    }

    #[test]
    fn test_appointment_candidate_deserialization() {
        let json = r#"{
            "startTime": "2024-01-15T09:00:00Z",
            "endTime": "2024-01-15T10:00:00Z"
        }"#;
        let candidate: AppointmentCandidate = serde_json::from_str(json).unwrap();
        assert_eq!(
            candidate.start_time,
            Some("2024-01-15T09:00:00Z".to_string())
        );
    }
}
