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

Minimum stake: 1,000 TWL. Higher stake = proportionally more fee earnings.

```
# Via Polkadot.js:
# Developer → Extrinsics → mining → registerValidator
# Parameter: stake (minimum 1,000 TWL = 1_000_000_000_000 planck)
```

You can deregister at any time with `mining → deregisterValidator`.
Staked TWL is slashed (burned) for inactivity — submit at least one proof every ~3 days.

---

## 7. Run a Bootnode (Help the Network)

Bootnodes help new nodes find the network. They require a stable public IP and port 30333 open.
Minimum spec: 2 CPU, 4 GB RAM, 50 GB SSD, 100 Mbps connection.

```bash
# 1. Generate a stable node key — back this up, losing it changes your peer ID
./target/release/twill key generate-node-key --file /etc/twill/bootnode-key
chmod 400 /etc/twill/bootnode-key

# 2. Get your peer ID (share this when announcing)
./target/release/twill key inspect-node-key --file /etc/twill/bootnode-key
# → prints something like: 12D3KooWAbcXyz...

# 3. Open port 30333 in your firewall
ufw allow 30333/tcp   # Ubuntu / ufw
# or: iptables -A INPUT -p tcp --dport 30333 -j ACCEPT

# 4. Start the bootnode (runs in foreground — use systemd or screen/tmux for production)
./scripts/run-bootnode.sh

# 5. Announce your multiaddr in the community:
# /ip4/YOUR_PUBLIC_IP/tcp/30333/p2p/12D3KooWAbcXyz...
```

**Systemd service (recommended for stability):**
```ini
# /etc/systemd/system/twill-bootnode.service
[Unit]
Description=Twill Bootnode
After=network.target

[Service]
ExecStart=/path/to/twill/target/release/twill \
  --chain /path/to/mainnet-raw.json \
  --node-key-file /etc/twill/bootnode-key \
  --listen-addr /ip4/0.0.0.0/tcp/30333 \
  --no-telemetry \
  --no-prometheus \
  --base-path /data/twill-bootnode
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```
```bash
systemctl daemon-reload
systemctl enable twill-bootnode
systemctl start twill-bootnode
```

---

## 8. Run a Bridge Relay (Enable BTC/ETH Settlements)

Bridge relayers watch Bitcoin, Ethereum, and Solana chains and submit on-chain confirmations
when a deposit arrives. Without at least 2 active relayers, BTC/ETH/SOL settlement legs cannot
execute. This is a trusted community role — relayers must be added by the board via
`bridge.addRelayer(who)` after launch.

**What a relayer does:**
1. Watches a designated deposit address on Bitcoin / Ethereum / Solana
2. When a deposit arrives and reaches sufficient confirmations (6+ for BTC, 12+ for ETH), calls:
   ```
   bridge.confirmDeposit(exchange_id, leg_index, asset, chain_txid, amount)
   ```
3. Once `ConfirmationThreshold` relayers (default: 2) agree on the same txid + amount for a leg,
   the settlement engine marks that leg confirmed and allows execution.

**Key parameters:**
- `exchange_id`: H256 hash identifying the settlement (from the `SettlementProposed` event)
- `leg_index`: 0-based index of the BTC/ETH/SOL leg within the settlement (from `LegAdded` event)
- `chain_txid`: The transaction hash on the external chain
- `amount`: Amount in the asset's native smallest unit (satoshi for BTC, wei for ETH)

**Conflict prevention:** All relayers must submit identical `chain_txid` and `amount` for the same
`(exchange_id, leg_index)`. A mismatched submission is rejected on-chain — a single malicious
relayer cannot corrupt a confirmation set.

**Becoming a relayer:**
1. Run a full Twill node (required to submit extrinsics)
2. Run Bitcoin Core / Geth / Solana validator, or use a trusted RPC provider
3. Write or run relay software that watches deposit addresses and calls `confirmDeposit`
4. Contact the board to be added via `bridge.addRelayer(your_address)`

