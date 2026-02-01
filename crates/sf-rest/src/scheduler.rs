//! Scheduler types for the Salesforce REST API.

use serde::{Deserialize, Serialize};

/// Request for appointment candidates.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppointmentCandidatesRequest {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    #[serde(rename = "workTypeGroupId", skip_serializing_if = "Option::is_none")]
    pub work_type_group_id: Option<String>,
    #[serde(rename = "workTypeId", skip_serializing_if = "Option::is_none")]
    pub work_type_id: Option<String>,
    #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(rename = "territoryIds", skip_serializing_if = "Option::is_none")]
    pub territory_ids: Option<Vec<String>>,
}

/// Response with appointment candidates.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppointmentCandidatesResponse {
    #[serde(default)]
    pub candidates: Vec<AppointmentCandidate>,
}

/// An appointment candidate (time slot).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppointmentCandidate {
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "endTime")]
    pub end_time: String,
    #[serde(rename = "territoryId")]
    pub territory_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_appointment_candidates_request_serialize() {
        let request = AppointmentCandidatesRequest {
            start_time: "2024-01-01T09:00:00.000Z".to_string(),
            end_time: "2024-01-01T17:00:00.000Z".to_string(),
            work_type_group_id: Some("0VSxx0000000001".to_string()),
            work_type_id: None,
            account_id: None,
            territory_ids: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["startTime"], "2024-01-01T09:00:00.000Z");
        assert_eq!(json["workTypeGroupId"], "0VSxx0000000001");
        assert!(json.get("workTypeId").is_none());
    }

    #[test]
    fn test_appointment_candidates_response_deserialize() {
        let json = json!({
            "candidates": [{
                "startTime": "2024-01-01T09:00:00.000Z",
                "endTime": "2024-01-01T10:00:00.000Z",
                "territoryId": "0Hhxx0000000001"
            }]
        });
        let response: AppointmentCandidatesResponse = serde_json::from_value(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(
            response.candidates[0].start_time,
            "2024-01-01T09:00:00.000Z"
        );
    }

    #[test]
    fn test_appointment_candidates_response_empty() {
        let json = json!({"candidates": []});
        let response: AppointmentCandidatesResponse = serde_json::from_value(json).unwrap();
        assert!(response.candidates.is_empty());
    }
}
