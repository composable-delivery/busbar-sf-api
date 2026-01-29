# busbar-sf-client

Core HTTP client infrastructure shared by the Salesforce API crates in this repository (retry, compression, rate limiting primitives, request/response wiring).

This crate is part of the **busbar-sf-api** workspace.

- Prefer the facade crate for most usage: https://crates.io/crates/busbar-sf-api
- Docs: https://docs.rs/busbar-sf-client
- Repo: https://github.com/composable-delivery/busbar-sf-api

## When to use this crate directly

Use `busbar-sf-client` if you’re building your own Salesforce API surface but want to reuse the HTTP + retry foundation.