The deposit address scheme (what addresses to watch for each settlement) is communicated
off-chain between the two parties initiating a cross-chain swap. Relayers watch a known set
of addresses and match incoming transactions to open settlement IDs using event logs.

---

## 9. Configure Oracle Price Feeds (Validator Role)

PoSe validators can optionally submit price data to the oracle. This is not required but
improves the quality of reserve valuations and settlement price gating.

**How oracle prices are used:**
- Reserve revaluation (`reserve.revalue`)
- Settlement price gate (Pass 1.5 — prevents economically irrational swaps)
- Carbon credit redemption calculations

**Configure your validator's oracle endpoint:**

Set a price API endpoint in your node's offchain local storage. The OCW (off-chain worker)
built into the oracle pallet will poll this URL and submit the median price on-chain.

```bash
# Via Polkadot.js → Developer → RPC calls → offchain → localStorageSet
# Key:   twill::oracle::endpoint
# Value: https://your-price-api.example.com/twl-pairs
```

**Expected API response format (JSON):**
```json
{
  "BTC_TWL": 42000000000000,
  "ETH_TWL": 2800000000000,
  "SOL_TWL": 95000000000,
  "CARBON_TWL": 15000000000
}
```
Values are in planck (10^-12 TWL) per asset unit (satoshi for BTC, gwei for ETH).

Price submissions are aggregated by median. Outliers are ignored. No individual validator
can move the price more than the outlier rejection threshold.

---

## 10. Governance

All TWL holders can:
- Submit proposals (no deposit — quorum is the spam filter)
- Vote on proposals (equal-weight until 10M TWL mined, then stake-weighted)
- Nominate for the board election
- Vote in board elections

Proposals pass with: 50% Aye, 10% quorum (of circulating supply).
Board elected every 5 years (~26.28M blocks). Max 7 members.

**Starting the first election** — after the chain launches, anyone calls:
```
# Polkadot.js → Developer → Extrinsics → governance → startElection
```
This bootstraps the first board. No TWL required to trigger it.

**Submitting a proposal:**
```
# governance → submitProposal
# ProposalKind options: TreasurySpend, SetBoardPay, SetMiningTreasuryShare,
#   SetMaxVoteWeight, SwitchToTwlWeightedVoting, RuntimeUpgrade, BoardRecall
```

---

## 11. Reserve Redemption

If you hold TWL and want to redeem it for a share of the protocol reserve:

```
# Developer → Extrinsics → reserve → requestRedemption
# Parameters:
#   desiredAsset: BTC | ETH | SOL | CarbonCredit
#   twlAmount:    amount in planck (1 TWL = 1_000_000_000_000)
```

Your TWL is locked immediately. The board processes the off-chain transfer and
calls `fulfillRedemption` when complete. You can cancel at any time before
fulfillment to retrieve your locked TWL via `cancelRedemption(requestId)`.

The `expectedAssetAmount` shown at request time is informational — it reflects the
current oracle price and is not a guarantee. Actual fulfillment amount is set by
the board based on the price at time of transfer.

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

## Community

Twill is permissionless. The community coordinates off-chain through:

- **Discord** — real-time discussion, node announcements, bridge relay coordination
- **On-chain governance** — all binding decisions happen on-chain

**Recommended Discord channels:**
```
#general          — open discussion
#announcements    — chain status, upgrade votes, election notices
#mining           — mining tips, difficulty tracking, hardware
#validators       — staking, inactivity alerts, fee distribution
#bridge-relay     — relay operators, deposit address coordination
#oracle           — price feed operators, API endpoint sharing
#governance       — proposal discussion before on-chain submission
#bootnodes        — bootnode addresses, network health
#dev              — integrations, tooling, protocol development
```

**On-chain first.** Discord is coordination. Decisions that matter — proposal votes,
board elections, relay additions — happen on-chain and cannot be overridden by any
off-chain communication.

---

## Source Code

The source code is public. Apache 2.0. Fork it. Audit it. Run it.
No permission needed from anyone.
