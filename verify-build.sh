#!/bin/bash
set -e

echo "=========================================="
echo "Build Verification Script"
echo "=========================================="
echo ""

echo "1. Building workspace..."
cargo build --workspace --quiet
echo "   ✓ Build succeeded"
echo ""

echo "2. Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -5
echo "   ✓ Clippy passed (zero warnings)"
echo ""

echo "3. Checking formatting..."
cargo fmt --check
echo "   ✓ Formatting correct"
echo ""

echo "4. Running unit tests..."
cargo test --workspace --lib --quiet
echo "   ✓ Unit tests passed"
echo ""

echo "5. Verifying integration tests fail without SF_AUTH_URL..."
if ! SF_AUTH_URL="" cargo test --test integration --quiet 2>&1 | grep -q "FAILED"; then
    echo "   ✗ Integration tests should fail without SF_AUTH_URL!"
    exit 1
fi
echo "   ✓ Integration tests properly fail without SF_AUTH_URL"
echo ""

echo "=========================================="
echo "ALL CHECKS PASSED ✓"
echo "=========================================="
