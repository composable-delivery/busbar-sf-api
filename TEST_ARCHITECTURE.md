# Test Architecture

## Overview

This project maintains a clear separation between unit tests and integration tests.

## Unit Tests (No SF_AUTH_URL Required)

**Location:** `crates/*/src/**/*.rs` in `#[cfg(test)]` modules

**Purpose:** Test individual functions and methods with mocked dependencies

**Run:** `cargo test --lib` or `cargo test --workspace --lib`

**Requirements:**
- No network calls to real Salesforce org
- No SF_AUTH_URL environment variable needed
- Use mocked HTTP responses
- Fast execution

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_url_construction() {
        let client = ToolingClient::new("https://test.salesforce.com", "token").unwrap();
        let url = client.client.tooling_url("composite");
        assert_eq!(url, "https://test.salesforce.com/services/data/v62.0/tooling/composite");
    }
}
```

## Integration Tests (SF_AUTH_URL Required)

**Location:** `tests/integration/*.rs`

**Purpose:** Test actual API functionality against a real Salesforce org

**Run:** `SF_AUTH_URL=... cargo test --test integration`

**Requirements:**
- **MUST** run against a real Salesforce org
- **MUST** fail if SF_AUTH_URL is not set (no silent skipping)
- Test actual API behavior with assertions
- Verify responses, not just "didn't crash"

**Organization:**
- `common.rs` - Shared helpers (get_credentials)
- `auth.rs` - OAuth/authentication tests
- `bulk.rs` - Bulk API 2.0 tests
- `metadata.rs` - Metadata API tests
- `rest.rs` - REST API tests (SOQL, composite, collections)
- `tooling.rs` - Tooling API tests

**Example:**
```rust
#[tokio::test]
async fn test_composite_batch_executes_subrequests() {
    let creds = common::get_credentials().await;
    let client = ToolingClient::new(creds.instance_url(), creds.access_token())
        .expect("Failed to create client");
    
    let request = CompositeBatchRequest { /* ... */ };
    let response = client.composite_batch(&request).await
        .expect("Batch request should succeed");
    
    assert_eq!(response.results.len(), 2, "Should have 2 results");
    assert_eq!(response.results[0].status_code, 200, "First request should succeed");
}
```

## CI/CD Workflow

### Unit Test Job
- Runs on: push, pull_request
- Matrix: ubuntu, windows, macos × stable, beta
- Command: `cargo test --workspace`
- No SF_AUTH_URL needed

### Integration Test Job  
- Runs on: push, pull_request
- Environment: `scratch` (provides SF_AUTH_URL secret)
- Command: `cargo test --workspace --test integration`
- Requires: SF_AUTH_URL secret set in GitHub

## Best Practices

### DO ✅
- Write unit tests for all public methods
- Mock external dependencies in unit tests
- Write integration tests that validate actual API behavior
- Use descriptive test names explaining what's being tested
- Assert on specific values and behavior
- Clean up test data after tests

### DON'T ❌
- Make unit tests require SF_AUTH_URL
- Write integration tests that just wrap example code
- Skip integration tests silently when SF_AUTH_URL not set
- Write tests with no assertions
- Leave test data in Salesforce org

## Migration from Old Pattern

**Old (WRONG):**
```rust
let Some(creds) = require_credentials().await else {
    return;  // Silently skips test
};
```

**New (CORRECT):**
```rust
let creds = common::get_credentials().await;  // Panics with helpful error if not configured
```
