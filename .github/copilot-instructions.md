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

- Unit tests for all new public methods
- Mock HTTP responses using the project's test infrastructure
- Doc tests with `/// # Examples` blocks for public APIs
- Integration tests go in `tests/` (require Salesforce org credentials)
- Test names should be descriptive: `test_get_deleted_returns_deleted_records`

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

- Default API version: `65.0`
- All clients support version override via `with_api_version()`
- New endpoints should document the minimum API version they require
