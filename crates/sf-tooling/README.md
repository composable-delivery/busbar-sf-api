# busbar-sf-tooling

Salesforce Tooling API client (Apex operations, debug logs, code coverage, etc.).

This crate is part of the **busbar-sf-api** workspace.

- Prefer the facade crate for most usage: https://crates.io/crates/busbar-sf-api
- Docs: https://docs.rs/busbar-sf-tooling
- Repo: https://github.com/composable-delivery/busbar-sf-api

## When to use this crate directly

Use `busbar-sf-tooling` if you only need Tooling endpoints without the broader API surface.

## Optional Features

- **`dependencies`** - Enables support for MetadataComponentDependency (Beta) queries. Limited to 2000 records per query.
  - Enable with: `busbar-sf-tooling = { version = "0.0.2", features = ["dependencies"] }`
  - For larger queries (up to 100,000 records), use the Bulk API instead.
