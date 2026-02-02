use serde::de::DeserializeOwned;
use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};

use crate::error::{Error, ErrorKind, Result};
use crate::list_views::{ListView, ListViewCollection, ListViewDescribe, ListViewResult};

impl super::SalesforceRestClient {
    /// List all list views for an SObject.
    #[instrument(skip(self))]
    pub async fn list_views(&self, sobject: &str) -> Result<ListViewCollection> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get a specific list view by ID.
    #[instrument(skip(self))]
    pub async fn get_list_view(&self, sobject: &str, list_view_id: &str) -> Result<ListView> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Describe a list view (get columns, filters, etc.).
    #[instrument(skip(self))]
    pub async fn describe_list_view(
        &self,
        sobject: &str,
        list_view_id: &str,
    ) -> Result<ListViewDescribe> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}/describe", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Execute a list view and return its results.
    #[instrument(skip(self))]
    pub async fn execute_list_view<T: DeserializeOwned>(
        &self,
        sobject: &str,
        list_view_id: &str,
    ) -> Result<ListViewResult<T>> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        if !url_security::is_valid_salesforce_id(list_view_id) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_ID".to_string(),
                message: "Invalid Salesforce ID format".to_string(),
            }));
        }
        let path = format!("sobjects/{}/listviews/{}/results", sobject, list_view_id);
        self.client.rest_get(&path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_list_views_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.list_views("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_get_list_view_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.get_list_view("Account", "bad-id").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_describe_list_view_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .describe_list_view("Bad'; DROP--", "00Bxx0000000001AAA")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_execute_list_view_invalid_id() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client
            .execute_list_view::<serde_json::Value>("Account", "bad-id")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_ID"));
    }

    #[tokio::test]
    async fn test_list_views_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "done": true,
            "nextRecordsUrl": null,
            "listviews": [{
                "id": "00Bxx0000000001AAA",
                "developerName": "AllAccounts",
                "label": "All Accounts",
                "describeUrl": "/describe",
                "resultsUrl": "/results",
                "sobjectType": "Account"
            }]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/listviews$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .list_views("Account")
            .await
            .expect("list_views should succeed");
        assert!(result.done);
        assert_eq!(result.listviews.len(), 1);
        assert_eq!(result.listviews[0].developer_name, "AllAccounts");
    }

    #[tokio::test]
    async fn test_get_list_view_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "id": "00Bxx0000000001AAA",
            "developerName": "AllAccounts",
            "label": "All Accounts",
            "describeUrl": "/describe",
            "resultsUrl": "/results",
            "sobjectType": "Account"
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/listviews/00Bxx0000000001AAA$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .get_list_view("Account", "00Bxx0000000001AAA")
            .await
            .expect("get_list_view should succeed");
        assert_eq!(result.id, "00Bxx0000000001AAA");
    }

    #[tokio::test]
    async fn test_describe_list_view_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "id": "00Bxx0000000001AAA",
            "developerName": "AllAccounts",
            "label": "All Accounts",
            "sobjectType": "Account",
            "query": "SELECT Id, Name FROM Account",
            "columns": [{
                "fieldNameOrPath": "Name",
                "label": "Account Name",
                "sortable": true,
                "type": "string"
            }],
            "orderBy": [],
            "whereCondition": null
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/listviews/00Bxx0000000001AAA/describe$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_list_view("Account", "00Bxx0000000001AAA")
            .await
            .expect("describe_list_view should succeed");
        assert_eq!(result.columns.len(), 1);
    }

    #[tokio::test]
    async fn test_execute_list_view_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "done": true,
            "id": "00Bxx0000000001AAA",
            "label": "All Accounts",
            "records": [{"Id": "001xx", "Name": "Acme"}],
            "size": 1,
            "developerName": "AllAccounts",
            "nextRecordsUrl": null
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/listviews/00Bxx0000000001AAA/results$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .execute_list_view::<serde_json::Value>("Account", "00Bxx0000000001AAA")
            .await
            .expect("execute_list_view should succeed");
        assert!(result.done);
        assert_eq!(result.size, 1);
    }
}
