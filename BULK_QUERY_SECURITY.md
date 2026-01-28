# Bulk Query Security Implementation

## Problem Statement

The initial implementation of bulk query operations did not use QueryBuilder for SOQL injection prevention, making it vulnerable to attacks when handling user input.

## Solution: Security by Default

This greenfield codebase now implements **security by default** with **no unsafe escape hatches**.

### Design Principles

1. **No Insecure APIs** - Only secure methods are exposed in the public API
2. **Security as Default** - No methods with "safe" or "secure" in names (it's just the baseline)
3. **No Compromises** - No low-level APIs that bypass security
4. **Impossible to Misuse** - API design prevents SOQL injection at compile time

### Implementation

#### Public API (Secure Only)

```rust
use busbar_sf_bulk::{BulkApiClient, QueryBuilder};

// The ONLY way to execute bulk queries - automatic SOQL injection prevention
let result = client.execute_query(
    QueryBuilder::<Account>::new("Account")?
        .select(&["Id", "Name", "Industry"])
        .where_eq("Name", user_input)?  // Automatically escaped!
        .limit(10000)
).await?;
```

#### What's NOT Possible (By Design)

```rust
// ❌ REMOVED: Cannot pass raw SOQL strings
// client.execute_query("SELECT Id FROM Account WHERE Name = '{}'", user_input)

// ❌ REMOVED: Cannot create query jobs directly
// client.create_query_job(CreateQueryJobRequest::new(soql))

// ❌ REMOVED: Cannot access low-level query job methods
// client.get_query_job(job_id)
// client.wait_for_query_job(job_id)
```

### Changes Made

#### 1. QueryBuilder Integration

- Added `busbar-sf-rest` dependency (enabled by default via `query-builder` feature)
- Re-exported `QueryBuilder` from `busbar-sf-bulk` for convenient access
- `execute_query()` ONLY accepts `QueryBuilder<T>` - no raw SOQL

#### 2. Removed Unsafe APIs

| Removed API | Reason |
|------------|--------|
| `create_query_job()` | Accepts raw SOQL - unsafe |
| `get_query_job()` | Enables bypassing QueryBuilder |
| `wait_for_query_job()` | Enables bypassing QueryBuilder |
| Public `CreateQueryJobRequest` | Takes raw SOQL - made internal |

#### 3. Internal Implementation

Low-level query operations are now internal implementation details:
- `CreateQueryJobRequest` is `pub(crate)` - not exposed
- Internal methods are private
- Used only by `execute_query()` after QueryBuilder validation

### Security Guarantees

1. **Compile-Time Safety**: Cannot call bulk query APIs without QueryBuilder
2. **Automatic Escaping**: All user input automatically escaped by QueryBuilder
3. **No Bypass Mechanisms**: No way to circumvent security (even for "advanced" users)
4. **Type Safety**: Generic type parameter ensures proper deserialization

### Example Usage

```rust
use busbar_sf_bulk::{BulkApiClient, QueryBuilder};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct Account {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BulkApiClient::new(instance_url, access_token)?;

    // User input is automatically escaped
    let user_input = "O'Brien's Company; DROP TABLE--";
    
    let result = client.execute_query(
        QueryBuilder::<Account>::new("Account")?
            .select(&["Id", "Name"])
            .where_eq("Name", user_input)?  // Escaped: "O\\'Brien\\'s Company; DROP TABLE--"
            .limit(100)
    ).await?;

    println!("Retrieved {} records", result.job.number_records_processed);
    Ok(())
}
```

### Comparison with REST API

Both APIs now have consistent security:

| API | Method | Security |
|-----|--------|----------|
| REST | `query()` with QueryBuilder | ✅ Secure by default |
| REST | `query()` with raw SOQL | ❌ Removed in review |
| Bulk | `execute_query()` with QueryBuilder | ✅ Secure by default |
| Bulk | Raw SOQL methods | ❌ Never exposed |

### Migration Guide

**Before (Insecure - Removed):**
```rust
// This API never existed in production releases
let soql = format!("SELECT Id FROM Account WHERE Name = '{}'", user_input);
let result = client.execute_query(&soql).await?;
```

**After (Secure - Current API):**
```rust
let result = client.execute_query(
    QueryBuilder::<Account>::new("Account")?
        .select(&["Id", "Name"])
        .where_eq("Name", user_input)?  // Automatically escaped!
).await?;
```

## Testing

All tests pass with new implementation:
- 9 unit tests in `busbar-sf-bulk`
- No clippy warnings
- Full workspace builds successfully
- Example code demonstrates secure usage

## Documentation

- Updated `lib.rs` with secure-by-default examples
- Updated `bulk_operations.rs` example
- Removed all references to unsafe APIs
- Added comprehensive security documentation

## Conclusion

This implementation ensures that **SOQL injection is impossible** by design. There are no unsafe APIs, no escape hatches, and no way to accidentally introduce vulnerabilities. Security is not optional - it's the only way the API works.

This is the advantage of a **greenfield codebase**: we can build security in from day one without backwards compatibility concerns.
