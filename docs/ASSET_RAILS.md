# Twill — Supported Assets & Settlement Rails

This document is the definitive reference for what assets Twill can move and swap,
what is live today, and exactly what the community needs to do to activate each rail.

---

## Overview

Twill settles any asset against any other asset atomically. A settlement is a set of legs.
Each leg carries a specific asset on a specific rail. All legs succeed or all refund.
No counterparty risk. No custodian. No intermediary.

TWL is the on-chain settlement currency. Every cross-asset swap involves TWL as the
common ledger. Carbon bonds require TWL. The reserve vault grows from TWL inflows.
The floor price rises as the network settles more.

---

## Asset Status Table

| Asset | Rail | Domain | Status | What's Needed to Activate |
|-------|------|--------|--------|--------------------------|
| TWL | TwillInternal | Crypto | **LIVE** | Nothing. Works now. |
| Bitcoin (BTC) | Bitcoin | Crypto | **Protocol ready** | Oracle nodes + bridge infrastructure |
| Ethereum (ETH) | Ethereum | Crypto | **Protocol ready** | Oracle nodes + bridge infrastructure |
| Solana (SOL) | Solana | Crypto | **Protocol ready** | Oracle nodes + bridge infrastructure |
| USDC | Ethereum | Crypto | **Protocol ready** | Oracle nodes + bridge infrastructure |
| Carbon (tCO2e) — Verra | Verra | Carbon | **Protocol ready** | Oracle nodes + Verra registry contact |
| Carbon (tCO2e) — Gold Standard | GoldStandard | Carbon | **Protocol ready** | Oracle nodes + Gold Standard contact |
| EUR (SEPA) | Sepa | Fiat | **Governance upgrade** | Community vote to activate + oracle nodes monitoring SEPA confirmations |
| USD (ACH) | Ach | Fiat | **Governance upgrade** | Community vote to activate + oracle nodes monitoring ACH confirmations |
| International wire | Swift | Fiat | **Governance upgrade** | Community vote to activate + oracle nodes monitoring SWIFT confirmations |
| INR (UPI) | Upi | Fiat | **Governance upgrade** | Community vote to activate + oracle nodes monitoring UPI confirmations |
| GBP (Faster Payments) | Faster | Fiat | **Governance upgrade** | Community vote to activate + oracle nodes monitoring Faster Payments confirmations |

**"Protocol ready"** means the settlement engine, oracle pallet, and reserve pallet are
fully implemented and compiled. The on-chain code is complete. The off-chain infrastructure
(oracles submitting prices, bridges confirming external transactions) must be stood up by
the community and board.

---

## What Works Right Now (No Setup Required)

### TWL ↔ TWL
Direct on-chain transfer. Settlement legs use `RailKind::TwillInternal`.
Fees: 0.10% of debit volume. Atomic. Final. Immediate.

### Carbon Credit Lifecycle (on-chain)
Anyone can issue a carbon credit by posting a **100 TWL bond**.
- Credit enters a 7-day dispute window.
- If unchallenged, bond is returned and credit is active.
- If challenged and found fraudulent, governance slashes the bond (burns TWL).
- Active credits can be locked, traded via settlement, or permanently retired.

The carbon credit data is entirely on-chain. The `verification_hash` in each credit
is the issuer's proof that the underlying registry entry exists.

### TWL ↔ Carbon Atomic Swap
A settlement with two legs:
- Leg A: TWL debit (buyer sends TWL)
- Leg B: Carbon credit leg (seller delivers tCO2e)

Works today. No bridge needed. Both assets are native to Twill.

---

## What Needs Community Activation

### 1. Oracle Nodes (Required for BTC, ETH, Carbon pricing)

The oracle pallet is live. Oracle nodes are not yet running.

**What oracles do:**
- Submit BTC/TWL, ETH/TWL, USDC/TWL, and Carbon/TWL price pairs on-chain each era.
- A price is accepted when a consensus majority (⌈n/2⌉ + 1) of oracle nodes agree.
- Oracle nodes must have at least 1,000 TWL staked (same as PoSe validators).

**What the community must do:**
1. Run oracle nodes — anyone with 1,000 TWL staked can submit prices.
2. Source reliable price feeds (Chainlink, CoinGecko, exchange APIs).
3. Establish a minimum of 3 independent oracle operators before the reserve valuation is considered reliable.

