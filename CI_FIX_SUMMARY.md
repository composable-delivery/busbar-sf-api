# CI Build Failure - Root Cause and Fix

## Problem

The CI builds were failing with both unit tests and integration tests failing, even though local tests passed with `./verify-build.sh`.

## Root Cause Analysis

### Issue 1: Test Job Running Integration Tests ❌

**File:** `.github/workflows/ci.yml` Line 49

**Problem:**
```yaml
- name: Run tests
  run: cargo test --workspace --verbose
```

This command runs **ALL** tests including:
- ✅ Unit tests in `crates/*/src/` (these pass)
- ❌ Integration tests in `tests/integration/` (these FAIL without SF_AUTH_URL)

The "Test" job runs on a matrix of OS/Rust combinations but **does not have SF_AUTH_URL** set. When integration tests run, they immediately panic with:

```
╔══════════════════════════════════════════════════════════════════════╗
║ INTEGRATION TEST CONFIGURATION ERROR                                 ║
║ SF_AUTH_URL environment variable is NOT set!                         ║
╚══════════════════════════════════════════════════════════════════════╝
```

This causes all 48 integration tests to fail, making the build fail.

### Issue 2: Useless Job Summary ❌

**File:** `.github/workflows/ci.yml` Line 211-220

**Problem:**
```yaml
- name: Integration tests summary
  if: always()
  run: |
      echo "### Integration Tests (Real Org)" >> "$GITHUB_STEP_SUMMARY"
      echo "- SF_AUTH_URL available: true" >> "$GITHUB_STEP_SUMMARY"
      echo "- Job status: ${{ job.status }}" >> "$GITHUB_STEP_SUMMARY"
```

The summary only showed:
- SF_AUTH_URL is available (not useful - we expect this)
- Job status (just says "failed", no details)

**Missing:**
- Which tests failed
- How many tests passed vs failed
- Error messages
- Coverage information

### Why Local Tests Passed

The `verify-build.sh` script correctly runs:
```bash
cargo test --workspace --lib --quiet  # Only unit tests
```

This never tries to run integration tests, so it succeeds locally.

## The Fix

### 1. Fixed Test Job ✅

**Changed Line 49:**
```yaml
- name: Run unit tests
  run: cargo test --workspace --lib --verbose
```

Now the Test job only runs unit tests (123 tests), which don't need SF_AUTH_URL.

Integration tests (48 tests) only run in the dedicated "Integration Tests (Real Org)" job which has SF_AUTH_URL from the `scratch` environment.

### 2. Improved Job Summaries ✅

**Unit Test Coverage Job:**
- Captures test output to file
- Shows test exit code
- Shows test result counts (passed/failed/ignored)
- Shows coverage details in formatted code blocks
- Runs summary even if tests fail (`if: always()`)

**Integration Test Job:**
- Captures test output to file
- Shows test exit code
- Shows test result counts
- **Lists failed tests** (up to 20)
- **Shows error details** (first 30 lines)
- Shows coverage details
- All in readable markdown code blocks

### Example New Summary Output

```markdown
## Integration Tests (Real Org)

### Test Results
Test exit code: 1
test result: FAILED. 35 passed; 8 failed; 5 ignored; 0 measured

### Failed Tests
test rest::test_invalid_query ... FAILED
test rest::test_missing_field ... FAILED
test bulk::test_invalid_sobject ... FAILED
...

### Error Details
---- rest::test_invalid_query stdout ----
thread 'rest::test_invalid_query' panicked at 'assertion failed: result.is_err()'
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
...

### Coverage
Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed
-------------------------------------------------------------------------------------------------------------
crates/sf-rest/src/client.rs       256                12    95.31%        89                 3    96.63%
```

## Verification

All checks now pass locally:

```bash
$ ./verify-build.sh
==========================================
Build Verification Script
==========================================

1. Building workspace...
   ✓ Build succeeded

2. Running clippy...
   ✓ Clippy passed (zero warnings)

3. Checking formatting...
   ✓ Formatting correct

4. Running unit tests...
   ✓ Unit tests passed

5. Verifying integration tests fail without SF_AUTH_URL...
   ✓ Integration tests properly fail without SF_AUTH_URL

==========================================
ALL CHECKS PASSED ✓
==========================================
```

## Expected CI Behavior

With these fixes:

1. **Test Job** - Runs on all OS/Rust combinations
   - Runs unit tests only
   - ✅ Should pass (123 tests)

2. **Integration Tests Job** - Runs on Ubuntu with SF_AUTH_URL
   - Runs integration tests only
   - ✅ Should pass (48 tests) if SF_AUTH_URL is valid
   - Shows detailed summary with test counts, failures, and coverage

3. **All other jobs** (clippy, fmt, docs, coverage)
   - ✅ Should continue to pass as before
