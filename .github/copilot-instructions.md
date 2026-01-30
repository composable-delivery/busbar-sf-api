# Agent Instructions for busbar-sf-api

## Project Overview

busbar-sf-api is a suite of Rust crates implementing typed clients for Salesforce APIs.
It provides a low-level, generic framework for building orchestrations against Salesforce
REST, Bulk 2.0, Metadata (SOAP), and Tooling APIs.

This is a **low-level API client library only**. No orchestration, no examples directory,
no wrapper scripts. Documentation belongs in rustdoc (`///` comments) and on docs.rs.

## Workspace Structure

```
crates/
  sf-client/     Core HTTP client: retry, compression, rate limiting, conditional requests
  sf-auth/       Authentication: OAuth 2.0 Web Flow, JWT Bearer, Refresh Token, SFDX CLI
  sf-rest/       REST API: SObject CRUD, Query, Search, Composite, Collections, Describe
  sf-bulk/       Bulk API 2.0: Ingest jobs, Query jobs, CSV upload/download
  sf-metadata/   Metadata API: Deploy, Retrieve, List/Describe metadata (SOAP/XML)
  sf-tooling/    Tooling API: Execute Apex, Debug Logs, Code Coverage, Tooling Query
tests/
  integration/   Real-org integration tests (one file per API module)
```

## Enforcement Rules

These rules are non-negotiable. CI will fail if any are violated.

1. **No examples/ directory.** Documentation goes in rustdoc. Remove any `examples/` files.
2. **No standalone documentation files.** Do not create `*.md` files for CI fixes, test
   architecture, coverage reports, job summary formats, or other operational notes.
   The only allowed markdown files are: `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`,
   `SECURITY.md`, and `.github/copilot-instructions.md`.
3. **No shell scripts.** Do not create `*.sh` verification scripts or build helpers.
4. **`cargo fmt --all --check` must pass.** Always run `cargo fmt` before committing.
5. **`cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass.**
6. **All unit tests must pass:** `cargo test --workspace --lib`
7. **All integration tests must pass against a real org:**
   `SF_AUTH_URL=... cargo test --test integration`
8. **Every public client method must have an integration test** in `tests/integration/`.
9. **Coverage must not decrease.** Codecov enforces this on every PR.

## Code Patterns

### Adding a new REST/Tooling endpoint

1. Add the method to the client struct in `crates/sf-{crate}/src/client.rs`
2. Follow existing method patterns:
   ```rust
   pub async fn new_endpoint(&self, param: &str) -> Result<ResponseType> {
       let url = self.client.rest_url(&format!("path/{}", param));
       self.client.get_json(&url).await.map_err(Into::into)
   }
   ```
3. Define request/response types with `#[derive(Debug, Clone, Serialize, Deserialize)]`
4. Place types in `client.rs` or a dedicated module (e.g., `composite.rs`)
5. Re-export public types from `lib.rs`
6. Add a unit test with mocked HTTP in the same file's `#[cfg(test)]` module
7. Add an integration test in `tests/integration/{module}.rs`

### SObject Collections responses

The Salesforce SObject Collections GET endpoint returns a JSON array that may contain
`null` entries for records that could not be retrieved. Always deserialize as
`Vec<Option<T>>` and filter out `None` values:

```rust
let results: Vec<Option<T>> = self.client.get_json(&url).await?;
Ok(results.into_iter().flatten().collect())
```

### Adding a new Metadata SOAP operation

1. Add the method to `MetadataClient` in `crates/sf-metadata/src/client.rs`
2. Build the SOAP XML envelope following existing patterns
3. Use `xml::escape()` from sf-client for any user-provided values
4. Parse the SOAP response XML to extract the result
5. Define result types matching the Salesforce Metadata API documentation

## Security Requirements

- **SOQL Injection Prevention**: Always use `QueryBuilder` or `soql::escape_string()`
- **XML Escaping**: Use `xml::escape()` for values inserted into SOAP envelopes
- **ID Validation**: Use `url::is_valid_salesforce_id()` before interpolating IDs into URLs
- **URL Encoding**: Use `url::encode_param()` for query parameters
- **Credential Redaction**: Never log or expose access tokens

## Testing Requirements

### Unit Tests (No SF_AUTH_URL Required)

- Unit tests for all new public methods in `crates/sf-*/src/*.rs`
- Mock HTTP responses using wiremock
- Doc tests with `/// # Examples` blocks for public APIs
- **Unit tests MUST NOT require SF_AUTH_URL or a real Salesforce org**
- Run with: `cargo test --workspace --lib`

### Integration Tests (SF_AUTH_URL Required)

- Integration tests go in `tests/integration/` directory
- Organized by API module: `auth.rs`, `bulk.rs`, `metadata.rs`, `rest.rs`, `tooling.rs`
- **Integration tests MUST run against a real Salesforce org**
- **Integration tests MUST fail clearly** if SF_AUTH_URL is not set
- Run with: `SF_AUTH_URL=... cargo test --test integration`

### Integration Test Principles

1. **Use `common::get_credentials()`** which panics with a clear message if SF_AUTH_URL
   is not set or invalid. Never silently skip tests.

2. **Test behavior, not execution.** Every test must have meaningful assertions:
   - Assert on specific values, not just "didn't error"
   - Verify state changes (create -> verify -> delete -> verify deleted)
   - Test error conditions explicitly

3. **Clean up after yourself.** Always delete test data you create.

4. **Use descriptive test names:**
   - `test_composite_batch_executes_subrequests_independently`
   - `test_query_with_invalid_field_returns_error`

### Test Organization

```
crates/sf-*/src/
    *.rs            # Unit tests in #[cfg(test)] modules
tests/integration/
    common.rs       # Shared test helpers (get_credentials)
    auth.rs         # Authentication/OAuth tests
    bulk.rs         # Bulk API 2.0 tests
    metadata.rs     # Metadata API tests
    rest.rs         # REST API tests
    tooling.rs      # Tooling API tests
```

## CI Pipeline

The CI pipeline (`ci.yml`) runs these parallel jobs on every PR:

| Job | What it checks |
|-----|---------------|
| **fmt** | `cargo fmt --all --check` |
| **clippy** | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| **test** | Unit tests + coverage via `cargo-llvm-cov`, uploaded to Codecov |
| **docs** | `cargo doc` with `-D warnings` |
| **msrv** | `cargo check` with Rust 1.88 |
| **integration** | Integration tests + coverage against real org (requires `scratch` environment) |

Coverage is uploaded to Codecov with separate flags for `unit-tests` and `integration-tests`.
The `codecov.yml` config enforces that coverage does not decrease on any PR.

## Style Guide

- Run `cargo fmt --workspace` before committing
- Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Public APIs must have `///` doc comments
- Use `Result<T>` with the crate's error type (not `anyhow` or `Box<dyn Error>`)
- Follow Rust API Guidelines: https://rust-lang.github.io/api-guidelines/

## API Version

- Default API version: `62.0`
- All clients support version override via `with_api_version()`
- New endpoints should document the minimum API version they require
