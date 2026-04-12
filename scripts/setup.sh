#!/usr/bin/env bash
# =============================================================================
# Twill Network — Development Environment Setup
# =============================================================================
#
# Installs all prerequisites and builds the Twill node.
# Run once on a fresh machine.
# =============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Twill Network — Setup${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Darwin) PLATFORM="macOS" ;;
    Linux)  PLATFORM="Linux" ;;
    *)      echo -e "${RED}Unsupported OS: $OS${NC}"; exit 1 ;;
esac
echo -e "Platform: ${GREEN}${PLATFORM}${NC}"

# 1. Install Rust
echo ""
echo -e "${YELLOW}Step 1: Rust toolchain${NC}"
if command -v rustup &> /dev/null; then
    echo "  Rust already installed: $(rustc --version)"
    rustup update stable
    rustup default stable
else
    echo "  Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
fi

# Add WASM target
echo "  Adding wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown

# 2. Install system dependencies
echo ""
echo -e "${YELLOW}Step 2: System dependencies${NC}"
if [ "$PLATFORM" = "macOS" ]; then
    if ! command -v brew &> /dev/null; then
        echo -e "${RED}  Homebrew not found. Install from https://brew.sh${NC}"
        exit 1
    fi
    echo "  Installing via Homebrew..."
    brew install cmake pkg-config openssl protobuf 2>/dev/null || true
elif [ "$PLATFORM" = "Linux" ]; then
    echo "  Installing via apt..."
    sudo apt-get update -qq
    sudo apt-get install -y -qq \
        cmake pkg-config libssl-dev git gcc build-essential \
        clang libclang-dev protobuf-compiler
fi

# 3. Install subkey (for key generation)
echo ""
echo -e "${YELLOW}Step 3: Installing subkey${NC}"
if command -v subkey &> /dev/null; then
    echo "  subkey already installed"
else
    echo "  Building subkey from source (this takes a few minutes)..."
    cargo install subkey --git https://github.com/paritytech/polkadot-sdk --force
fi

# 4. Build Twill
echo ""
echo -e "${YELLOW}Step 4: Building Twill node${NC}"
echo "  This will take 10-30 minutes on first build..."
echo ""
cargo build --release 2>&1 | tail -5

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Setup Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Next steps:"
echo "  1. Generate keys:    ./scripts/generate-keys.sh"
echo "  2. Run dev node:     ./target/release/twill --dev"
echo "  3. Run tests:        cargo test --all"
echo ""
echo "Dev node RPC:  http://localhost:9944"
echo "Polkadot.js:   https://polkadot.js.org/apps/?rpc=ws://localhost:9944"
echo ""
