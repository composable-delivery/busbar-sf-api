# busbar-sf-auth

Salesforce authentication helpers (OAuth 2.0 flows, JWT Bearer, credentials management).

This crate is part of the **busbar-sf-api** workspace.

- Prefer the facade crate for most usage: https://crates.io/crates/busbar-sf-api
- Docs: https://docs.rs/busbar-sf-auth
- Repo: https://github.com/composable-delivery/busbar-sf-api

## When to use this crate directly

Use `busbar-sf-auth` if you only need auth/token acquisition and will call Salesforce endpoints yourself.
