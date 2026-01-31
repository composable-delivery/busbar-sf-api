use crate::deploy::{ComponentFailure, DeployResult, DeployStatus};
use crate::describe::{DescribeMetadataResult, MetadataType};
use crate::error::{Error, ErrorKind, Result};
use crate::list::MetadataComponent;
use crate::retrieve::{RetrieveMessage, RetrieveResult, RetrieveStatus};
use crate::types::{ComponentSuccess, FileProperties, SoapFault, TestFailure};

impl super::MetadataClient {
    /// Parse a SOAP fault from the response.
    pub(crate) fn parse_soap_fault(&self, xml: &str) -> Option<SoapFault> {
        if !xml.contains("faultcode") {
            return None;
        }

        let fault_code = self.extract_element(xml, "faultcode")?;
        let fault_string = self
            .extract_element(xml, "faultstring")
            .unwrap_or_else(|| "Unknown error".to_string());

        Some(SoapFault {
            fault_code,
            fault_string,
        })
    }

    /// Extract a simple element value from XML.
    pub(crate) fn extract_element(&self, xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start_patterns = [
            start_tag.clone(),
            format!("<{}:{}>", "sf", tag),
            format!("<{}:{}>", "met", tag),
            format!("<{}:{}>", "tns", tag),
        ];

        for start in &start_patterns {
            if let Some(start_idx) = xml.find(start) {
                let content_start = start_idx + start.len();
                let search_from = &xml[content_start..];
                if let Some(end_idx) = search_from.find(&end_tag).or_else(|| {
                    search_from.find(&format!("</{}", tag.split(':').next_back().unwrap_or(tag)))
                }) {
                    return Some(search_from[..end_idx].to_string());
                }
            }
        }
        None
    }

    /// Extract all elements with a given tag.
    pub(crate) fn extract_elements(&self, xml: &str, tag: &str) -> Vec<String> {
        let mut results = Vec::new();
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(&start_tag) {
            let content_start = start_idx + start_tag.len();
            let remaining = &search_from[content_start..];
            if let Some(end_idx) = remaining.find(&end_tag) {
                results.push(remaining[..end_idx].to_string());
                search_from = &remaining[end_idx + end_tag.len()..];
            } else {
                break;
            }
        }
        results
    }

    /// Parse deploy result from XML.
    pub(crate) fn parse_deploy_result(&self, xml: &str) -> Result<DeployResult> {
        let id = self
            .extract_element(xml, "id")
            .ok_or_else(|| Error::new(ErrorKind::InvalidResponse("Missing id".to_string())))?;

        let done = self
            .extract_element(xml, "done")
            .map(|s| s == "true")
            .unwrap_or(false);

        let status_str = self
            .extract_element(xml, "status")
            .unwrap_or_else(|| "Pending".to_string());
        let status = status_str.parse().unwrap_or(DeployStatus::Pending);

        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);

        let error_message = self.extract_element(xml, "errorMessage");
        let state_detail = self.extract_element(xml, "stateDetail");

        let number_components_deployed = self
            .extract_element(xml, "numberComponentsDeployed")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_components_errors = self
            .extract_element(xml, "numberComponentErrors")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_components_total = self
            .extract_element(xml, "numberComponentsTotal")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let number_tests_completed = self
            .extract_element(xml, "numberTestsCompleted")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_tests_errors = self
            .extract_element(xml, "numberTestErrors")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let number_tests_total = self
            .extract_element(xml, "numberTestsTotal")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let component_failures = self.parse_component_failures(xml);
        let component_successes = self.parse_component_successes(xml);
        let test_failures = self.parse_test_failures(xml);

