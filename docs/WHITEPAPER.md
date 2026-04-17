# Twill Network: A Settlement Commodity Protocol

**Version 1.0 — April 2026**

---

> *Twill is the first asset whose scarcity is proven by real financial infrastructure, not by burning electricity.*

---

## Table of Contents

1. [Abstract](#1-abstract)
2. [Introduction](#2-introduction)
3. [The Settlement Commodity](#3-the-settlement-commodity)
4. [Consensus Mechanism](#4-consensus-mechanism)
5. [Token Economics](#5-token-economics)
6. [Reserve System](#6-reserve-system)
7. [Settlement Protocol](#7-settlement-protocol)
8. [Carbon Integration](#8-carbon-integration)
9. [Governance](#9-governance)
10. [Network Properties](#10-network-properties)
11. [Roadmap](#11-roadmap)
12. [Conclusion](#12-conclusion)

---

## 1. Abstract

We propose a Layer 1 blockchain protocol in which the native asset, TWL, derives its value from provable settlement throughput rather than computational waste or stake-weighted governance. Twill introduces a unified consensus mechanism combining Proof of Compute (PoC) with Proof of Settlement (PoSe) in a single atomic operation, binding block production directly to the verification and execution of real economic transactions. Every block in the Twill chain contains cryptographic proofs of genuine financial activity.

TWL is a fixed-supply digital asset with a hard cap of 50,000,000 tokens and 12 decimal places of precision. It is the settlement currency of the Twill Network — the asset through which any other asset moves. Bitcoin settles against TWL. Carbon credits require a TWL bond to issue. Reserve deposits are valued in TWL. TWL is collateralized by a reserve vault that grows with every asset deposited into the network, establishing a deterministic price floor. Market price trades above this floor, reflecting the network's settlement utility premium.

The result is the first settlement commodity: a store of value whose scarcity is a function of real financial infrastructure, whose price floor is established by auditable reserves, whose consensus mechanism makes mining and settlement verification a single operation, and whose utility is the cross-border atomic movement of every asset class that matters.

This paper describes the architecture, consensus model, token economics, settlement protocol, reserve mechanics, carbon integration, and governance model that together constitute the Twill Network.

---

## 2. Introduction

### 2.1 Cross-Border Settlement Is Broken

Every day, $150 trillion in economic activity requires one fundamental operation: moving value from one party to another, across borders, across asset classes, with finality. This operation — settlement — is the plumbing of the global economy. And it is catastrophically inefficient.

**International payments** route through chains of correspondent banks. A business sending payment from one country to another typically touches four to seven intermediaries, each adding fees, delays, and failure points. Average cost: 6.25% of transaction value. Average time: three to five business days. In that window, either party can default. The recipient can receive funds before the sender has cleared. The sender can clear before the recipient's account is confirmed. There is no atomicity. There is no guarantee. There is just trust in a chain of institutions, each of which can fail, freeze, or seize.

**Cross-asset settlement** — exchanging Bitcoin for a carbon credit, or Ethereum for a commodity forward — does not exist without a trusted intermediary. Today, if a party wants to swap Bitcoin for carbon credits, they find a broker. The broker holds both assets. The broker is the counterparty. The broker charges for the privilege and introduces the risk that they will fail, run, or be regulated out of existence before the settlement completes. There is no protocol. There is no code. There is only trust.

**Voluntary carbon markets** represent one of the fastest-growing asset classes in the global economy — projected to exceed $50 billion annually by 2030. They have zero settlement infrastructure. Carbon credits are bought and sold over the phone, tracked in spreadsheets, and transferred via emails to registries. Double-counting is endemic. Retirement proof is a PDF. There is no on-chain delivery-versus-payment. There is no atomic exchange. There is no protocol anyone owns.

**This is the gap Twill fills.**

Twill is a permissionless cross-border atomic settlement protocol. Any two parties. Any two assets. Any two jurisdictions. They lock their assets against a hashlock. The preimage reveals. All legs settle simultaneously or all refund. No bank. No broker. No correspondent. No custodian. No SWIFT. No intermediary of any kind.

TWL is the settlement currency. It is the on-chain common ledger that makes cross-asset atomicity possible. When Bitcoin needs to exchange for a carbon credit, the settlement routes through TWL — both sides lock, both sides deliver, both sides receive, simultaneously and irrevocably. Carbon credit issuers post a TWL bond. The reserve vault holds every deposited asset and values it in TWL. The floor price of TWL rises as the network settles more.

This is not a roadmap. This is the protocol as it exists today.

### 2.2 The Problem with Existing Digital Assets

Three classes of digital assets dominate the current landscape. Each suffers from a fundamental structural deficiency.

**Bitcoin and Proof-of-Work chains** demonstrated that decentralized consensus is possible, but at extraordinary cost. The Bitcoin network consumes more electricity annually than many nation-states to produce blocks that carry no intrinsic economic function beyond their own transfer. The work in Proof-of-Work is, by design, useless. It exists solely to make block production expensive. The asset's scarcity is real, but its value proposition is circular: Bitcoin is valuable because it is scarce, and it is scarce because mining it is expensive.

**Ethereum and Proof-of-Stake chains** replaced energy expenditure with capital lockup. This improved efficiency but introduced plutocratic governance: those with the most capital dictate consensus. Ethereum's transition to Proof-of-Stake concentrated validation power among a small number of large stakers and liquid staking derivatives. The chain itself has become a general-purpose computation platform without a clear thesis on what, specifically, it should be used for. Execution capacity is abundant. Direction is not.

**Stablecoins** solved the volatility problem but reintroduced the trust problem. USDT, USDC, and their derivatives are IOUs issued by centralized entities, redeemable at the discretion of those entities, and subject to the banking relationships and regulatory postures of their issuers. They are not trustless. They are not permissionless. They are digital representations of the same system they claim to disintermediate.

### 2.3 The Governance Gap

Nearly every layer of modern financial infrastructure is owned, licensed, or gated by institutions. Payment rails are controlled by card networks and correspondent banks. Securities settlement runs through central depositories. Foreign exchange flows through a handful of prime brokers. Even most blockchain projects reproduce this pattern — foundation-controlled upgrades, VC-funded token allocations, and governance structures that concentrate power in the hands of early insiders.

But two domains remain structurally ungoverned.

**Atomic settlement** — the peer-to-peer exchange of assets without intermediaries — has no incumbent. No bank owns it. No clearinghouse licenses it. HTLC-based settlement is a cryptographic primitive, not a regulated product. The technology exists. The infrastructure to harness it at scale does not. Every day this gap persists is a day ordinary participants can build, own, and operate the settlement layer that institutions have not yet claimed.

**Voluntary carbon markets** are fragmented, illiquid, and largely unregulated at the protocol level. Carbon registries (Verra, Gold Standard) verify credits, but no single entity controls the on-chain settlement, pricing, or retirement of those credits. Governments are still debating frameworks. Banks are still building desks. The infrastructure layer — the rails on which carbon credits move, price, and retire — is unclaimed.

These are not permanent conditions. Institutions are aware of both gaps. Regulatory frameworks for peer-to-peer settlement are advancing. Carbon market consolidation is underway. The window in which permissionless infrastructure can be established — before licensing regimes and institutional gatekeepers arrive — is finite and narrowing.

Twill is built for this window. It is a settlement protocol that operates in the space between what institutions have captured and what they have not yet reached. Every block mined, every settlement executed, and every carbon credit retired on Twill is infrastructure built by participants, for participants, before the door closes.

### 2.4 The Missing Asset Class

What does not exist is a digital asset that:

1. Has provable, finite scarcity.
2. Derives value from real economic activity, not from energy waste or governance power.
3. Maintains a deterministic price floor backed by auditable reserves.
4. Operates on infrastructure that serves a purpose beyond its own perpetuation.
5. Is built in the governance gap — on rails that no institution yet controls.

Twill is designed to fill this gap. The TWL token is a settlement commodity — an asset whose value accrues from the financial infrastructure it powers, whose supply is fixed and verifiably distributed, and whose consensus mechanism converts block production into settlement verification.

### 2.5 Design Principles

The Twill Network is built on four principles:

- **Settlement as consensus.** The chain does not merely record settlements; it validates them. Block production and settlement verification are the same operation.
- **Intrinsic collateralization.** Every asset that enters the network contributes to a reserve vault, establishing a price floor that is deterministic, not speculative.
- **Wide distribution.** 100% of the total TWL supply is distributed through mining, ensuring that token ownership is broad and permissionless from genesis. No pre-mine. No founder allocation. Every TWL is earned.
- **Useful work.** Computation expended to produce blocks incorporates settlement Merkle roots. Mining is not wasted effort — it is the mechanism by which the network verifies economic activity.

---

## 3. The Settlement Commodity

### 3.1 Definition

A settlement commodity is a digital asset whose market value is a function of the settlement throughput it facilitates, collateralized by a reserve of real assets, and governed by a consensus mechanism that makes block production and settlement verification indistinguishable.

TWL is the first implementation of this concept.

Unlike Bitcoin, TWL's scarcity is not sustained by energy expenditure alone. Unlike Ethereum, TWL's utility is not derived from general-purpose computation. Unlike stablecoins, TWL is not a claim on a centralized reserve that can be frozen or revoked.

TWL occupies a new position in the digital asset taxonomy:

| Property | Bitcoin | Ethereum | Stablecoins | TWL |
|---|---|---|---|---|
| Supply cap | Fixed | Deflationary target | Elastic | Fixed (50M) |
| Value source | Scarcity via energy | Compute utility | Dollar peg | Settlement throughput |
| Consensus | PoW (waste) | PoS (capital) | Centralized | PoC + PoSe (useful work) |
| Price floor | None | None | $1 (trust-based) | Reserve-backed (trustless) |
| Permissionless | Yes | Yes | No | Yes |

### 3.2 Value Accrual

TWL accrues value through three mechanisms:

1. **Settlement fees.** Every atomic settlement on the Twill Network incurs a fee of 10 basis points (0.10%), denominated in TWL. As settlement volume grows, demand for TWL increases. 80% of fees flow to PoSe stakers stake-weighted. 20% flows to the protocol treasury, spendable only via passed governance proposals.

2. **Reserve accumulation.** Assets deposited into the network are held 1:1 in the Reserve Vault. As the reserve grows, the floor price of TWL rises monotonically.

3. **Supply scarcity.** With a hard cap of 50,000,000 tokens and no inflationary mechanism, increasing demand against fixed supply produces price appreciation above the floor.

The floor price is defined as:

```
floor_price(TWL) = total_reserve_value / circulating_TWL_supply
```

Market price trades at or above the floor. The spread between floor and market price represents the network's settlement utility premium — the market's assessment of future settlement throughput.

---

## 4. Consensus Mechanism

### 4.1 Overview

Twill employs a unified consensus mechanism that fuses two complementary proof systems into a single atomic operation:

- **Block Mining (PoC+PoSe unified):** A miner solves the hash puzzle (Proof of Compute) with the settlement Merkle root embedded in the proof (Proof of Settlement). Mining the block and validating the settlement ledger are one atomic operation. The miner earns 100% of the block reward. New TWL is created exclusively through this process.

- **PoSe Staking:** Participants stake TWL to back the settlement infrastructure. Staked TWL is the collateral that makes settlements trustworthy. Stakers earn settlement fees — existing, already-minted TWL redistributed from the fee pool — proportional to their stake. Staking does not create new TWL.

Neither mechanism operates in isolation. PoC produces blocks that contain settlement Merkle roots. PoSe stakers collateralize the settlement infrastructure that produces those roots. The two systems are coupled: mining without settlement data produces no valid blocks, and settlements without block inclusion have no finality.

### 4.2 Block Mining (PoC+PoSe Unified)

#### 4.2.1 Block Puzzle Construction

Block puzzles are constructed as follows:

1. The miner collects pending settlement proofs from the mempool.
2. A Merkle tree is constructed from the settlement proof set.
3. The Merkle root is incorporated into the block header as a mandatory field.
4. The miner searches for a nonce such that:

```
SHA256(nonce || settlement_root || parent_hash) < difficulty_target
```

The critical distinction from traditional Proof-of-Work: the block header includes the settlement Merkle root as a mandatory field. A block with an empty or invalid Merkle root is rejected by consensus. Mining IS settlement verification — the two operations cannot be separated. This is not PoC and PoSe running side by side. It is one operation that satisfies both.

#### 4.2.2 ASIC Resistance

The hash function used in block mining is designed to be memory-hard, requiring substantial RAM access patterns that are difficult to optimize in fixed-function hardware. The goal is to keep mining accessible to commodity hardware (CPUs and GPUs), preserving the permissionless character of block production.

The specific hash function may be upgraded via governance as hardware evolves. The design principle is permanent: mining should remain accessible to participants with general-purpose hardware.

#### 4.2.3 Difficulty Adjustment

Block difficulty adjusts every 2,016 blocks using a weighted moving average of recent block intervals. The target block time is 6 seconds.

### 4.3 PoSe Staking

#### 4.3.1 Staking Mechanics

Participants stake TWL to back the settlement infrastructure. The minimum stake is 1,000 TWL. Staking is fully permissionless — register, stake, earn. No delegation, no nomination committees, no governance approval.

Stakers earn settlement fees: existing, already-minted TWL redistributed from the fee pool, distributed stake-weighted. No new minting — does not affect the hard cap. New TWL comes exclusively from GPU block mining.

Stakers who become inactive are automatically slashed. First offense: 50% of stake. Second offense: 100% of stake and automatic deregistration. The inactivity threshold is approximately 3 days (~43,800 blocks at 6-second block time).

#### 4.3.2 Autonomy

After genesis, the staking system runs itself:

- Stakers register and deregister permissionlessly.
- Settlement fees auto-distribute to stakers every block (stake-weighted).
- Slashing is automatic (inactivity detection in `on_finalize`).
- Epoch transitions (halvings) are automatic.

No admin keys. No root extrinsics. No human intervention required.

### 4.4 Reward Model

**Block miners earn 100% of block rewards.** New TWL is created exclusively by GPU miners solving the PoC+PoSe puzzle. The block reward halves every 4 years (21,024,000 blocks). After the mining pool is exhausted, settlement fees sustain the network.

**PoSe stakers earn settlement fees.** As settlement volume grows, so do staker earnings. Stakers earn their share of the 10 bps (0.10%) settlement fee redistributed from the fee pool — existing TWL, no new minting. The more economic activity on the network, the more stakers earn.

---

## 5. Token Economics

### 5.1 Supply Parameters

| Parameter | Value |
|---|---|
| Token | TWL |
| Maximum supply | 50,000,000 |
| Decimal precision | 12 |
| Base unit | 1 planck = 10^-12 TWL |
| Inflation | None (hard cap) |
| Burn protocol | None (voluntary burn wallets available) |

The 50,000,000 TWL hard cap is enforced at the protocol level every block. The runtime checks total issuance against `TOTAL_SUPPLY` on every `on_initialize`. No governance vote, upgrade, or fork can increase the maximum supply. This is a constitutional constraint of the network.

### 5.2 Allocation

| Category | Percentage | Tokens | Mechanism |
|---|---|---|---|
| Mining pool | 100% | 50,000,000 | PoC+PoSe unified emission schedule |

One allocation. No dev fund. No community fund. No founder pre-mine. No advisory tokens. No investors. No pre-sale. 100% mined. Every TWL is earned. That is the entire distribution.

#### 5.2.1 Mining Pool (100%)

All 50,000,000 TWL are distributed through mining. No entity holds pre-mined tokens. No allocation exists that grants anyone a head start. The first block produces the first TWL, available to whoever mines it.

Block miners earn 100% of block rewards. The 50,000,000 TWL mining pool is exhausted entirely by GPU miners over ~20 epochs. The total emission follows a halving schedule, metered by the hard cap.

#### 5.2.2 Halving Schedule

Block rewards halve every 4 years (21,024,000 blocks at 6-second block time):

| Epoch | Years | Block Reward | Epoch Emission (approx.) | Cumulative (approx.) |
|---|---|---|---|---|
| 1 | 0-4 | ~1.189 TWL | ~25,000,000 | ~25,000,000 |
| 2 | 4-8 | ~0.595 TWL | ~12,500,000 | ~37,500,000 |
| 3 | 8-12 | ~0.297 TWL | ~6,250,000 | ~43,750,000 |
| 4 | 12-16 | ~0.149 TWL | ~3,125,000 | ~46,875,000 |
| 5 | 16-20 | ~0.074 TWL | ~1,562,500 | ~48,437,500 |
| 6+ | 20+ | Tail emission | Asymptotic | Approaches 50,000,000 |

The initial block reward is precisely 1,189,117,199,390 planck (~1.1891 TWL). This value is calculated so that Epoch 1 emits exactly half the mining pool (~25,000,000 TWL) across 21,024,000 blocks, with a sub-1-TWL undershoot across the entire epoch. The reward halves via right-shift (`INITIAL_BLOCK_REWARD >> epoch`), producing the same geometric decay as Bitcoin's halving model.

After 20 epochs, block rewards reach zero and the mining pool is exhausted. Transaction fees sustain both miners and stakers in perpetuity.

### 5.3 Bootstrap Period

The chain starts with zero balances — no pre-mine, no endowed accounts, no faucet. This creates a bootstrap problem: miners need TWL to pay transaction fees, but mining is how TWL is created.

Twill solves this the same way Bitcoin solves it: the work is the authorization. During the bootstrap period (first 10,000,000 TWL mined, approximately 20% of total supply), mining proof submissions are unsigned and fee-free. The proof-of-work itself serves as spam protection — invalid proofs are rejected at the transaction pool level before ever entering a block.

After 10M TWL has been mined, miners switch to standard signed transactions with small fees. By that point, every active miner already holds TWL from earlier blocks. The transition is seamless — the miner software handles it automatically.

This is not a subsidy or a special privilege. It is the minimum viable mechanism to bootstrap a chain that starts empty. The threshold was chosen so that enough TWL circulates across enough wallets to make fee-paying mining sustainable for the entire network.

### 5.4 Voluntary Burn

Twill does not implement a protocol-level burn mechanism. There is no automatic fee burn, no deflationary schedule, and no algorithmic supply reduction.

However, the protocol supports voluntary burn wallets — deterministic addresses derived from `SHA256("twill_safety_wallet:" || "burn")` with no known private key. Any TWL holder may send tokens to the burn wallet, permanently removing them from circulating supply. The burn wallet balance is synced every block via `on_initialize`, catching both explicit burns and direct transfers.

The rationale: supply reduction should be a market decision, not a protocol mandate. If participants believe reducing supply is in their interest, they are free to do so. The protocol does not impose this choice.

---

## 6. Reserve System

### 6.1 Architecture

The Reserve Vault is a protocol-native system that holds assets deposited into the Twill Network at a 1:1 ratio. It is the mechanism by which TWL establishes and maintains a deterministic price floor.

The vault is fully autonomous. No human can manually deposit into or withdraw from the reserve. The settlement engine is the only path in. The reserve grows organically from real economic activity, not from admin transactions or governance decisions.

The vault operates as follows:

1. A wrapped crypto asset or carbon credit enters the network through an atomic settlement.
2. The settlement pallet transfers the asset to the reserve vault account and records the deposit via the `ReserveInterface` trait.
3. The reserve pallet values the asset using the oracle price feed and adds it to the total reserve.
4. The reserve value is publicly auditable on-chain at all times, with automatic snapshots every 100 blocks (~10 minutes).

### 6.2 Floor Price Mechanism

The floor price of TWL is defined by the reserve ratio:

```
floor_price = total_reserve_value / circulating_supply
```

This floor is not a peg. TWL does not target a specific dollar price. The floor is a minimum — the price below which TWL cannot rationally trade, because the reserve backing per token exceeds the market price.

If TWL trades at or below the floor price, a rational actor can acquire TWL on the open market and redeem it against the reserve at a profit. This arbitrage mechanism enforces the floor.

The floor price increases monotonically as:

- Settlement fees add value to the reserve.
- New assets are deposited into the vault through settlements.
- Voluntary burns reduce circulating supply without reducing reserve value.

The floor can never decrease unless assets are withdrawn from the reserve — an operation governed by strict protocol rules requiring supermajority governance approval.

### 6.3 Intrinsic Pricing

The market price of TWL is the sum of two components:

```
market_price = floor_price + settlement_premium
```

The **floor price** is deterministic and on-chain. It represents the liquidation value of TWL — what the reserve guarantees.

The **settlement premium** is market-determined. It represents the market's valuation of TWL's utility as a settlement medium, its expected future settlement throughput, and the scarcity dynamics of a fixed-supply asset with growing demand.

This two-component pricing model gives TWL a unique property among digital assets: it has a verifiable minimum value (the floor) and an open-ended maximum value (the premium). The downside is bounded. The upside is not.

### 6.4 Reserve Composition

The Reserve Vault accepts the following asset classes, valued via decentralized oracle price feeds:

| Asset Class | Role | Oracle Pair | On-chain Representation |
|---|---|---|---|
| Wrapped Bitcoin | Hard-asset reserve | BTC/TWL | wBTC (Asset ID 1) |
| Wrapped Ethereum | Crypto reserve | ETH/TWL | wETH (Asset ID 2) |
| Wrapped USDC | Stablecoin reserve | USDC/TWL | wUSDC (Asset ID 3) |
| Wrapped Solana | Crypto reserve | SOL/TWL | wSOL (Asset ID 4) |
| Carbon credits | Environmental backing | Carbon/TWL | Verified carbon offsets |
| Settlement fees | Organic reserve growth | — | TWL-denominated fees |

The composition of the reserve is tracked per asset kind and reported on-chain. Concentration limits prevent over-exposure to any single asset type. Promoting any new asset to first-class reserve status is a runtime upgrade decided by community governance — additional assets can be wrapped in the meantime via the `Other` reserve bucket.

---

## 7. Settlement Protocol

### 7.1 Overview

The Twill settlement protocol is an atomic, multi-asset settlement system built natively into the chain. Settlement is not a dApp deployed on Twill — it is a core protocol function, as fundamental as block production or balance transfers.

Every settlement on Twill follows the same lifecycle:

```
PROPOSE -> LOCK -> SETTLE -> FINALIZE
```

Each stage produces a cryptographic proof. The complete proof set is included in the settlement Merkle tree, which is incorporated into the block header by PoC miners. If a settlement is not completed before its timelock expires, it is automatically refunded — either by the `on_initialize` expiry queue or by a manual permissionless refund call.

### 7.2 HTLC Atomic Settlement

Settlements use Hash Time-Locked Contracts (HTLCs) to ensure atomicity. An HTLC guarantees that either all legs of a settlement execute, or none do. There is no intermediate state in which one party has sent funds and the other has not received them.

The HTLC lifecycle:

1. **Propose.** The initiator creates a settlement with a hashlock — the SHA256 hash of a secret preimage. A timelock is set automatically (default: 20 blocks, ~2 minutes at 6-second block time).
2. **Lock legs.** Each participant locks their asset leg against the hashlock. Debit legs on the Twill-internal rail reserve TWL in escrow. Credit legs record the expected payout.
3. **Settle.** The initiator reveals the secret preimage. The protocol verifies it against the hashlock. All locked legs claim atomically — debit escrows transfer to credit receivers, less the settlement fee.
4. **Finality.** The settlement Merkle root is computed from all leg proofs and pushed to the mining pallet. It becomes part of the next block's PoC proof, coupling mining to settlement permanently.

If the initiator fails to reveal the preimage before the timelock expires, all contracts refund automatically. Settlement is atomic: all-or-nothing.

### 7.3 TWL as the Settlement Rail

TWL is not merely a fee token. It is the settlement currency — the common ledger through which any asset can atomically exchange with any other asset. This is the core architectural decision that makes cross-border atomic settlement possible without a trusted intermediary.

**The mechanism is simple.** A settlement is a set of legs. Each leg is a debit or credit of a specific asset on a specific rail. When a party wants to exchange Bitcoin for a carbon credit, they construct a settlement with:

- A Bitcoin debit leg (party A sends BTC on the Bitcoin rail)
- A TWL credit leg (party A receives TWL on-chain)
- A TWL debit leg (party B sends TWL on-chain)
- A Carbon credit leg (party B delivers the tCO2e credit on-chain)

All four legs settle atomically. Either all succeed or all refund. The Bitcoin network does not need to know about the carbon credit. The carbon registry does not need to know about Bitcoin. TWL is the on-chain proof of exchange — the common unit that makes the atomic guarantee possible.

**Carbon bonds amplify this.** Every carbon credit issued on Twill requires the issuer to lock a TWL bond. This means carbon credits are economically backed by TWL from the moment of issuance. When carbon trades, TWL flows. When credits are retired, the bond is returned. If a credit is found fraudulent, governance slashes the bond — burning TWL and permanently invalidating the credit.

**Supported rails:**

| Rail | Type | Description |
|------|------|-------------|
| TwillInternal | Native | On-chain TWL transfers, settlement-grade finality |
| Bitcoin | Crypto | BTC HTLC contracts, hashlock-coordinated with TWL settlement |
| Ethereum | Crypto | ETH/ERC-20 HTLC contracts |
| Solana | Crypto | SOL/SPL HTLC contracts |
| Verra | Carbon | Verified Carbon Units (VCU) |
| GoldStandard | Carbon | Gold Standard Verified Emission Reductions (VER) |

Each leg specifies its domain (Crypto or Carbon), rail, side (Debit or Credit), amount, and currency hash. Multi-leg settlements with up to 10 legs per exchange are supported, enabling any cross-asset combination to settle atomically in a single operation.

### 7.4 Settlement Finality

A settlement achieves finality when:

1. The hashlock preimage has been revealed and verified.
2. All legs have transitioned to `Claimed` status.
3. The settlement Merkle root has been included in a block's proof.
4. The block has been confirmed by the network.

Once final, a settlement cannot be reversed, censored, or modified. Finality is cryptographic, not administrative.

### 7.5 Fee Structure

Settlement fees are calculated as 10 basis points (0.10%) of the TWL-internal debit volume:

```
fee = max(min_fee, twl_debit_volume * 0.001)
```

The minimum fee is 0.1 TWL, preventing dust-value settlements from consuming network resources without compensation.

Settlement fees split automatically every block via protocol constants:

- **PoSe Stakers (80%, `FEE_STAKER_SHARE_BPS = 8000`):** Distributed stake-weighted to all active stakers. Primary incentive for collateralizing settlement infrastructure.
- **Treasury (20%, `FEE_COMMUNITY_SHARE_BPS = 2000`):** Transferred to the keyless `SHA256("treasury")` account. Spendable only by a passed governance proposal. Accumulates from block one — even with zero stakers active. Community can also vote to redirect up to 10% of block rewards to the treasury (default: 0% at genesis).

No individual controls the treasury. Both constants are baked into the runtime.

---

## 8. Carbon Integration

### 8.1 Rationale

Voluntary carbon markets represent one of the last major asset classes without centralized settlement infrastructure. Registries verify credits. Brokers trade them. But no protocol owns the rails — the on-chain movement, pricing, and retirement of carbon credits at scale.

This is not an accident. Carbon markets are young, fragmented, and still operating on spreadsheets and bilateral agreements. Banks are building carbon trading desks. Regulators are drafting frameworks. But the infrastructure layer — the settlement protocol that makes carbon credits liquid, verifiable, and atomically tradable — does not yet have an incumbent.

Twill integrates carbon credits directly into its settlement protocol and reserve vault. This is not a bolt-on offset program. Carbon credits are first-class assets on Twill: issuable, tradable, settleable, retirable, and reserve-eligible. TWL is partially carbon-backed — a portion of its floor price is attributable to verified carbon credits held in reserve.

The integration serves a dual purpose. It diversifies the reserve with an asset class uncorrelated to financial markets. And it builds permissionless carbon infrastructure in the window before institutions consolidate it.

### 8.2 Mechanism

Carbon credits enter the Twill network through a fully permissionless issuance model:

1. **Issuance.** Anyone can issue a carbon credit by providing a verification hash (proof of registry verification) and posting a bond of 100 TWL. No admin approval required.
2. **Dispute window.** Credits enter a dispute window (~7 days, 100,800 blocks). If unchallenged, the bond is returned and the credit is fully active. If challenged and found invalid, the bond is burned.
3. **Settlement.** Active carbon credits can be locked, traded via atomic settlement, and retired with on-chain retirement certificates.
4. **Reserve inclusion.** When carbon credits enter the network through settlements on Verra or Gold Standard rails, they are valued via the Carbon/TWL oracle pair and added to the reserve vault.

### 8.3 Retirement Certificates

When a carbon credit is retired, the protocol generates an immutable on-chain retirement certificate containing the credit ID, retiree account, amount, registry, and timestamp. Certificates are sequentially numbered and permanently stored on-chain. Retirement is irreversible — retired credits cannot be re-issued, unlocked, or transferred.

### 8.4 Implications

Carbon integration gives TWL several properties:

- **Infrastructure before incumbents.** Carbon settlement rails built now, by participants, become the default before institutions arrive to license and gate them.
- **Regulatory positioning.** As carbon pricing regimes expand globally, an asset partially backed by carbon credits is positioned favorably relative to assets with no environmental consideration.
- **Diversified reserve.** Carbon credits are uncorrelated with traditional financial assets, improving the reserve's risk profile.
- **Floor price contribution.** Carbon credits directly contribute to the TWL floor price, adding a value component that is independent of financial market conditions.
- **Permissionless participation.** No accreditation, no KYC, no approval committee. Post a bond, provide a verification hash, wait for the dispute window. The protocol handles the rest.

---

## 9. Governance

### 9.1 Design Philosophy

After genesis, the Twill protocol has no admin keys, no sudo, and no root extrinsics. Day-to-day operation is fully autonomous. But long-duration protocols need a mechanism for code maintenance, parameter updates, and adaptation to changing conditions.

Twill addresses this with a two-layer governance model: a community board for operational stewardship and direct community voting for all binding decisions. The board proposes. The community decides. No action takes effect without majority approval from TWL holders.

### 9.2 Community Board

The board consists of 5 to 7 members elected by TWL holders. Terms are 5 years (~26,280,000 blocks). Elections are permissionless.

**First election (genesis):** No deposit required, 1 address = 1 vote. The first board must be seated before significant TWL is in circulation, so no economic barrier applies.

**Subsequent elections:** Self-nomination requires a 100 TWL deposit. Voting is stake-weighted: 1 TWL = 1 vote. The top nominees by vote weight win seats.

The board's role is limited to operational stewardship:

- Proposing runtime upgrades and parameter changes.
- Coordinating code maintenance and security responses.
- Submitting proposals for community vote.

The board cannot unilaterally modify the protocol. Every proposal — runtime upgrades, parameter changes, reserve actions — requires community approval through on-chain voting.

### 9.3 Community Voting

All TWL holders can vote on proposals. Voting weight is phase-dependent: during the first 20% of supply mined (0–10M TWL), voting is equal-weight (1 address = 1 vote) to prevent early whales from capturing governance while the network bootstraps. After 10M TWL has been mined, voting automatically switches to stake-weighted (1 TWL = 1 vote, capped at 100K TWL per address to limit concentration). A proposal passes when:

- **Quorum:** At least 10% of circulating supply participates (Aye + Nay + Abstain).
- **Approval:** Aye votes exceed Nay votes (simple majority).
- **Emergency threshold:** Actions affecting the reserve or board recall require 75% Aye.

There is no deposit required to submit a proposal. The quorum requirement (10% of circulating supply) is the spam filter — proposals without genuine community support simply expire without action. Approved proposals are enacted after a 7-day delay (~100,800 blocks).

### 9.4 Safeguards

- No proposal can increase the 50,000,000 TWL hard cap. This is a constitutional constraint.
- No proposal can create admin keys, sudo access, or privileged accounts.
- Board members can be recalled by community vote (75% threshold).
- All governance actions are on-chain and publicly auditable.

---

## 10. Network Properties

### 10.1 Network Effects

Settlement networks strengthen with use. Each participant who settles through Twill expands the counterparty graph — more addresses with active HTLCs means tighter price discovery and faster settlement matching for every subsequent party.

Three compounding loops drive this:

**Settlement liquidity loop:** More active settlement counterparties → better pricing → more settlement volume → more fees → more stakers → better settlement reliability → more volume.

**Security loop:** More staked TWL → higher cost to attack settlement proofs → more institutional confidence → more high-value settlements → higher TWL demand → more stakers.

**Carbon loop:** More carbon credits retired on-chain → more auditable retirement certificates → more demand from compliance buyers → more carbon settlement volume → more fees for stakers.

Each loop reinforces the others. A protocol that settles more becomes more valuable to settle through.

### 10.2 Developer Ecosystem

Twill is built on polkadot-sdk — a modular FRAME runtime framework with a large global developer community. Every developer familiar with Substrate can read, audit, and extend Twill's pallets from day one. No proprietary SDK, no new programming model.

FRAME pallets are composable modules. New asset types, new rail integrations, and new settlement logic can be added in isolation without touching consensus, storage, or existing settlement logic. The pallet boundary is a clean interface — `pallet-carbon` knows nothing about `pallet-settlement` internals and vice versa.

The WebAssembly runtime upgrade mechanism means protocol improvements deploy without hard forks. A governance-approved upgrade takes effect at the next block after the enactment delay — no coordinated node restart, no network split.

### 10.3 Security Model

Twill's consensus, storage, and cryptographic layer is polkadot-sdk stable2409 — one of the most extensively reviewed blockchain codebases running in production. The Merklized state trie, block production engine, and peer networking are not bespoke implementations.

Settlement security does not depend on any trusted party:

- No custodian holds assets during an HTLC.
- No oracle determines settlement validity.
- No admin can pause, cancel, or modify an in-flight settlement.
- The SHA256 hashlock preimage is the only key. Whoever holds the preimage claims the output — no exceptions.

The protocol has no admin keys, no sudo, and no privileged accounts. An attacker who compromises every board member's credentials cannot steal funds, cannot pause the chain, and cannot alter settlement logic. The protocol enforces itself.

### 10.4 Total Value Locked

Twill accumulates TVL across three independent pools:

**Staking TVL** — TWL reserved by PoSe stakers as collateral backing settlement integrity. Stakers earn 80% of all settlement fees proportional to their stake weight. Slashed stake is burned — stakers are financially accountable for the settlements they collateralize.

**Settlement TVL** — TWL locked in active HTLC escrows during in-flight settlements. At meaningful settlement volume, this represents real productive capital — not idle speculation.

**Reserve TVL** — wBTC, wETH, and wUSDC held in the protocol reserve vault. Governance-controlled, not withdrawable without a community vote. Sets the mathematical floor: `floor = reserve_value / circulating_supply`. Reserve TVL cannot be diluted — no new TWL is ever minted outside of mining.

---

## 11. Roadmap

### Phase 1 — Foundation

- Testnet launch with PoC+PoSe unified consensus.
- Settlement pallet deployment and testing.
- Reserve Vault integration and audit.
- Initial mining client release (CPU/GPU).
- Security audit by independent firms.

### Phase 2 — Genesis

- Mainnet launch.
- Block mining begins; 50,000,000 TWL emission schedule activates.
- Reserve Vault begins accumulating from settlement activity.
- Settlement protocol goes live for atomic TWL settlements.
- Wallet and block explorer launch.
- After genesis, the protocol is autonomous. No admin keys remain.

### Phase 3 — Growth

- PoSe staker onboarding scales.
- Block reward halves at the first halving; settlement fee earnings grow with volume.
- Multi-asset settlement support across crypto and carbon rails.
- Carbon credit integration with Verra and Gold Standard registries.
- **EVM activation** — the board submits an `ActivateEvm` governance proposal. The community votes. If approved, the board deploys a Frontier-enabled runtime upgrade. Once enacted, Twill supports full Ethereum-compatible smart contract execution: any Solidity contract deployable on Ethereum deploys on Twill unchanged, using existing Ethereum tooling (Hardhat, Foundry, ethers.js, wagmi). This decision belongs entirely to the board and community — no external permission required.
- Oracle network expansion for reserve asset valuation.

### Phase 4 — Maturation

- Block reward halves again at second halving; staker fee income becomes primary earning.
- Settlement partnerships.
- Reserve diversification across asset classes.

### Phase 5 — Steady State

- Self-sustaining fee economy — stakers earn from settlement volume.
- Reserve vault at scale.
- Mining pool approaches exhaustion; transaction fees sustain the network.
- TWL established as a recognized settlement commodity.

---

## 12. Conclusion

The digital asset landscape has produced stores of value without utility, utility tokens without value floors, and stable assets without decentralization. Each addresses part of the problem. None addresses all of it.

More fundamentally, nearly every piece of financial infrastructure built in the last century is owned by someone. Payment networks. Clearinghouses. Exchanges. Depositories. The entities that control these rails extract rent from every transaction that flows through them. Most blockchain projects, despite their rhetoric, reproduce this pattern with different labels — foundations instead of corporations, token allocations instead of equity stakes, governance tokens instead of board seats.

Two domains remain open: atomic peer-to-peer settlement and voluntary carbon markets. Neither has an incumbent infrastructure layer. Neither is yet licensed, gated, or consolidated. This will not last. But today, the rails are unbuilt and the opportunity belongs to whoever builds them.

Twill is designed to unify these properties. TWL is scarce — 50,000,000 tokens, no exceptions. It is useful — every block contains proofs of real settlement activity. It has a floor — the reserve vault guarantees a minimum value per token. It is decentralized — 100% of supply is mined by anyone with commodity hardware. No pre-mine. No founder tokens.

The Twill consensus mechanism makes mining and settlement verification the same operation — not two systems running in parallel, but one atomic proof that satisfies both. The reserve system makes the price floor a protocol property, not a market hope. The carbon integration makes the asset compatible with the regulatory and environmental constraints of the coming decades. The governance model keeps the protocol adaptable without making it capturable.

No admin keys. No root extrinsics. No kill switches. The settlements are the consensus. The reserve is the floor. The window is now.

---

*Owned by nobody.*

**License:** This document is released under the Creative Commons Attribution 4.0 International License (CC BY 4.0).
