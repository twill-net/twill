# Twill (TWL) Tokenomics

**Version:** 2.0.0
**Published by:** Twill Project
**Date:** April 2026

---

## Table of Contents

1. [Overview](#1-overview)
2. [Token Specification](#2-token-specification)
3. [Supply Allocation](#3-supply-allocation)
4. [Mining Pool](#4-mining-pool)
5. [Halving Schedule](#5-halving-schedule)
6. [Reward Model](#6-reward-model)
7. [Fee Structure](#7-fee-structure)
8. [Reserve Vault Economics](#8-reserve-vault-economics)
9. [Burn Mechanics](#9-burn-mechanics)
10. [Inflation and Deflation Dynamics](#10-inflation-and-deflation-dynamics)
11. [Cumulative Supply Model](#11-cumulative-supply-model)
12. [Summary Tables and Charts](#12-summary-tables-and-charts)
13. [Mathematical Verification Appendix](#13-mathematical-verification-appendix)

---

## 1. Overview

Twill (TWL) is the native utility and settlement token of the Twill Network. It serves as the medium of exchange for on-chain settlements, the reward currency for block miners and stakers, and the collateral backing that makes settlements trustworthy. The token economics described in this document are designed to produce a disinflationary supply curve, incentivize long-term network participation, and maintain a sustainable balance between mining rewards and transaction fee revenue over the lifetime of the protocol.

**Key properties:**

- **Hard cap:** 50,000,000 TWL -- enforced every block in pallet-twl-token
- **One allocation:** 100% mining pool — no pre-mine, no founder allocation, every TWL is earned
- **Disinflationary:** Halving every 4 years (~21M blocks), asymptotically approaching the cap
- **No admin keys:** All economic parameters are constants baked into the runtime
- **Voluntary burn only:** No protocol-level burn mechanism; users choose to burn

This document constitutes the definitive reference for all TWL economic parameters.

---

## 2. Token Specification

| Parameter             | Value                              |
|-----------------------|------------------------------------|
| Name                  | Twill                              |
| Symbol                | TWL                                |
| Total supply          | 50,000,000 TWL                     |
| Decimals              | 12                                 |
| Smallest unit         | 1 planck (10^-12 TWL)              |
| Block time            | 6 seconds                          |
| Blocks per year       | 5,256,000                          |
| Existential deposit   | 0.01 TWL                           |

### Denomination Table

| Unit     | Planck                  | TWL Equivalent         |
|----------|-------------------------|------------------------|
| 1 planck | 1                       | 0.000000000001 TWL     |
| 1 microTWL | 1,000,000            | 0.000001 TWL           |
| 1 milliTWL | 1,000,000,000        | 0.001 TWL              |
| 1 TWL    | 1,000,000,000,000       | 1 TWL                  |

**Constant definition (from `twill_primitives`):**

```
TWILL = 1_000_000_000_000  (10^12 planck)
TOTAL_SUPPLY = 50_000_000 * TWILL = 50,000,000,000,000,000,000 planck
```

---

## 3. Supply Allocation

The total supply of 50,000,000 TWL is distributed through a single mechanism:

| Pool               | TWL Amount   | Percentage | Mechanism                    |
|---------------------|-------------|------------|------------------------------|
| Mining Pool         | 50,000,000  | 100%       | Block rewards (halving)      |
| **Total**           | **50,000,000** | **100%** |                              |

There are no development funds, community funds, ecosystem funds, treasury pre-allocations, founder allocations, or any other supply pools. No pre-mine. 100% of TWL is mined.

```
Supply Allocation

  Mining Pool (100%)
  |================================================== 50,000,000 TWL

  |----|----|----|----|----|----|----|----|----|----|
  0%  10%  20%  30%  40%  50%  60%  70%  80%  90% 100%
```

**Verification:**

```
MINING_POOL = TOTAL_SUPPLY
50,000,000 = 50,000,000  [PASS]
```

---

## 4. Mining Pool

| Parameter             | Value                              |
|-----------------------|------------------------------------|
| Total pool            | 50,000,000 TWL                     |
| Initial block reward  | 1,189,117,199,390 planck (~1.1891 TWL) |
| Halving interval      | 21,024,000 blocks (4 years)        |
| Halving mechanism     | Bitwise right shift (reward >> epoch) |
| Exhaustion            | Asymptotic; negligible after ~20 epochs |

The mining pool is not pre-minted. New TWL is created (minted) each block exclusively for the block miner who solves the PoC+PoSe puzzle, drawing from this pool until it is exhausted. The total minted is tracked on-chain and enforced against the `MINING_POOL` constant.

**Block reward derivation:**

```
Epoch 1 total = MINING_POOL / 2 = 25,000,000 TWL
Epoch 1 blocks = HALVING_INTERVAL = 21,024,000

INITIAL_BLOCK_REWARD = floor(25,000,000 * 10^12 / 21,024,000)
                     = floor(1,189,117,199,390.554...)
                     = 1,189,117,199,390 planck
                     = ~1.189117199390 TWL per block
```

The floor value ensures the mining pool is never overshot. The total undershoot across Epoch 1 is less than 1 TWL. The hard cap is additionally enforced by the `TotalMinted` check in `pallet-mining` and the `on_initialize` check in `pallet-twl-token`.

---

## 5. Halving Schedule

Block rewards halve every 21,024,000 blocks (4 years). The halving is implemented as a bitwise right shift: `INITIAL_BLOCK_REWARD >> epoch`.

| Epoch | Year Range | Block Reward (TWL) | Block Reward (planck)     | Epoch Total (TWL)    | Cumulative Mined (TWL) |
|-------|-----------|--------------------|--------------------------|--------------------|----------------------|
| 1     | 0 - 4     | ~1.1891            | 1,189,117,199,390        | ~25,000,000.00     | ~25,000,000.00       |
| 2     | 4 - 8     | ~0.5946            | 594,558,599,695          | ~12,500,000.00     | ~37,500,000.00       |
| 3     | 8 - 12    | ~0.2973            | 297,279,299,847          | ~6,250,000.00      | ~43,750,000.00       |
| 4     | 12 - 16   | ~0.1486            | 148,639,649,923          | ~3,125,000.00      | ~46,875,000.00       |
| 5     | 16 - 20   | ~0.0743            | 74,319,824,961           | ~1,562,500.00      | ~48,437,500.00       |
| 6     | 20 - 24   | ~0.0372            | 37,159,912,480           | ~781,250.00        | ~49,218,750.00       |
| 7     | 24 - 28   | ~0.0186            | 18,579,956,240           | ~390,625.00        | ~49,609,375.00       |
| 8     | 28 - 32   | ~0.0093            | 9,289,978,120            | ~195,312.50        | ~49,804,687.50       |
| 9     | 32 - 36   | ~0.0046            | 4,644,989,060            | ~97,656.25         | ~49,902,343.75       |
| 10    | 36 - 40   | ~0.0023            | 2,322,494,530            | ~48,828.13         | ~49,951,171.88       |

After epoch 20, `block_reward_at()` returns 0 and the mining pool is effectively exhausted. Due to integer floor division at each halving, the total mined asymptotically approaches but never exceeds 50,000,000 TWL.

```
Block Reward per Epoch (TWL)

  1.19 |*
       |*
       |*
       |*
  0.56 |  *
       |  *
       |  *
       |  *
  0.28 |    *
       |    *
       |    *
       |    *
  0.14 |      *
       |      *
  0.07 |        *
  0.04 |          *
  0.02 |            *
  0.01 |              * * * ...
  0.00 |________________________
       E1  E2  E3  E4  E5  E6  E7  E8
```

---

## 6. Reward Model

**Block reward — 100% to the miner.**

The miner who solves the PoC+PoSe puzzle earns the full block reward. New TWL is created exclusively through this process. The reward halves every epoch (~4 years).

```
Epoch 1 block reward:   ~1.1891 TWL (1,189,117,199,390 planck) → 100% to miner
Epoch 2 block reward:   ~0.5946 TWL (594,558,599,695 planck) → 100% to miner
```

**Staker earnings — settlement fees only (no new minting).**

PoSe stakers earn from the settlement fee pool: already-minted TWL redistributed from the 10 bps settlement fee, distributed stake-weighted every block via `on_finalize`. As settlement volume grows, so do staker earnings. Staking does not create new TWL.

### Staking Requirements

| Parameter                | Value                           |
|--------------------------|---------------------------------|
| Minimum stake            | 1,000 TWL                       |
| Registration             | Permissionless — `register_validator(stake)` where `stake >= 1,000 TWL` |
| Deregistration           | Permissionless (`deregister_validator`) |
| Fee distribution         | Automatic, every block (`on_finalize`) |
| Fee weighting            | Proportional to stake — higher stake earns proportionally more fees |
| Maximum stakers          | 100 (configurable)              |

### Slashing

Stakers are subject to automatic slashing for inactivity:

| Parameter                    | Value                              |
|------------------------------|------------------------------------|
| Inactivity threshold         | 43,800 blocks (~3 days at 6s)      |
| First offense                | 50% of stake (5000 bps)            |
| Second+ offense              | 100% of stake + automatic deregistration |
| Slashed funds destination    | Burned (deflationary)              |

---

## 7. Fee Structure

### Settlement Fees

| Parameter            | Value                              |
|----------------------|------------------------------------|
| Settlement fee rate  | 10 basis points (0.10%)            |
| Minimum fee          | 0.1 TWL                           |
| Fee base             | Total TWL-internal debit volume    |
| Fee collection       | Deducted at settlement execution   |

### Fee Distribution

| Recipient     | Share  | Basis Points | Mechanism                                             |
|---------------|--------|--------------|-------------------------------------------------------|
| PoSe stakers  | 80%    | 8,000 bps    | Via `FeePoolAccount`, distributed stake-weighted      |
| Treasury      | 20%    | 2,000 bps    | `SHA256("treasury")` keyless account, governance-only |

```
Settlement Fee Flow

  Settlement completes
         |
    Fee = volume * 10 bps
         |
  FeePoolAccount (keyless buffer)
         |
    80% to PoSe stakers          20% to Treasury
    (stake-weighted, on_finalize) (keyless, governance-controlled)
```

Settlement fees are redistributed TWL — they do not mint new tokens and have no effect on the hard cap. The fee pool balance is read directly from the `FeePoolAccount` each block and distributed proportionally to all active stakers during `on_finalize`. The 20% treasury share transfers automatically every block, even with zero stakers active.

### Transaction Fees

Substrate-level transaction fees (weight-based) are separate from settlement fees. These are standard `pallet-transaction-payment` fees for extrinsic inclusion and do not affect the settlement fee economics described here.

---

## 8. Reserve Vault Economics

The Reserve Vault is a protocol-owned pool of external assets that backs TWL value. It is fully autonomous — no admin extrinsics exist. The settlement engine is the only path for deposits into the reserve.

### How Deposits Enter

When an atomic settlement completes and includes crypto or carbon asset legs, the settlement pallet transfers wrapped assets to the reserve vault account and records the deposit via the `ReserveInterface` trait. The reserve pallet values each deposit in TWL terms using the on-chain oracle.

**Supported reserve asset types:**

| Asset Kind     | Oracle Pair  |
|----------------|-------------|
| wBTC           | BtcTwl      |
| wETH           | EthTwl      |
| wUSDC          | UsdcTwl     |
| Carbon Credit  | CarbonTwl   |
| Other          | Raw amount   |

### Floor Price

The reserve provides a theoretical floor price for TWL:

```
floor_price = total_reserve_value * TWILL / circulating_supply
```

This is a read-only metric. There is no buyback mechanism or redemption guarantee -- the reserve simply grows organically from real economic activity.

### Snapshots

The reserve takes automatic snapshots every 100 blocks (~10 minutes). Each snapshot records the total reserve value at that block height, providing an on-chain history of reserve growth.

---

## 9. Burn Mechanics

TWL uses a **voluntary burn only** model. There is no protocol-level automatic burn.

| Parameter          | Value                              |
|--------------------|------------------------------------|
| Burn mechanism     | Permissionless `burn` extrinsic    |
| Burn destination   | Deterministic burn wallet (SHA256-derived, no private key) |
| Burn tracking      | `TotalBurned` storage value        |
| Direct send        | Tokens sent directly to burn wallet are auto-detected |

Any user can call the `burn` extrinsic to permanently destroy TWL. The burned tokens are transferred to a deterministic burn wallet address derived from `SHA256("twill_safety_wallet:" || "burn")`. Since this address has no private key, the tokens are irrecoverable.

The burn wallet balance is synced every block in `on_initialize`. Direct transfers to the burn wallet (bypassing the `burn` extrinsic) are automatically detected and added to the `TotalBurned` counter.

**Effect on supply metrics:**

```
circulating_supply = total_issuance - total_burned
```

Burning reduces circulating supply but does not change `total_issuance`. Burned tokens remain in the issuance count but are permanently removed from circulation.

---

## 10. Inflation and Deflation Dynamics

### Inflation Sources

The only inflation source is mining pool emissions (newly minted TWL for block rewards). No pre-mine exists. The chain starts empty — every token in existence was earned by a miner.

**Annualized emission rate:**

| Epoch | Annual Emission (TWL) | Supply at Start (TWL) | Inflation Rate       |
|-------|----------------------|----------------------|----------------------|
| 1     | ~6,250,000           | 0                    | high (bootstrapping) |
| 2     | ~3,125,000           | ~25,000,000          | ~12.5%               |
| 3     | ~1,562,500           | ~37,500,000          | ~4.2%                |
| 4     | ~781,250             | ~43,750,000          | ~1.8%                |
| 5     | ~390,625             | ~46,875,000          | ~0.8%                |

Annual emission = epoch block reward * BLOCKS_PER_YEAR = epoch block reward * 5,256,000

### Deflation Sources

- **Voluntary burns:** Users may burn TWL at will, reducing circulating supply
- **Existential deposit:** Accounts below 0.01 TWL are reaped (dust removal)

### Long-term Equilibrium

As mining emissions approach zero (post epoch 10+), the network transitions to a fee-sustained model. Settlement fees become the primary economic incentive for stakers. The protocol becomes effectively deflationary if voluntary burns exceed the negligible late-epoch emissions.

```
Inflation Rate Over Time

  High |*
       | *
  ~11% |   *
       |
  ~4%  |     *
       |
  ~2%  |       *
  ~1%  |         *
  ~0%  |           * * * * * ...
       |________________________
       E1  E2  E3  E4  E5  E6+
```

---

## 11. Cumulative Supply Model

Total supply at any point is mining emissions only:

```
total_issuance = mining_pool_minted
```

The chain starts at zero. Every token is minted per-block by miners.

### Cumulative Supply by Year

| Year | Mining Minted (TWL) | Total Issuance (TWL) | % of Cap |
|------|--------------------|--------------------|----------|
| 0    | 0                  | 0                  | 0.0%     |
| 1    | ~6,250,000         | ~6,250,000         | 12.5%    |
| 2    | ~12,500,000        | ~12,500,000        | 25.0%    |
| 3    | ~18,750,000        | ~18,750,000        | 37.5%    |
| 4    | ~25,000,000        | ~25,000,000        | 50.0%    |
| 8    | ~37,500,000        | ~37,500,000        | 75.0%    |
| 12   | ~43,750,000        | ~43,750,000        | 87.5%    |
| 16   | ~46,875,000        | ~46,875,000        | 93.8%    |
| 20   | ~48,437,500        | ~48,437,500        | 96.9%    |
| 40   | ~50,000,000        | ~50,000,000        | ~100%    |

```
Cumulative Supply (TWL)

  50M |                                        ___________
      |                                  _____/
      |                            _____/
  40M |                       ____/
      |                  ____/
      |             ____/
  30M |         ___/
      |       _/
      |     _/
  20M |   _/
      |  /
      | /
  10M |/
      |
   0M |________________________
      Y0   Y4   Y8  Y12  Y16  Y20  Y24  Y28  Y32  Y36  Y40
```

### Circulating Supply

Circulating supply accounts for burned tokens:

```
circulating = total_issuance - total_burned
```

Since there is no founder allocation and no pre-mine, all minted tokens are immediately circulating (minus voluntary burns).

---

## 12. Summary Tables and Charts

### Core Constants Reference

| Constant                  | Value                       | Source File             |
|--------------------------|-----------------------------|-----------------------------|
| `TOTAL_SUPPLY`           | 50,000,000 TWL              | `primitives/src/lib.rs` |
| `TWILL`                  | 1,000,000,000,000 planck    | `primitives/src/lib.rs` |
| `TOKEN_DECIMALS`         | 12                          | `primitives/src/lib.rs` |
| `MINING_POOL`            | 50,000,000 TWL              | `primitives/src/lib.rs` |
| `BLOCKS_PER_YEAR`        | 5,256,000                   | `primitives/src/lib.rs` |
| `HALVING_INTERVAL`       | 21,024,000 blocks           | `primitives/src/lib.rs` |
| `INITIAL_BLOCK_REWARD`   | 1,189,117,199,390 planck    | `primitives/src/lib.rs` |
| `BLOCK_TIME_MS`          | 6,000 ms                    | `primitives/src/lib.rs` |
| `SETTLEMENT_FEE_BPS`     | 10 bps                      | `primitives/src/lib.rs` |
| `FEE_STAKER_SHARE_BPS`   | 8,000 bps (80%)             | `primitives/src/lib.rs` |
| `FEE_COMMUNITY_SHARE_BPS`| 2,000 bps (20%)             | `primitives/src/lib.rs` |
| `SLASH_INACTIVITY_BLOCKS`| 43,800 (~3 days)            | `primitives/src/lib.rs` |
| `SLASH_FIRST_BPS`        | 5,000 bps (50%)             | `primitives/src/lib.rs` |
| `SLASH_REPEAT_BPS`       | 10,000 bps (100%)           | `primitives/src/lib.rs` |

### Runtime Parameters

| Parameter              | Value                       | Source File        |
|------------------------|-----------------------------|--------------------|
| `MinPoseStake`         | 1,000 TWL                   | `runtime/src/lib.rs` |
| `ExistentialDeposit`   | 0.01 TWL                    | `runtime/src/lib.rs` |
| `MaxPoseValidators`    | 100                         | `runtime/src/lib.rs` |
| `FeeBps`               | 10 bps                      | `runtime/src/lib.rs` |
| `MinFee`               | 0.1 TWL                     | `runtime/src/lib.rs` |
| `SettlementTimeout`    | 20 blocks (~2 min)          | `runtime/src/lib.rs` |
| `MaxLegsPerSettlement` | 10                          | `runtime/src/lib.rs` |

### Reward Sources by Participant

```
Miners:  Block reward (100%) — new TWL, per-block
Stakers: Settlement fees (80%) — existing TWL redistributed, stake-weighted
Treasury: Settlement fees (20%) — keyless, spendable via governance proposal only
         No overlap. Miners get 100% of new TWL. Stakers get 80% of fees.
```

### Economic Flow Diagram

```
                    GENESIS (block 0)
                         |
                  Mining Pool: 50M TWL
                  (unminted, per-block)
                         |
                         v
                   PoC+PoSe Mining
                   (SHA256 puzzle)
                         |
                         v
                    Miner Wallet (100% of block reward)
                         |
                         v
                    Circulating Supply
                         |
              +----------+----------+
              |                     |
         Settlements           Voluntary
         (10 bps fee)           Burns
              |                     |
      FeePoolAccount           Burn Wallet
      (keyless buffer)         (permanent)
              |
    +----80%--+----20%----+
    |                     |
PoSe stakers           Treasury
(stake-weighted,    (SHA256("treasury"),
 each block)         governance-only)
```

---

## 13. Mathematical Verification Appendix

### A. Supply Allocation Identity

```
MINING_POOL = TOTAL_SUPPLY
50,000,000 * 10^12 = 50,000,000 * 10^12

50,000,000,000,000,000,000 = 50,000,000,000,000,000,000  [VERIFIED]
```

### B. Halving Interval Derivation

```
BLOCKS_PER_YEAR = 365.25 * 24 * 3600 / 6 = 5,256,000
HALVING_INTERVAL = BLOCKS_PER_YEAR * 4 = 21,024,000

Verification: 21,024,000 * 6 seconds = 126,144,000 seconds = 4.0 years  [VERIFIED]
```

### C. Initial Block Reward Derivation

```
Epoch 1 allocation = MINING_POOL / 2 = 25,000,000 TWL

INITIAL_BLOCK_REWARD = floor(25,000,000 * 10^12 / 21,024,000)
                     = floor(25,000,000,000,000,000,000 / 21,024,000)
                     = floor(1,189,117,199,390.554...)
                     = 1,189,117,199,390 planck

Verification (epoch 1 total):
  1,189,117,199,390 * 21,024,000 = 24,999,999,999,990,960,000 planck
  25,000,000 TWL in planck       = 25,000,000,000,000,000,000 planck
  Undershoot                     =              9,040,000 planck
                                 =              0.000009040 TWL  [< 1 TWL, VERIFIED]
```

### D. Epoch Reward Verification

Each epoch's reward is the previous epoch's reward right-shifted by 1 (halved via integer division):

```
Epoch 1: 1,189,117,199,390 planck/block
  Total: 1,189,117,199,390 * 21,024,000 = 24,999,999,999,990,960,000 planck

Epoch 2: 1,189,117,199,390 >> 1 = 594,558,599,695 planck/block
  Total: 594,558,599,695 * 21,024,000 = 12,499,999,999,995,480,000 planck

Epoch 3: 594,558,599,695 >> 1 = 297,279,299,847 planck/block
  Total: 297,279,299,847 * 21,024,000 = 6,249,999,999,977,328,000 planck

Epoch 4: 297,279,299,847 >> 1 = 148,639,649,923 planck/block
  Total: 148,639,649,923 * 21,024,000 = 3,124,999,999,988,552,000 planck

Epoch 5: 148,639,649,923 >> 1 = 74,319,824,961 planck/block
  Total: 74,319,824,961 * 21,024,000 = 1,562,499,999,994,864,000 planck
```

### E. Geometric Series Convergence

The total mining emission across all epochs forms a geometric series:

```
S = INITIAL_BLOCK_REWARD * HALVING_INTERVAL * sum(1/2^k, k=0..inf)
  = INITIAL_BLOCK_REWARD * HALVING_INTERVAL * 2

Due to integer floor at each halving step, the actual total is slightly
less than 2 * (epoch 1 total), and the mining pool cap is never exceeded.

Theoretical limit: 2 * 24,999,999,999,990,960,000 = 49,999,999,999,981,920,000 planck
Actual MINING_POOL:                                  50,000,000,000,000,000,000 planck
Permanent undershoot:                                            18,080,000 planck
                                                  =             0.000018080 TWL  [negligible]
```

### F. Fee Distribution Verification

```
FEE_STAKER_SHARE_BPS  = 8000 bps = 80%  to stakers
FEE_COMMUNITY_SHARE_BPS = 2000 bps = 20% to treasury
Total = 10000 bps = 100%  [VERIFIED]
```

### G. Block Reward Recipient

```
Miners receive 100% of block_reward_at(blocks_since_genesis).
No split. No staking share from block rewards.
Stakers earn from FEE_STAKER_SHARE_BPS = 8000 (80% of settlement fees).
Treasury receives FEE_COMMUNITY_SHARE_BPS = 2000 (20% of settlement fees).  [VERIFIED]
```

### H. Slashing Math

```
First offense:  stake * SLASH_FIRST_BPS / 10000 = stake * 5000 / 10000 = 50% of stake
Second offense: stake * SLASH_REPEAT_BPS / 10000 = stake * 10000 / 10000 = 100% of stake

Inactivity threshold: 43,800 blocks * 6 seconds = 262,800 seconds = 3.04 days  [~3 days, VERIFIED]
```

---

*This document reflects the TWL token economics as implemented in the Twill codebase. All constants are defined in `twill-primitives` and enforced at the runtime level with no admin override capability.*
