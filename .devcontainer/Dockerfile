# Dockerfile
# Use the official Dev Containers Rust image so Codespaces comes with:
# - rustup/cargo preinstalled
# - a non-root user
# - sensible defaults for VS Code + rust-analyzer
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Dev container images should not try to build the project at image-build time.
# Codespaces will mount the repository into /workspaces/<repo>.
WORKDIR /workspaces/busbar-sf-api

# Optional: preinstall common components (kept in postCreateCommand too for safety)
RUN rustup component add clippy rustfmt
