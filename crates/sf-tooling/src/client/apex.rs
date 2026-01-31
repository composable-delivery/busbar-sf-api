use busbar_sf_client::security::{soql, url as url_security};
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};
use crate::types::*;

impl super::ToolingClient {
    // =========================================================================
    // Apex Class Operations
    // =========================================================================

    /// Get all Apex classes in the org.
    #[instrument(skip(self))]
    pub async fn get_apex_classes(&self) -> Result<Vec<ApexClass>> {
        self.query_all("SELECT Id, Name, Body, Status, IsValid, ApiVersion, NamespacePrefix, CreatedDate, LastModifiedDate FROM ApexClass")
            .await
    }

    /// Get an Apex class by name.
    #[instrument(skip(self))]
    pub async fn get_apex_class_by_name(&self, name: &str) -> Result<Option<ApexClass>> {
        let safe_name = soql::escape_string(name);
        let soql = format!(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, NamespacePrefix, CreatedDate, LastModifiedDate FROM ApexClass WHERE Name = '{}'",
            safe_name
        );
        let mut classes: Vec<ApexClass> = self.query_all(&soql).await?;
        Ok(classes.pop())
    }

    /// Get an Apex class by ID.
    #[instrument(skip(self))]
    pub async fn get_apex_class(&self, id: &str) -> Result<ApexClass> {
        if !url_security::is_valid_salesforce_id(id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/ApexClass/{}", id);
        self.client.tooling_get(&path).await.map_err(Into::into)
    }

    // =========================================================================
    // Apex Trigger Operations
    // =========================================================================

    /// Get all Apex triggers in the org.
    #[instrument(skip(self))]
    pub async fn get_apex_triggers(&self) -> Result<Vec<ApexTrigger>> {
        self.query_all(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, TableEnumOrId FROM ApexTrigger",
        )
        .await
    }

    /// Get an Apex trigger by name.
    #[instrument(skip(self))]
    pub async fn get_apex_trigger_by_name(&self, name: &str) -> Result<Option<ApexTrigger>> {
        let safe_name = soql::escape_string(name);
        let soql = format!(
            "SELECT Id, Name, Body, Status, IsValid, ApiVersion, TableEnumOrId FROM ApexTrigger WHERE Name = '{}'",
            safe_name
        );
        let mut triggers: Vec<ApexTrigger> = self.query_all(&soql).await?;
        Ok(triggers.pop())
    }
}