        Ok(DeployResult {
            id,
            done,
            status,
            success,
            error_message,
            number_components_deployed,
            number_components_errors,
            number_components_total,
            number_tests_completed,
            number_tests_errors,
            number_tests_total,
            component_failures,
            component_successes,
            test_failures,
            state_detail,
        })
    }

    /// Parse component failures from XML.
    pub(crate) fn parse_component_failures(&self, xml: &str) -> Vec<ComponentFailure> {
        let mut failures = Vec::new();
        let pattern = "<componentFailures>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</componentFailures>") {
                let block = &remaining[..end + "</componentFailures>".len()];

                let failure = ComponentFailure {
                    component_type: self.extract_element(block, "componentType"),
                    file_name: self.extract_element(block, "fileName"),
                    full_name: self.extract_element(block, "fullName"),
                    line_number: self
                        .extract_element(block, "lineNumber")
                        .and_then(|s| s.parse().ok()),
                    column_number: self
                        .extract_element(block, "columnNumber")
                        .and_then(|s| s.parse().ok()),
                    problem: self
                        .extract_element(block, "problem")
                        .unwrap_or_else(|| "Unknown problem".to_string()),
                    problem_type: self
                        .extract_element(block, "problemType")
                        .unwrap_or_else(|| "Error".to_string()),
                    created: self
                        .extract_element(block, "created")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                    deleted: self
                        .extract_element(block, "deleted")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                };

                failures.push(failure);
                search_from = &remaining[end + "</componentFailures>".len()..];
            } else {
                break;
            }
        }

        failures
    }

    /// Parse component successes from XML.
    pub(crate) fn parse_component_successes(&self, xml: &str) -> Vec<ComponentSuccess> {
        let mut successes = Vec::new();
        let pattern = "<componentSuccesses>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</componentSuccesses>") {
                let block = &remaining[..end + "</componentSuccesses>".len()];

                let success = ComponentSuccess {
                    component_type: self.extract_element(block, "componentType"),
                    file_name: self.extract_element(block, "fileName"),
                    full_name: self.extract_element(block, "fullName"),
                    created: self
                        .extract_element(block, "created")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                    deleted: self
                        .extract_element(block, "deleted")
                        .map(|s| s == "true")
                        .unwrap_or(false),
                };

                successes.push(success);
                search_from = &remaining[end + "</componentSuccesses>".len()..];
            } else {
                break;
            }
        }

        successes
    }

    /// Parse test failures from XML.
    pub(crate) fn parse_test_failures(&self, xml: &str) -> Vec<TestFailure> {
        let mut failures = Vec::new();
        let pattern = "<failures>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</failures>") {
                let block = &remaining[..end + "</failures>".len()];

                let failure = TestFailure {
                    name: self.extract_element(block, "name"),
                    method_name: self.extract_element(block, "methodName"),
                    message: self.extract_element(block, "message"),
                    stack_trace: self.extract_element(block, "stackTrace"),
                    namespace: self.extract_element(block, "namespace"),
                };

                failures.push(failure);
                search_from = &remaining[end + "</failures>".len()..];
            } else {
                break;
            }
        }

        failures
    }

    /// Parse retrieve result from XML.
    pub(crate) fn parse_retrieve_result(&self, xml: &str) -> Result<RetrieveResult> {
        let id = self
            .extract_element(xml, "id")
            .ok_or_else(|| Error::new(ErrorKind::InvalidResponse("Missing id".to_string())))?;

        let done = self
            .extract_element(xml, "done")
            .map(|s| s == "true")
            .unwrap_or(false);

        let status_str = self
            .extract_element(xml, "status")
            .unwrap_or_else(|| "Pending".to_string());
        let status = status_str.parse().unwrap_or(RetrieveStatus::Pending);

        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);

        let error_message = self.extract_element(xml, "errorMessage");
        let error_status_code = self.extract_element(xml, "errorStatusCode");
        let zip_file = self.extract_element(xml, "zipFile");

        let file_properties = self.parse_file_properties(xml);
        let messages = self.parse_retrieve_messages(xml);

        Ok(RetrieveResult {
            id,
            done,
            status,
            success,
            error_message,
            error_status_code,
            zip_file,
            file_properties,
            messages,
        })
    }

    /// Parse file properties from retrieve result.
    pub(crate) fn parse_file_properties(&self, xml: &str) -> Vec<FileProperties> {
        let mut properties = Vec::new();
        let pattern = "<fileProperties>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</fileProperties>") {
                let block = &remaining[..end + "</fileProperties>".len()];

                if let (Some(file_name), Some(full_name), Some(id)) = (
                    self.extract_element(block, "fileName"),
                    self.extract_element(block, "fullName"),
                    self.extract_element(block, "id"),
                ) {
                    let prop = FileProperties {
                        created_by_id: self
                            .extract_element(block, "createdById")
                            .unwrap_or_default(),
                        created_by_name: self
                            .extract_element(block, "createdByName")
                            .unwrap_or_default(),
                        created_date: self
                            .extract_element(block, "createdDate")
                            .unwrap_or_default(),
                        file_name,
                        full_name,
                        id,
                        last_modified_by_id: self
                            .extract_element(block, "lastModifiedById")
                            .unwrap_or_default(),
                        last_modified_by_name: self
                            .extract_element(block, "lastModifiedByName")
                            .unwrap_or_default(),
                        last_modified_date: self
                            .extract_element(block, "lastModifiedDate")
                            .unwrap_or_default(),
                        manageable_state: self.extract_element(block, "manageableState"),
                        namespace_prefix: self.extract_element(block, "namespacePrefix"),
                        component_type: self.extract_element(block, "type").unwrap_or_default(),
                    };
                    properties.push(prop);
                }

                search_from = &remaining[end + "</fileProperties>".len()..];
            } else {
                break;
            }
        }

        properties
    }

    /// Parse retrieve messages from XML.
    pub(crate) fn parse_retrieve_messages(&self, xml: &str) -> Vec<RetrieveMessage> {
        let mut messages = Vec::new();
        let pattern = "<messages>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</messages>") {
                let block = &remaining[..end + "</messages>".len()];

                if let (Some(file_name), Some(problem)) = (
                    self.extract_element(block, "fileName"),
                    self.extract_element(block, "problem"),
                ) {
                    messages.push(RetrieveMessage { file_name, problem });
                }

                search_from = &remaining[end + "</messages>".len()..];
            } else {
                break;
            }
        }

        messages
    }

    /// Parse list metadata result.
    pub(crate) fn parse_list_metadata_result(
        &self,
        xml: &str,
        metadata_type: &str,
    ) -> Result<Vec<MetadataComponent>> {
        let mut results = Vec::new();
        let pattern = "<result>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                if let Some(full_name) = self.extract_element(block, "fullName") {
                    let info = MetadataComponent {
                        created_by_id: self.extract_element(block, "createdById"),
                        created_by_name: self.extract_element(block, "createdByName"),
                        created_date: self.extract_element(block, "createdDate"),
                        file_name: self.extract_element(block, "fileName"),
                        full_name,
                        id: self.extract_element(block, "id"),
                        last_modified_by_id: self.extract_element(block, "lastModifiedById"),
                        last_modified_by_name: self.extract_element(block, "lastModifiedByName"),
                        last_modified_date: self.extract_element(block, "lastModifiedDate"),
                        manageable_state: self.extract_element(block, "manageableState"),
                        namespace_prefix: self.extract_element(block, "namespacePrefix"),
                        metadata_type: self
                            .extract_element(block, "type")
                            .unwrap_or_else(|| metadata_type.to_string()),
                    };
                    results.push(info);
                }

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse describe metadata result.
    pub(crate) fn parse_describe_metadata_result(
        &self,
        xml: &str,
    ) -> Result<DescribeMetadataResult> {
        let mut metadata_objects = Vec::new();
        let pattern = "<metadataObjects>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</metadataObjects>") {
                let block = &remaining[..end + "</metadataObjects>".len()];

                if let Some(xml_name) = self.extract_element(block, "xmlName") {
                    let child_names = self.extract_elements(block, "childXmlNames");

                    let desc = MetadataType {
                        xml_name,
                        directory_name: self.extract_element(block, "directoryName"),
                        suffix: self.extract_element(block, "suffix"),
                        in_folder: self
                            .extract_element(block, "inFolder")
                            .map(|s| s == "true")
                            .unwrap_or(false),
                        meta_file: self
                            .extract_element(block, "metaFile")
                            .map(|s| s == "true")
                            .unwrap_or(false),
                        child_xml_names: child_names,
                    };
                    metadata_objects.push(desc);
                }

                search_from = &remaining[end + "</metadataObjects>".len()..];
            } else {
                break;
            }
        }

        let organization_namespace = self.extract_element(xml, "organizationNamespace");
        let partial_save_allowed = self
            .extract_element(xml, "partialSaveAllowed")
            .map(|s| s == "true")
            .unwrap_or(false);
        let test_required = self
            .extract_element(xml, "testRequired")
            .map(|s| s == "true")
            .unwrap_or(false);

        Ok(DescribeMetadataResult {
            metadata_objects,
            organization_namespace,
            partial_save_allowed,
            test_required,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::MetadataClient;
    use crate::deploy::DeployStatus;
    use crate::retrieve::RetrieveStatus;

    #[test]
    fn test_extract_element() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><id>12345</id><done>true</done></root>";

        assert_eq!(client.extract_element(xml, "id"), Some("12345".to_string()));
        assert_eq!(
            client.extract_element(xml, "done"),
            Some("true".to_string())
        );
        assert_eq!(client.extract_element(xml, "missing"), None);
    }

    #[test]
    fn test_extract_element_with_namespaced_open_tag() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><sf:id>12345</id></root>";
        assert_eq!(client.extract_element(xml, "id"), Some("12345".to_string()));

        let xml = "<root><met:status>Succeeded</status></root>";
        assert_eq!(
            client.extract_element(xml, "status"),
            Some("Succeeded".to_string())
        );

        let xml = "<root><tns:done>true</done></root>";
        assert_eq!(
            client.extract_element(xml, "done"),
            Some("true".to_string())
        );
    }

    #[test]
    fn test_extract_elements_multiple() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><name>Alice</name><name>Bob</name><name>Charlie</name></root>";
        let names = client.extract_elements(xml, "name");
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_extract_elements_empty() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = "<root><other>value</other></root>";
        let names = client.extract_elements(xml, "name");
        assert!(names.is_empty());
    }

    #[test]
    fn test_parse_soap_fault() {
        let client = MetadataClient::from_parts("url", "token");

        let xml = r#"
        <soap:Envelope>
            <soap:Body>
                <soap:Fault>
                    <faultcode>sf:INVALID_SESSION_ID</faultcode>
                    <faultstring>Session expired or invalid</faultstring>
                </soap:Fault>
            </soap:Body>
        </soap:Envelope>"#;

        let fault = client.parse_soap_fault(xml).unwrap();
        assert_eq!(fault.fault_code, "sf:INVALID_SESSION_ID");
        assert_eq!(fault.fault_string, "Session expired or invalid");
    }

    #[test]
    fn test_parse_soap_fault_returns_none_for_success() {
        let client = MetadataClient::from_parts("url", "token");
        let xml =
            "<soap:Envelope><soap:Body><result><id>123</id></result></soap:Body></soap:Envelope>";
        assert!(client.parse_soap_fault(xml).is_none());
    }

    #[test]
    fn test_parse_deploy_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <checkDeployStatusResponse>
                <result>
                    <id>0Af123</id>
                    <done>true</done>
                    <status>Succeeded</status>
                    <success>true</success>
                    <numberComponentsDeployed>5</numberComponentsDeployed>
                    <numberComponentErrors>0</numberComponentErrors>
                    <numberComponentsTotal>5</numberComponentsTotal>
                </result>
            </checkDeployStatusResponse>
        "#;

        let result = client.parse_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af123");
        assert!(result.done);
        assert_eq!(result.status, DeployStatus::Succeeded);
        assert!(result.success);
        assert_eq!(result.number_components_deployed, 5);
    }

    #[test]
    fn test_parse_deploy_result_with_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <id>0Af456</id>
                <done>true</done>
                <status>Failed</status>
                <success>false</success>
                <numberComponentsDeployed>3</numberComponentsDeployed>
                <numberComponentErrors>2</numberComponentErrors>
                <numberComponentsTotal>5</numberComponentsTotal>
                <numberTestsCompleted>10</numberTestsCompleted>
                <numberTestErrors>1</numberTestErrors>
                <numberTestsTotal>11</numberTestsTotal>
                <errorMessage>Deployment failed</errorMessage>
                <componentFailures>
                    <componentType>ApexClass</componentType>
                    <fullName>BadClass</fullName>
                    <problem>Compilation error</problem>
                    <problemType>Error</problemType>
                    <lineNumber>42</lineNumber>
                    <columnNumber>5</columnNumber>
                    <created>false</created>
                    <deleted>false</deleted>
                </componentFailures>
            </result>
        "#;

        let result = client.parse_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af456");
        assert!(!result.success);
        assert_eq!(result.status, DeployStatus::Failed);
        assert_eq!(result.number_components_deployed, 3);
        assert_eq!(result.number_components_errors, 2);
        assert_eq!(result.number_tests_completed, 10);
        assert_eq!(result.number_tests_errors, 1);
        assert_eq!(result.error_message, Some("Deployment failed".to_string()));
        assert_eq!(result.component_failures.len(), 1);
        assert_eq!(
            result.component_failures[0].full_name,
            Some("BadClass".to_string())
        );
        assert_eq!(result.component_failures[0].line_number, Some(42));
        assert_eq!(result.component_failures[0].column_number, Some(5));
    }

    #[test]
    fn test_parse_component_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <componentFailures>
                <componentType>ApexClass</componentType>
                <fileName>classes/MyClass.cls</fileName>
                <fullName>MyClass</fullName>
                <lineNumber>10</lineNumber>
                <problem>Missing semicolon</problem>
                <problemType>Error</problemType>
                <created>false</created>
                <deleted>false</deleted>
            </componentFailures>
        "#;

        let failures = client.parse_component_failures(xml);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].component_type, Some("ApexClass".to_string()));
        assert_eq!(failures[0].problem, "Missing semicolon");
        assert_eq!(failures[0].line_number, Some(10));
    }

    #[test]
    fn test_parse_component_successes() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <componentSuccesses>
                <componentType>ApexClass</componentType>
                <fileName>classes/MyClass.cls</fileName>
                <fullName>MyClass</fullName>
                <created>true</created>
                <deleted>false</deleted>
            </componentSuccesses>
        "#;

        let successes = client.parse_component_successes(xml);
        assert_eq!(successes.len(), 1);
        assert_eq!(successes[0].component_type, Some("ApexClass".to_string()));
        assert_eq!(successes[0].full_name, Some("MyClass".to_string()));
        assert!(successes[0].created);
        assert!(!successes[0].deleted);
    }

    #[test]
    fn test_parse_test_failures() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <failures>
                <name>MyTestClass</name>
                <methodName>testInsert</methodName>
                <message>System.AssertException: Expected 1, got 2</message>
                <stackTrace>Class.MyTestClass.testInsert: line 15, column 1</stackTrace>
            </failures>
            <failures>
                <name>MyTestClass</name>
                <methodName>testUpdate</methodName>
                <message>Null pointer</message>
            </failures>
        "#;

        let failures = client.parse_test_failures(xml);
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].name, Some("MyTestClass".to_string()));
        assert_eq!(failures[0].method_name, Some("testInsert".to_string()));
        assert!(failures[0]
            .message
            .as_ref()
            .unwrap()
            .contains("AssertException"));
        assert!(failures[0].stack_trace.is_some());
        assert!(failures[1].stack_trace.is_none());
    }

    #[test]
    fn test_parse_retrieve_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <checkRetrieveStatusResponse>
                <result>
                    <id>09S123</id>
                    <done>true</done>
                    <status>Succeeded</status>
                    <success>true</success>
                    <zipFile>UEsDBBQ...</zipFile>
                </result>
            </checkRetrieveStatusResponse>
        "#;

        let result = client.parse_retrieve_result(xml).unwrap();
        assert_eq!(result.id, "09S123");
        assert!(result.done);
        assert_eq!(result.status, RetrieveStatus::Succeeded);
        assert!(result.zip_file.is_some());
    }

    #[test]
    fn test_parse_retrieve_result_with_messages_and_files() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <id>09S789</id>
                <done>true</done>
                <status>Succeeded</status>
                <success>true</success>
                <fileProperties>
                    <fullName>MyClass</fullName>
                    <fileName>classes/MyClass.cls</fileName>
                    <type>ApexClass</type>
                    <id>01p123</id>
                </fileProperties>
                <messages>
                    <fileName>classes/OldClass.cls</fileName>
                    <problem>Entity is deleted</problem>
                </messages>
            </result>
        "#;

        let result = client.parse_retrieve_result(xml).unwrap();
        assert!(result.success);
        assert_eq!(result.file_properties.len(), 1);
        assert_eq!(result.file_properties[0].full_name, "MyClass");
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].file_name, "classes/OldClass.cls");
        assert_eq!(result.messages[0].problem, "Entity is deleted");
    }
}
