use busbar_sf_client::security::{soql, url as url_security};
use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};

impl super::ToolingClient {
    /// Get multiple Tooling API records by ID in a single request.
    ///
    /// Uses a Tooling API SOQL query (`WHERE Id IN (...)`) internally.
    /// The Tooling API's SObject Collections GET endpoint is documented but
    /// does not work reliably for most objects, so we use SOQL instead.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `ids` - Array of record IDs to retrieve
    /// * `fields` - Array of field names to return
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let classes: Vec<serde_json::Value> = client
    ///     .get_multiple("ApexClass", &["01p...", "01p..."], &["Id", "Name", "Body"])
    ///     .await?;
    /// ```
    #[instrument(skip(self))]
    pub async fn get_multiple<T: serde::de::DeserializeOwned + Clone>(
        &self,
        sobject: &str,
        ids: &[&str],
        fields: &[&str],
    ) -> Result<Vec<T>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        for id in ids {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let safe_fields: Vec<&str> = soql::filter_safe_fields(fields.iter().copied()).collect();
        if safe_fields.is_empty() {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELDS".to_string(),
                message: "No valid field names provided".to_string(),
            }));
        }
        // Build a SOQL query: SELECT fields FROM sobject WHERE Id IN ('id1','id2',...)
        // IDs are already validated by is_valid_salesforce_id (alphanumeric only),
        // so they are safe to embed directly.
        let fields_clause = safe_fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );
        self.query_all(&soql).await
    }

    /// Create multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `records` - Array of records to create
    /// * `all_or_none` - If true, all records must succeed or all fail
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let records = vec![
    ///     json!({"Name": "TestClass1", "Body": "public class TestClass1 {}"}),
    ///     json!({"Name": "TestClass2", "Body": "public class TestClass2 {}"}),
    /// ];
    ///
    /// let results = client.create_multiple("ApexClass", &records, false).await?;
    /// ```
    #[instrument(skip(self, records))]
    pub async fn create_multiple<T: serde::Serialize>(
        &self,
        sobject: &str,
        records: &[T],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let request = busbar_sf_rest::CollectionRequest {
            all_or_none,
            records: records
                .iter()
                .map(|r| {
                    let mut value = serde_json::to_value(r).unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Object(ref mut map) = value {
                        map.insert(
                            "attributes".to_string(),
                            serde_json::json!({"type": sobject}),
                        );
                    }
                    value
                })
                .collect(),
        };
        let url = self.client.tooling_url("composite/sobjects");
        self.client
            .post_json(&url, &request)
            .await
            .map_err(Into::into)
    }

    /// Update multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `sobject` - The SObject type (e.g., "ApexClass", "CustomField")
    /// * `records` - Array of (id, record) tuples to update
    /// * `all_or_none` - If true, all records must succeed or all fail
    #[instrument(skip(self, records))]
    pub async fn update_multiple<T: serde::Serialize>(
        &self,
        sobject: &str,
        records: &[(String, T)],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        // Validate all IDs
        for (id, _) in records {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let request = busbar_sf_rest::CollectionRequest {
            all_or_none,
            records: records
                .iter()
                .map(|(id, r)| {
                    let mut value = serde_json::to_value(r).unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Object(ref mut map) = value {
                        map.insert(
                            "attributes".to_string(),
                            serde_json::json!({"type": sobject}),
                        );
                        map.insert("Id".to_string(), serde_json::json!(id));
                    }
                    value
                })
                .collect(),
        };

        let url = self.client.tooling_url("composite/sobjects");
        let request_builder = self.client.patch(&url).json(&request)?;
        let response = self.client.execute(request_builder).await?;
        response.json().await.map_err(Into::into)
    }

    /// Delete multiple Tooling API records in a single request (up to 200).
    ///
    /// Available since API v45.0.
    ///
    /// # Arguments
    /// * `ids` - Array of record IDs to delete
    /// * `all_or_none` - If true, all records must succeed or all fail
    #[instrument(skip(self))]
    pub async fn delete_multiple(
        &self,
        ids: &[&str],
        all_or_none: bool,
    ) -> Result<Vec<busbar_sf_rest::CollectionResult>> {
        // Validate all IDs before proceeding
        for id in ids {
            if !url_security::is_valid_salesforce_id(id) {
                return Err(Error::new(ErrorKind::Salesforce {
                    error_code: "INVALID_ID".to_string(),
                    message: "Invalid Salesforce ID format".to_string(),
                }));
            }
        }
        let ids_param = ids.join(",");
        let url = format!(
            "{}/services/data/v{}/tooling/composite/sobjects?ids={}&allOrNone={}",
            self.client.instance_url(),
            self.client.api_version(),
            ids_param,
            all_or_none
        );
        let request = self.client.delete(&url);
        let response = self.client.execute(request).await?;
        response.json().await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ToolingClient;

    #[test]
    fn test_collections_get_soql_construction() {
        let sobject = "ApexClass";
        let ids = ["01p000000000001AAA", "01p000000000002AAA"];
        let fields = ["Id", "Name"];

        let fields_clause = fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );

        assert_eq!(
            soql,
            "SELECT Id, Name FROM ApexClass WHERE Id IN ('01p000000000001AAA', '01p000000000002AAA')"
        );
    }

    #[test]
    fn test_collections_create_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let url = client.client.tooling_url("composite/sobjects");
        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/sobjects"
        );
    }

    #[test]
    fn test_collections_delete_url_construction() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();

        let ids = ["01p000000000001AAA", "01p000000000002AAA"];
        let ids_param = ids.join(",");

        let url = format!(
            "{}/services/data/v{}/tooling/composite/sobjects?ids={}&allOrNone={}",
            client.client.instance_url(),
            client.client.api_version(),
            ids_param,
            false
        );

        assert_eq!(
            url,
            "https://na1.salesforce.com/services/data/v62.0/tooling/composite/sobjects?ids=01p000000000001AAA,01p000000000002AAA&allOrNone=false"
        );
    }

    #[tokio::test]
    async fn test_get_multiple_empty_ids_returns_empty() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: Vec<serde_json::Value> = client
            .get_multiple("ApexClass", &[], &["Id", "Name"])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_sobject_name() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple("Robert'; DROP TABLE--", &["01p000000000001AAA"], &["Id"])
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_SOBJECT"),
            "Expected INVALID_SOBJECT error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_id_format() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple("ApexClass", &["not-a-valid-sf-id!"], &["Id"])
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_ID"),
            "Expected INVALID_ID error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_get_multiple_invalid_fields_filtered() {
        let client = ToolingClient::new("https://na1.salesforce.com", "token").unwrap();
        let result: std::result::Result<Vec<serde_json::Value>, _> = client
            .get_multiple(
                "ApexClass",
                &["01p000000000001AAA"],
                &["'; DROP TABLE--", "1=1 OR"],
            )
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("INVALID_FIELDS"),
            "Expected INVALID_FIELDS error, got: {err}"
        );
    }

    #[test]
    fn test_get_multiple_soql_construction_with_many_ids() {
        let sobject = "ApexClass";
        let ids = [
            "01p000000000001AAA",
            "01p000000000002AAA",
            "01p000000000003AAA",
        ];
        let fields = ["Id", "Name", "Body"];

        let fields_clause = fields.join(", ");
        let ids_clause: Vec<String> = ids.iter().map(|id| format!("'{id}'")).collect();
        let soql = format!(
            "SELECT {} FROM {} WHERE Id IN ({})",
            fields_clause,
            sobject,
            ids_clause.join(", ")
        );

        assert_eq!(
            soql,
            "SELECT Id, Name, Body FROM ApexClass WHERE Id IN ('01p000000000001AAA', '01p000000000002AAA', '01p000000000003AAA')"
        );
    }
}
