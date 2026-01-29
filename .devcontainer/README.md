# Devcontainer / Codespaces

This repo is intended to work out-of-the-box in GitHub Codespaces.

## What you should have

- `cargo` and `rustup` available in the terminal
- `rustfmt` and `clippy` installed (via `rustup component add clippy rustfmt`)

## If you opened a Codespace before this change

Dev containers cache images. Rebuild to pick up the fixed image:

- VS Code Command Palette: **Dev Containers: Rebuild Container**

Then verify:

- `cargo --version`
- `rustup show`

## Why this exists

The dev container image should be a *development environment* only.
We intentionally avoid building the Rust workspace at image build-time because:

- Workspace members live under `crates/` and must be present to compile.
- Image builds should be fast and deterministic; compilation belongs in CI and local commands.
