# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.3] - 2026-02-01

### Architecture

- **Modular client architecture**: Split monolithic `client.rs` files into per-feature modules
  - `sf-rest`: 1 file (970 lines) → 21 module files
  - `sf-tooling`: 1 file (974 lines) → 14 module files
  - `sf-metadata`: 1 file (1,387 lines) → 7 module files
- No public API changes — all `lib.rs` re-exports remain identical
- **166 public async methods** across 5 crates (up from ~40 in 0.0.2)
- **547 tests**: 411 unit tests + 136 integration tests (every endpoint tested against live Salesforce org)

### Added — REST API (`busbar-sf-rest`)

**Quick Actions** — `quick_actions.rs`
- `list_global_quick_actions()`, `describe_global_quick_action()`
- `list_quick_actions()`, `describe_quick_action()`, `invoke_quick_action()`

**List Views** — `list_views.rs`
- `list_views()`, `get_list_view()`, `describe_list_view()`, `execute_list_view()`

**Process Rules & Approvals** — `process.rs`
- `list_process_rules()`, `list_process_rules_for_sobject()`, `trigger_process_rules()`
- `list_pending_approvals()`, `submit_approval()`

**Invocable Actions** — `invocable_actions.rs`
- `list_standard_actions()`, `list_custom_action_types()`, `list_custom_actions()`
- `describe_standard_action()`, `describe_custom_action()`
- `invoke_standard_action()`, `invoke_custom_action()`

**Composite Graph** — `composite.rs`
- `composite_graph()` for multi-level dependent operations

**Parameterized Search** — `search.rs`
- `parameterized_search()`, `search_suggestions()`, `search_scope_order()`, `search_result_layouts()`

**Consent API** — `consent.rs`
- `read_consent()`, `write_consent()`, `read_multi_consent()`

**Knowledge Management** — `knowledge.rs`
- `knowledge_settings()`, `knowledge_articles()`, `data_category_groups()`, `data_categories()`

**Scheduler** — `scheduler.rs`
- `appointment_candidates()`, `appointment_slots()`

**User Password** — `user_password.rs`
- `get_user_password_status()`, `set_user_password()`, `reset_user_password()`

**Embedded Service** — `embedded_service.rs`
- `get_embedded_service_config()`

**Tabs, Theme & App Menu** — `standalone.rs`
- `tabs()`, `theme()`, `app_menu()`, `compact_layouts()`
- `recent_items()`, `relevant_items()`

**Platform Events & Lightning Metrics** — `standalone.rs`
- `platform_event_schema()`, `lightning_toggle_metrics()`, `lightning_usage()`

**SObject Sync** — `sync.rs`
- `get_deleted()`, `get_updated()`

**Binary Content** — `binary.rs`
- `get_blob()`, `get_rich_text_image()`, `get_relationship()`, `get_sobject_basic_info()`

**Layout Enhancements** — `layout.rs`
- `describe_named_layout()`, `describe_approval_layouts()`, `describe_global_publisher_layouts()`

### Added — Tooling API (`busbar-sf-tooling`)

**CRUD & Discovery** — `sobject.rs`, `describe.rs`
- `get()`, `create()`, `update()`, `delete()`
- `describe_global()`, `describe_sobject()`, `basic_info()`, `resources()`

**Test Execution** — `test_execution.rs`
- `run_tests_async()`, `run_tests_sync()`, `discover_tests()`, `run_tests()`

**Code Intelligence** — `code_intelligence.rs`
- `completions_apex()`, `completions_visualforce()`

**SObject Collections** — `collections.rs`
- `get_multiple()`, `create_multiple()`, `update_multiple()`, `delete_multiple()`

**Composite API** — `composite.rs`
- `composite()`, `composite_batch()`, `composite_tree()`

**MetadataComponentDependency** — `dependencies.rs`
- `get_metadata_component_dependencies()` — via Tooling API SOQL (feature-gated: `dependencies`)

### Added — Metadata API (`busbar-sf-metadata`)

**CRUD Sync Operations** — `crud_sync.rs`
- `create_metadata()`, `read_metadata()`, `update_metadata()`
- `upsert_metadata()`, `delete_metadata()`, `rename_metadata()`

**Deployment Enhancements** — `deploy.rs`
- `cancel_deploy()`, `deploy_recent_validation()`

**SOAP Describe** — `describe.rs`
- `describe_value_type()` — full ValueTypeDescribe with field-level metadata

### Added — Bulk API 2.0 (`busbar-sf-bulk`)

**Parallel Query Results** — `client.rs`
- `get_parallel_query_results()`, `get_all_query_results_parallel()`
- Fetches query result locators in parallel for large datasets

### Fixed

