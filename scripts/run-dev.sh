#!/usr/bin/env bash
set -euo pipefail

echo "Starting Twill development node..."
echo ""
echo "Chain: Twill Development"
echo "Token: TWL"
echo "Block Time: 6 seconds"
echo "Consensus: PoC + PoSe (dev mode: instant seal)"
echo ""

./target/release/twill \
    --dev \
    --tmp \
    --rpc-cors all \
    --rpc-methods unsafe \
    --rpc-port 9944 \
    --port 30333

# Development flags:
#   --dev          Use dev chain spec with pre-funded accounts
#   --tmp          Use temporary storage (purged on restart)
#   --rpc-cors all Allow all CORS origins for development
#   --rpc-port     WebSocket/HTTP RPC on port 9944
#   --port         P2P networking on port 30333
