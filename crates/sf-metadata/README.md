# busbar-sf-metadata

Salesforce Metadata API client (deploy/retrieve and related operations).

This crate is part of the **busbar-sf-api** workspace.

- Prefer the facade crate for most usage: https://crates.io/crates/busbar-sf-api
- Docs: https://docs.rs/busbar-sf-metadata
- Repo: https://github.com/composable-delivery/busbar-sf-api

## When to use this crate directly

Use `busbar-sf-metadata` if you only need metadata deploy/retrieve without pulling in the other APIs.

## Optional Features

### `typed` - Typed Metadata Operations

Enable the `typed` feature to use fully-typed metadata structures from `busbar-sf-types`:

```toml
[dependencies]
busbar-sf-metadata = { version = "0.0.3", features = ["typed"] }
busbar-sf-types = "0.0.1"
```

This enables the `TypedMetadataExt` trait for type-safe deploy operations with automatic packaging:

```rust
use busbar_sf_metadata::{MetadataClient, TypedMetadataExt, DeployOptions};
use busbar_sf_types::metadata::objects::CustomObject;

let obj = CustomObject {
    full_name: Some("MyObject__c".to_string()),
    label: Some("My Object".to_string()),
    ..Default::default()
};

let async_id = client.deploy_typed(&obj, DeployOptions::default()).await?;
```
