# Contributing to busbar-sf-api

Thank you for your interest in contributing to busbar-sf-api! We welcome contributions from the community.

## Code of Conduct

This project adheres to a code of conduct that we expect all contributors to follow. Please be respectful and constructive in your interactions.

## How to Contribute

### Reporting Issues

If you find a bug or have a feature request:

1. Check if the issue already exists in the [issue tracker](https://github.com/composable-delivery/busbar-sf-api/issues)
2. If not, create a new issue with:
   - A clear, descriptive title
   - Detailed description of the problem or feature
   - Steps to reproduce (for bugs)
   - Expected vs. actual behavior
   - Rust version and platform information

### Discussing Ideas

For questions, ideas, or general discussion:

- Use [GitHub Discussions](https://github.com/composable-delivery/busbar-sf-api/discussions)
- Check existing discussions before creating a new one
- Use appropriate categories (Q&A, Ideas, Show and Tell, etc.)

### Submitting Pull Requests

1. **Fork the repository** and create your branch from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**:
   - Follow the existing code style
   - Write clear, concise commit messages
   - Add tests for new functionality
   - Update documentation as needed

3. **Test your changes**:
   ```bash
   # Run all tests
   cargo test --workspace

   # Run clippy (linter)
   cargo clippy --workspace -- -D warnings

   # Format code
   cargo fmt --workspace

   # Check formatting
   cargo fmt --workspace --check
   ```

4. **Create a pull request**:
   - Provide a clear description of the changes
   - Reference any related issues
   - Ensure all CI checks pass

## Development Setup

### Prerequisites

- Rust 1.88 or later (MSRV)
- Cargo (comes with Rust)
- Git

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/busbar-sf-api.git
cd busbar-sf-api

# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p busbar-sf-auth
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run all tests (integration tests auto-skip if env is missing)
cargo test --workspace

# Run tests for specific crate
cargo test -p busbar-sf-rest

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

### Integration Tests (Real Org)

Integration tests require a real org and use `SF_AUTH_URL`:

```bash
# Run all integration tests
SF_AUTH_URL=your_auth_url \
   cargo test --test integration -- --nocapture

# Run a specific integration module (REST/Bulk/Tooling/Metadata/Examples/Scratch)
SF_AUTH_URL=your_auth_url \
   cargo test --test integration rest:: -- --nocapture
```

### Coverage

We use `cargo-llvm-cov` for coverage.

```bash
# Install coverage tooling
cargo install cargo-llvm-cov

# Unit test coverage (all crates)
mkdir -p coverage
cargo llvm-cov --workspace --all-features --lcov --output-path coverage/lcov.info

# Integration test coverage (real org)
SF_AUTH_URL=your_auth_url \
   cargo llvm-cov --workspace --all-features --test integration --lcov \
   --output-path coverage/lcov.info -- --nocapture

# Human-readable summary
cargo llvm-cov report --summary-only
```

### Linting and Formatting

We use `rustfmt` for code formatting and `clippy` for linting:

```bash
# Format all code
cargo fmt --workspace

# Check formatting without making changes
cargo fmt --workspace --check

# Run clippy
cargo clippy --workspace

# Run clippy with all warnings as errors
cargo clippy --workspace -- -D warnings
```

### Pre-commit Hooks (Recommended)

We provide pre-commit hooks to automatically check formatting and run clippy before each commit:

```bash
# Install pre-commit (if not already installed)
# On macOS/Linux:
pip install pre-commit
# Or with Homebrew:
brew install pre-commit
# Or with pipx:
pipx install pre-commit

# Install the git hooks
pre-commit install

# Run manually on all files (optional)
pre-commit run --all-files
```

Once installed, the hooks will automatically run on staged files before each commit. This helps catch issues early and ensures consistent code quality.

## Code Style

- Follow Rust naming conventions
- Use `rustfmt` for formatting (automatic via CI)
- Pass `clippy` lints without warnings
- Write documentation comments (`///`) for public APIs
- Include examples in documentation where helpful

## Testing Guidelines

- Write unit tests for all new functionality
- Add integration tests for complex workflows
- Use descriptive test names that explain what is being tested
- Mock external dependencies when appropriate
- Ensure tests are deterministic and don't depend on external state

Example test structure:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = "test data";
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }

    #[tokio::test]
    async fn test_async_feature() {
        // Test async code
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

## Documentation

- Document all public APIs with `///` comments
- Include examples in documentation
- Update README.md for significant changes
- Keep documentation up-to-date with code changes

Example documentation:

```rust
/// Retrieves an account by ID from Salesforce.
///
/// # Arguments
///
/// * `account_id` - The Salesforce ID of the account to retrieve
///
/// # Returns
///
/// Returns `Result<Account, Error>` containing the account data or an error.
///
/// # Example
///
/// ```no_run
/// # use busbar_sf_rest::SalesforceRestClient;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = SalesforceRestClient::new("instance_url", "token")?;
/// let account = client.get_account("001xx000003DXXXAAA").await?;
/// # Ok(())
/// # }
/// ```
pub async fn get_account(&self, account_id: &str) -> Result<Account, Error> {
    // Implementation
}
```

## Commit Messages

Write clear, descriptive commit messages:

- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Move cursor to..." not "Moves cursor to...")
- Limit first line to 72 characters
- Reference issues and PRs when relevant

Example:
```
Add OAuth refresh token support

- Implement automatic token refresh
- Add retry logic for expired tokens
- Update documentation with examples

Fixes #123
```

## Pull Request Process

1. Update documentation for any changed functionality
2. Add tests for new features
3. Ensure all tests pass and code is formatted
4. Update CHANGELOG.md if applicable
5. Request review from maintainers
6. Address review feedback
7. Squash commits if requested

## Release Process

Releases are handled by maintainers:

1. Update version in `Cargo.toml`
2. Update CHANGELOG.md
3. Create a git tag
4. Publish to crates.io
5. Create GitHub release

## Questions?

If you have questions about contributing:

- Check existing [GitHub Discussions](https://github.com/composable-delivery/busbar-sf-api/discussions)
- Open a new discussion in the Q&A category
- Review closed issues and PRs for similar questions

## License

By contributing, you agree that your contributions will be licensed under both the MIT License and Apache License 2.0, consistent with the project's dual licensing.
