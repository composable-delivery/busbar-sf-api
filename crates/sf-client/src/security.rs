//! Security utilities for Salesforce API operations.
//!
//! This module provides universal security utilities that MUST be used
//! across all Salesforce API crates to prevent injection attacks.
//!
//! ## SOQL Injection Prevention
//!
//! **CRITICAL**: All user-provided values in SOQL queries MUST be escaped
//! using the functions in this module. Failure to do so creates injection
//! vulnerabilities.
//!
//! ```rust
//! use busbar_sf_client::security::soql;
//!
//! // CORRECT - Always escape user input
//! let name = soql::escape_string("O'Brien");
//! let query = format!("SELECT Id FROM Account WHERE Name = '{}'", name);
//!
//! // WRONG - NEVER do this
//! // let query = format!("SELECT Id FROM Account WHERE Name = '{}'", user_input);
//! ```
//!
//! ## URL Parameter Encoding
//!
//! User-provided values in URLs MUST be encoded:
//!
//! ```rust
//! use busbar_sf_client::security::url;
//!
//! // CORRECT
//! let encoded_id = url::encode_param("001/test");
//! let url = format!("/services/data/v62.0/sobjects/Account/{}", encoded_id);
//!
//! // WRONG - NEVER do this with user input
//! // let url = format!("/services/data/v62.0/sobjects/Account/{}", user_id);
//! ```

