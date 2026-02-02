use tracing::instrument;

use busbar_sf_client::security::{soql, url as url_security};

use crate::error::{Error, ErrorKind, Result};

impl super::SalesforceRestClient {
    /// Get all page layouts for a specific SObject.
    ///
    /// This returns metadata about all page layouts configured for the SObject,
    /// including sections, rows, items, and field metadata.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let layouts = client.describe_layouts("Account").await?;
    /// println!("Account layouts: {:?}", layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/layouts`.
    #[instrument(skip(self))]
    pub async fn describe_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::DescribeLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/layouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get a specific named layout for an SObject.
    ///
    /// This returns the layout metadata for a specific named layout.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let layout = client.describe_named_layout("Account", "MyCustomLayout").await?;
    /// println!("Layout metadata: {:?}", layout);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/namedLayouts/{layoutName}`.
    #[instrument(skip(self))]
    pub async fn describe_named_layout(
        &self,
        sobject: &str,
        layout_name: &str,
    ) -> Result<crate::layout::NamedLayoutResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        // URL-encode the layout name to handle special characters
        let encoded_name = url_security::encode_param(layout_name);
        let path = format!(
            "sobjects/{}/describe/namedLayouts/{}",
            sobject, encoded_name
        );
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get approval process layouts for a specific SObject.
    ///
    /// This returns the approval process layout information including
    /// approval steps, actions, and field mappings.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let approval_layouts = client.describe_approval_layouts("Account").await?;
    /// println!("Approval layouts: {:?}", approval_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/approvalLayouts`.
    #[instrument(skip(self))]
    pub async fn describe_approval_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::ApprovalLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/approvalLayouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get compact layouts for a specific SObject.
    ///
    /// Compact layouts are used in the Salesforce mobile app and Lightning Experience
    /// to show a preview of a record in a compact space.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let compact_layouts = client.describe_compact_layouts("Account").await?;
    /// println!("Compact layouts: {:?}", compact_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/{sobject}/describe/compactLayouts`.
    #[instrument(skip(self))]
    pub async fn describe_compact_layouts(
        &self,
        sobject: &str,
    ) -> Result<crate::layout::CompactLayoutsResult> {
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: "Invalid SObject name".to_string(),
            }));
        }
        let path = format!("sobjects/{}/describe/compactLayouts", sobject);
        self.client.rest_get(&path).await.map_err(Into::into)
    }

    /// Get global publisher layouts (global quick actions).
    ///
    /// This returns global quick actions and publisher layouts that are
    /// available across the entire organization, not tied to a specific SObject.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let global_layouts = client.describe_global_publisher_layouts().await?;
    /// println!("Global layouts: {:?}", global_layouts);
    /// ```
    ///
    /// This is equivalent to calling `/services/data/vXX.0/sobjects/Global/describe/layouts`.
    #[instrument(skip(self))]
    pub async fn describe_global_publisher_layouts(
        &self,
    ) -> Result<crate::layout::GlobalPublisherLayoutsResult> {
        let path = "sobjects/Global/describe/layouts";
        self.client.rest_get(path).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::super::SalesforceRestClient;

    #[tokio::test]
    async fn test_describe_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"id": "00h000000000001", "name": "Account Layout"}],
            "recordTypeMappings": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_layouts("Account")
            .await
            .expect("describe_layouts should succeed");

        assert!(result["layouts"].is_array());
        assert_eq!(result["layouts"][0]["name"], "Account Layout");
    }

    #[tokio::test]
    async fn test_describe_layouts_invalid_sobject() {
        let client = SalesforceRestClient::new("https://test.salesforce.com", "token").unwrap();
        let result = client.describe_layouts("Bad'; DROP--").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("INVALID_SOBJECT"));
    }

    #[tokio::test]
    async fn test_describe_named_layout_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"detailLayoutSections": [], "editLayoutSections": []}]
        });

        Mock::given(method("GET"))
            .and(path_regex(
                ".*/sobjects/Account/describe/namedLayouts/MyLayout",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_named_layout("Account", "MyLayout")
            .await
            .expect("describe_named_layout should succeed");

        assert!(result["layouts"].is_array());
    }

    #[tokio::test]
    async fn test_describe_approval_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "approvalLayouts": []
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/approvalLayouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_approval_layouts("Account")
            .await
            .expect("describe_approval_layouts should succeed");

        assert!(result["approvalLayouts"].is_array());
    }

    #[tokio::test]
    async fn test_describe_compact_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "compactLayouts": [{"id": "0AH000000000001", "name": "System Default"}],
            "defaultCompactLayoutId": "0AH000000000001"
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Account/describe/compactLayouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_compact_layouts("Account")
            .await
            .expect("describe_compact_layouts should succeed");

        assert!(result["compactLayouts"].is_array());
        assert_eq!(result["compactLayouts"][0]["name"], "System Default");
    }

    #[tokio::test]
    async fn test_describe_global_publisher_layouts_wiremock() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let body = serde_json::json!({
            "layouts": [{"id": "00h000000000002", "name": "Global Layout"}]
        });

        Mock::given(method("GET"))
            .and(path_regex(".*/sobjects/Global/describe/layouts$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&mock_server)
            .await;

        let client = SalesforceRestClient::new(mock_server.uri(), "test-token").unwrap();
        let result = client
            .describe_global_publisher_layouts()
            .await
            .expect("describe_global_publisher_layouts should succeed");

        assert!(result["layouts"].is_array());
        assert_eq!(result["layouts"][0]["name"], "Global Layout");
    }
}
