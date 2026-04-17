#!/usr/bin/env bash
# Twill bootnode one-shot setup.
# Run as root on a fresh Debian/Ubuntu box.
# Usage: bash bootnode-setup.sh

set -euo pipefail

TWILL_USER="twill"
REPO="https://github.com/twill-net/twill"
CHAIN_DATA="/home/${TWILL_USER}/chain-data"
TWILL_DIR="/home/${TWILL_USER}/twill"
SERVICE="twill-bootnode"

# ── helpers ──────────────────────────────────────────────────────────────────

log() { echo "==> $*"; }

die() { echo "ERROR: $*" >&2; exit 1; }

# ── 1. system packages ───────────────────────────────────────────────────────

log "Updating package lists..."
apt-get update -qq

log "Installing build deps..."
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
  build-essential clang libclang-dev libssl-dev pkg-config \
  protobuf-compiler curl git ufw fail2ban

# ── 2. swap (avoids OOM during Rust link step on small boxes) ────────────────

if ! swapon --show | grep -q '/swapfile'; then
  log "Creating 4 GB swapfile..."
  fallocate -l 4G /swapfile
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

# ── 3. firewall ───────────────────────────────────────────────────────────────

log "Configuring firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp
ufw allow 30333/tcp
ufw --force enable
systemctl enable --now fail2ban

# ── 4. create twill user ──────────────────────────────────────────────────────

if ! id "${TWILL_USER}" &>/dev/null; then
  log "Creating user ${TWILL_USER}..."
  adduser --disabled-password --gecos "" "${TWILL_USER}"
fi

# Copy root's authorized_keys to twill if none exist yet
if [ ! -f "/home/${TWILL_USER}/.ssh/authorized_keys" ] && \
   [ -f "/root/.ssh/authorized_keys" ]; then
  log "Copying authorized_keys to ${TWILL_USER}..."
  mkdir -p "/home/${TWILL_USER}/.ssh"
  cp /root/.ssh/authorized_keys "/home/${TWILL_USER}/.ssh/authorized_keys"
  chown -R "${TWILL_USER}:${TWILL_USER}" "/home/${TWILL_USER}/.ssh"
  chmod 700 "/home/${TWILL_USER}/.ssh"
  chmod 600 "/home/${TWILL_USER}/.ssh/authorized_keys"
fi

# ── 5. install Rust ───────────────────────────────────────────────────────────

if [ ! -f "/home/${TWILL_USER}/.cargo/bin/cargo" ]; then
  log "Installing Rust for ${TWILL_USER}..."
  su - "${TWILL_USER}" -c \
    'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path'
fi

CARGO="/home/${TWILL_USER}/.cargo/bin/cargo"
RUSTUP="/home/${TWILL_USER}/.cargo/bin/rustup"

su - "${TWILL_USER}" -c "${RUSTUP} target add wasm32-unknown-unknown" || true

# ── 6. clone + build ──────────────────────────────────────────────────────────

if [ ! -d "${TWILL_DIR}" ]; then
  log "Cloning repo..."
  su - "${TWILL_USER}" -c "git clone ${REPO} ${TWILL_DIR}"
else
  log "Repo exists — pulling latest..."
  su - "${TWILL_USER}" -c "git -C ${TWILL_DIR} pull --ff-only"
fi

log "Building twill-node (this takes 30–90 min on a 2-vCPU box)..."
su - "${TWILL_USER}" -c \
  "cd ${TWILL_DIR} && ${CARGO} build --release -p twill-node 2>&1"

BINARY="${TWILL_DIR}/target/release/twill"
[ -f "${BINARY}" ] || die "Build failed — binary not found at ${BINARY}"

# ── 7. generate stable node key ───────────────────────────────────────────────

NETWORK_DIR="${CHAIN_DATA}/chains/mainnet/network"
KEY_FILE="${NETWORK_DIR}/secret_ed25519"

mkdir -p "${NETWORK_DIR}"
chown -R "${TWILL_USER}:${TWILL_USER}" "${CHAIN_DATA}"

if [ ! -f "${KEY_FILE}" ]; then
  log "Generating node key..."
  su - "${TWILL_USER}" -c \
    "${BINARY} key generate-node-key --file ${KEY_FILE}" \
    | tee /tmp/twill-peerid.txt
  chown "${TWILL_USER}:${TWILL_USER}" "${KEY_FILE}"
  chmod 600 "${KEY_FILE}"
else
  log "Node key already exists — skipping key generation."
  # Still print the PeerId for reference
  PEER_ID=$(su - "${TWILL_USER}" -c "${BINARY} key inspect-node-key --file ${KEY_FILE}" 2>&1 || true)
  echo "${PEER_ID}" | tee /tmp/twill-peerid.txt
fi

# ── 8. install systemd service ────────────────────────────────────────────────

log "Installing systemd service..."
MAINNET_SPEC="${TWILL_DIR}/mainnet-raw.json"
[ -f "${MAINNET_SPEC}" ] || die "mainnet-raw.json not found — did the build include it?"

cat > /etc/systemd/system/${SERVICE}.service <<EOF
[Unit]
Description=Twill bootnode
After=network-online.target
Wants=network-online.target

[Service]
User=${TWILL_USER}
Group=${TWILL_USER}
WorkingDirectory=${TWILL_DIR}
ExecStart=${BINARY} \\
  --base-path ${CHAIN_DATA} \\
  --chain ${MAINNET_SPEC} \\
  --node-key-file ${KEY_FILE} \\
  --listen-addr /ip4/0.0.0.0/tcp/30333 \\
  --no-rpc \\
  --no-telemetry \\
  --no-prometheus \\
  --no-mdns \\
  --in-peers 64 \\
  --out-peers 32
Restart=always
RestartSec=5
LimitNOFILE=65535
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=${CHAIN_DATA}
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable --now ${SERVICE}

# ── 9. print result ───────────────────────────────────────────────────────────

SERVER_IP=$(curl -s https://api.ipify.org || hostname -I | awk '{print $1}')

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Twill bootnode is running."
echo ""
echo "  Service status:  systemctl status ${SERVICE}"
echo "  Live logs:       journalctl -u ${SERVICE} -f"
echo ""
echo "  Back up this file (lose it = lose your PeerId):"
echo "    ${KEY_FILE}"
echo ""
PEER_ID=$(cat /tmp/twill-peerid.txt 2>/dev/null | head -1 | tr -d '[:space:]' || echo "<check journalctl for PeerId>")
echo "  Your bootnode multiaddr:"
echo "    /ip4/${SERVER_IP}/tcp/30333/p2p/${PEER_ID}"
echo ""
echo "  Paste the multiaddr back to update the chain spec."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
