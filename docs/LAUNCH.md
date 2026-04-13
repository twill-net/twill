# Running a Twill Node

Twill is a permissionless blockchain. Anyone can run a node, mine TWL, and participate in governance.
No permission required. No registration. No keys issued by anyone.

---

## Requirements

- Linux, macOS, or Windows (WSL)
- 4 CPU cores, 8 GB RAM minimum (8 cores + 16 GB recommended for mining)
- 100 GB SSD storage
- Stable internet connection
- Rust toolchain (for building from source)

---

## 1. Build

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Clone and build
git clone <twill-source-repo>
cd twill
cargo build --release

# Binary is at:
./target/release/twill --version
```

Build takes 10-20 minutes on first run (compiling Substrate dependencies).

---

## 2. Get the Chain Spec

The chain spec is the canonical genesis state. All nodes must use the same one.

**Download the raw spec:**
```bash
# From the repo root (already committed):
ls mainnet-raw.json
```

Or regenerate it yourself (produces identical output — the genesis is deterministic):
```bash
./scripts/build-mainnet-spec.sh
```

---

## 3. Run a Node

```bash
# Basic — connect and sync
./scripts/run-node.sh

# With a bootnode (needed until peer discovery fills in)
BOOTNODE=/ip4/BOOTNODE_IP/tcp/30333/p2p/PEER_ID ./scripts/run-node.sh

# Custom data directory
BASE_PATH=/data/twill ./scripts/run-node.sh
```

Your node is running when you see:
```
💤 Idle (0 peers), best: #0 (0x...)
```

Once connected to at least one peer, it will start syncing:
```
⚙️  Syncing  12.3 bps, target=#1042 (1 peers)
```

---

## 4. Connect to Your Node

**Polkadot.js Apps:**
1. Go to [polkadot.js.org/apps](https://polkadot.js.org/apps)
2. Click the network icon (top left)
3. Scroll to "Development" → "Custom"
4. Enter: `ws://127.0.0.1:9944`
5. Click "Switch"

You'll see the Twill chain, TWL token (12 decimals), and live blocks.

**Network settings:**
- Token: TWL
- Decimals: 12
- SS58 prefix: 42
- RPC: `ws://127.0.0.1:9944` (local)

---

## 5. Mine TWL

Mining on Twill is permissionless. Find a nonce where:

```
SHA256(nonce || settlement_root || parent_hash) < difficulty
```

Submit it via the `mining.submitPocProof(nonce, settlement_root)` extrinsic.
One miner wins per block — first valid proof included by the network wins.

**Quick start with the automated miner script:**

```bash
cd scripts
npm install
MNEMONIC="your twelve word seed phrase here" node mine.js
```

The script subscribes to new blocks, searches for a valid nonce immediately,
and submits it. At launch difficulty (~1/256 success per hash), a modern CPU
finds a winning nonce in milliseconds.

**To maximize earnings:**
- Run your own node (zero latency to broadcast — no relay hop)
- Start hashing the instant a new block header arrives
- Use multiple CPU threads (run multiple `mine.js` processes)
- Keep the miner colocated with the node (same machine or same datacenter)

Block reward schedule (100% to the winning miner):
| Period | Block Reward |
|--------|-------------|
| Years 0-4 | ~1.189 TWL |
| Years 4-8 | ~0.595 TWL |
| Years 8-12 | ~0.297 TWL |
| Years 12+ | ~0.149 TWL |

Hard cap: 50,000,000 TWL. No more will ever exist.

---

## 6. Stake TWL (PoSe)

Staking earns you a share of:
- Settlement fees (80% to stakers, stake-weighted; 20% to treasury)

Minimum stake: 1,000 TWL

```
# Via Polkadot.js — Developer → Extrinsics → mining → registerStaker
```

---

## 7. Run a Bootnode (Help the Network)

Bootnodes help new nodes find the network. Run one if you have a stable public IP.

```bash
# Generate a stable node key (keep this — same key = same peer ID)
./target/release/twill key generate-node-key --file bootnode-key

# Get your peer ID (share this publicly)
./target/release/twill key inspect-node-key --file bootnode-key

# Start the bootnode
./scripts/run-bootnode.sh

# Announce your address on the forum:
# /ip4/YOUR_PUBLIC_IP/tcp/30333/p2p/YOUR_PEER_ID
```

---

## 8. Governance

All TWL holders can:
- Submit proposals (no deposit — quorum is the spam filter)
- Vote on proposals (equal-weight until 10M TWL mined, then stake-weighted)
- Nominate for the board election
- Vote in board elections

Proposals pass with: 50% Aye, 10% quorum (of circulating supply).
Board elected every 5 years (~26.28M blocks). Max 7 members.

```
# Polkadot.js → Developer → Extrinsics → governance
```

---

## Troubleshooting

**Node won't connect to peers:**
- Pass `--bootnodes` with a known bootnode address
- Check firewall: port 30333 (P2P) must be open

**RPC not accessible:**
- Default RPC listens on `127.0.0.1:9944` (local only)
- For public access: add `--rpc-external` flag (careful with security)

**Database corruption:**
- Purge and resync: `./target/release/twill purge-chain --chain mainnet-raw.json`

**Out of disk space:**
- Chain grows ~1-5 GB/month depending on activity
- Use `--state-pruning` flag to limit state history

---

## Source Code

The source code is public. Apache 2.0. Fork it. Audit it. Run it.
No permission needed from anyone.
