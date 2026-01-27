# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive code review documentation (CODE_REVIEW.md)
- Security policy and best practices (SECURITY.md)
- Example programs demonstrating API usage:
  - `basic_auth.rs` - Authentication methods (OAuth, JWT, SFDX, environment)
  - `rest_crud.rs` - REST API CRUD operations
  - `queries.rs` - SOQL queries with security best practices
  - `error_handling.rs` - Error handling patterns and retry logic
  - `bulk_operations.rs` - Bulk API 2.0 operations
- This CHANGELOG file

## [0.1.0] - 2026-01-27

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

[Unreleased]: https://github.com/composable-delivery/busbar-sf-api/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/composable-delivery/busbar-sf-api/releases/tag/v0.1.0
