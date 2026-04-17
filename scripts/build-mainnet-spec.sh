#!/usr/bin/env bash
# Build the mainnet raw chain spec.
# Run this ONCE before launch. The output file is the canonical genesis.
# All nodes on the network must use the same raw spec.
#
# Usage: ./scripts/build-mainnet-spec.sh
# Output: mainnet-raw.json (commit this to the repo before launch)

set -euo pipefail

BINARY="${1:-./target/release/twill}"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found at $BINARY"
    echo "Build first: cargo build --release"
    exit 1
fi

echo "Building mainnet raw chain spec..."
"$BINARY" build-spec --chain mainnet --disable-default-bootnode > mainnet.json
"$BINARY" build-spec --chain mainnet --raw --disable-default-bootnode > mainnet-raw.json

echo ""
echo "Done."
echo "  Human-readable: mainnet.json"
echo "  Raw (canonical): mainnet-raw.json"
echo ""
echo "Commit mainnet-raw.json to the repository."
echo "All nodes MUST use the same raw spec file."