**Without oracles:**
- BTC/ETH legs still settle (the HTLC logic doesn't require price oracles).
- Reserve vault valuation is zero until prices are submitted.
- TWL floor price calculation requires oracle input.

---

### 2. Bitcoin Bridge (Required for BTC settlement at scale)

The settlement engine records Bitcoin legs atomically. The actual BTC transfer happens
on the Bitcoin network. The on-chain proof is the settlement record.

**How it works (current design):**
1. Alice locks BTC in a Bitcoin HTLC address (generated from the settlement hashlock).
2. Alice creates a settlement on Twill referencing that Bitcoin leg.
3. When the settlement executes, the HTLC on Bitcoin releases simultaneously.

**What the community must do — Phase 1 (custodial bridge):**
1. Engage a qualified custodian (BitGo, Anchorage, Fireblocks) to hold reserve BTC.
2. The custodian confirms BTC deposits via API — oracle nodes relay confirmation on-chain.
3. Publish the reserve BTC address publicly. Anyone can verify on-chain.

**What the community must do — Phase 2 (trustless bridge):**
1. Implement a threshold-signature (FROST/MPC) Bitcoin bridge.
2. Requires governance approval and a separate security audit.
3. Replaces the custodial model. No single party controls reserve BTC.

The settlement pallet is already architected for this. No code changes needed for Phase 1.

---

### 3. Ethereum Bridge (Required for ETH/USDC settlement)

Same pattern as Bitcoin. The Ethereum side uses ERC-20 HTLC contracts.

**What the community must do:**
1. Deploy the Twill HTLC contract on Ethereum mainnet (governance proposal + vote).
2. Engage custodian or implement threshold-sig bridge for reserve ETH/USDC.
3. Oracle nodes confirm Ethereum transaction finality (12 block confirmations minimum).

---

### 4. Carbon Registry Integration (Required for off-chain credit verification)

On-chain carbon credits work today. The gap is linking the `verification_hash` in each
on-chain credit to the actual registry entry at Verra or Gold Standard.

**Current state:**
- Anyone can issue an on-chain carbon credit with any `verification_hash`.
- The dispute window + bond mechanism is the economic defense against fraud.
- A challenger who proves a credit is fraudulent triggers governance to slash the bond.

**What the board must do:**

**Step 1 — Contact Gold Standard Foundation**
- Start here. More technically progressive than Verra.
- Goal: API access to confirm that a `verification_hash` corresponds to a real, unretired VER.
- Contact: goldstandard.org → Registry Services

**Step 2 — Contact Verra (VCS Program)**
- Largest registry (~75% of voluntary market).
- Goal: same API access to confirm VCUs.
- Contact: registryservices@verra.org

**Step 3 — Define the retirement loop**
When a credit is retired on Twill, the corresponding registry entry must be marked retired.
Three options (board chooses):
- (a) Registry API webhook — Twill retirement event triggers direct registry API call.
- (b) Oracle-confirmed retirement — Oracle nodes confirm the registry retirement and submit on-chain.
- (c) Periodic batch reconciliation — Human-reviewed batch submission to registry monthly.

Option (b) is the most trustless. Option (a) requires a formal partnership. Option (c) is the
quickest to launch.

**Step 4 — First live settlement**
A real delivery-versus-payment swap of a carbon credit on Twill, witnessed by a registry
partner, proves the system works end-to-end. This is the milestone that unlocks institutional interest.

---

## Full Cross-Border Swap: BTC ↔ Carbon (Example)

This is the flagship use case. It requires both bridges and oracle nodes to be running.

```
Alice has 0.1 BTC. She wants 10 tCO2e (Verra).
Bob has 10 tCO2e. He wants 0.1 BTC.

1. Alice and Bob agree on the swap off-chain.
2. Alice creates a settlement on Twill with hashlock H(secret).
3. Legs:
   - Leg 0: Alice debits 0.1 BTC (rail: Bitcoin) — locks BTC HTLC on Bitcoin network
   - Leg 1: Alice credits 10 tCO2e (rail: Verra) — expects to receive carbon
   - Leg 2: Bob debits 10 tCO2e (rail: Verra) — locks carbon credit on Twill
   - Leg 3: Bob credits 0.1 BTC (rail: Bitcoin) — expects to receive BTC
4. Alice reveals the preimage. All four legs execute simultaneously.
   - Alice receives Bob's carbon credit on Twill.
   - Bob receives Alice's BTC on the Bitcoin network via HTLC.
5. If either side doesn't lock, the timelock expires and both refund.
```

No bank. No broker. No SWIFT. No central counterparty. No custody of both assets by any single party.

**What makes this possible today:** The settlement engine, carbon pallet, and oracle pallet are live.
**What the community activates:** Oracle price feeds and the Bitcoin HTLC confirmation layer.

---

## Protocol-Controlled Accounts

These are keyless deterministic accounts. No private key exists for any of them.
They are controlled entirely by the protocol — no human can spend from them directly.
The community should monitor these addresses to verify fees are flowing correctly.

### Fee Pool Account
**Derivation:** `SHA256("fee_pool")`

Holds settlement fees as a buffer before distributing the staker share (80%) to
active PoSe validators each block. **When there are no active stakers**, the staker
portion remains in this account accumulating until validators register. The 20%
treasury share is always transferred to the treasury regardless of staker status.

**Community action required:** None to activate. The account exists from genesis.
The board should publish the derived SS58 address so anyone can monitor it on-chain.

### Treasury Account
**Derivation:** `SHA256("treasury")`

Receives 20% of all settlement fees automatically every block — **even when there
are zero stakers active**. Also receives any governance-voted share of block rewards
(default 0%, community can vote up to 10%). Accumulates from day one of settlement activity.

**Community action required:**
1. The board must publish the derived SS58 address so anyone can verify the balance.
2. Governance proposes how to spend accumulated funds (audits, development, bounties).
3. Community votes on any disbursement. No human can spend from this account without a passed governance proposal executing a runtime call.

To derive the addresses yourself (verify them independently):
```rust
// In any Substrate environment:
use sp_core::crypto::Ss58Codec;
let treasury = twill_primitives::derive_safety_wallet(b"treasury");
let fee_pool = twill_primitives::derive_safety_wallet(b"fee_pool");
// Convert to SS58 AccountId for on-chain lookup
```

### Reserve Vault Account
**Derivation:** `SHA256("reserve_vault")` (managed by pallet-reserve)

Holds deposited reserve assets (wBTC, wETH, wUSDC, carbon credits). The reserve
value is publicly auditable on-chain at all times via `pallet_reserve::TotalReserveValue`.

---

## Security Note: ForceOrigin on Assets Pallet

Auditors will find one `ForceOrigin = EnsureRoot` in the runtime — on the **Assets pallet only**, which manages bridge-wrapped assets (wBTC, wETH, wUSDC). This is a custodial bridge admin key for Phase 1, required to mint/burn wrapped asset representations when the custodial bridge processes deposits and withdrawals.

**What it controls:** Only the Assets pallet — creation and management of wrapped asset tokens.
**What it cannot do:** It has no access to settlement logic, staked funds, mining rewards, the treasury, the fee pool, or the reserve vault. It cannot pause the chain, cancel settlements, or modify governance.

This key is a known Phase 1 trust assumption. Phase 2 replaces it with a trustless bridge (e.g. threshold-signature multisig or light-client proof). The community should treat it as a temporary custodial risk and prioritize the Phase 2 trustless bridge upgrade.

**The settlement pallets, mining pallet, and governance pallet have zero privileged origins.** All calls require a signed user account or are triggered automatically by the protocol itself.

---

## Community Activation Priority Order

| Priority | Action | Unlocks |
|----------|--------|---------|
| 1 | Publish pool account SS58 addresses | Transparency, community monitoring |
| 2 | Run oracle nodes (3+ independent) | Reserve valuation, floor price, Carbon/TWL pricing |
| 3 | Engage Verra / Gold Standard | Verified carbon credit issuance at scale |
| 4 | Deploy custodial BTC bridge | BTC ↔ TWL and BTC ↔ Carbon settlements |
| 5 | Deploy custodial ETH/USDC bridge | ETH ↔ TWL and ETH ↔ Carbon settlements |
| 6 | First live carbon registry settlement | Institutional credibility, partnership proof |
| 7 | Governance vote to activate fiat rails (SEPA, ACH, etc.) | Fiat ↔ TWL ↔ Carbon cross-border settlements |
| 8 | Transition to trustless MPC bridges | Full decentralization of reserve custody |

Each activation is a governance proposal + community vote. The protocol is ready.
The community decides when and how to activate each rail.

---

*This document is maintained by the board. Update it as rails are activated.*
