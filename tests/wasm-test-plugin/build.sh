#!/usr/bin/env bash
# Build script for the WASM test plugin
#
# This script must be run from the repository root:
#   bash tests/wasm-test-plugin/build.sh
#
# This script works around workspace inheritance issues by temporarily
# creating a patched version of the guest SDK Cargo.toml for building.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_ROOT"

echo "Building WASM test plugin..."
echo "Repository root: $REPO_ROOT"

# Ensure wasm32-unknown-unknown target is installed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Create a temporary patched guest SDK Cargo.toml
GUEST_SDK_DIR="$REPO_ROOT/crates/sf-guest-sdk"
GUEST_SDK_TOML="$GUEST_SDK_DIR/Cargo.toml"
GUEST_SDK_BACKUP="$GUEST_SDK_DIR/Cargo.toml.backup"

# Backup original
cp "$GUEST_SDK_TOML" "$GUEST_SDK_BACKUP"

# Create patched version with explicit values instead of workspace inheritance
cat > "$GUEST_SDK_TOML" << 'TOML_EOF'
[package]
name = "busbar-sf-guest-sdk"
version = "0.0.3"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/composable-delivery/busbar-sf-api"
rust-version = "1.88"
description = "Guest SDK for building WASM plugins that interact with Salesforce via the bridge"

# Exclude from workspace - compiles to wasm32-unknown-unknown only
[workspace]

[dependencies]
busbar-sf-wasm-types = { version = "0.0.3", path = "../sf-wasm-types" }
extism-pdk = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rmp-serde = "1"
TOML_EOF

# Build the plugin
echo "Building with patched guest SDK..."
cargo build \
    --manifest-path tests/wasm-test-plugin/Cargo.toml \
    --target wasm32-unknown-unknown \
    --release

# Restore original Cargo.toml
mv "$GUEST_SDK_BACKUP" "$GUEST_SDK_TOML"

WASM_FILE="$REPO_ROOT/target/wasm32-unknown-unknown/release/wasm_test_plugin.wasm"

if [ -f "$WASM_FILE" ]; then
    WASM_SIZE=$(du -h "$WASM_FILE" | cut -f1)
    echo "✓ WASM plugin built successfully"
    echo "  Location: $WASM_FILE"
    echo "  Size: $WASM_SIZE"
else
    echo "✗ WASM plugin build failed - file not found at $WASM_FILE"
    mv "$GUEST_SDK_BACKUP" "$GUEST_SDK_TOML" 2>/dev/null || true
    exit 1
fi
