# Copilot Instructions for busbar-sf-api

## Project Overview

busbar-sf-api is a suite of Rust crates implementing typed clients for Salesforce APIs.
It provides a low-level, generic framework for building orchestrations against Salesforce
REST, Bulk 2.0, Metadata (SOAP), and Tooling APIs.

## Workspace Structure

```
crates/
  sf-client/     Core HTTP client: retry, compression, rate limiting, conditional requests
  sf-auth/       Authentication: OAuth 2.0 Web Flow, JWT Bearer, Refresh Token, SFDX CLI
  sf-rest/       REST API: SObject CRUD, Query, Search, Composite, Collections, Describe
  sf-bulk/       Bulk API 2.0: Ingest jobs, Query jobs, CSV upload/download
  sf-metadata/   Metadata API: Deploy, Retrieve, List/Describe metadata (SOAP/XML)
  sf-tooling/    Tooling API: Execute Apex, Debug Logs, Code Coverage, Tooling Query
```

## Code Patterns

### Adding a new REST/Tooling endpoint

1. Add the method to the client struct in `crates/sf-{crate}/src/client.rs`
2. Follow existing method patterns — e.g., for a GET endpoint:
   ```rust
   pub async fn new_endpoint(&self, param: &str) -> Result<ResponseType> {
       let url = self.client.rest_url(&format!("path/{}", param));
       self.client.get_json(&url).await
   }
   ```
3. Define request/response types with `#[derive(Debug, Clone, Serialize, Deserialize)]`
4. Place types either in `client.rs` or in a dedicated module (e.g., `search.rs`, `composite.rs`)
5. Re-export public types from `lib.rs`

### Adding a new Metadata SOAP operation

1. Add the method to `MetadataClient` in `crates/sf-metadata/src/client.rs`
2. Build the SOAP XML envelope following existing patterns (see `deploy()`, `list_metadata()`)
3. Use `xml::escape()` from sf-client for any user-provided values
4. Parse the SOAP response XML to extract the result
5. Define result types matching the Salesforce Metadata API documentation

### Adding a new Bulk API endpoint

1. Add the method to `BulkApiClient` in `crates/sf-bulk/src/client.rs`
2. Use `self.client.bulk_url()` for URL construction
3. Follow existing patterns for job state management and polling

## Security Requirements

- **SOQL Injection Prevention**: Always use `QueryBuilder` or `soql::escape_string()` for user input in SOQL
- **XML Escaping**: Use `xml::escape()` for values inserted into SOAP XML envelopes
- **ID Validation**: Use `url::is_valid_salesforce_id()` before interpolating IDs into URLs
- **URL Encoding**: Use `url::encode_param()` for query parameters
- **Credential Redaction**: Never log or expose access tokens — credentials implement redacted `Debug`

## Testing Requirements

### Unit Tests (No SF_AUTH_URL Required)
- Unit tests for all new public methods in `src/**/*.rs`
- Mock HTTP responses using the project's test infrastructure
- Doc tests with `/// # Examples` blocks for public APIs
- **Unit tests MUST NOT require SF_AUTH_URL or a real Salesforce org**
- Run with: `cargo test --lib`

### Integration Tests (SF_AUTH_URL Required)
- Integration tests go in `tests/integration/` directory
- Organized by API module: `auth.rs`, `bulk.rs`, `metadata.rs`, `rest.rs`, `tooling.rs`
- **Integration tests MUST run against a real Salesforce org**
- Run with: `SF_AUTH_URL=... cargo test --test integration`
- Test names should be descriptive: `test_get_deleted_returns_deleted_records`

### Integration Test Guidelines

Integration tests in `tests/integration/` **MUST** run against a real Salesforce org and **MUST** fail if the environment is not properly configured.

**Key Principles:**

1. **Never Skip Tests**: Use `common::get_credentials()` which panics with helpful error messages if SF_AUTH_URL is not set or invalid.

2. **Test Behavior, Not Just Execution**: Integration tests should:
   - Validate actual API responses and behavior
   - Test edge cases and error conditions
   - Assert on specific values, not just "didn't error"
   - Verify state changes (create → verify created → delete → verify deleted)

