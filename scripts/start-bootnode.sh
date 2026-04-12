#!/usr/bin/env bash
# Twill Network — Bootnode
# Run this on your public server so miners can find the network.
#
# Your permanent peer ID: 12D3KooWMWk4E5sP3fwduagbikEA1YRKtZqwLgb2tQ2K9WpkvydV
#
# Once this is running, update chainspec.json bootNodes to:
#   /ip4/YOUR_PUBLIC_IP/tcp/30333/p2p/12D3KooWMWk4E5sP3fwduagbikEA1YRKtZqwLgb2tQ2K9WpkvydV
# Then rebuild chainspec-raw.json and push to git.
#
# Usage on your server:
#   1. Copy this repo + node.key to the server
#   2. bash scripts/start-bootnode.sh
#   Firewall: open TCP port 30333

set -e

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT/target/release/twill"
DATA_DIR="$ROOT/bootnode-data"
CHAINSPEC="$ROOT/chainspec-raw.json"
NODE_KEY="$ROOT/node.key"

if [ ! -f "$BINARY" ]; then
  echo "ERROR: binary not found. Run: cargo build --release"
  exit 1
fi

mkdir -p "$DATA_DIR"

echo "Starting Twill bootnode..."
echo "  Peer ID: 12D3KooWMWk4E5sP3fwduagbikEA1YRKtZqwLgb2tQ2K9WpkvydV"
echo "  Port:    30333 (must be open on firewall)"
echo ""

exec "$BINARY" \
  --base-path "$DATA_DIR" \
  --chain "$CHAINSPEC" \
  --node-key-file "$NODE_KEY" \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --no-telemetry \
  --no-prometheus \
  --listen-addr /ip4/0.0.0.0/tcp/30333