- Upsert now handles HTTP 200 (newer Salesforce API versions) in addition to 201/204
- Per-SObject process rules endpoint deserialization (array response, not map)
- External ID field excluded from upsert request body (URL path only)
- `InvocableActionResult.errors` handles `null` from Salesforce API
- `RunTestsSyncResult` tolerates missing fields across API versions
- OAuth token revocation tests handle parallel test interference
- Codecov coverage reports include all workspace crates (`--workspace`)
- CI job summaries show real coverage data across all crates
- Setup binary (`src/bin/`) excluded from patch coverage calculations

### Testing

- **411 unit tests** — wiremock-based HTTP tests for every client method, deserialization edge cases (null arrays, missing fields, dotted action names)
- **136 integration tests** — live Salesforce org tests covering every endpoint, no `#[ignore]`, no silent error swallowing
- **100% patch coverage** on all changed code (target: 80%)
- Scratch org setup binary (`src/bin/setup_scratch_org.rs`) for reproducible integration test environments

## [0.0.2]

### Added
- Reconfigured release workflows and information for better publishing on crates.io and docs.rs
- Comprehensive code review documentation (CODE_REVIEW.md)
- Security policy and best practices (SECURITY.md)
- Example programs demonstrating API usage:
  - `basic_auth.rs` - Authentication methods (OAuth, JWT, SFDX, environment)
  - `rest_crud.rs` - REST API CRUD operations
  - `queries.rs` - SOQL queries with security best practices
  - `error_handling.rs` - Error handling patterns and retry logic
  - `bulk_operations.rs` - Bulk API 2.0 operations
- This CHANGELOG file

## [0.0.1] - 2026-01-27

### Added
- **Core Infrastructure** (`busbar-sf-client`)
  - HTTP client with automatic retry logic
  - Exponential backoff for transient errors
  - Rate limit detection and handling
  - Comprehensive error types with context
  - Security utilities for injection prevention:
    - SOQL injection prevention (escape_string, escape_like, field validation)
    - URL parameter encoding
    - XML escaping for Metadata API
    - Salesforce ID validation
  - Request/response tracing with sensitive data redaction

- **Authentication** (`busbar-sf-auth`)
  - OAuth 2.0 Web Server Flow
  - OAuth 2.0 JWT Bearer Flow for server-to-server
  - OAuth token refresh
  - OAuth token validation and revocation
  - Salesforce CLI (SFDX) integration
  - Environment variable configuration
  - Secure credential storage with file permissions
  - Automatic credential redaction in logs/debug output

- **REST API** (`busbar-sf-rest`)
  - CRUD operations (Create, Read, Update, Delete, Upsert)
  - SOQL query execution with automatic pagination
  - SOSL search
  - Describe operations (global and per-SObject)
  - Composite API for batching requests
  - SObject Collections (bulk operations up to 200 records)
  - API limits checking
  - API version discovery

- **Bulk API 2.0** (`busbar-sf-bulk`)
  - Ingest jobs (Insert, Update, Upsert, Delete, HardDelete)
  - Query jobs for large data extraction
  - CSV data upload and download
  - Job monitoring with configurable polling
  - Automatic job completion waiting
  - Success/failure/unprocessed results retrieval
  - High-level convenience methods

- **Metadata API** (`busbar-sf-metadata`)
  - Metadata describe operations
  - Metadata list operations
  - Deploy operations
  - Retrieve operations
  - SOAP envelope handling
  - ZIP file processing for metadata packages

- **Tooling API** (`busbar-sf-tooling`)
  - Apex execution (anonymous and named)
  - Debug log management
  - Code coverage operations
  - Query execution on tooling objects

### Security
- All sensitive credentials automatically redacted in Debug output
- Tracing/logging excludes authentication parameters
- SOQL injection prevention utilities with comprehensive tests
- URL parameter encoding to prevent path traversal
- XML escaping for SOAP APIs
- Salesforce ID format validation
- Field name validation
- Secure token storage with restrictive file permissions (Unix: 0o600)
- POST with body for token operations (not GET with query params)

### Documentation
- Comprehensive README with examples
- API documentation for all public types and methods
- Security best practices guide
- Contribution guidelines
- Dual licensing (MIT / Apache 2.0)

### Dependencies
- `tokio` 1.40 - Async runtime
- `reqwest` 0.13 - HTTP client with rustls
- `serde` 1.0 - Serialization framework
- `serde_json` 1.0 - JSON handling
- `tracing` 0.1 - Structured logging
- `thiserror` 2.0 - Error handling
- `chrono` 0.4 - Date/time handling
- `jsonwebtoken` 9.3 - JWT creation
- `quick-xml` 0.36 - XML parsing for SOAP
- `csv` 1.3 - CSV handling for Bulk API

[Unreleased]: https://github.com/composable-delivery/busbar-sf-api/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/composable-delivery/busbar-sf-api/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/composable-delivery/busbar-sf-api/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/composable-delivery/busbar-sf-api/releases/tag/v0.0.1
