#!/usr/bin/env bash
# Twill Network — start a full node (miner or peer)
#
# Usage:
#   bash scripts/start-node.sh
#
# Chain data persists at ./chain-data/ — never delete that folder.

set -e

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT/target/release/twill"
DATA_DIR="$ROOT/chain-data"
CHAINSPEC="$ROOT/chainspec-raw.json"
NODE_KEY="$ROOT/node.key"

if [ ! -f "$BINARY" ]; then
  echo "ERROR: node binary not found. Run: cargo build --release"
  exit 1
fi

if [ ! -f "$CHAINSPEC" ]; then
  echo "ERROR: chainspec-raw.json not found."
  exit 1
fi

mkdir -p "$DATA_DIR"

echo "Starting Twill node..."
echo "  Binary:     $BINARY"
echo "  Chainspec:  $CHAINSPEC"
echo "  Chain data: $DATA_DIR"
echo "  RPC:        ws://127.0.0.1:9944"
echo ""
echo "Press Ctrl+C to stop. Chain data is safe in ./chain-data/"
echo ""

# If a node.key exists, use it so peer ID stays consistent across restarts
NODE_KEY_ARG=""
if [ -f "$NODE_KEY" ]; then
  NODE_KEY_ARG="--node-key-file $NODE_KEY"
fi

exec "$BINARY" \
  --base-path "$DATA_DIR" \
  --chain "$CHAINSPEC" \
  $NODE_KEY_ARG \
  --rpc-external \
  --rpc-port 9944 \
  --rpc-cors all \
  --no-telemetry \
  --no-prometheus
