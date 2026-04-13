# Twill Network (TWL)

**The cross-border atomic settlement rail.**

Twill is a Layer 1 blockchain built for one purpose: to settle any asset against any other asset, across any border, atomically. Bitcoin to Carbon. Ethereum to BTC. Carbon to TWL. All-or-nothing, no counterparty, no custodian, no rails you don't own.

TWL is the settlement currency. It is how assets move. When you want carbon credits, you pay TWL. When you want BTC settled on-chain, you route through TWL. Carbon credit issuers post a TWL bond. The reserve vault grows with every asset deposited. Every TWL in circulation was earned — mined or earned through settlement — never allocated.

No admin keys. No owner. After genesis, the chain runs itself.

## Quick Facts

| Property | Value |
|----------|-------|
| Token | TWL |
| Total Supply | 50,000,000 (hard cap) |
| Mining Pool | 50,000,000 (100%) |
| Decimals | 12 |
| Block Time | 6 seconds |
| Consensus | Proof of Compute + Proof of Settlement (unified) |
| Settlement | Cross-border HTLC atomic swaps — any asset, any rail |

## How It Works

**Settlement.** Atomic HTLC multi-leg swaps. Lock funds, reveal the preimage, all legs settle simultaneously or all refund. BTC ↔ TWL ↔ Carbon. Any combination. No counterparty risk.

**TWL as the rail.** Every cross-asset swap routes through TWL. Carbon credits require a TWL bond to issue. Reserve assets are priced in TWL. This is not a fee mechanism — it is how value flows through the protocol.

**Mining.** Anyone submits Proof of Compute solutions. The block hash incorporates the settlement Merkle root — block production and settlement verification are a single operation. Block rewards halve every 4 years over ~20 years.

**Staking.** Stake TWL to process settlements and submit oracle prices. Rewards are 100% settlement fees, stake-weighted. Inactive stakers get auto-slashed.

**Carbon.** Permissionless carbon credit issuance. Post a TWL bond, issue verified tCO2e credits on-chain. Lock, retire, or trade atomically against any other asset. Credits are slashable if fraudulent.

## Token Distribution

| Allocation | Amount | How |
|------------|--------|-----|
| Mining Pool | 50,000,000 (100%) | PoC + PoSe block rewards, halving every 4 years |

No pre-mine. No founder allocation. No dev fund. No treasury. Every TWL is mined.

## Autonomous Design

After genesis block 0:
- **Zero admin extrinsics** in any pallet
- **Automatic** epoch transitions, reward distribution, slashing, settlement expiry
- **Permissionless** mining, settlement, staker registration, oracle feeds, carbon issuance
- **No sudo, no governance keys** — the chain moderates itself

## Architecture

Built on Substrate in Rust.

```
twill/
├── primitives/          # Shared types, constants, crypto
├── pallets/
│   ├── settlement/      # HTLC atomic settlement engine
│   ├── reserve/         # Reserve vault (oracle-valued)
│   ├── mining/          # PoC mining + stake-weighted PoSe + auto-slashing
│   ├── carbon/          # Carbon credit lifecycle
│   ├── oracle/          # Permissionless price feeds
│   ├── governance/      # Board elections + community proposals
│   └── twl-token/       # Cap enforcement, burn
├── runtime/             # Chain runtime
├── node/                # Node binary + genesis config
├── scripts/             # Setup, key generation
└── docs/                # Whitepaper, specs
```

## Getting Started

```bash
cargo build --release
./target/release/twill --dev   # Run dev node
cargo test --all               # Run tests
```

## Docs

| Document | What It Covers |
|----------|---------------|
| [ASSET_RAILS.md](docs/ASSET_RAILS.md) | Every asset, its status (live vs. needs activation), and exactly what the community must do to activate each rail |
| [WHITEPAPER.md](docs/WHITEPAPER.md) | Protocol design, tokenomics, reserve mechanics, cross-border settlement thesis |
| [TECHNICAL_SPEC.md](docs/TECHNICAL_SPEC.md) | Full protocol specification, pallet internals, consensus algorithm |
| [TOKENOMICS.md](docs/TOKENOMICS.md) | Supply, emission schedule, fee distribution |
| [LAUNCH.md](docs/LAUNCH.md) | How to run a node |
| [JOIN.md](JOIN.md) | How to start mining |
| [COMMUNITY_BOARD_CHECKLIST.md](docs/COMMUNITY_BOARD_CHECKLIST.md) | Board mandate, audit requirements, corridor activation plan |
| [FAILURE_MODES.md](docs/FAILURE_MODES.md) | Every known failure mode, protocol response, and board/community action required |

## Wallets

**Native:** Polkadot.js, Talisman, SubWallet

## License

Apache 2.0

---

*Made by Monk. Owned by nobody.*
