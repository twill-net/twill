#!/usr/bin/env bash
set -euo pipefail

echo "Building Twill Network..."
echo ""

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "Error: Rust is not installed."
    echo "Install from: https://rustup.rs/"
    exit 1
fi

RUST_VERSION=$(rustc --version)
echo "Rust: $RUST_VERSION"

# Add wasm target if not present
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Adding wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build
echo ""
echo "Building in release mode..."
cargo build --release

echo ""
echo "Build complete."
echo "Binary: ./target/release/twill"
