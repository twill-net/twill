#!/usr/bin/env bash
# Run the Twill Network bootstrap node.
# The bootnode is the first entry point for new nodes joining the network.
# It does NOT mine, stake, or hold funds. It only provides peer discovery.
#
# Setup:
#   1. cargo build --release
#   2. Generate a stable node key (same key every time, so peer ID stays constant):
#      ./target/release/twill key generate-node-key --file bootnode-key
#   3. Get your peer ID (share this publicly):
#      ./target/release/twill key inspect-node-key --file bootnode-key
#   4. Publish: /ip4/YOUR_PUBLIC_IP/tcp/30333/p2p/YOUR_PEER_ID
#
# Usage: ./scripts/run-bootnode.sh

set -euo pipefail

BINARY="${BINARY:-./target/release/twill}"
CHAIN_SPEC="${CHAIN_SPEC:-mainnet-raw.json}"
BASE_PATH="${BASE_PATH:-$HOME/.twill-bootnode}"
NODE_KEY_FILE="${NODE_KEY_FILE:-bootnode-key}"
P2P_PORT="${P2P_PORT:-30333}"

if [ ! -f "$BINARY" ]; then
    echo "Binary not found. Build with: cargo build --release"
    exit 1
fi

if [ ! -f "$NODE_KEY_FILE" ]; then
    echo "Node key not found: $NODE_KEY_FILE"
    echo "Generate with: $BINARY key generate-node-key --file $NODE_KEY_FILE"
    exit 1
fi

PEER_ID=$("$BINARY" key inspect-node-key --file "$NODE_KEY_FILE")
echo "Bootnode starting..."
echo "  Peer ID: $PEER_ID"
echo "  Announce: /ip4/\$(curl -s ifconfig.me)/tcp/$P2P_PORT/p2p/$PEER_ID"
echo ""

exec "$BINARY" \
    --chain "$CHAIN_SPEC" \
    --base-path "$BASE_PATH" \
    --node-key-file "$NODE_KEY_FILE" \
    --port "$P2P_PORT" \
    --no-telemetry \
    --no-prometheus \
    --listen-addr "/ip4/0.0.0.0/tcp/$P2P_PORT" \
    "$@"
