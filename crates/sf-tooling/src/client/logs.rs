use busbar_sf_client::security::url as url_security;
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};
use crate::types::ApexLog;

impl super::ToolingClient {
    /// Get recent Apex logs.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of logs to return (defaults to 20)
    #[instrument(skip(self))]
    pub async fn get_apex_logs(&self, limit: Option<u32>) -> Result<Vec<ApexLog>> {
        let limit = limit.unwrap_or(20);
        let soql = format!(
            "SELECT Id, LogUserId, LogUser.Name, LogLength, LastModifiedDate, StartTime, Status, Operation, Request, Application, DurationMilliseconds, Location FROM ApexLog ORDER BY LastModifiedDate DESC LIMIT {}",
            limit
        );
        self.query_all(&soql).await
    }

    /// Get the body of a specific Apex log.
    #[instrument(skip(self))]
    pub async fn get_apex_log_body(&self, log_id: &str) -> Result<String> {
        if !url_security::is_valid_salesforce_id(log_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/ApexLog/{}/Body",
            self.client.instance_url(),
            self.client.api_version(),
            log_id
        );

        let request = self.client.get(&url);
        let response = self.client.execute(request).await?;
        response.text().await.map_err(Into::into)
    }

    /// Delete an Apex log.
    #[instrument(skip(self))]
    pub async fn delete_apex_log(&self, log_id: &str) -> Result<()> {
        if !url_security::is_valid_salesforce_id(log_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let url = format!(
            "{}/services/data/v{}/tooling/sobjects/ApexLog/{}",
            self.client.instance_url(),
            self.client.api_version(),
            log_id
        );

        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;

        if response.status() == 204 || response.is_success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Salesforce {
                error_code: "DELETE_FAILED".to_string(),
                message: format!("Failed to delete log: status {}", response.status()),
            }))
        }
    }

    /// Delete all Apex logs for the current user.
    #[instrument(skip(self))]
    pub async fn delete_all_apex_logs(&self) -> Result<u32> {
        let logs = self.get_apex_logs(Some(200)).await?;
        let count = logs.len() as u32;

        for log in logs {
            self.delete_apex_log(&log.id).await?;
        }

        Ok(count)
    }
}
