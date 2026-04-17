# Join the Twill Network

Mine TWL and earn block rewards. 100% of every block reward goes to the miner.
No founder allocation. No pre-mine. Zero admin keys.

## Connecting

Run your own node (recommended). The instructions below build a node from source
and connect it to the network. Once your node is synced, point Polkadot.js Apps
at `ws://127.0.0.1:9944` to view chain state.

---

## Requirements

- Linux x86_64 or macOS (Apple Silicon or Intel)
- GPU strongly recommended for actual mining (SHA-256 brute force via WGSL compute shaders — Vulkan, Metal, or DX12)
- Rust toolchain (to build the node and the GPU miner helper)
- Node.js 18+ (for the orchestration script)
- ~2 GB disk space for chain data

---

## Step 1 — Get the node binary

**Option A: Download pre-built binary (easiest)**

Download the latest binary for your OS from the Twill release page:

```bash
# Linux — download the linux-x86_64 binary, then:
chmod +x twill

# macOS — download the macos-arm64 binary, then:
chmod +x twill
```

**Option B: Build from source (verify the code yourself)**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Linux only
sudo apt-get install -y clang libclang-dev protobuf-compiler

# macOS only
brew install protobuf

# Clone and build
git clone https://github.com/twill-net/twill
cd twill
cargo build --release
cp target/release/twill ./twill
```

---

## Step 2 — Get the chainspec

`mainnet-raw.json` is in the repo root if you built from source. Otherwise download it from the Twill release page. This file is the canonical genesis — every node on the network must use the exact same `mainnet-raw.json`.

The genesis bootnode is embedded in `mainnet-raw.json`, so your node discovers peers automatically:

```
/ip4/140.82.10.138/tcp/30333/p2p/12D3KooWGrvFo7bFjgWyVj5boBumVYEQq2Q6VywKht9Pgsz4RUMa
```

---

## Step 3 — Start your node

```bash
mkdir -p chain-data

./twill \
  --base-path ./chain-data \
  --chain mainnet-raw.json \
  --rpc-port 9944 \
  --rpc-cors none \
  --rpc-methods Safe \
  --no-telemetry \
  --no-prometheus
```

Leave this running in one terminal. Your node will connect to the genesis bootnode and sync with the network.

---

## Step 4 — Generate a mining address

```bash
./twill key generate --scheme sr25519
```

Save the **secret phrase** (12 words) somewhere safe. This is your wallet.
The **SS58 Address** is your public mining address where rewards are sent.

---

## Step 5 — Install miner dependencies

```bash
# Inside the cloned repo
cd scripts
npm install
```

Or install manually:
```bash
npm install @polkadot/api @polkadot/keyring @polkadot/util-crypto
```

---

## Step 6 — Build the GPU helper (recommended)

The miner is split into two parts: the JS script (`scripts/mine.js`) handles
substrate RPC, settlement-root tracking, signing and submission; the Rust
helper (`twill-miner`) does the actual SHA-256 brute force on your GPU using a
WGSL compute shader (works on Vulkan, Metal, DirectX 12).

```bash
# From the repo root
cargo build --release -p twill-miner
```

That produces `target/release/twill-miner`. The JS script auto-detects this
binary and uses it. Without it, mining falls back to a slow JS CPU loop —
fine for trying out the dev chain, useless for actually winning blocks at
mainnet difficulty.

To force CPU-only (e.g. headless server with no GPU):

```bash
TWILL_MINER=cpu MNEMONIC="your words" node scripts/mine.js
```

To point at a different miner binary (cross-compiled, custom build):

```bash
TWILL_MINER_BIN=/path/to/twill-miner MNEMONIC="your words" node scripts/mine.js
```

---

## Step 7 — Start mining

```bash
MNEMONIC="your twelve words here" node scripts/mine.js
```

You'll see one of:

```
Engine: gpu (/path/to/target/release/twill-miner)
Engine: cpu fallback — build the GPU helper for real hashpower:
          cargo build --release -p twill-miner
Engine: cpu (forced via TWILL_MINER=cpu)
```

**Optional env vars:**
```bash
# Mine against your own local node (default)
RPC=ws://127.0.0.1:9944 MNEMONIC="your words" node scripts/mine.js

# Mine against any other Twill RPC
RPC=ws://your-node:9944 MNEMONIC="your words" node scripts/mine.js

LOG_EVERY=5000   # log progress every N hashes (default 10000, cpu fallback only)
```

You'll see:
```
✓ Block mined! Reward: 1.189 TWL → 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
  Hash attempts: 847 | Block #1042
```

---

## Economics

- **Block reward**: starts at ~1.189 TWL per block, halves every 21,024,000 blocks (~4 years)
- **Hard cap**: 50,000,000 TWL total, ever
- **Bootstrap period**: first 10,000,000 TWL mined are fee-free (no TWL needed to start)
- **Staker earnings**: settlement fees only — miners keep 100% of block rewards

---

## View your balance

Open [Polkadot.js Apps](https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944) while your node is running:

- **Accounts** → add your address to see balance
- **Developer → Chain State → mining → totalMinted** → total TWL mined so far
- **Developer → Chain State → balances → account(yourAddress)** → your balance

---

## Troubleshooting

**Node won't start:**
```bash
# Check if port 9944 is already in use
lsof -i :9944
# Kill it and retry
```

**Miner not connecting:**
```bash
# Make sure your node is running first, then:
MNEMONIC="your words" node scripts/mine.js
```

**Balance shows 0 after restart:**
- Make sure you did NOT use `--tmp` when starting the node
- Your `chain-data/` directory must be preserved between restarts

---

## Questions

Post in the Twill community forum.
