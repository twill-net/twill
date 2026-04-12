# Join the Twill Network

Mine TWL and earn block rewards. 100% of every block reward goes to the miner.
No founder allocation. No pre-mine. Zero admin keys.

## Public RPC

Connect to the live network:
```
wss://rider-treaty-rocket-phrases.trycloudflare.com
```

View chain state in your browser:
[Polkadot.js Apps → Twill Network](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frider-treaty-rocket-phrases.trycloudflare.com)

---

## Requirements

- Linux x86_64 or macOS (Apple Silicon or Intel)
- GPU strongly recommended (SHA256 mining)
- Node.js 18+ (for the miner script)
- ~2 GB disk space for chain data

---

## Step 1 — Get the node binary

**Option A: Download pre-built binary (easiest)**

Go to [Releases](https://github.com/twill-net/twill/releases) and download the latest binary for your OS:

```bash
# Linux
curl -L https://github.com/twill-net/twill/releases/latest/download/twill-linux-x86_64 -o twill
chmod +x twill

# macOS
curl -L https://github.com/twill-net/twill/releases/latest/download/twill-macos-arm64 -o twill
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
git clone https://github.com/twill-net/twill.git
cd twill
cargo build --release
cp target/release/twill ./twill
```

---

## Step 2 — Download the chainspec

```bash
curl -L https://github.com/twill-net/twill/releases/latest/download/chainspec-raw.json -o chainspec-raw.json
```

Or if you built from source, it's already in the repo root.

---

## Step 3 — Start your node

```bash
mkdir -p chain-data

./twill \
  --base-path ./chain-data \
  --chain chainspec-raw.json \
  --rpc-port 9944 \
  --rpc-cors all \
  --no-telemetry \
  --no-prometheus
```

Leave this running in one terminal. Your node will sync with the network.

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

## Step 6 — Start mining

```bash
MNEMONIC="your twelve words here" node scripts/mine.js
```

**Optional env vars:**
```bash
# Mine against your own local node (default)
RPC=ws://127.0.0.1:9944 MNEMONIC="your words" node scripts/mine.js

# Mine against the public RPC (no local node required)
RPC=wss://rider-treaty-rocket-phrases.trycloudflare.com MNEMONIC="your words" node scripts/mine.js

LOG_EVERY=5000   # log progress every N hashes (default 10000)
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

Open an issue on GitHub: https://github.com/twill-net/twill/issues
