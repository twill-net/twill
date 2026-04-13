# Twill Network (TWL) Technical Specification

**Version:** 0.2.0-draft
**Status:** Pre-release Draft
**Authors:** Twill Project
**License:** Apache-2.0
**Date:** 2026-04-09

---

## Table of Contents

1. [Abstract](#1-abstract)
2. [Protocol Overview](#2-protocol-overview)
3. [Consensus Mechanism](#3-consensus-mechanism)
4. [Token Economics](#4-token-economics)
5. [Settlement Protocol](#5-settlement-protocol)
6. [Reserve System](#6-reserve-system)
7. [Carbon Integration](#7-carbon-integration)
8. [Future: EVM Compatibility](#8-future-evm-compatibility)
9. [Network Architecture](#9-network-architecture)
10. [Security Model](#10-security-model)
11. [Governance](#11-governance)

---

## 1. Abstract

Twill is a Substrate-based Layer 1 blockchain designed for atomic multi-asset settlement with integrated carbon accounting. The protocol introduces a unified consensus mechanism combining Proof of Compute (PoC) with Proof of Settlement (PoSe), where a block miner solves a hash puzzle that embeds the settlement Merkle root, unifying block production and settlement verification into a single atomic operation. Stakers back the settlement infrastructure with bonded TWL and earn stake-weighted settlement fees. The native token TWL operates under a hard supply cap of 50,000,000 units with 12 decimal places of precision, backed by a transparent reserve vault holding auditable crypto assets and carbon credits. This specification defines the protocol's consensus algorithms, token economics, settlement engine, reserve mechanics, carbon integration, network architecture, security model, and governance framework.

---

## 2. Protocol Overview

### 2.1 Design Principles

1. **Settlement-First Architecture.** All protocol operations orient around the settlement of real-world and digital assets. Consensus, mining, and validation exist to serve settlement correctness.

2. **Conservation of Value.** The Reserve Vault enforces a floor price for TWL by maintaining verifiable 1:1 backing for all ingested assets. No token can be minted without either mining work or a corresponding reserve deposit.

3. **Carbon as a First-Class Asset.** Tonnes of CO2-equivalent (tCO2e) enter the reserve alongside BTC and ETH. Carbon retirement is a native on-chain operation, not a wrapper around an external registry.

4. **Permissionless Participation.** Any participant with commodity hardware (GPU) can mine. Any participant with 1,000 TWL can stake to back settlements and earn rewards.

5. **Zero-Reserve Settlement.** Counterparties need not maintain standing bilateral reserves. The HTLC-based atomic settlement engine requires only time-bounded escrow during the settlement window.

6. **No Admin Keys.** After genesis the protocol runs itself. Miners submit proofs permissionlessly, stakers register and deregister permissionlessly, rewards auto-distribute, slashing is automatic, and epoch transitions are automatic.

### 2.2 System Components

```
+-----------------------------------------------------------------------+
|                          Twill Network                                |
|                                                                       |
|  +---------------------+    +---------------------+                  |
|  | PoC+PoSe Unified    |    |   PoSe Staking      |                  |
|  | Block Mining         |<-->|   Layer              |                  |
|  | (pallet-mining)      |    |   (pallet-mining)    |                  |
|  +---------------------+    +---------------------+                  |
|            |                          |                               |
|            v                          v                               |
|  +----------------------------------------------------+              |
|  |              Consensus Engine                       |              |
|  |   Block production, finality, fork choice           |              |
|  +----------------------------------------------------+              |
|            |                                                          |
|            v                                                          |
|  +----------------------------------------------------+              |
|  |              Runtime (WASM)                         |              |
|  |                                                     |              |
|  |  +----------------+  +----------------+             |              |
|  |  | pallet-        |  | pallet-        |             |              |
|  |  | settlement     |  | reserve        |             |              |
|  |  +----------------+  +----------------+             |              |
|  |  +----------------+  +----------------+             |              |
|  |  | pallet-        |  | pallet-        |             |              |
|  |  | carbon         |  | twl-token      |             |              |
|  |  +----------------+  +----------------+             |              |
|  |  +----------------+  +----------------+             |              |
|  |  | pallet-        |  | pallet-        |             |              |
|  |  | governance     |  | oracle         |             |              |
|  |  +----------------+  +----------------+             |              |
|  +----------------------------------------------------+              |
|            |                                                          |
|            v                                                          |
|  +----------------------------------------------------+              |
|  |              Networking (libp2p)                    |              |
|  |   Peer discovery, block propagation, tx gossip      |              |
|  +----------------------------------------------------+              |
+-----------------------------------------------------------------------+
```

### 2.3 Notation Conventions

| Symbol | Meaning |
|--------|---------|
| `TWL` | The native token, 1 TWL = 10^12 planck |
| `planck` | Smallest indivisible unit (10^-12 TWL) |
| `tCO2e` | Tonnes of CO2-equivalent |
| `H(x)` | Cryptographic hash function (BLAKE2b-256 unless otherwise noted) |
| `||` | Byte concatenation |
| `[n]` | Block number n |
| `B_n` | Block at height n |
| `S_n` | Settlement Merkle root at block n |
| `R_n` | Reserve state root at block n |

---

## 3. Consensus Mechanism

### 3.1 Overview

Twill uses a hybrid consensus combining two complementary mechanisms:

- **Proof of Compute (PoC) + Proof of Settlement (PoSe) Unified Block Mining:** A memory-hard mining puzzle whose hash function ingests the current settlement Merkle root. Mining a block and validating the settlement ledger are one atomic operation. The block miner earns 100% of the block reward. New TWL is created exclusively through this process.
- **Proof of Settlement (PoSe) Staking:** Participants stake TWL to back the settlement infrastructure. Staked TWL is the collateral that makes settlements trustworthy. Stakers earn settlement fees — already-minted TWL redistributed from the fee pool — proportional to their stake. Staking does not create new TWL.

The 50,000,000 TWL mining pool is exhausted entirely by GPU miners. PoC provides Sybil resistance and fair token distribution. PoSe staking provides settlement throughput guarantees and economic finality.

### 3.2 Block Parameters

| Parameter | Value |
|-----------|-------|
| Target block time | 6 seconds |
| Block size limit | 5 MiB (encoded extrinsics) |
| Max extrinsics per block | 4096 |
| Block header size | 256 bytes (fixed) |
| Epoch length | 2400 blocks (~4 hours) |
| Era length | 14400 blocks (~24 hours) |

### 3.3 Proof of Compute (PoC) -- Block Mining

#### 3.3.1 Design Goals

PoC block mining is designed to be:

1. **ASIC-resistant.** The puzzle is memory-hard, requiring random access to a large scratchpad, making custom silicon uneconomical relative to commodity GPUs.
2. **Settlement-coupled.** The mining function incorporates the settlement Merkle root, ensuring miners must possess and verify the current settlement state to produce valid blocks. Mining a block and validating settlements are one unified operation (PoC+PoSe).
3. **Difficulty-adaptive.** Difficulty adjusts per-epoch to maintain the 6-second target block time.

#### 3.3.2 Mining Algorithm: TwillHash

TwillHash is a memory-hard hash function inspired by RandomX, adapted for settlement verification coupling. It operates in three phases.

**Phase 1: Scratchpad Initialization**

The miner constructs a 2 GiB scratchpad `SP` from the block template:

```
seed = H(parent_hash || settlement_merkle_root || miner_pubkey || nonce)

SP[0] = AES-256-CTR(key=seed[0..32], iv=seed[0..16], plaintext=0^64)
for i in 1..33554432:
    SP[i] = AES-256-CTR(key=SP[i-1][0..32], iv=SP[i-1][32..48], plaintext=SP[i-1])
```

Each scratchpad entry is 64 bytes. Total: 33,554,432 entries * 64 bytes = 2 GiB.

The inclusion of `settlement_merkle_root` in the seed means the scratchpad is invalid if the settlement state is incorrect or stale. A miner who does not possess the valid settlement Merkle root cannot produce a valid scratchpad. This is the mechanism by which block mining and settlement verification are unified.

**Phase 2: Random Memory Access Computation**

The miner executes a sequence of 65,536 iterations of random reads and computations against the scratchpad:

```
state = seed
for round in 0..65536:
    addr_a = state[0..4] mod 33554432
    addr_b = state[4..8] mod 33554432
    chunk_a = SP[addr_a]
    chunk_b = SP[addr_b]

    // Floating-point and integer arithmetic mix
    fp_a = reinterpret_f64(chunk_a[0..8])
    fp_b = reinterpret_f64(chunk_b[0..8])
    int_a = reinterpret_u64(chunk_a[8..16])
    int_b = reinterpret_u64(chunk_b[8..16])

    result_fp = sqrt(abs(fp_a * fp_b + fp_a)) XOR reinterpret_u64(fp_b)
    result_int = (int_a * int_b) XOR rotate_left(int_a, int_b mod 64)

    mixed = H(result_fp || result_int || chunk_a[16..48] || chunk_b[16..48])

    // Write back to scratchpad (read-write, prevents caching optimizations)
    SP[addr_a] = mixed || chunk_a[0..32]
    SP[addr_b] = chunk_b[32..64] || mixed

    state = H(state || mixed)
```

The random addressing pattern, combined with scratchpad writes, ensures that:
- The full 2 GiB must be resident in memory (no time-memory tradeoff below ~1.5 GiB).
- Computation cannot be pipelined across nonces without independent scratchpads.
- Both floating-point and integer units are exercised, resisting FPGA optimization.

**Phase 3: Final Hash**

```
output = BLAKE2b-256(state || SP[state[0..4] mod 33554432] || settlement_merkle_root)
```

The block is valid if:

```
output <= target_difficulty
```

#### 3.3.3 Difficulty Adjustment

Difficulty adjusts at each epoch boundary (every 2400 blocks). The algorithm targets a 6-second average block time.

```
expected_duration = 2400 * 6 = 14400 seconds
actual_duration = timestamp(B_epoch_end) - timestamp(B_epoch_start)

adjustment_ratio = expected_duration / actual_duration

// Clamp to prevent extreme swings
adjustment_ratio = clamp(adjustment_ratio, 0.75, 1.25)

new_difficulty = current_difficulty * adjustment_ratio

// Minimum difficulty floor
new_difficulty = max(new_difficulty, MIN_DIFFICULTY)
```

Where `MIN_DIFFICULTY` is set such that the easiest valid hash requires at minimum 2^20 hash evaluations on reference hardware.

#### 3.3.4 Settlement Merkle Root Construction

The settlement Merkle root `S_n` included in the PoC puzzle is computed from all pending and recently-finalized settlement transactions:

```
S_n = MerkleRoot(
    sorted_by_hash([
        H(htlc.id || htlc.hashlock || htlc.timelock || htlc.amount || htlc.status)
        for htlc in active_settlements
    ])
)
```

The Merkle tree uses SHA-256 and is constructed as a binary tree with sorted leaf insertion. Empty trees produce `S_n = H256::zero()`.

This root is included in the block header and is a required input to TwillHash. Miners must maintain a synchronized view of the settlement mempool to compute valid proofs. The settlement root embedding is what makes block mining simultaneously a proof of settlement correctness.

#### 3.3.5 Block Header (PoC Fields)

```rust
struct PoCHeader {
    nonce: u64,
    twill_hash_output: [u8; 32],
    settlement_merkle_root: [u8; 32],
    miner_pubkey: AccountId,  // sr25519
    difficulty_target: U256,
}
```

### 3.4 Proof of Settlement (PoSe) -- Staking

#### 3.4.1 Staker Requirements

To participate as a PoSe staker, an account must satisfy:

| Requirement | Value |
|-------------|-------|
| Minimum stake | 1,000 TWL |
| Maximum staker set size | 100 (genesis), adjustable via governance |
| Registration | Permissionless (call `register_staker`) |
| Deregistration | Permissionless (call `deregister_staker`) |

Stakers register by calling `register_staker`, which reserves `MinPoseStake` (1,000 TWL) from their free balance. Registration is permissionless -- any account meeting the minimum stake can participate. Stakers deregister by calling `deregister_staker`, which unreserves their stake.

#### 3.4.2 Staking Mechanics

Stakers back the settlement infrastructure with bonded TWL. Their stake serves as collateral that makes settlements trustworthy. In return, stakers earn:

1. **Settlement fees:** Accumulated settlement fees (10 basis points per settlement) are distributed to stakers proportional to their stake. This is already-minted TWL redistributed from the fee pool — no new minting.

```rust
struct PoseValidator<T: Config> {
    pub account: T::AccountId,
    pub stake: BalanceOf<T>,
    pub registered_at: BlockNumberFor<T>,
}
```

#### 3.4.3 Staking Reward Distribution

Settlement fees are distributed automatically in `on_finalize` each block. The protocol reads the `FeePoolAccount` balance directly, splits 80% to stakers stake-weighted and 20% to the treasury:

```
fee_pool_balance = FeePoolAccount.free_balance()
treasury_share   = fee_pool_balance * FEE_COMMUNITY_SHARE_BPS / 10_000
staker_pool      = fee_pool_balance - treasury_share
for each staker in active_validator_set:
    staker_reward = staker_pool * (staker.stake / total_staked)
```

No new TWL is minted — this is redistribution of existing tokens from the fee pool account.

#### 3.4.4 Inactivity Slashing

Stakers are subject to automatic slashing for inactivity. The `LastActiveBlock` storage tracks each staker's last observed activity:

| Parameter | Value |
|-----------|-------|
| Inactivity threshold | ~3 days (43,800 blocks at 6s) |
| First offense slash | 50% of stake (5,000 bps) |
| Second+ offense slash | 100% of stake (10,000 bps) + auto-deregistration |

```rust
// Inactivity detection runs in on_finalize
let blocks_inactive = now - LastActiveBlock::<T>::get(&staker);
if blocks_inactive > SLASH_INACTIVITY_BLOCKS {
    let offense = SlashCount::<T>::get(&staker);
    let slash_bps = if offense == 0 { SLASH_FIRST_BPS } else { SLASH_REPEAT_BPS };
    let slash_amount = stake * slash_bps / 10_000;
    // Slash reserved funds, increment offense count
    // Second+ offense: auto-deregister staker
}
```

Slashed tokens are burned — deflationary, not redirected to any account.

### 3.5 Block Reward Model

Block rewards go 100% to the miner who solves the PoC+PoSe puzzle. New TWL is created exclusively through GPU mining. The reward halves every `HALVING_INTERVAL` blocks (~4 years):

```rust
pub fn block_reward_at(blocks_since_genesis: u64) -> u128 {
    let epoch = blocks_since_genesis / HALVING_INTERVAL;
    INITIAL_BLOCK_REWARD >> epoch
}
```

Stakers earn settlement fees only — existing, already-minted TWL redistributed from the fee pool. As settlement volume grows over time, staker earnings grow proportionally.

### 3.6 Fork Choice Rule

Twill uses a heaviest-chain fork choice rule where chain weight incorporates both PoC difficulty and PoSe attestation density:

```
chain_weight(chain) = sum(
    poc_difficulty(B_i) + pose_attestation_count(B_i) * ATTESTATION_WEIGHT
    for B_i in chain
)
```

Where `ATTESTATION_WEIGHT = median_difficulty(last_epoch) / MAX_VALIDATORS`. This ensures PoSe attestations contribute meaningfully to fork choice without dominating PoC work.

### 3.7 Finality

Blocks achieve instant finality under the permissionless instant-seal consensus. Each valid block submitted via `submit_poc_proof` is immediately sealed and finalized. There is no probabilistic confirmation period and no separate finality gadget:

1. A miner submits a valid PoC+PoSe proof via `submit_poc_proof`.
2. The block is sealed immediately upon successful proof verification.
3. Finality is instant — once sealed, a block cannot be reverted.
4. No GRANDPA, no Aura, no authority set. Finality is a function of proof validity, not committee votes.

---

## 4. Token Economics

### 4.1 Token Parameters

| Parameter | Value |
|-----------|-------|
| Token name | Twill |
| Token symbol | TWL |
| Decimal places | 12 |
| Smallest unit name | planck |
| Hard supply cap | 50,000,000 TWL |
| Existential deposit | 0.01 TWL (10^10 planck) |

### 4.2 Supply Allocation

| Allocation | Amount (TWL) | Percentage | Distribution Mechanism |
|------------|-------------|------------|----------------------|
| Mining Pool | 50,000,000 | 100% | Block rewards (PoC+PoSe unified mining), halving schedule |

There is one allocation. No development fund. No community fund. No founder allocation. No pre-mine. Every TWL is earned:

```
MINING_POOL == TOTAL_SUPPLY
50,000,000 == 50,000,000
```

### 4.3 Mining Pool Emission Schedule

The 50,000,000 TWL mining pool follows a halving schedule with halvings every 4 years (21,024,000 blocks).

**Per-block reward calculation:**

```
blocks_per_halving_period = 4 * 5_256_000 = 21_024_000

pub fn block_reward_at(block_number: u64) -> u128 {
    let epoch = block_number / HALVING_INTERVAL;
    if epoch >= 20 {
        return 0;  // Mining pool exhausted
    }
    INITIAL_BLOCK_REWARD >> epoch  // right shift = divide by 2^epoch
}
```

Where `INITIAL_BLOCK_REWARD = 1,189,117,199,390` planck (~1.189 TWL per block).

Derivation: The first halving period emits half the mining pool. `25,000,000 TWL * 10^12 planck / 21,024,000 blocks = 1,189,117,199,390.554...` planck. The floor value is used; total undershoot is less than 0.001 TWL. The mining pool cap is enforced in the pallet to prevent overshoot.

**Emission table:**

| Halving Period | Years | Per-Block Reward (TWL) | Period Emission (TWL) | Cumulative (TWL) |
|---------------|-------|----------------------|---------------------|-----------------|
| 0 | 0 -- 4 | ~1.189117199390 | 25,000,000 | 25,000,000 |
| 1 | 4 -- 8 | ~0.594558599695 | 12,500,000 | 37,500,000 |
| 2 | 8 -- 12 | ~0.297279299847 | 6,250,000 | 43,750,000 |
| 3 | 12 -- 16 | ~0.148639649923 | 3,125,000 | 46,875,000 |
| 4 | 16 -- 20 | ~0.074319824961 | 1,562,500 | 48,437,500 |
| 5+ | 20+ | <0.04 | <781,250 | approaching 50,000,000 |

The geometric series converges: `25,000,000 * (1 + 1/2 + 1/4 + ...) = 25,000,000 * 2 = 50,000,000`. The full 50M pool is asymptotically exhausted. After approximately 20 years, > 99% of the mining pool will have been emitted. Residual sub-planck amounts are rounded down (truncated), ensuring the hard cap is never exceeded. The mining pallet enforces `remaining = MINING_POOL.saturating_sub(total_minted)` and caps the actual reward to `remaining`.

### 4.4 Transaction Fees

Transaction fees are denominated in TWL and computed as:

```
fee = base_fee + (weight_fee * extrinsic_weight) + length_fee + tip

base_fee = 0.001 TWL (fixed)
weight_fee = 0.000_000_001 TWL per unit of weight
length_fee = 0.000_001 TWL per byte of encoded extrinsic
tip = optional, user-specified priority fee
```

**Bootstrap Period (first 10M TWL):** Mining proof submissions (`submit_poc_proof_unsigned`) are fee-free during the bootstrap period. The chain starts with zero balances — no pre-mine, no endowed accounts — so miners cannot pay fees until TWL is in circulation. During bootstrap, the proof-of-work itself serves as spam protection: invalid proofs are rejected at the transaction pool level before entering a block. After 10,000,000 TWL has been mined (~20% of total supply), miners switch to signed submissions with standard transaction fees. By that point, miners already hold TWL from earlier blocks.

```rust
pub const BOOTSTRAP_THRESHOLD: u128 = 10_000_000 * TWILL; // 20% of supply
```

Fee distribution: 80% of settlement fees go to PoSe stakers, 20% to the treasury. Distribution reads `FeePoolAccount` balance directly each block — no separate counter.

```rust
pub const FEE_STAKER_SHARE_BPS: u16 = 8_000;    // 80% to stakers
pub const FEE_COMMUNITY_SHARE_BPS: u16 = 2_000; // 20% to treasury
```

### 4.6 Burn Mechanism

There is no protocol-level automatic burn. However, the protocol maintains a canonical burn address (a deterministic, keyless account derived from `SHA256("twill/burn")`):

```rust
pub BurnAccount: AccountId = AccountId::new(
    twill_primitives::derive_safety_wallet(twill_primitives::SAFETY_WALLET_BURN)
);
```

Any TWL sent to this address is permanently removed from circulating supply. Burns are voluntary and irreversible. The `pallet-twl-token` module tracks cumulative burns and synchronizes on every block in `on_initialize`:

```rust
#[pallet::storage]
pub type TotalBurned<T: Config> = StorageValue<_, u128, ValueQuery>;
```

The effective circulating supply is:

```
circulating_supply = total_issued - total_burned
```

---

## 5. Settlement Protocol

### 5.1 Overview

The Twill Settlement Engine implements atomic multi-asset settlement using Hash Time-Locked Contracts (HTLCs). It enables trustless exchange of heterogeneous assets — cryptocurrency, carbon credits — without requiring standing bilateral reserves between counterparties. Only time-bounded escrow accounts are needed during the atomic settlement window. Settlement fees are 10 basis points (0.10%) per settlement.

### 5.2 Settlement Data Structures

```rust
/// Settlement status lifecycle
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SettlementStatus {
    Proposed,
    Locked,
    Settled,
    Refunded,
    Expired,
}

/// Leg status within a settlement
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum LegStatus {
    Pending,
    Locked,
    Claimed,
    Refunded,
}

/// Asset domains
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AssetDomain {
    Crypto,
    Carbon,
}

/// Settlement rail types
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RailKind {
    // Crypto
    Bitcoin, Ethereum, Solana,
    // Carbon
    Verra, GoldStandard,
    // Native
    TwillInternal,
}

/// A single leg of an atomic settlement.
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct SettlementLeg {
    pub leg_id: H256,
    pub exchange_id: H256,
    pub domain: AssetDomain,
    pub rail: RailKind,
    pub side: LegSide,  // Debit or Credit
    pub amount: u128,
    pub currency_hash: H256,  // keccak of "BTC", "ETH", "tCO2e", etc.
    pub status: LegStatus,
}

/// HTLC hashlock: SHA256 of the secret preimage
pub type Hashlock = H256;
```

### 5.3 Settlement Parameters

| Parameter | Value |
|-----------|-------|
| Settlement fee | 10 basis points (0.10%) |
| Minimum fee | 0.1 TWL |
| Maximum legs per settlement | 10 |
| Settlement timeout | 20 blocks (~2 minutes) |
| Maximum payload per leg | 1,024 bytes |
| Maximum expiries per block | 50 |

### 5.4 Settlement Lifecycle

#### 5.4.1 Phase 1: Proposal

The initiating party submits a `propose_settlement` extrinsic:

```rust
#[pallet::call]
fn propose_settlement(
    origin: OriginFor<T>,
    exchange_id: H256,
    legs: BoundedVec<SettlementLeg, T::MaxLegsPerSettlement>,
    hashlock: H256,
    timeout_blocks: BlockNumberFor<T>,
) -> DispatchResult;
```

**Validation rules:**
- `legs.len() >= 1` (minimum one leg).
- `legs.len() <= 10` (maximum legs per settlement).
- `timeout_blocks` is clamped to `[20, 14_400]` — minimum 20 blocks (~2 min) to maximum 14,400 blocks (~24 hours). Choose based on off-chain settlement requirements: BTC/ETH legs need 60–600 blocks; TWL-only or carbon-only can use 20.
- Every settlement must include at least one `TwillInternal` (TWL) leg — direct cross-asset swaps without a TWL leg are rejected.
- `hashlock` must not match any active settlement's hashlock (collision prevention).
- Settlement fee (10 bps)

**Settlement ID:** Each settlement is identified by its `exchange_id` (an H256 hash).

#### 5.4.2 Phase 2: Locking

Each participant locks their leg by confirming participation:

```rust
#[pallet::call]
fn lock_leg(
    origin: OriginFor<T>,
    exchange_id: H256,
    leg_index: u32,
    payload: BoundedVec<u8, T::MaxPayloadSize>,
) -> DispatchResult;
```

On-chain assets (TWL) are reserved from the sender's account upon locking.

#### 5.4.3 Phase 3: Settlement (Secret Reveal)

The party holding the preimage reveals it to settle:

```rust
#[pallet::call]
fn settle(
    origin: OriginFor<T>,
    exchange_id: H256,
    preimage: BoundedVec<u8, ConstU32<64>>,
) -> DispatchResult;
```

**Validation:**
```
SHA256(preimage) == htlc.hashlock
current_block <= htlc.timelock
htlc.status == Locked
```

Upon successful settlement:
1. All on-chain escrowed assets are released to their respective receivers.
2. The settlement fee (10 bps) is transferred to the `FeePoolAccount` (keyless buffer), where it accumulates for automatic distribution each block.
3. The settlement's Merkle leaf is updated and included in the next block's settlement Merkle root via `CurrentSettlementRoot` storage.
4. A `SettlementCompleted` event is emitted.

#### 5.4.4 Phase 4: Refund / Expiry

If the preimage is not revealed before the timelock expires:

```rust
#[pallet::call]
fn refund_settlement(
    origin: OriginFor<T>,
    exchange_id: H256,
) -> DispatchResult;
```

Expired settlements are also automatically cleaned up in `on_initialize` (up to `MaxExpiryPerBlock = 50` per block). All escrowed on-chain assets are returned to their original senders.

### 5.5 Multi-Leg Atomic Swaps

The settlement engine supports multi-party, multi-asset atomic swaps with up to 10 legs where all legs succeed or all legs fail. Consider a three-party carbon-for-crypto swap:

```
Leg 0: Alice sends 100 tCO2e to Bob        (rail: Verra, domain: Carbon)
Leg 1: Bob sends 2.5 ETH to Carol          (rail: Ethereum, domain: Crypto)
Leg 2: Carol sends 0.1 BTC to Alice        (rail: Bitcoin, domain: Crypto)
```

All three legs share the same `hashlock`. Alice generates the preimage `p`, computes `hashlock = SHA256(p)`, and creates the settlement. Bob and Carol lock their legs. Alice reveals `p` to settle, atomically executing all three legs.

### 5.6 HTLC Hashlock Verification

```rust
pub fn verify_hashlock(preimage: &[u8], hashlock: &Hashlock) -> bool {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(preimage);
    let result = hasher.finalize();
    H256::from_slice(&result) == *hashlock
}
```

### 5.7 Settlement Storage

```rust
#[pallet::storage]
pub type Settlements<T: Config> =
    StorageMap<_, Blake2_128Concat, H256, Settlement<T>, OptionQuery>;

#[pallet::storage]
pub type SettlementLegs<T: Config> =
    StorageDoubleMap<_, Blake2_128Concat, H256, Blake2_128Concat, u32, Leg<T>, OptionQuery>;

#[pallet::storage]
pub type PendingExpiries<T: Config> =
    StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, BoundedVec<H256, MaxExpiryPerBlock>, ValueQuery>;
```

---

## 6. Reserve System

### 6.1 Overview

The Reserve Vault is a protocol-managed asset pool that provides a verifiable floor price for TWL. Assets entering the vault are tracked 1:1, and the ratio of total reserve value to circulating TWL supply defines the minimum exchange value. The market price of TWL floats above this floor based on demand.

### 6.2 Reserve Vault Data Structures

```rust
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReserveAssetKind {
    BTC,
    ETH,
    SOL,
    CarbonCredit,
}

/// Reserve deposit record
struct ReserveDeposit<T: Config> {
    settlement_id: H256,
    asset_kind: ReserveAssetKind,
    value_twl: u128,
    deposited_at: BlockNumberFor<T>,
}
```

### 6.3 Reserve Deposit Protocol

Assets enter the reserve vault through settlement completion. When a settlement completes that involves reserve-eligible assets, the reserve pallet records the deposit:

```rust
#[pallet::call]
fn record_deposit(
    origin: OriginFor<T>,
    settlement_id: H256,
    asset_kind: ReserveAssetKind,
    value_twl: u128,
) -> DispatchResult;
```

The reserve pallet tracks total reserve value and emits periodic snapshots:

```rust
#[pallet::event]
pub enum Event<T: Config> {
    ReserveDeposited {
        settlement_id: H256,
        asset_kind: ReserveAssetKind,
        value_twl: u128,
        total_reserve: u128,
    },
    ReserveSnapshot {
        block_number: BlockNumberFor<T>,
        total_value: u128,
    },
}
```

### 6.4 Floor Price Calculation

The floor price is recomputed at every era boundary (every 14400 blocks, ~24 hours) and cached:

```
floor_price = total_reserve_value / circulating_supply

where:
    circulating_supply = total_issued - total_burned
```

All values use 18-decimal fixed-point representation to avoid floating-point precision issues.

### 6.5 Oracle System

The reserve valuation depends on external price feeds for off-chain assets. The oracle system operates as follows:

```rust
#[derive(Encode, Decode, Clone, TypeInfo)]
struct OracleSubmission {
    asset_pair: AssetPair,      // BtcTwl, EthTwl, UsdcTwl, CarbonTwl
    price_twl: u128,            // 18 decimal precision, TWL-denominated
    timestamp: u64,
    oracle_id: AccountId,
    signature: sr25519::Signature,
}
```

**Aggregation:**
- Each oracle submits a price every era.
- The protocol uses the median of all submissions (resistant to single-oracle manipulation).
- A price is accepted only if >= `ceil(n/2) + 1` oracles submit within the era.
- Outlier rejection: submissions deviating more than 10% from the median are discarded and the oracle is flagged.

**Staleness protection:**
- If no valid oracle update is received for 3 consecutive eras, the reserve uses the last known price with a 5% discount applied per missed era (conservative valuation).
- If no update for 10 eras, reserve deposits are paused until oracle connectivity is restored.

### 6.6 Reserve Vault Storage

```rust
#[pallet::storage]
pub type ReserveDeposits<T: Config> =
    StorageMap<_, Blake2_128Concat, H256, ReserveDeposit<T>, OptionQuery>;

#[pallet::storage]
pub type TotalReserveValue<T: Config> =
    StorageValue<_, u128, ValueQuery>;

#[pallet::storage]
pub type ReserveByKind<T: Config> =
    StorageMap<_, Blake2_128Concat, ReserveAssetKind, u128, ValueQuery>;

#[pallet::storage]
pub type MaxReserveAssets: u32 = 20;
```

### 6.7 Reserve Withdrawal

Reserve assets may only be withdrawn under governance-approved circumstances:

1. **Redemption:** TWL holders burn TWL to withdraw pro-rata reserve assets. This maintains the floor price for remaining holders.
2. **Rebalancing:** Governance may approve swapping reserve assets (e.g., converting wUSDC to wBTC) without changing the total value.
3. **Emergency:** A supermajority governance vote (> 75% of staked TWL) can authorize extraordinary withdrawals.

Redemption formula:

```
redeemable_value(twl_amount) = twl_amount * floor_price
```

---

## 7. Carbon Integration

### 7.1 Overview

Twill treats carbon credits (measured in tonnes of CO2-equivalent, tCO2e) as first-class on-chain assets. Carbon credits can be issued, traded, settled atomically, deposited into the reserve vault, and permanently retired via an on-chain retirement operation. This enables TWL to be partially backed by carbon credits, creating a direct linkage between the token's value and climate mitigation assets.

### 7.2 Carbon Credit Data Model

```rust
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CarbonRegistry {
    Verra,
    GoldStandard,
    OnChain,
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CarbonStatus {
    Issued,
    Locked,
    Retired,
}

struct CarbonCredit<T: Config> {
    credit_id: H256,
    owner: T::AccountId,
    registry: CarbonRegistry,
    project_id: BoundedVec<u8, T::MaxProjectIdLength>,
    vintage_year: u16,
    amount: u128,          // micro-tCO2e (10^-6)
    status: CarbonStatus,
    issued_at: BlockNumberFor<T>,
}
```

### 7.3 Carbon Constants

| Parameter | Value |
|-----------|-------|
| Issuance bond | 100 TWL |
| Dispute window | 100,800 blocks (~7 days) |
| Maximum issuance amount | 1,000,000 tCO2e (in micro units) |
| Maximum project ID length | 128 bytes |

### 7.4 Carbon Credit Lifecycle

#### 7.4.1 Issuance

Carbon credits are issued on-chain with an issuance bond:

```rust
#[pallet::call]
fn issue_credit(
    origin: OriginFor<T>,
    credit_id: H256,
    registry: CarbonRegistry,
    project_id: BoundedVec<u8, T::MaxProjectIdLength>,
    vintage_year: u16,
    amount: u128,
) -> DispatchResult;
```

The issuer must deposit `CARBON_ISSUANCE_BOND` (100 TWL) as collateral, refundable after the dispute window (100,800 blocks, ~7 days) if no dispute is raised.

#### 7.4.2 Trading

Tokenized carbon credits are tradeable via:
- Direct transfer (`transfer_credit` extrinsic).
- Atomic settlement (as a leg in the settlement engine with `RailKind::Verra` or `RailKind::GoldStandard`).

#### 7.4.3 Reserve Deposit

Carbon credits can be deposited into the Reserve Vault through the reserve pallet, with their TWL-equivalent value (via Carbon/TWL oracle pricing) added to the reserve total.

#### 7.4.4 Retirement

Retirement is a permanent, irreversible on-chain operation:

```rust
#[pallet::call]
fn retire_credit(
    origin: OriginFor<T>,
    credit_id: H256,
    amount: u128,
    certificate_id: H256,
) -> DispatchResult;
```

**Retirement semantics:**
1. The specified amount is subtracted from the credit's balance.
2. The credit's status transitions to `Retired`.
3. A `CreditRetired` event is emitted with the full retirement details.
4. The retirement is recorded in an append-only on-chain retirement ledger.

### 7.5 Carbon Events

```rust
#[pallet::event]
pub enum Event<T: Config> {
    CreditIssued {
        credit_id: H256, owner: T::AccountId,
        registry: CarbonRegistry, amount: u128, vintage_year: u16,
    },
    CreditLocked { credit_id: H256, amount: u128 },
    CreditUnlocked { credit_id: H256, amount: u128 },
    CreditRetired {
        credit_id: H256, certificate_id: H256,
        retiree: T::AccountId, amount: u128,
    },
    CreditTransferred {
        credit_id: H256, from: T::AccountId,
        to: T::AccountId, amount: u128,
    },
}
```

### 7.6 Carbon Pricing Oracle

Carbon credits are valued in TWL using the Carbon/TWL oracle pair, following the same oracle infrastructure as other reserve assets (Section 6.5). Prices are submitted per-registry and per-vintage:

```
carbon_price_key = H(registry_id || vintage_year)
```

The protocol maintains a price matrix rather than a single carbon price, reflecting the real-world variance between registries and vintages.

### 7.7 Carbon Credit Storage

```rust
#[pallet::storage]
pub type Credits<T: Config> =
    StorageMap<_, Blake2_128Concat, H256, CarbonCredit<T>, OptionQuery>;

#[pallet::storage]
pub type CreditsByOwner<T: Config> =
    StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, H256, (), OptionQuery>;

#[pallet::storage]
pub type RetirementCertificates<T: Config> =
    StorageMap<_, Blake2_128Concat, H256, RetirementCertificate<T>, OptionQuery>;

#[pallet::storage]
pub type TotalRetired<T: Config> =
    StorageValue<_, u128, ValueQuery>;
```

---

## 8. Future: EVM Compatibility

EVM compatibility via the Substrate Frontier framework is planned for a future protocol upgrade. When implemented, it will enable Solidity and Vyper smart contract deployment on Twill alongside the native Substrate pallets. EVM integration will be proposed and approved through the on-chain governance process described in Section 11.

Until then, all settlement, carbon, reserve, and staking operations are performed exclusively through the native Substrate extrinsic interface.

---

## 9. Network Architecture

### 9.1 Node Types

| Node Type | Role | Requirements |
|-----------|------|--------------|
| Full Node | Stores complete blockchain state, validates blocks, serves RPC | 8 GiB RAM, 500 GiB SSD, 100 Mbps |
| Archive Node | Full node + historical state at every block | 16 GiB RAM, 2 TiB SSD, 100 Mbps |
| Mining Node | Full node + PoC mining (TwillHash) | 16 GiB RAM (2 GiB for scratchpad), GPU (4+ GiB VRAM), 500 GiB SSD |
| Staker Node | Full node + PoSe staking + settlement fee earning | 8 GiB RAM, 500 GiB SSD, 100 Mbps, high uptime (99.5%+), 1,000+ TWL stake |
| Light Client | Block header verification, Merkle proof validation | 512 MiB RAM, minimal storage |

### 9.2 Networking

Twill uses libp2p for all peer-to-peer communication:

| Protocol | Transport | Purpose |
|----------|-----------|---------|
| `/twill/block-announces/1` | Notification | New block announcements |
| `/twill/transactions/1` | Notification | Transaction propagation |
| `/twill/settlement-attestations/1` | Notification | PoSe settlement attestations |
| `/twill/sync/2` | Request-response | Block and state synchronization |
| `/twill/light/2` | Request-response | Light client requests |
| `/twill/kad` | Kademlia DHT | Peer discovery |

**Peer limits:**
- Default max peers: 50 (full/archive), 25 (light)
- Reserved peers: up to 10 (configured by node operator)
- Ban duration for misbehaving peers: 30 minutes (progressive)

### 9.3 Block Structure

```
+--------------------------------------------------------------+
| Block Header (256 bytes fixed)                                |
|--------------------------------------------------------------|
| parent_hash:          [u8; 32]    // BLAKE2b-256             |
| block_number:         u64                                    |
| state_root:           [u8; 32]    // Merkle-Patricia trie    |
| extrinsics_root:      [u8; 32]    // Merkle root of extrs   |
| digest:               DigestItems                            |
|   - poc_header:       PoCHeader   // nonce, hash, difficulty |
|   - settlement_root:  [u8; 32]    // settlement Merkle root  |
|   - reserve_root:     [u8; 32]    // reserve state root      |
|   - seal:             sr25519::Signature                     |
+--------------------------------------------------------------+
| Block Body                                                   |
|--------------------------------------------------------------|
| inherents:                                                   |
|   - timestamp                                                |
|   - settlement_merkle_root_update                            |
| signed_extrinsics:                                           |
|   - user transactions (transfers, settlements, etc.)         |
|   - oracle price submissions                                 |
|   - governance votes                                         |
|   - settlement attestations                                  |
+--------------------------------------------------------------+
```

### 9.4 State Storage

The Twill state is stored in a Merkle-Patricia trie (same as Substrate default) with the following key prefixes:

| Prefix | Pallet | Contents |
|--------|--------|----------|
| `0x26aa394eea5630e07c48ae0c9558cef7` | System | Account nonces, block info |
| `0xc2261276cc9d1f8598ea4b6a74b15c2f` | Balances | TWL balances |
| `0x...settlement` | Settlement | HTLC state, Merkle roots |
| `0x...reserve` | Reserve | Vault assets, oracle prices |
| `0x...carbon` | Carbon | Credit registry, retirements |
| `0x...mining` | Mining | Difficulty, epoch state, stakers |
| `0x...governance` | Governance | Proposals, votes, tallies |

### 9.5 Chain Specification (Genesis)

The genesis configuration includes:

```json
{
    "name": "Twill Network",
    "id": "twill_mainnet",
    "chainType": "Live",
    "bootNodes": [],
    "protocolId": "twl",
    "properties": {
        "tokenSymbol": "TWL",
        "tokenDecimals": 12,
        "ss58Format": 42
    },
    "genesis": {
        "runtime": {
            "system": { "code": "<wasm_runtime_blob>" },
            "balances": {
                "balances": []
            },
            "twlToken": {
                "initialize": true
            },
            "mining": {
                "initial_difficulty": "0x0000000000000000000000000000000000000000000000000000ffffffffffff",
                "blocks_per_epoch": 2400,
                "blocks_per_halving": 21024000,
                "initial_reward_planck": 1189117199390,
                "max_pose_validators": 100,
                "min_pose_stake_twl": 1000
            },
            "settlement": {
                "max_legs_per_settlement": 10,
                "settlement_timeout_blocks_min": 20,
                "settlement_timeout_blocks_max": 14400,
                "fee_bps": 10
            },
            "reserve": {
                "oracle_set": [],
                "oracle_threshold": 3,
                "staleness_threshold_eras": 3,
                "max_reserve_assets": 20
            },
            "carbon": {
                "issuance_bond_twl": 100,
                "dispute_window_blocks": 100800,
                "max_project_id_length": 128
            },
            "governance": {
                "board_size": 5
            }
        }
    }
}
```

Note: Genesis balances are empty. No pre-mine. No founder allocation. The mining pool is emitted block-by-block via the mining pallet. The fee pool account is a deterministic keyless account derived from SHA-256 of `"fee_pool"` — a transient buffer that holds settlement fees until distributed to stakers.

### 9.6 Runtime Upgrades

The Twill runtime is compiled to WebAssembly (WASM) and stored on-chain. Runtime upgrades require a two-step governance process:

**Step 1 — Authorize (on-chain vote):**
1. Submitter publishes the new WASM build and its `blake2_256` hash off-chain.
2. A `RuntimeUpgrade { code_hash }` governance proposal is submitted with only the hash (not the full WASM blob).
3. TWL holders vote. If approved (> 50% participating, quorum met), the hash is written to `ApprovedUpgrade` storage after the 7-day enactment delay.

**Step 2 — Apply (permissionless extrinsic):**
4. Any account calls `governance.apply_upgrade(wasm_bytes)`.
5. The pallet verifies `blake2_256(wasm_bytes) == ApprovedUpgrade`. If mismatch, the call is rejected.
6. On success, `set_code` is invoked with root privileges; all nodes execute the new runtime on the next block.

No hard fork is required. If the WASM is not submitted within a reasonable window the governance proposal can be re-run with a corrected hash.

---

## 10. Security Model

### 10.1 Threat Model

| Threat | Mitigation |
|--------|-----------|
| 51% hashrate attack | Instant-seal finality makes reversion impossible once a valid proof is accepted. Each block is independently verified and sealed. |
| Settlement front-running | Settlement IDs are deterministic from HTLC parameters; preimage secrecy prevents front-running. HTLC claim transactions are prioritized by the block producer. |
| Oracle manipulation | Median aggregation, outlier rejection, multi-oracle threshold, staked oracle collateral subject to slashing. |
| Staker collusion | Staker selection weighted by stake; automatic inactivity slashing (~3 days threshold, 50% first / 100% second). |
| ASIC centralization | TwillHash memory-hard design (2 GiB scratchpad) with mixed arithmetic. Annual review of ASIC resistance with governance-approved algorithm adjustments if needed. |
| Reserve insolvency | 1:1 backing with on-chain verifiability. Floor price computation is deterministic from on-chain state. Oracle staleness triggers conservative valuation discounts. |
| Double-spend (settlement) | HTLC atomicity guarantees. On-chain escrow prevents double-spend of on-chain assets. Off-chain legs rely on hash-preimage revelation for cross-chain atomicity. |
| Long-range attacks | Instant-seal finality eliminates reorg risk. Each block is independently sealed upon valid proof submission. |

### 10.2 Slashing Conditions

PoSe stakers are subject to slashing for inactivity:

| Violation | Slash Amount | Threshold |
|-----------|-------------|-----------|
| Inactivity (first offense) | 50% of stake (5,000 bps) | ~3 days inactive (43,800 blocks) |
| Inactivity (second+ offense) | 100% of stake (10,000 bps) + auto-deregistration | ~3 days inactive after first slash |

Slashing is automatic: the mining pallet checks `LastActiveBlock` in `on_finalize` and slashes stakers who exceed the inactivity threshold. The slash count is tracked per staker in `SlashCount` storage.

```rust
// From primitives
pub const SLASH_INACTIVITY_BLOCKS: u64 = 43_800;  // ~3 days
pub const SLASH_FIRST_BPS: u16 = 5000;             // 50%
pub const SLASH_REPEAT_BPS: u16 = 10000;            // 100%
```

Slashed tokens are burned — deflationary, not redirected to any account.

### 10.3 Cryptographic Primitives

| Usage | Algorithm | Security Level |
|-------|-----------|---------------|
| Hashing (general) | BLAKE2b-256 | 128-bit |
| HTLC hashlock | SHA-256 | 128-bit (SHA-256 for cross-chain compatibility) |
| Settlement Merkle tree | SHA-256 | 128-bit (for cross-chain compatibility) |
| Mining (TwillHash) | Custom (see 3.3.2) | Memory-hard, 128-bit output |
| Account keys (Substrate) | sr25519 (Schnorrkel/Ristretto) | ~128-bit |
| Account keys (EVM) | secp256k1 ECDSA | ~128-bit |
| State trie | BLAKE2b-256 Merkle-Patricia | 128-bit |
| Block seal | sr25519 signature | ~128-bit |
| Safety wallet derivation | SHA-256 | 128-bit |

### 10.4 Network Security Parameters

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Finality threshold | > 2/3 staker stake | BFT standard; tolerates < 1/3 Byzantine |
| Oracle threshold | > 1/2 oracle set | Simple majority for price feeds |
| Governance approval | > 50% participating stake | Democratic majority |
| Emergency governance | > 75% total stake | Supermajority for extraordinary actions |
| Inactivity slash threshold | 43,800 blocks (~3 days) | Ensures staker liveness |
| Maximum finality lag | 100 blocks (~10 min) | Prevents unbounded reorgs |

### 10.5 Denial of Service Protections

1. **Transaction fees:** All extrinsics require TWL fees, preventing spam. Exception: during the bootstrap period (first 10M TWL mined), mining proof submissions are unsigned and fee-free — the proof-of-work itself is the anti-spam mechanism.
2. **Block weight limits:** Each block has a maximum execution weight, preventing computational DoS.
3. **Settlement limits:** Maximum 10 legs per settlement; 20-block timeout; 50 max auto-expiries per block.
4. **Peer scoring:** libp2p peer reputation system bans peers that send invalid blocks, stale transactions, or excessive traffic.
6. **Staker set cap:** Maximum 100 stakers prevents unbounded reward distribution overhead.

---

## 11. Governance

### 11.1 Overview

Twill governance enables TWL holders to propose and vote on protocol changes. Voting weight is phase-dependent: equal-weight (1 address = 1 vote) during the bootstrap phase (0–10M TWL mined), then automatically switches to stake-weighted (1 TWL = 1 vote, capped at 100K TWL per address) once 10M TWL has been mined.

### 11.2 Governance Parameters

| Parameter | Value |
|-----------|-------|
| Proposal deposit | None (quorum is the spam filter) |
| Voting period | 7 days (100,800 blocks at 6s per block = 604,800 seconds) |
| Enactment delay | 7 days (after approval, before execution) |
| Approval threshold | > 50% of participating stake |
| Emergency threshold | > 75% of total stake |
| Proposal cooldown | 1 era (24h) between proposals from same account |
| Maximum active proposals | 10 |

### 11.3 Proposal Types

```rust
#[derive(Encode, Decode, Clone, TypeInfo)]
enum ProposalKind {
    /// Authorize a runtime upgrade. Stores the WASM blake2_256 hash on-chain.
    /// Anyone can then call `apply_upgrade(wasm_bytes)` to complete the upgrade.
    RuntimeUpgrade { code_hash: H256 },

    /// Recall a seated board member by account ID.
    /// Requires 75% emergency threshold.
    BoardRecall { member: AccountId },

    /// Emergency reserve action (rebalance, asset adjustment).
    /// Requires 75% emergency threshold.
    EmergencyReserveAction,

    /// Enable EVM compatibility (Frontier). Marks the upgrade path for Solidity support.
    ActivateEvm,

    /// Set the share of block rewards routed to the treasury (max 10%).
    SetMiningTreasuryShare { bps: u16 },

    /// Set board member pay rate (planck per block, divided equally among seated members).
    SetBoardPay { amount_per_block: Balance },

    /// Switch to TWL-weighted voting before the automatic 10M TWL threshold.
    SwitchToTwlWeightedVoting,

    /// Set the per-address cap on vote weight in TWL-weighted phase.
    SetMaxVoteWeight { twl_amount: u128 },

    /// Transfer TWL from the treasury to a beneficiary.
    TreasurySpend { beneficiary: AccountId, amount: Balance },
}
```

**Thresholds:**
- Standard proposals: > 50% Aye of participating weight, quorum ≥ 10%
- Emergency proposals (`BoardRecall`, `EmergencyReserveAction`): > 75% Aye of total issuance

### 11.4 Voting Mechanism

```rust
#[derive(Encode, Decode, Clone, TypeInfo)]
struct Vote {
    voter: AccountId,
    proposal_id: u32,
    direction: VoteDirection,
    weight: Balance,  // TWL staked for this vote
}

#[derive(Encode, Decode, Clone, TypeInfo)]
enum VoteDirection {
    Aye,
    Nay,
    Abstain,  // counted for quorum but not for/against
}
```

**Vote weight:** Phase-dependent. During the bootstrap phase (0–10M TWL mined, ~20% of total supply), voting is equal-weight: 1 address = 1 vote. This prevents early large miners from capturing governance before broad distribution exists. Once 10M TWL has been mined, the chain auto-switches to stake-weighted voting: 1 TWL = 1 vote, capped at 100,000 TWL per address. The cap prevents whale concentration while still giving larger stakeholders proportionally more say than equal-weight. Total balance (free + reserved) counts toward vote weight; tokens in active settlement escrow do not.

**Quorum requirement:** A proposal is valid only if total participating stake (Aye + Nay + Abstain) >= 10% of circulating supply.

### 11.5 Board Elections

Board elections are triggered via the permissionless `start_election()` extrinsic:

- **First election:** No preconditions. Any account can call `start_election()` at any time to bootstrap the initial board. Since TWL is not yet circulating at genesis, voting is equal-weight (1 address = 1 vote).
- **Subsequent elections:** `start_election()` can be called once the seated board's 5-year term has elapsed. The chain enforces the term length on-chain; early calls are rejected.
- Candidates nominate themselves (100 TWL deposit for post-genesis elections).
- Voting runs for the standard voting period; the top 5–7 candidates by vote weight are seated.

### 11.6 Governance Lifecycle

```
1. Proposal Submission
   - Any TWL holder calls `submit_proposal(kind)` (no deposit required)
   - Proposal enters 'Pending' state

2. Voting Period (7 days)
   - TWL holders vote Aye/Nay/Abstain via `vote(proposal_id, direction)`
   - Weight: equal (1 address = 1 vote) during bootstrap; TWL-weighted after 10M mined

3. Tally
   - If quorum met AND Aye > Nay: Approved
   - If quorum met AND Nay >= Aye: Rejected
   - If quorum not met: Expired

4. Enactment Delay (7 days)
   - Approved proposals wait 7 days before execution
   - Allows participants to adjust positions if they disagree

5. Execution
   - Proposal enacted automatically at the scheduled block
   - RuntimeUpgrade: stores code_hash → anyone calls apply_upgrade(wasm) to complete
   - TreasurySpend: transfer executed if treasury has sufficient balance
   - Parameter changes: storage values updated immediately
```

### 11.6 Governance Storage

```rust
#[pallet::storage]
pub type Proposals<T: Config> =
    StorageMap<_, Blake2_128Concat, u32, Proposal<T>, OptionQuery>;

#[pallet::storage]
pub type ProposalCount<T: Config> =
    StorageValue<_, u32, ValueQuery>;

#[pallet::storage]
pub type Votes<T: Config> =
    StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, AccountId, Vote, OptionQuery>;

#[pallet::storage]
pub type ProposalTally<T: Config> =
    StorageMap<_, Blake2_128Concat, u32, Tally, OptionQuery>;
```

### 11.7 Future Governance Extensions

The following governance features are under consideration for future protocol upgrades:

1. **Conviction Voting:** Lock tokens for longer periods to amplify vote weight (1x -- 6x multiplier for 0 -- 6 lock periods).
2. **Delegated Voting:** Token holders delegate their voting power to representatives.
3. **Technical Committee:** An elected body of protocol experts for fast-track emergency proposals.
4. **Proposal Bounties:** Community members earn TWL for implementing approved proposals.
5. **Sub-DAOs:** Specialized governance bodies for carbon policy, reserve management, and EVM standards.

---

## Appendix A: Constants Reference

| Constant | Value | Unit |
|----------|-------|------|
| `BLOCK_TIME_MS` | 6,000 | milliseconds |
| `EPOCH_LENGTH` | 2,400 | blocks |
| `ERA_LENGTH` | 14,400 | blocks |
| `BLOCKS_PER_YEAR` | 5,256,000 | blocks |
| `HALVING_INTERVAL` | 21,024,000 | blocks (4 years) |
| `TOTAL_SUPPLY` | 50,000,000 | TWL |
| `MINING_POOL` | 50,000,000 | TWL |
| `INITIAL_BLOCK_REWARD` | 1,189,117,199,390 | planck (~1.189 TWL) |
| `BOOTSTRAP_THRESHOLD` | 10,000,000 | TWL (fee-free mining until reached) |
| `TOKEN_DECIMALS` | 12 | - |
| `PLANCK_PER_TWL` | 1,000,000,000,000 | planck |
| `MIN_POSE_STAKE` | 1,000 | TWL |
| `MAX_POSE_VALIDATORS` | 100 | - |
| `SLASH_INACTIVITY_BLOCKS` | 43,800 | blocks (~3 days) |
| `SLASH_FIRST_BPS` | 5,000 | bps (50%) |
| `SLASH_REPEAT_BPS` | 10,000 | bps (100%) |
| `FEE_STAKER_SHARE_BPS` | 8,000 | bps (80% of fees to stakers) |
| `FEE_COMMUNITY_SHARE_BPS` | 2,000 | bps (20% of fees to treasury) |
| `SETTLEMENT_FEE_BPS` | 10 | bps (0.10%) |
| `SETTLEMENT_TIMEOUT_BLOCKS` | 20 | blocks (~2 min) |
| `MAX_LEGS_PER_SETTLEMENT` | 10 | - |
| `CARBON_ISSUANCE_BOND` | 100 | TWL |
| `CARBON_DISPUTE_WINDOW` | 100,800 | blocks (~7 days) |
| `SCRATCHPAD_SIZE` | 2 | GiB |
| `SCRATCHPAD_ENTRIES` | 33,554,432 | - |
| `TWILL_HASH_ROUNDS` | 65,536 | - |
| `EXISTENTIAL_DEPOSIT` | 0.01 | TWL |
| `PROPOSAL_DEPOSIT` | 100 | TWL |
| `VOTING_PERIOD` | 302,400 | seconds (7 days) |
| `ENACTMENT_DELAY` | 302,400 | seconds (7 days) |

## Appendix B: Event Reference

All protocol-significant state transitions emit events queryable via RPC and indexable by block explorers.

| Pallet | Event | Fields |
|--------|-------|--------|
| Mining | `BlockMined` | `miner, reward, block_number` |
| Mining | `FeesDistributed` | `fee_reward, staker_count, block_number` |
| Mining | `StakerRegistered` | `staker, stake` |
| Mining | `StakerDeregistered` | `staker` |
| Mining | `StakerSlashed` | `staker, amount, offense_number, auto_deregistered` |
| Mining | `EpochChanged` | `epoch, new_block_reward` |
| Mining | `MiningPoolExhausted` | `total_minted, final_block` |
| Mining | `SettlementRootUpdated` | `merkle_root` |
| Mining | `DifficultyAdjusted` | `old_difficulty, new_difficulty, block_number` |
| Settlement | `SettlementProposed` | `exchange_id, proposer, hashlock, timelock_block` |
| Settlement | `LegLocked` | `exchange_id, leg_index, participant, amount` |
| Settlement | `SettlementCompleted` | `exchange_id, settler, merkle_root, total_volume` |
| Settlement | `SettlementRefunded` | `exchange_id, refunder` |
| Settlement | `SettlementExpired` | `exchange_id` |
| Reserve | `ReserveDeposited` | `settlement_id, asset_kind, value_twl, total_reserve` |
| Reserve | `ReserveSnapshot` | `block_number, total_value` |
| Carbon | `CreditIssued` | `credit_id, owner, registry, amount, vintage_year` |
| Carbon | `CreditLocked` | `credit_id, amount` |
| Carbon | `CreditUnlocked` | `credit_id, amount` |
| Carbon | `CreditRetired` | `credit_id, certificate_id, retiree, amount` |
| Carbon | `CreditTransferred` | `credit_id, from, to, amount` |
| Token | `Burned` | `from, amount, total_burned` |
| Governance | `ProposalSubmitted` | `proposal_id, proposer, type` |
| Governance | `VoteCast` | `proposal_id, voter, direction, weight` |
| Governance | `ProposalApproved` | `proposal_id, aye_weight, nay_weight` |
| Governance | `ProposalRejected` | `proposal_id, aye_weight, nay_weight` |
| Governance | `ProposalEnacted` | `proposal_id, block_number` |

## Appendix C: RPC Methods

In addition to standard Substrate JSON-RPC methods, Twill exposes custom RPC endpoints:

| Method | Parameters | Returns |
|--------|-----------|---------|
| `twill_getFloorPrice` | `()` | `{ floor_price, total_reserve_value, circulating_supply }` |
| `twill_getSettlement` | `(exchange_id)` | Full settlement state |
| `twill_getActiveSettlements` | `(account_id)` | List of active settlement IDs for account |
| `twill_getSettlementMerkleProof` | `(exchange_id)` | Merkle inclusion proof for settlement |
| `twill_getMiningInfo` | `()` | `{ difficulty, epoch, reward, hashrate_estimate }` |
| `twill_getReserveState` | `()` | Full reserve vault state |
| `twill_getCarbonCredit` | `(credit_id)` | Carbon credit details |
| `twill_getRetirementHistory` | `(account_id)` | List of carbon retirements by account |
| `twill_getStakerSet` | `()` | Active PoSe staker set with stakes |
| `twill_getGovernanceProposal` | `(proposal_id)` | Proposal details and current tally |
| `twill_getStakerInfo` | `(account_id)` | Staker details, slash count, last active block |
| `twill_getRemainingPool` | `()` | Remaining mining pool TWL |

---

*This specification is a living document maintained by the Twill Project. Protocol changes are governed by on-chain governance as described in Section 11.*
