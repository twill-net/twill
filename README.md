# Twill Network (TWL)

**Community settlement, woven together atomically.**

Twill is a Layer 1 blockchain for one job: settle any asset against any other asset, across any border, in a single atomic operation. Bitcoin to Carbon. Ethereum to BTC. Carbon to TWL. All-or-nothing — no counterparty, no custodian, no rails anyone can pull.

TWL is the thread. Every cross-asset swap is woven through it. Every TWL in circulation was earned — mined or paid as a settlement fee — never allocated. The chain has no owner and no admin shortcut: every privileged path runs through community governance.

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
| Network Status | Live — genesis bootnode shipped in `mainnet-raw.json` |
| Genesis Bootnode | `/ip4/140.82.10.138/tcp/30333/p2p/12D3KooWGrvFo7bFjgWyVj5boBumVYEQq2Q6VywKht9Pgsz4RUMa` |

## How It Works

**Settlement.** Atomic HTLC multi-leg swaps. Lock funds, reveal the preimage, every leg settles or every leg refunds. The HTLC engine is asset-agnostic: any rail wired up to it can participate in the same atomic swap. No counterparty risk.

**TWL as the thread.** Every cross-asset swap routes through TWL. Carbon credits require a TWL bond to issue. Reserve assets are priced in TWL. This is not a fee mechanism — it is how value moves through the protocol.

**What ships at genesis vs. what is scaffolded.** The settlement engine, mining, staking, oracle, governance, carbon issuance, and the TWL ↔ TWL leg are live the moment the chain produces blocks. External-asset rails (BTC, ETH, SOL, USDC, fiat) are scaffolded — the settlement engine accepts them, but the bridges and oracle feeds that make those rails trustless are activated by the community board after audit. See [ASSET_RAILS.md](docs/ASSET_RAILS.md) for the per-asset status and activation path.

**Mining.** Anyone submits Proof of Compute solutions. The block hash incorporates the settlement Merkle root — block production and settlement verification are a single operation. Block rewards halve every 21,024,000 blocks (~4 years at 6s blocks).

**Staking.** Stake TWL to process settlements and submit oracle prices. Rewards are settlement fees, stake-weighted. Inactive stakers get auto-slashed.

**Carbon.** Permissionless carbon credit issuance. Post a TWL bond, issue verified tCO2e credits on-chain. Lock, retire, or trade atomically against any other asset. Credits are slashable if fraudulent.

## Token Distribution

| Allocation | Amount | How |
|------------|--------|-----|
| Mining Pool | 50,000,000 (100%) | PoC + PoSe block rewards, halving every 21,024,000 blocks |

No pre-mine. No founder allocation. No dev fund. No team treasury. Every TWL is mined.

## Governance Posture

After genesis block 0:

- **No sudo pallet.** No `sudo`, no root key, no foundation multisig.
- **Privileged extrinsics exist** in some pallets (relayer registry, carbon bond slashing, reserve redemption fulfillment) but every one is gated behind `EnsureRoot`. The only way to reach `EnsureRoot` is a runtime upgrade — and runtime upgrades themselves are governance proposals voted on by the community board.
- **Automatic** epoch transitions, reward distribution, slashing, and settlement expiry — no human in the loop.
- **Permissionless** mining, settlement submission, staker registration, oracle feeds, and carbon issuance.

The chain runs itself. Anything that touches privileged state has to be proposed, voted, and executed on-chain.

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

## Wallets

**Native:** Polkadot.js, Talisman, SubWallet

## License

Apache 2.0
