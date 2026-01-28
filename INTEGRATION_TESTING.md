# Integration Testing Guide

This document describes how to run integration tests for busbar-sf-api against a real Salesforce org.

## Overview

The integration tests validate all client methods and endpoints against a live Salesforce org. There are three test suites:

1. **integration_sf_auth_url.rs** - Comprehensive tests using `SF_AUTH_URL` (25+ tests)
2. **integration_examples.rs** - Tests for all example programs (15+ tests)
3. **integration_test.rs** - Original tests using SF CLI (15 tests)

## Prerequisites

### Option 1: Using SF_AUTH_URL (Recommended)

This is the easiest method and what CI uses.

1. **Get an SFDX Auth URL** from a Salesforce org:
   ```bash
   sf org display --verbose --target-org <your-org-alias>
   ```
   Look for the "Sfdx Auth Url" in the output. It looks like:
   ```
   force://<client_id>:<client_secret>:<refresh_token>@<instance_url>
   ```

2. **Set the environment variable**:
   ```bash
   export SF_AUTH_URL="force://..."
   ```

### Option 2: Using Salesforce CLI (Alternative)

1. **Install Salesforce CLI** (if not already installed):
   ```bash
   # On macOS
   brew install sf

   # On Linux/Windows - download from:
   # https://developer.salesforce.com/tools/salesforcecli
   ```

2. **Authenticate to a Salesforce org**:
   ```bash
   # For sandbox/scratch org
   sf org login web --alias test-org

   # Or for production
   sf org login web --instance-url https://login.salesforce.com --alias test-org
   ```

3. **Set the test org alias** (optional):
   ```bash
   export SF_TEST_ORG_ALIAS="test-org"
   ```
   If not set, defaults to `roundtrip-org-a`.

## Running Integration Tests

### Run All Integration Tests

Using SF_AUTH_URL (recommended):
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_sf_auth_url -- --ignored
SF_AUTH_URL="your-auth-url" cargo test --test integration_examples -- --ignored
```

Using SF CLI:
```bash
cargo test --test integration_test -- --ignored
```

### Run Specific Test Suites

**Comprehensive API tests:**
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_sf_auth_url -- --ignored
```

**Example program tests:**
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_examples -- --ignored
```

**Original SF CLI tests:**
```bash
SF_TEST_ORG_ALIAS="my-org" cargo test --test integration_test -- --ignored
```

### Run Specific Tests

Run a single test:
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_sf_auth_url test_rest_crud_lifecycle -- --ignored
```

Run tests matching a pattern:
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_sf_auth_url rest_ -- --ignored
```

### View Test Output

Use `--nocapture` to see println! output:
```bash
SF_AUTH_URL="your-auth-url" cargo test --test integration_examples -- --ignored --nocapture
```

## Test Coverage

### integration_sf_auth_url.rs

**REST API Tests:**
- ✅ Composite API (multiple operations in one request)
- ✅ SOSL search
- ✅ Batch operations (create/get/update/delete multiple)
- ✅ Query pagination
- ✅ Upsert operations

**QueryBuilder Security Tests:**
- ✅ SQL injection prevention
- ✅ LIKE clause escaping

**Bulk API 2.0 Tests:**
- ✅ Insert lifecycle
- ✅ Query operation
- ✅ Update operation

**Tooling API Tests:**
- ✅ Query ApexClass
- ✅ Execute anonymous Apex (success)
- ✅ Execute anonymous Apex (compile error)
- ✅ Query with pagination

**Error Handling Tests:**
- ✅ Invalid field names
- ✅ Invalid SObject names
- ✅ Invalid record IDs

**Security Tests:**
- ✅ Credentials redaction in Debug output
- ✅ Client redaction in Debug output

**Type-Safe Pattern Tests:**
- ✅ CRUD with type-safe structs
- ✅ Query with type-safe structs

### integration_examples.rs

Tests that all example programs work correctly:
- ✅ basic_auth.rs - Authentication from SF_AUTH_URL
- ✅ rest_crud.rs - Type-safe and dynamic CRUD
- ✅ queries.rs - QueryBuilder, relationships, aggregates
- ✅ bulk_operations.rs - Bulk insert and query
- ✅ error_handling.rs - Error handling patterns
- ✅ Full integration test combining all examples

## CI/CD

The integration tests run automatically on GitHub Actions when:
- Manually triggered via workflow_dispatch
- On push to main branch (when SF_AUTH_URL secret is available)

The tests use the `copilot` environment which has access to the `SF_AUTH_URL` secret.

## Troubleshooting

### "SF_AUTH_URL environment variable must be set"

Make sure you have exported the `SF_AUTH_URL` variable:
```bash
export SF_AUTH_URL="force://..."
```

Or pass it inline:
```bash
SF_AUTH_URL="force://..." cargo test --test integration_sf_auth_url -- --ignored
```

### "Failed to authenticate from SF_AUTH_URL"

Check that your auth URL is valid and not expired. Get a fresh one:
```bash
sf org display --verbose --target-org <your-org>
```

### "sf org display failed"

For the SF CLI-based tests, make sure you're authenticated:
```bash
sf org list
sf org login web --alias test-org
```

### Test Failures

Some tests create and clean up data in your org. If tests fail:
1. Check that you have sufficient API limits
2. Verify the org has standard objects (Account, Contact, etc.)
3. For upsert tests, ensure AccountNumber field exists and is marked as External ID
4. Check that you have permissions to create/modify records

### Rate Limiting

If you hit API limits:
1. Use a dedicated test org
2. Wait for daily limits to reset
3. Reduce test concurrency by running tests sequentially

## Best Practices

1. **Use a dedicated test org** - Don't run integration tests against production
2. **Use scratch orgs** - They're perfect for testing and automatically expire
3. **Monitor API usage** - The tests make many API calls
4. **Run locally before CI** - Catch issues early
5. **Clean up test data** - Tests should clean up after themselves

## Example: Setting Up a Scratch Org for Testing

```bash
# Create a scratch org
sf org create scratch --definition-file config/project-scratch-def.json --alias test-org --duration-days 30

# Get the auth URL
sf org display --verbose --target-org test-org

# Copy the Sfdx Auth Url and export it
export SF_AUTH_URL="force://..."

# Run integration tests
cargo test --test integration_sf_auth_url -- --ignored
cargo test --test integration_examples -- --ignored
```

## Questions?

If you encounter issues:
1. Check the test output with `--nocapture`
2. Verify your org is accessible and you have permissions
3. Review the test code to understand what it's testing
4. Open an issue with the error message and test name
