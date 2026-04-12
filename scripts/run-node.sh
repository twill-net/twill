#!/usr/bin/env bash
# Run a Twill Network node.
#
# First-time setup:
#   1. Build: cargo build --release
#   2. Generate your node key: ./target/release/twill key generate-node-key --file node-key
#   3. Get your peer ID: ./target/release/twill key inspect-node-key --file node-key
#   4. Share your IP + peer ID on the forum so others can connect
#
# Usage:
#   ./scripts/run-node.sh                          # Connect to mainnet (needs bootnodes)
#   ./scripts/run-node.sh --bootnodes BOOTNODE_URL # Connect via a specific bootnode
#   BOOTNODE=... ./scripts/run-node.sh             # Set via environment variable
#
# Bootnode URL format: /ip4/IP/tcp/30333/p2p/PEER_ID

set -euo pipefail

BINARY="${BINARY:-./target/release/twill}"
CHAIN_SPEC="${CHAIN_SPEC:-mainnet-raw.json}"
BASE_PATH="${BASE_PATH:-$HOME/.twill}"
RPC_PORT="${RPC_PORT:-9944}"
P2P_PORT="${P2P_PORT:-30333}"
BOOTNODE="${BOOTNODE:-}"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Build with: cargo build --release"
    exit 1
fi

if [ ! -f "$CHAIN_SPEC" ]; then
    echo "Chain spec not found: $CHAIN_SPEC"
    echo "Either:"
    echo "  - Download mainnet-raw.json from the Twill GitHub"
    echo "  - Or run: ./scripts/build-mainnet-spec.sh"
    exit 1
fi

BOOTNODE_ARG=""
if [ -n "$BOOTNODE" ]; then
    BOOTNODE_ARG="--bootnodes $BOOTNODE"
fi

echo "Starting Twill Network node..."
echo "  Chain: $CHAIN_SPEC"
echo "  Data: $BASE_PATH"
echo "  RPC: ws://0.0.0.0:$RPC_PORT"
echo "  P2P: 0.0.0.0:$P2P_PORT"
echo ""

exec "$BINARY" \
    --chain "$CHAIN_SPEC" \
    --base-path "$BASE_PATH" \
    --port "$P2P_PORT" \
    --rpc-port "$RPC_PORT" \
    --rpc-cors all \
    --rpc-methods unsafe \
    --name "twill-node-$(hostname)" \
    $BOOTNODE_ARG \
    "$@"