3. **NOT Example Code**: Integration tests are NOT for wrapping example code. They test specific API behaviors with assertions. Keep actual examples in `examples/` directory for documentation.

4. **Good vs Bad Examples**:
   
   ❌ **BAD** - Just runs without assertions:
   ```rust
   #[tokio::test]
   async fn test_create_account() {
       let creds = common::get_credentials().await;
       let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
           .expect("Failed to create client");
       let _id = client.create("Account", &json!({"Name": "Test"})).await?;
       // No assertions - just "it didn't crash"
   }
   ```
   
   ✅ **GOOD** - Tests actual behavior:
   ```rust
   #[tokio::test]
   async fn test_create_account_sets_name_correctly() {
       let creds = common::get_credentials().await;
       let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
           .expect("Failed to create client");
       
       let account_name = format!("Integration Test {}", chrono::Utc::now().timestamp());
       
       // Create and verify ID is returned
       let id = client.create("Account", &json!({"Name": account_name}))
           .await
           .expect("Failed to create account");
       assert!(!id.is_empty(), "Account ID should not be empty");
       assert!(id.starts_with("001"), "Account ID should start with 001 prefix");
       
       // Verify the account was actually created with correct values
       let created: serde_json::Value = client.get("Account", &id, Some(&["Id", "Name"]))
           .await
           .expect("Failed to retrieve created account");
       assert_eq!(created["Name"], account_name, "Account name should match");
       
       // Cleanup
       client.delete("Account", &id).await.expect("Failed to delete test account");
   }
   ```

5. **Error Testing**: Test that errors happen when they should:
   ```rust
   #[tokio::test]
   async fn test_invalid_sobject_returns_error() {
       let creds = common::get_credentials().await;
       let client = SalesforceRestClient::new(creds.instance_url(), creds.access_token())
           .expect("Failed to create client");
       
       let result = client.create("InvalidSObject__c", &json!({"Name": "Test"})).await;
       assert!(result.is_err(), "Creating invalid SObject should fail");
       
       let err = result.unwrap_err();
       assert!(
           format!("{}", err).contains("InvalidSObject__c") || 
           format!("{}", err).contains("NOT_FOUND"),
           "Error should mention the invalid SObject name"
       );
   }
   ```

6. **Use Descriptive Names**: Test names should explain what behavior is being tested:
   - ✅ `test_composite_batch_executes_subrequests_independently`
   - ✅ `test_query_with_invalid_field_returns_error`
   - ❌ `test_composite_api`
   - ❌ `test_query`

7. **Clean Up After Yourself**: Always delete test data you create, even if the test fails (use proper cleanup patterns).

8. **Document What You're Testing**: Add comments explaining the behavior being validated:
   ```rust
   // Test that composite subrequests can reference results from earlier requests
   // using the @{referenceId.field} syntax
   ```

### Test Organization

```
crates/
  sf-*/src/
    lib.rs          # Unit tests in #[cfg(test)] modules
    client.rs       # Unit tests with mocked HTTP responses
tests/
  integration/
    common.rs       # Shared test helpers (get_credentials)
    auth.rs         # Authentication/OAuth integration tests
    bulk.rs         # Bulk API 2.0 integration tests
    metadata.rs     # Metadata API integration tests
    rest.rs         # REST API integration tests
    tooling.rs      # Tooling API integration tests
```

## Style Guide

- Run `cargo fmt --workspace` before committing
- Run `cargo clippy --workspace -- -D warnings` — zero warnings required
- Public APIs must have `///` doc comments
- Use `Result<T>` with the crate's error type (not `anyhow` or `Box<dyn Error>`)
- Follow Rust API Guidelines: https://rust-lang.github.io/api-guidelines/

## Branch and PR Conventions

- Branch naming: `feature/{issue-number}-short-description` (e.g., `feature/42-add-composite-batch`)
- Reference the issue in the PR description: `Closes #42`
- PR description should include a test plan
- One logical change per PR — don't combine unrelated endpoint additions

## API Version

- Default API version: `62.0`
- All clients support version override via `with_api_version()`
- New endpoints should document the minimum API version they require
