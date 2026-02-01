use crate::deploy::{ComponentFailure, DeployResult, DeployStatus};
use crate::describe::{DescribeMetadataResult, MetadataType};
use crate::error::{Error, ErrorKind, Result};
use crate::list::MetadataComponent;
use crate::retrieve::{RetrieveMessage, RetrieveResult, RetrieveStatus};
use crate::types::{
    ComponentSuccess, DeleteResult, FileProperties, MetadataError, ReadResult, SaveResult,
    SoapFault, TestFailure, UpsertResult,
};
use busbar_sf_client::security::xml;

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

    /// Parse describe value type result from XML.
    pub(crate) fn parse_describe_value_type_result(
        &self,
        xml: &str,
    ) -> Result<crate::describe::DescribeValueTypeResult> {
        let value_type_fields = self.parse_value_type_fields(xml, "valueTypeFields");

        let parent_field = if xml.contains("<parentField>") {
            self.parse_single_value_type_field(xml, "parentField")
        } else {
            None
        };

        Ok(crate::describe::DescribeValueTypeResult {
            value_type_fields,
            parent_field,
        })
    }

    /// Parse all ValueTypeField elements with a specific tag.
    pub(crate) fn parse_value_type_fields(
        &self,
        xml: &str,
        tag: &str,
    ) -> Vec<crate::describe::ValueTypeField> {
        let mut fields = Vec::new();
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(&start_tag) {
            let remaining = &search_from[start_idx + start_tag.len()..];

            let mut depth = 1;
            let mut pos = 0;
            let mut found_end = None;

            while pos < remaining.len() && depth > 0 {
                if remaining[pos..].starts_with(&start_tag) {
                    depth += 1;
                    pos += start_tag.len();
                } else if remaining[pos..].starts_with(&end_tag) {
                    depth -= 1;
                    if depth == 0 {
                        found_end = Some(pos);
                        break;
                    }
                    pos += end_tag.len();
                } else {
                    pos += 1;
                }
            }

            if let Some(end_pos) = found_end {
                let field_content = &remaining[..end_pos];
                if let Some(field) = self.parse_value_type_field_from_content(field_content) {
                    fields.push(field);
                }
                search_from = &remaining[end_pos + end_tag.len()..];
            } else {
                break;
            }
        }
        fields
    }

    /// Parse a single ValueTypeField with a specific tag.
    pub(crate) fn parse_single_value_type_field(
        &self,
        xml: &str,
        tag: &str,
    ) -> Option<crate::describe::ValueTypeField> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_idx) = xml.find(&start_tag) {
            let remaining = &xml[start_idx + start_tag.len()..];
            if let Some(end_idx) = remaining.find(&end_tag) {
                let content = &remaining[..end_idx];
                return self.parse_value_type_field_from_content(content);
            }
        }
        None
    }

    /// Parse nested ValueTypeField elements within a parent field block.
    pub(crate) fn parse_nested_value_type_fields_in_block(
        &self,
        block: &str,
    ) -> Vec<crate::describe::ValueTypeField> {
        let mut nested_fields = Vec::new();
        let start_tag = "<valueTypeFields>";
        let end_tag = "</valueTypeFields>";

        let mut search_from = block;

        while let Some(start_idx) = search_from.find(start_tag) {
            let remaining = &search_from[start_idx + start_tag.len()..];

            let mut depth = 1;
            let mut pos = 0;
            let mut found_end = None;

            while pos < remaining.len() && depth > 0 {
                if remaining[pos..].starts_with(start_tag) {
                    depth += 1;
                    pos += start_tag.len();
                } else if remaining[pos..].starts_with(end_tag) {
                    depth -= 1;
                    if depth == 0 {
                        found_end = Some(pos);
                        break;
                    }
                    pos += end_tag.len();
                } else {
                    pos += 1;
                }
            }

            if let Some(end_pos) = found_end {
                let field_content = &remaining[..end_pos];
                if let Some(field) = self.parse_value_type_field_from_content(field_content) {
                    nested_fields.push(field);
                }
                search_from = &remaining[end_pos + end_tag.len()..];
            } else {
                break;
            }
        }

        nested_fields
    }

    /// Parse a ValueTypeField from the content (without the wrapping tags).
    pub(crate) fn parse_value_type_field_from_content(
        &self,
        content: &str,
    ) -> Option<crate::describe::ValueTypeField> {
        let name = self.extract_element(content, "name")?;
        let soap_type = self.extract_element(content, "soapType")?;

        let is_foreign_key = self
            .extract_element(content, "isForeignKey")
            .map(|s| s == "true")
            .unwrap_or(false);

        let foreign_key_domain = self.extract_element(content, "foreignKeyDomain");

        let is_name_field = self
            .extract_element(content, "isNameField")
            .map(|s| s == "true")
            .unwrap_or(false);

        let min_occurs = self
            .extract_element(content, "minOccurs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let max_occurs = self
            .extract_element(content, "maxOccurs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let picklist_values = self.parse_picklist_entries(content);
        let fields = self.parse_nested_value_type_fields_in_block(content);

        Some(crate::describe::ValueTypeField {
            name,
            soap_type,
            is_foreign_key,
            foreign_key_domain,
            is_name_field,
            min_occurs,
            max_occurs,
            fields,
            picklist_values,
        })
    }

    /// Parse all PicklistEntry elements from XML.
    pub(crate) fn parse_picklist_entries(&self, xml: &str) -> Vec<crate::describe::PicklistEntry> {
        let mut entries = Vec::new();
        let start_tag = "<picklistValues>";
        let end_tag = "</picklistValues>";

        let mut search_from = xml;
        while let Some(start_idx) = search_from.find(start_tag) {
            let remaining = &search_from[start_idx..];
            if let Some(end_idx) = remaining.find(end_tag) {
                let block = &remaining[..end_idx + end_tag.len()];

                let active = self
                    .extract_element(block, "active")
                    .map(|s| s == "true")
                    .unwrap_or(false);

                let default_value = self
                    .extract_element(block, "defaultValue")
                    .map(|s| s == "true")
                    .unwrap_or(false);

                let label = self.extract_element(block, "label").unwrap_or_default();
                let value = self.extract_element(block, "value").unwrap_or_default();

                entries.push(crate::describe::PicklistEntry {
                    active,
                    default_value,
                    label,
                    value,
                });

                search_from = &remaining[end_idx + end_tag.len()..];
            } else {
                break;
            }
        }
        entries
    }

    /// Parse cancel deploy result from XML.
    pub(crate) fn parse_cancel_deploy_result(
        &self,
        xml: &str,
    ) -> Result<crate::deploy::CancelDeployResult> {
        let id = self
            .extract_element(xml, "id")
            .ok_or_else(|| Error::new(ErrorKind::InvalidResponse("Missing id".to_string())))?;

        let done = self
            .extract_element(xml, "done")
            .map(|s| s == "true")
            .unwrap_or(false);

        Ok(crate::deploy::CancelDeployResult { id, done })
    }

    /// Build a metadata element for SOAP body.
    pub(crate) fn build_metadata_element(
        &self,
        metadata_type: &str,
        metadata_obj: &serde_json::Value,
    ) -> String {
        let mut element = format!(
            "      <met:metadata xsi:type=\"met:{}\">\n",
            xml::escape(metadata_type)
        );

        if let Some(obj) = metadata_obj.as_object() {
            for (key, value) in obj {
                element.push_str(&Self::build_xml_field(key, value, 8));
            }
        }

        element.push_str("      </met:metadata>");
        element
    }

    /// Build an XML field for a metadata object.
    pub(crate) fn build_xml_field(key: &str, value: &serde_json::Value, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        let escaped_key = xml::escape(key);

        match value {
            serde_json::Value::String(s) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces,
                    escaped_key,
                    xml::escape(s),
                    escaped_key
                )
            }
            serde_json::Value::Number(n) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces, escaped_key, n, escaped_key
                )
            }
            serde_json::Value::Bool(b) => {
                format!(
                    "{}<met:{}>{}</met:{}>\n",
                    spaces, escaped_key, b, escaped_key
                )
            }
            serde_json::Value::Null => {
                format!("{}<met:{} xsi:nil=\"true\"/>\n", spaces, escaped_key)
            }
            serde_json::Value::Object(obj) => {
                let mut result = format!("{}<met:{}>\n", spaces, escaped_key);
                for (nested_key, nested_value) in obj {
                    result.push_str(&Self::build_xml_field(nested_key, nested_value, indent + 2));
                }
                result.push_str(&format!("{}</met:{}>\n", spaces, escaped_key));
                result
            }
            serde_json::Value::Array(arr) => {
                let mut result = String::new();
                for item in arr {
                    result.push_str(&Self::build_xml_field(key, item, indent));
                }
                result
            }
        }
    }

    /// Parse SaveResult elements from SOAP response.
    pub(crate) fn parse_save_results(&self, xml: &str) -> Result<Vec<SaveResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(SaveResult {
                    full_name,
                    success,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse UpsertResult elements from SOAP response.
    pub(crate) fn parse_upsert_results(&self, xml: &str) -> Result<Vec<UpsertResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let created = self
                    .extract_element(block, "created")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(UpsertResult {
                    full_name,
                    success,
                    created,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse DeleteResult elements from SOAP response.
    pub(crate) fn parse_delete_results(&self, xml: &str) -> Result<Vec<DeleteResult>> {
        let mut results = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let full_name = self.extract_element(block, "fullName").unwrap_or_default();
                let success = self
                    .extract_element(block, "success")
                    .map(|s| s == "true")
                    .unwrap_or(false);
                let errors = self.parse_metadata_errors(block);

                results.push(DeleteResult {
                    full_name,
                    success,
                    errors,
                });

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(results)
    }

    /// Parse ReadResult from SOAP response.
    pub(crate) fn parse_read_result(&self, xml: &str) -> Result<ReadResult> {
        let mut records = Vec::new();
        let pattern = "<result";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</result>") {
                let block = &remaining[..end + "</result>".len()];

                let mut metadata_obj = serde_json::Map::new();

                if let Some(full_name) = self.extract_element(block, "fullName") {
                    metadata_obj
                        .insert("fullName".to_string(), serde_json::Value::String(full_name));
                }

                if let Some(label) = self.extract_element(block, "label") {
                    metadata_obj.insert("label".to_string(), serde_json::Value::String(label));
                }

                metadata_obj.insert(
                    "_rawXml".to_string(),
                    serde_json::Value::String(block.to_string()),
                );

                records.push(serde_json::Value::Object(metadata_obj));

                search_from = &remaining[end + "</result>".len()..];
            } else {
                break;
            }
        }

        Ok(ReadResult { records })
    }

    /// Parse single SaveResult from rename operation.
    pub(crate) fn parse_rename_result(&self, xml: &str) -> Result<SaveResult> {
        let full_name = self.extract_element(xml, "fullName").unwrap_or_default();
        let success = self
            .extract_element(xml, "success")
            .map(|s| s == "true")
            .unwrap_or(false);
        let errors = self.parse_metadata_errors(xml);

        Ok(SaveResult {
            full_name,
            success,
            errors,
        })
    }

    /// Parse MetadataError elements from a result block.
    pub(crate) fn parse_metadata_errors(&self, xml: &str) -> Vec<MetadataError> {
        let mut errors = Vec::new();
        let pattern = "<errors>";
        let mut search_from = xml;

        while let Some(start) = search_from.find(pattern) {
            let remaining = &search_from[start..];
            if let Some(end) = remaining.find("</errors>") {
                let block = &remaining[..end + "</errors>".len()];

                let status_code = self
                    .extract_element(block, "statusCode")
                    .unwrap_or_default();
                let message = self.extract_element(block, "message").unwrap_or_default();
                let fields = self.extract_elements(block, "fields");

                errors.push(MetadataError {
                    status_code,
                    message,
                    fields,
                });

                search_from = &remaining[end + "</errors>".len()..];
            } else {
                break;
            }
        }

        errors
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

    #[test]
    fn test_parse_describe_value_type_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <describeValueTypeResponse>
                <result>
                    <valueTypeFields>
                        <name>fullName</name>
                        <soapType>xsd:string</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>true</isNameField>
                        <minOccurs>1</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </valueTypeFields>
                    <valueTypeFields>
                        <name>label</name>
                        <soapType>xsd:string</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>1</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </valueTypeFields>
                    <valueTypeFields>
                        <name>deploymentStatus</name>
                        <soapType>tns:DeploymentStatus</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                        <picklistValues>
                            <active>true</active>
                            <defaultValue>false</defaultValue>
                            <label>In Development</label>
                            <value>InDevelopment</value>
                        </picklistValues>
                        <picklistValues>
                            <active>true</active>
                            <defaultValue>true</defaultValue>
                            <label>Deployed</label>
                            <value>Deployed</value>
                        </picklistValues>
                    </valueTypeFields>
                    <parentField>
                        <name>Metadata</name>
                        <soapType>tns:Metadata</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                    </parentField>
                </result>
            </describeValueTypeResponse>
        "#;

        let result = client.parse_describe_value_type_result(xml).unwrap();
        assert_eq!(result.value_type_fields.len(), 3);
        assert_eq!(result.value_type_fields[0].name, "fullName");
        assert_eq!(result.value_type_fields[0].soap_type, "xsd:string");
        assert!(result.value_type_fields[0].is_name_field);
        assert!(!result.value_type_fields[0].is_foreign_key);
        assert_eq!(result.value_type_fields[0].min_occurs, 1);
        assert_eq!(result.value_type_fields[0].max_occurs, 1);
        assert_eq!(result.value_type_fields[1].name, "label");
        assert!(!result.value_type_fields[1].is_name_field);
        assert_eq!(result.value_type_fields[2].name, "deploymentStatus");
        assert_eq!(result.value_type_fields[2].picklist_values.len(), 2);
        assert_eq!(
            result.value_type_fields[2].picklist_values[0].label,
            "In Development"
        );
        assert_eq!(
            result.value_type_fields[2].picklist_values[0].value,
            "InDevelopment"
        );
        assert!(!result.value_type_fields[2].picklist_values[0].default_value);
        assert_eq!(
            result.value_type_fields[2].picklist_values[1].label,
            "Deployed"
        );
        assert!(result.value_type_fields[2].picklist_values[1].default_value);
        assert!(result.parent_field.is_some());
        let parent = result.parent_field.unwrap();
        assert_eq!(parent.name, "Metadata");
        assert_eq!(parent.soap_type, "tns:Metadata");
    }

    #[test]
    fn test_parse_describe_value_type_result_with_nested_fields() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <describeValueTypeResponse>
                <result>
                    <valueTypeFields>
                        <name>address</name>
                        <soapType>tns:Address</soapType>
                        <isForeignKey>false</isForeignKey>
                        <isNameField>false</isNameField>
                        <minOccurs>0</minOccurs>
                        <maxOccurs>1</maxOccurs>
                        <valueTypeFields>
                            <name>street</name>
                            <soapType>xsd:string</soapType>
                            <isForeignKey>false</isForeignKey>
                            <isNameField>false</isNameField>
                            <minOccurs>0</minOccurs>
                            <maxOccurs>1</maxOccurs>
                        </valueTypeFields>
                        <valueTypeFields>
                            <name>city</name>
                            <soapType>xsd:string</soapType>
                            <isForeignKey>false</isForeignKey>
                            <isNameField>false</isNameField>
                            <minOccurs>0</minOccurs>
                            <maxOccurs>1</maxOccurs>
                        </valueTypeFields>
                    </valueTypeFields>
                </result>
            </describeValueTypeResponse>
        "#;

        let result = client.parse_describe_value_type_result(xml).unwrap();
        assert_eq!(result.value_type_fields.len(), 1);
        let address_field = &result.value_type_fields[0];
        assert_eq!(address_field.name, "address");
        assert_eq!(address_field.soap_type, "tns:Address");
        assert_eq!(address_field.fields.len(), 2);
        assert_eq!(address_field.fields[0].name, "street");
        assert_eq!(address_field.fields[0].soap_type, "xsd:string");
        assert_eq!(address_field.fields[1].name, "city");
        assert_eq!(address_field.fields[1].soap_type, "xsd:string");
    }

    #[test]
    fn test_parse_cancel_deploy_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <cancelDeployResponse>
                <result>
                    <id>0Af123456789ABC</id>
                    <done>true</done>
                </result>
            </cancelDeployResponse>
        "#;

        let result = client.parse_cancel_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af123456789ABC");
        assert!(result.done);
    }

    #[test]
    fn test_parse_cancel_deploy_result_not_done() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <cancelDeployResponse>
                <result>
                    <id>0Af123456789ABC</id>
                    <done>false</done>
                </result>
            </cancelDeployResponse>
        "#;

        let result = client.parse_cancel_deploy_result(xml).unwrap();
        assert_eq!(result.id, "0Af123456789ABC");
        assert!(!result.done);
    }

    #[test]
    fn test_parse_save_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <createMetadataResponse>
                <result>
                    <fullName>MyObject__c</fullName>
                    <success>true</success>
                </result>
                <result>
                    <fullName>FailedObject__c</fullName>
                    <success>false</success>
                    <errors>
                        <statusCode>INVALID_FIELD</statusCode>
                        <message>Invalid field name</message>
                        <fields>Name</fields>
                    </errors>
                </result>
            </createMetadataResponse>
        "#;

        let results = client.parse_save_results(xml).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].full_name, "MyObject__c");
        assert!(results[0].success);
        assert_eq!(results[0].errors.len(), 0);
        assert_eq!(results[1].full_name, "FailedObject__c");
        assert!(!results[1].success);
        assert_eq!(results[1].errors.len(), 1);
        assert_eq!(results[1].errors[0].status_code, "INVALID_FIELD");
        assert_eq!(results[1].errors[0].message, "Invalid field name");
        assert_eq!(results[1].errors[0].fields, vec!["Name"]);
    }

    #[test]
    fn test_parse_upsert_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <upsertMetadataResponse>
                <result>
                    <fullName>CreatedObject__c</fullName>
                    <success>true</success>
                    <created>true</created>
                </result>
                <result>
                    <fullName>UpdatedObject__c</fullName>
                    <success>true</success>
                    <created>false</created>
                </result>
            </upsertMetadataResponse>
        "#;

        let results = client.parse_upsert_results(xml).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].full_name, "CreatedObject__c");
        assert!(results[0].success);
        assert!(results[0].created);
        assert_eq!(results[1].full_name, "UpdatedObject__c");
        assert!(results[1].success);
        assert!(!results[1].created);
    }

    #[test]
    fn test_parse_delete_results() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <deleteMetadataResponse>
                <result>
                    <fullName>DeletedObject__c</fullName>
                    <success>true</success>
                </result>
            </deleteMetadataResponse>
        "#;

        let results = client.parse_delete_results(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].full_name, "DeletedObject__c");
        assert!(results[0].success);
    }

    #[test]
    fn test_parse_read_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <readMetadataResponse>
                <result>
                    <fullName>Account</fullName>
                    <label>Account</label>
                </result>
                <result>
                    <fullName>Contact</fullName>
                    <label>Contact</label>
                </result>
            </readMetadataResponse>
        "#;

        let result = client.parse_read_result(xml).unwrap();
        assert_eq!(result.records.len(), 2);
        if let Some(obj) = result.records[0].as_object() {
            assert_eq!(
                obj.get("fullName").and_then(|v| v.as_str()),
                Some("Account")
            );
            assert_eq!(obj.get("label").and_then(|v| v.as_str()), Some("Account"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_parse_rename_result() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <renameMetadataResponse>
                <result>
                    <fullName>NewName__c</fullName>
                    <success>true</success>
                </result>
            </renameMetadataResponse>
        "#;

        let result = client.parse_rename_result(xml).unwrap();
        assert_eq!(result.full_name, "NewName__c");
        assert!(result.success);
    }

    #[test]
    fn test_parse_metadata_errors() {
        let client = MetadataClient::from_parts("url", "token");
        let xml = r#"
            <result>
                <errors>
                    <statusCode>INVALID_FIELD</statusCode>
                    <message>Invalid field</message>
                    <fields>Name</fields>
                    <fields>Type</fields>
                </errors>
                <errors>
                    <statusCode>DUPLICATE_VALUE</statusCode>
                    <message>Duplicate value found</message>
                </errors>
            </result>
        "#;

        let errors = client.parse_metadata_errors(xml);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].status_code, "INVALID_FIELD");
        assert_eq!(errors[0].message, "Invalid field");
        assert_eq!(errors[0].fields, vec!["Name", "Type"]);
        assert_eq!(errors[1].status_code, "DUPLICATE_VALUE");
        assert_eq!(errors[1].message, "Duplicate value found");
        assert_eq!(errors[1].fields.len(), 0);
    }
}
