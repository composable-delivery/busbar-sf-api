//! Safe query builder with automatic SOQL injection prevention.
//!
//! This module provides a fluent API for building SOQL queries with automatic
//! escaping of user input, making security the default.
//!
//! # Example
//!
//! ```rust,ignore
//! use sf_rest::QueryBuilder;
//!
//! // Type-safe query with automatic escaping
//! let accounts: Vec<Account> = QueryBuilder::new("Account")
//!     .select(&["Id", "Name", "Industry"])
//!     .where_eq("Name", user_input)  // Automatically escaped!
//!     .limit(10)
//!     .execute(&client)
//!     .await?;
//! ```

use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use busbar_sf_client::security::soql;
use crate::{Error, ErrorKind, Result, SalesforceRestClient};

/// Safe SOQL query builder with automatic injection prevention.
///
/// Generic over the result type `T` for type-safe query results.
pub struct QueryBuilder<T> {
    sobject: String,
    fields: Vec<String>,
    conditions: Vec<String>,
    order_by: Vec<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned + Clone> QueryBuilder<T> {
    /// Create a new query builder for the given SObject.
    ///
    /// Validates the SObject name for safety.
    pub fn new(sobject: impl AsRef<str>) -> Result<Self> {
        let sobject = sobject.as_ref();
        
        if !soql::is_safe_sobject_name(sobject) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_SOBJECT".to_string(),
                message: format!("Invalid SObject name: {}", sobject),
            }));
        }

        Ok(Self {
            sobject: sobject.to_string(),
            fields: Vec::new(),
            conditions: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            _phantom: PhantomData,
        })
    }

    /// Select fields to retrieve.
    ///
    /// Validates field names for safety. Invalid field names are silently ignored.
    pub fn select(mut self, fields: &[impl AsRef<str>]) -> Self {
        for field in fields {
            let field = field.as_ref();
            if soql::is_safe_field_name(field) {
                self.fields.push(field.to_string());
            }
        }
        self
    }

    /// Add a WHERE condition with equality check.
    ///
    /// User input is automatically escaped to prevent SOQL injection.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let query = QueryBuilder::new("Account")?
    ///     .select(&["Id", "Name"])
    ///     .where_eq("Name", "O'Brien's Company")  // Automatically escaped!
    ///     .build();
    /// ```
    pub fn where_eq(mut self, field: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        let field = field.as_ref();
        let value = value.as_ref();
        
        if !soql::is_safe_field_name(field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: format!("Invalid field name: {}", field),
            }));
        }
        
        let escaped_value = soql::escape_string(value);
        self.conditions.push(format!("{} = '{}'", field, escaped_value));
        Ok(self)
    }

    /// Add a WHERE condition with inequality check.
    pub fn where_ne(mut self, field: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        let field = field.as_ref();
        let value = value.as_ref();
        
        if !soql::is_safe_field_name(field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: format!("Invalid field name: {}", field),
            }));
        }
        
        let escaped_value = soql::escape_string(value);
        self.conditions.push(format!("{} != '{}'", field, escaped_value));
        Ok(self)
    }

    /// Add a WHERE LIKE condition.
    ///
    /// User input is automatically escaped including LIKE wildcards (%, _).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let query = QueryBuilder::new("Account")?
    ///     .select(&["Id", "Name"])
    ///     .where_like("Name", "tech%")  // Wildcards are escaped!
    ///     .build();
    /// ```
    pub fn where_like(mut self, field: impl AsRef<str>, pattern: impl AsRef<str>) -> Result<Self> {
        let field = field.as_ref();
        let pattern = pattern.as_ref();
        
        if !soql::is_safe_field_name(field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: format!("Invalid field name: {}", field),
            }));
        }
        
        let escaped_pattern = soql::escape_like(pattern);
        self.conditions.push(format!("{} LIKE '%{}%'", field, escaped_pattern));
        Ok(self)
    }

    /// Add a WHERE IN condition.
    ///
    /// Values are automatically escaped.
    pub fn where_in(mut self, field: impl AsRef<str>, values: &[impl AsRef<str>]) -> Result<Self> {
        let field = field.as_ref();
        
        if !soql::is_safe_field_name(field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: format!("Invalid field name: {}", field),
            }));
        }
        
        let escaped_values: Vec<String> = values
            .iter()
            .map(|v| format!("'{}'", soql::escape_string(v.as_ref())))
            .collect();
        
        self.conditions.push(format!("{} IN ({})", field, escaped_values.join(", ")));
        Ok(self)
    }

    /// Add a raw WHERE condition.
    ///
    /// **WARNING**: This does NOT escape user input! Only use with trusted input.
    /// Prefer `where_eq`, `where_like`, etc. for user-provided values.
    pub fn where_raw(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }

    /// Add ORDER BY clause.
    ///
    /// Validates field names for safety.
    pub fn order_by(mut self, field: impl AsRef<str>, ascending: bool) -> Result<Self> {
        let field = field.as_ref();
        
        if !soql::is_safe_field_name(field) {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "INVALID_FIELD".to_string(),
                message: format!("Invalid field name: {}", field),
            }));
        }
        
        let direction = if ascending { "ASC" } else { "DESC" };
        self.order_by.push(format!("{} {}", field, direction));
        Ok(self)
    }

    /// Set LIMIT clause.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set OFFSET clause.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Build the SOQL query string.
    ///
    /// Returns an error if no fields were selected.
    pub fn build(self) -> Result<String> {
        if self.fields.is_empty() {
            return Err(Error::new(ErrorKind::Salesforce {
                error_code: "NO_FIELDS".to_string(),
                message: "No fields selected for query".to_string(),
            }));
        }

        let mut query = format!("SELECT {} FROM {}", self.fields.join(", "), self.sobject);
        
        if !self.conditions.is_empty() {
            query.push_str(&format!(" WHERE {}", self.conditions.join(" AND ")));
        }
        
        if !self.order_by.is_empty() {
            query.push_str(&format!(" ORDER BY {}", self.order_by.join(", ")));
        }
        
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = self.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        
        Ok(query)
    }

    /// Execute the query and return the first page of results.
    pub async fn execute(self, client: &SalesforceRestClient) -> Result<Vec<T>> {
        let query = self.build()?;
        client.query_all(&query).await
    }

    /// Execute the query and return all results with automatic pagination.
    pub async fn execute_all(self, client: &SalesforceRestClient) -> Result<Vec<T>> {
        let query = self.build()?;
        client.query_all(&query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query_build() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name", "Industry"])
            .limit(10)
            .build()
            .unwrap();

        assert_eq!(query, "SELECT Id, Name, Industry FROM Account LIMIT 10");
    }

    #[test]
    fn test_where_eq_escaping() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name"])
            .where_eq("Name", "O'Brien's Company")
            .unwrap()
            .build()
            .unwrap();

        // Single quote should be escaped
        assert!(query.contains("O\\'Brien\\'s Company"));
    }

    #[test]
    fn test_where_like_escaping() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name"])
            .where_like("Name", "tech%")
            .unwrap()
            .build()
            .unwrap();

        // % should be escaped
        assert!(query.contains("tech\\%"));
    }

    #[test]
    fn test_where_in() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name"])
            .where_in("Industry", &["Technology", "Finance"])
            .unwrap()
            .build()
            .unwrap();

        assert!(query.contains("IN ('Technology', 'Finance')"));
    }

    #[test]
    fn test_order_by() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name"])
            .order_by("Name", true)
            .unwrap()
            .build()
            .unwrap();

        assert!(query.contains("ORDER BY Name ASC"));
    }

    #[test]
    fn test_invalid_sobject() {
        let result = QueryBuilder::<serde_json::Value>::new("Account'; DROP TABLE--");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_field_filtered() {
        let query = QueryBuilder::<serde_json::Value>::new("Account")
            .unwrap()
            .select(&["Id", "Name'; DROP TABLE--", "Industry"])
            .build()
            .unwrap();

        // Invalid field should be filtered out
        assert!(!query.contains("DROP TABLE"));
        assert_eq!(query, "SELECT Id, Industry FROM Account");
    }
}
