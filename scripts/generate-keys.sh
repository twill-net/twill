#!/usr/bin/env bash
# =============================================================================
# Twill Network — Key Generation Script
# =============================================================================
#
# Generates a keypair for participating in the Twill Network.
# Store the output SECURELY.
#
# No authority keys. No Aura. No GRANDPA. Permissionless from genesis.
#
# Requirements: subkey (install via `cargo install subkey`)
#
# Output: JSON file with public key + encrypted seed phrase
# =============================================================================

set -euo pipefail

KEYS_DIR="${1:-./keys}"
NETWORK_ID="twill"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="${KEYS_DIR}/twill-keys-${TIMESTAMP}.json"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Twill Network Key Generation${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check for subkey
if ! command -v subkey &> /dev/null; then
    echo -e "${RED}Error: 'subkey' not found.${NC}"
    echo "Install it with: cargo install --force subkey --git https://github.com/paritytech/polkadot-sdk"
    exit 1
fi

mkdir -p "$KEYS_DIR"
chmod 700 "$KEYS_DIR"

echo -e "${YELLOW}WARNING: This will generate a private key.${NC}"
echo -e "${YELLOW}Store the output file in a secure, encrypted location.${NC}"
echo -e "${YELLOW}Never share seed phrases. Never commit them to git.${NC}"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo "Generating key..."
echo ""

# Function to generate a key and extract info
generate_key() {
    local PURPOSE=$1
    local SCHEME=${2:-sr25519}

    echo -e "  ${GREEN}→${NC} Generating ${PURPOSE} key (${SCHEME})..."

    local OUTPUT
    OUTPUT=$(subkey generate --scheme "$SCHEME" --network "$NETWORK_ID" 2>&1)

    local SEED
    SEED=$(echo "$OUTPUT" | grep "Secret phrase:" | sed 's/.*Secret phrase: *//')

    local PUBLIC_KEY
    PUBLIC_KEY=$(echo "$OUTPUT" | grep "Public key (hex):" | sed 's/.*Public key (hex): *//')

    local SS58
    SS58=$(echo "$OUTPUT" | grep "SS58 Address:" | sed 's/.*SS58 Address: *//')

    local ACCOUNT_ID
    ACCOUNT_ID=$(echo "$OUTPUT" | grep "Account ID:" | sed 's/.*Account ID: *//')

    echo "    Seed: ${SEED:0:12}... (truncated for display)"
    echo "    Public: ${PUBLIC_KEY}"
    echo "    SS58: ${SS58}"
    echo ""

    # Return as JSON fragment
    cat <<KEYJSON
    {
      "purpose": "${PURPOSE}",
      "scheme": "${SCHEME}",
      "seed_phrase": "${SEED}",
      "public_key_hex": "${PUBLIC_KEY}",
      "account_id": "${ACCOUNT_ID}",
      "ss58_address": "${SS58}"
    }
KEYJSON
}

# Generate account key
ACCOUNT=$(generate_key "account" "sr25519")

# Write to file
{
echo "{"
echo '  "generated_at": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'",'
echo '  "network": "twill",'
echo '  "keys": ['
echo "${ACCOUNT}"
echo "  ]"
echo "}"
} > "$OUTPUT_FILE"

chmod 600 "$OUTPUT_FILE"

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Keys written to: ${OUTPUT_FILE}${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${RED}CRITICAL SECURITY STEPS:${NC}"
echo "  1. Copy ${OUTPUT_FILE} to an encrypted USB drive"
echo "  2. Delete it from this machine: rm -f ${OUTPUT_FILE}"
echo "  3. Store the USB in a physical safe"
echo "  4. Write down the seed phrase on paper as backup"
echo "  5. Never store seed phrases digitally long-term"
echo ""
echo -e "${YELLOW}No authority keys needed.${NC}"
echo "  Twill uses permissionless block production."
echo "  No Aura. No GRANDPA. Tethered to no one."
echo ""
echo -e "${GREEN}Done.${NC}"
