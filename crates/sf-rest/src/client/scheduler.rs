use tracing::instrument;

use crate::error::Result;
use crate::scheduler::{AppointmentCandidatesRequest, AppointmentCandidatesResponse};

impl super::SalesforceRestClient {
    /// Get appointment candidates based on scheduling parameters.
    #[instrument(skip(self, request))]
    pub async fn appointment_candidates(
        &self,
        request: &AppointmentCandidatesRequest,
    ) -> Result<AppointmentCandidatesResponse> {
        self.client
            .rest_post("scheduling/getAppointmentCandidates", request)
            .await
            .map_err(Into::into)
    }

    /// Get appointment slots based on scheduling parameters.
    #[instrument(skip(self, request))]
    pub async fn appointment_slots(
        &self,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.client
            .rest_post("scheduling/getAppointmentSlots", request)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_appointment_candidates_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "candidates": [{
                "startTime": "2024-01-01T09:00:00.000Z",
                "endTime": "2024-01-01T10:00:00.000Z",
                "territoryId": "0Hhxx0000000001"
            }]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/scheduling/getAppointmentCandidates$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = crate::scheduler::AppointmentCandidatesRequest {
            start_time: "2024-01-01T09:00:00.000Z".to_string(),
            end_time: "2024-01-01T17:00:00.000Z".to_string(),
            work_type_group_id: None,
            work_type_id: None,
            account_id: None,
            territory_ids: None,
        };
        let result = client
            .appointment_candidates(&request)
            .await
            .expect("appointment_candidates should succeed");
        assert_eq!(result.candidates.len(), 1);
    }

    #[tokio::test]
    async fn test_appointment_slots_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "timeSlots": [
                {"startTime": "2024-01-01T09:00:00.000Z", "endTime": "2024-01-01T10:00:00.000Z"}
            ]
        });

        Mock::given(method("POST"))
            .and(path_regex(".*/scheduling/getAppointmentSlots$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let request = serde_json::json!({
            "startTime": "2024-01-01T09:00:00.000Z",
            "endTime": "2024-01-01T17:00:00.000Z"
        });
        let result = client
            .appointment_slots(&request)
            .await
            .expect("appointment_slots should succeed");
        assert!(result["timeSlots"].is_array());
    }
}
