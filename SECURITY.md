# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.0.x   | :white_check_mark: |
| < 0.0.2 | :x:                |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security vulnerability in busbar-sf-api, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via:

1. **GitHub Security Advisories** (Preferred)
   - Navigate to the [Security tab](https://github.com/composable-delivery/busbar-sf-api/security/advisories)
   - Click "Report a vulnerability"
   - Fill out the form with details

2. **Email** (Alternative)
   - Send an email to: security@muselab.com
   - Include "busbar-sf-api Security" in the subject line
   - Provide detailed information about the vulnerability

### What to Include

Please include the following information:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Suggested fix (if you have one)
- Your contact information for follow-up

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: Within 7 days
  - High: Within 30 days
  - Medium: Within 90 days
  - Low: Next regular release

### What to Expect

1. We will acknowledge your report within 48 hours
2. We will investigate and confirm the vulnerability
3. We will work on a fix and coordinate a release
4. We will credit you in the security advisory (unless you prefer to remain anonymous)
5. We will notify affected users once a fix is available

## Security Features

### Built-in Security Protections

This library includes several security features to protect your Salesforce integration:

#### 1. Injection Prevention

**SOQL Injection Protection:**
```rust
use busbar_sf_client::security::soql;

// CORRECT - Always escape user input
let name = soql::escape_string(user_input);
let query = format!("SELECT Id FROM Account WHERE Name = '{}'", name);

// WRONG - NEVER do this
// let query = format!("SELECT Id FROM Account WHERE Name = '{}'", user_input);
```

**Available Functions:**
- `soql::escape_string()` - Escape strings in SOQL
- `soql::escape_like()` - Escape LIKE pattern wildcards
- `soql::is_safe_field_name()` - Validate field names
- `soql::filter_safe_fields()` - Filter unsafe field names
- `url::encode_param()` - URL-encode parameters
- `url::is_valid_salesforce_id()` - Validate Salesforce IDs
- `xml::escape()` - Escape XML content

#### 2. Credential Protection

**Sensitive Data Redaction:**
- All credentials are automatically redacted in `Debug` output
- Tokens are never logged by tracing/logging
- Authentication parameters are excluded from logs using `#[instrument(skip(...))]`

**Example:**
```rust
let creds = SalesforceCredentials::new(
    "https://na1.salesforce.com",
    "super_secret_token",
    "62.0"
);

println!("{:?}", creds);
// Output: SalesforceCredentials { instance_url: "...", access_token: "[REDACTED]", ... }
```

**Secure Token Storage:**
- File-based token storage uses restrictive permissions (Unix: 0o600)
- Storage location: `~/.sf-api/tokens/`
- Keys are sanitized before use as filenames

#### 3. Input Validation

**Automatic Validation:**
- Salesforce IDs: Must be 15 or 18 characters, alphanumeric only
- Field names: Must start with letter, contain only alphanumerics and underscores
- SObject names: Same validation as field names
- URL parameters: Automatically encoded to prevent path traversal

#### 4. Secure Communication

**TLS/SSL:**
- All communication with Salesforce uses HTTPS
- Uses rustls with platform-specific certificate verification
- HTTP/2 support for improved performance and security

**Token Handling:**
- Token validation uses POST with body (not GET with query params)
- Prevents tokens from appearing in server logs
- OAuth operations use proper content-type headers

## Security Best Practices

### 1. Credential Management

**DO:**
- ✅ Use environment variables for credentials in production
- ✅ Use credential storage with restrictive permissions
- ✅ Rotate access tokens regularly
- ✅ Use JWT Bearer flow for server-to-server integration
- ✅ Store private keys in secure key management systems

**DON'T:**
- ❌ Hard-code credentials in source code
- ❌ Commit credentials to version control
- ❌ Share credentials in log files or error messages
- ❌ Use the same credentials for dev/test/prod
- ❌ Store credentials in plain text files

### 2. SOQL Query Security

**DO:**
- ✅ Always escape user input using `soql::escape_string()`
- ✅ Validate field names using `soql::is_safe_field_name()`
- ✅ Use `soql::escape_like()` for LIKE patterns
- ✅ Consider building a query builder that escapes by default

**DON'T:**
- ❌ Concatenate user input directly into queries
- ❌ Trust client-provided field names without validation
- ❌ Use dynamic field selection without filtering

**Example:**
```rust
// Secure query implementation
pub async fn get_accounts_by_name(
    client: &SalesforceRestClient,
    user_provided_name: &str,
) -> Result<Vec<Account>> {
    let safe_name = soql::escape_string(user_provided_name);
    let query = format!(
        "SELECT Id, Name, Industry FROM Account WHERE Name = '{}'",
        safe_name
    );
    client.query_all(&query).await
}
```

### 3. Error Handling

**DO:**
- ✅ Handle authentication errors separately
- ✅ Implement proper retry logic for transient errors
- ✅ Log errors without exposing credentials
- ✅ Use structured error types

**DON'T:**
- ❌ Expose raw error messages to end users
- ❌ Include credentials in error messages
- ❌ Retry authentication errors indefinitely
- ❌ Ignore rate limit errors

### 4. API Usage

**DO:**
- ✅ Respect API rate limits
- ✅ Use Bulk API for large data operations
- ✅ Implement exponential backoff for retries
- ✅ Monitor API usage limits

**DON'T:**
- ❌ Make unnecessary API calls
- ❌ Ignore rate limit (429) responses
- ❌ Use REST API for operations > 2000 records
- ❌ Make parallel requests without rate limiting

### 5. OAuth 2.0 Security

**DO:**
- ✅ Use state parameter to prevent CSRF
- ✅ Validate redirect URIs
- ✅ Store refresh tokens securely
- ✅ Use PKCE for mobile/public clients
- ✅ Implement token refresh before expiration

**DON'T:**
- ❌ Use authorization code flow in public clients without PKCE
- ❌ Store tokens in browser localStorage
- ❌ Include tokens in URL parameters
- ❌ Share OAuth credentials across applications

### 6. Data Protection

**DO:**
- ✅ Encrypt sensitive data at rest
- ✅ Use field-level encryption when available
- ✅ Implement proper access controls
- ✅ Audit access to sensitive data
- ✅ Follow Salesforce Shield guidelines

**DON'T:**
- ❌ Store sensitive data in logs
- ❌ Cache sensitive data without encryption
- ❌ Expose sensitive data in debug output
- ❌ Transfer sensitive data over insecure channels

## Dependency Security

We actively monitor our dependencies for known vulnerabilities:

- **Automated Scanning**: GitHub Dependabot alerts enabled
- **Regular Updates**: Dependencies updated regularly
- **Minimal Dependencies**: We minimize third-party dependencies
- **Vetted Dependencies**: Only use well-maintained, trusted crates

### Current Security-Critical Dependencies

- `reqwest` - HTTP client with rustls for TLS
- `rustls` - Modern TLS implementation
- `jsonwebtoken` - JWT creation and validation
- `tokio` - Async runtime
- `serde` - Serialization framework

## Compliance

This library is designed to help you comply with:

- **GDPR** - Data protection and privacy
- **SOC 2** - Security controls
- **HIPAA** - Healthcare data protection (when properly configured)
- **PCI DSS** - Payment card data security

**Note:** Compliance is a shared responsibility. This library provides secure building blocks, but you must implement proper security controls in your application.

## Security Checklist for Production

Before deploying to production, ensure:

- [ ] All credentials are stored securely (not in code)
- [ ] Environment variables are properly secured
- [ ] User input is properly escaped in queries
- [ ] Error messages don't expose sensitive data
- [ ] Logging doesn't include credentials
- [ ] Rate limiting is implemented
- [ ] Retry logic includes exponential backoff
- [ ] TLS/SSL certificates are valid
- [ ] API access is properly restricted
- [ ] Monitoring and alerting are configured
- [ ] Incident response plan is in place
- [ ] Regular security audits are scheduled

## Security Updates

Subscribe to security updates:

1. **GitHub Watch** - Watch this repository for security advisories
2. **Release Notes** - Check CHANGELOG.md for security updates
3. **RSS Feed** - Subscribe to GitHub releases

## Additional Resources

- [Salesforce Security Best Practices](https://developer.salesforce.com/docs/atlas.en-us.securityImplGuide.meta/securityImplGuide/)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE/SANS Top 25](https://www.sans.org/top25-software-errors/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

## Contact

For security concerns, contact:
- Email: security@composable-delivery.com
- Security Advisories: https://github.com/composable-delivery/busbar-sf-api/security/advisories

## Acknowledgments

We thank the security researchers and community members who help keep busbar-sf-api secure.

### Hall of Fame

Contributors who have responsibly disclosed security vulnerabilities will be listed here (with their permission).

---

**Last Updated:** 2026-01-27