/// SOQL escaping utilities for injection prevention.
pub mod soql {
    /// Escape a string value for use in SOQL queries.
    ///
    /// This function escapes characters that have special meaning in SOQL string literals:
    /// - Single quotes (`'`) are escaped to (`\'`)
    /// - Backslashes (`\`) are escaped to (`\\`)
    /// - Newlines are escaped to (`\n`)
    /// - Carriage returns are escaped to (`\r`)
    /// - Tabs are escaped to (`\t`)
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::soql;
    ///
    /// let safe = soql::escape_string("O'Brien & Co.");
    /// assert_eq!(safe, "O\\'Brien & Co.");
    ///
    /// let query = format!("SELECT Id FROM Account WHERE Name = '{}'", safe);
    /// ```
    ///
    /// # SOQL Injection Context
    ///
    /// Without escaping, an attacker could manipulate queries:
    /// ```text
    /// Input: "' OR Name LIKE '%"
    /// Unsafe: SELECT Id FROM Account WHERE Name = '' OR Name LIKE '%'
    /// Safe:   SELECT Id FROM Account WHERE Name = '\' OR Name LIKE \'%'
    /// ```
    #[must_use]
    pub fn escape_string(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len() + 16);
        for ch in value.chars() {
            match ch {
                '\'' => escaped.push_str("\\'"),
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Escape a value for use in a SOQL LIKE clause.
    ///
    /// In addition to standard string escaping, this also escapes
    /// LIKE wildcards (`%` and `_`) to prevent pattern injection.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::soql;
    ///
    /// let pattern = soql::escape_like("test%value_here");
    /// // Returns: test\%value\_here
    ///
    /// let query = format!("SELECT Id FROM Account WHERE Name LIKE '%{}%'", pattern);
    /// ```
    #[must_use]
    pub fn escape_like(value: &str) -> String {
        let base_escaped = escape_string(value);
        let mut escaped = String::with_capacity(base_escaped.len() + 8);
        for ch in base_escaped.chars() {
            match ch {
                '%' => escaped.push_str("\\%"),
                '_' => escaped.push_str("\\_"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Validate that a field name contains only safe characters.
    ///
    /// Field names should only contain alphanumeric characters, underscores,
    /// and the `__c` / `__r` suffixes for custom fields/relationships.
    ///
    /// Returns `true` if the field name is safe, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::soql;
    ///
    /// assert!(soql::is_safe_field_name("Account"));
    /// assert!(soql::is_safe_field_name("Custom_Field__c"));
    /// assert!(!soql::is_safe_field_name("Bad'; DROP TABLE--"));
    /// ```
    #[must_use]
    pub fn is_safe_field_name(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Must start with a letter
        let first = name.chars().next().unwrap();
        if !first.is_ascii_alphabetic() {
            return false;
        }

        // Rest must be alphanumeric or underscore
        for ch in name.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return false;
            }
        }

        true
    }

    /// Validate a list of field names and return only safe ones.
    ///
    /// This filters out any field names that could be injection attempts.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::soql;
    ///
    /// let fields = ["Id", "Name", "Bad'; DROP--", "Custom__c"];
    /// let safe_fields: Vec<&str> = soql::filter_safe_fields(fields).collect();
    /// assert_eq!(safe_fields, vec!["Id", "Name", "Custom__c"]);
    /// ```
    pub fn filter_safe_fields<'a>(
        fields: impl IntoIterator<Item = &'a str>,
    ) -> impl Iterator<Item = &'a str> {
        fields.into_iter().filter(|f| is_safe_field_name(f))
    }

    /// Validate that a SObject name is safe.
    ///
    /// SObject names follow similar rules to field names.
    #[must_use]
    pub fn is_safe_sobject_name(name: &str) -> bool {
        is_safe_field_name(name)
    }

    /// Build a safe SELECT field list from user-provided field names.
    ///
    /// Filters out any unsafe field names and joins them with commas.
    /// Returns `None` if no safe fields remain.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::soql;
    ///
    /// let fields = vec!["Id", "Name", "Bad'--", "Email"];
    /// let select = soql::build_safe_select(&fields);
    /// assert_eq!(select, Some("Id, Name, Email".to_string()));
    /// ```
    #[must_use]
    pub fn build_safe_select(fields: &[&str]) -> Option<String> {
        let safe: Vec<_> = filter_safe_fields(fields.iter().copied()).collect();
        if safe.is_empty() {
            None
        } else {
            Some(safe.join(", "))
        }
    }
}

/// URL encoding utilities for parameter safety.
pub mod url {
    /// URL-encode a parameter value.
    ///
    /// This ensures that user-provided values cannot break out of URL paths
    /// or inject additional parameters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::url;
    ///
    /// let encoded = url::encode_param("001/../../secret");
    /// // Returns: 001%2F..%2F..%2Fsecret
    /// ```
    #[must_use]
    pub fn encode_param(value: &str) -> String {
        urlencoding::encode(value).into_owned()
    }

    /// Validate that a Salesforce ID has the correct format.
    ///
    /// Salesforce IDs are either 15 or 18 characters and contain only
    /// alphanumeric characters.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::url;
    ///
    /// assert!(url::is_valid_salesforce_id("001000000000001"));
    /// assert!(url::is_valid_salesforce_id("001000000000001AAA"));
    /// assert!(!url::is_valid_salesforce_id("invalid"));
    /// assert!(!url::is_valid_salesforce_id("001/../../etc"));
    /// ```
    #[must_use]
    pub fn is_valid_salesforce_id(id: &str) -> bool {
        let len = id.len();
        (len == 15 || len == 18) && id.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Build a safe SObject URL path.
    ///
    /// Validates the SObject name and ID before constructing the URL.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::url;
    ///
    /// let path = url::sobject_path("Account", "001000000000001");
    /// assert_eq!(path, Some("sobjects/Account/001000000000001".to_string()));
    ///
    /// let bad = url::sobject_path("Bad'; DROP--", "001000000000001");
    /// assert_eq!(bad, None);
    /// ```
    #[must_use]
    pub fn sobject_path(sobject: &str, id: &str) -> Option<String> {
        use super::soql::is_safe_sobject_name;

        if !is_safe_sobject_name(sobject) {
            return None;
        }
        if !is_valid_salesforce_id(id) {
            return None;
        }
        Some(format!("sobjects/{}/{}", sobject, id))
    }
}

/// XML escaping utilities for SOAP/Metadata API.
pub mod xml {
    /// Escape a string for safe inclusion in XML content.
    ///
    /// This escapes the five predefined XML entities.
    ///
    /// # Example
    ///
    /// ```rust
    /// use busbar_sf_client::security::xml;
    ///
    /// let safe = xml::escape("Hello <World> & 'Friends'");
    /// assert_eq!(safe, "Hello &lt;World&gt; &amp; &apos;Friends&apos;");
    /// ```
    #[must_use]
    pub fn escape(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len() + 16);
        for ch in value.chars() {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&apos;"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod soql_tests {
        use super::soql::*;

        #[test]
        fn test_escape_string_basic() {
            assert_eq!(escape_string("hello"), "hello");
            assert_eq!(escape_string("O'Brien"), "O\\'Brien");
            assert_eq!(escape_string("test\\path"), "test\\\\path");
        }

        #[test]
        fn test_escape_string_injection_attempts() {
            // Classic SQL injection patterns
            assert_eq!(
                escape_string("' OR '1'='1"),
                "\\' OR \\'1\\'=\\'1"
            );
            assert_eq!(
                escape_string("'; DELETE FROM Account--"),
                "\\'; DELETE FROM Account--"
            );
            assert_eq!(
                escape_string("' UNION SELECT Id FROM User--"),
                "\\' UNION SELECT Id FROM User--"
            );
        }

        #[test]
        fn test_escape_string_special_chars() {
            assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
            assert_eq!(escape_string("col1\tcol2"), "col1\\tcol2");
            assert_eq!(escape_string("text\r\n"), "text\\r\\n");
        }

        #[test]
        fn test_escape_string_mixed() {
            assert_eq!(
                escape_string("O'Brien's\tfile\\path\n"),
                "O\\'Brien\\'s\\tfile\\\\path\\n"
            );
        }

        #[test]
        fn test_escape_like() {
            assert_eq!(escape_like("100%"), "100\\%");
            assert_eq!(escape_like("test_value"), "test\\_value");
            assert_eq!(escape_like("O'Brien%"), "O\\'Brien\\%");
        }

        #[test]
        fn test_is_safe_field_name() {
            // Valid names
            assert!(is_safe_field_name("Id"));
            assert!(is_safe_field_name("Name"));
            assert!(is_safe_field_name("Custom_Field__c"));
            assert!(is_safe_field_name("Account__r"));
            assert!(is_safe_field_name("X123"));

            // Invalid names
            assert!(!is_safe_field_name("")); // empty
            assert!(!is_safe_field_name("123abc")); // starts with number
            assert!(!is_safe_field_name("field-name")); // contains dash
            assert!(!is_safe_field_name("field.name")); // contains dot
            assert!(!is_safe_field_name("field'name")); // contains quote
            assert!(!is_safe_field_name("field; DROP")); // injection
        }

        #[test]
        fn test_filter_safe_fields() {
            let fields = vec!["Id", "Name", "Bad'; DROP--", "Custom__c", "123start"];
            let safe: Vec<_> = filter_safe_fields(fields).collect();
            assert_eq!(safe, vec!["Id", "Name", "Custom__c"]);
        }

        #[test]
        fn test_build_safe_select() {
            assert_eq!(
                build_safe_select(&["Id", "Name", "Email"]),
                Some("Id, Name, Email".to_string())
            );
            assert_eq!(
                build_safe_select(&["Id", "Bad'--", "Name"]),
                Some("Id, Name".to_string())
            );
            assert_eq!(build_safe_select(&["Bad'; DROP--"]), None);
        }
    }

    mod url_tests {
        use super::url::*;

        #[test]
        fn test_encode_param() {
            assert_eq!(encode_param("simple"), "simple");
            assert_eq!(encode_param("has space"), "has%20space");
            assert_eq!(encode_param("path/traversal"), "path%2Ftraversal");
            assert_eq!(encode_param("../../etc/passwd"), "..%2F..%2Fetc%2Fpasswd");
        }

        #[test]
        fn test_is_valid_salesforce_id() {
            // Valid 15-char ID
            assert!(is_valid_salesforce_id("001000000000001"));
            // Valid 18-char ID
            assert!(is_valid_salesforce_id("001000000000001AAA"));
            // Contains letters (valid)
            assert!(is_valid_salesforce_id("001Abc000000XYZ"));

            // Invalid
            assert!(!is_valid_salesforce_id("")); // empty
            assert!(!is_valid_salesforce_id("short")); // too short
            assert!(!is_valid_salesforce_id("001/../../etc/passwd")); // path traversal
            assert!(!is_valid_salesforce_id("001000000000001!")); // special char
        }

        #[test]
        fn test_sobject_path() {
            assert_eq!(
                sobject_path("Account", "001000000000001"),
                Some("sobjects/Account/001000000000001".to_string())
            );
            assert_eq!(
                sobject_path("Custom__c", "a00000000000001AAA"),
                Some("sobjects/Custom__c/a00000000000001AAA".to_string())
            );

            // Invalid sobject
            assert_eq!(sobject_path("Bad'; DROP--", "001000000000001"), None);
            // Invalid ID
            assert_eq!(sobject_path("Account", "../../etc/passwd"), None);
        }
    }

    mod xml_tests {
        use super::xml::*;

        #[test]
        fn test_escape() {
            assert_eq!(escape("hello"), "hello");
            assert_eq!(escape("<tag>"), "&lt;tag&gt;");
            assert_eq!(escape("&amp;"), "&amp;amp;");
            assert_eq!(escape("\"quoted\""), "&quot;quoted&quot;");
            assert_eq!(escape("it's"), "it&apos;s");
            assert_eq!(
                escape("<script>alert('xss')</script>"),
                "&lt;script&gt;alert(&apos;xss&apos;)&lt;/script&gt;"
            );
        }
    }
}
