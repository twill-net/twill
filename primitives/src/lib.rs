//! Twill Network Primitives
//!
//! Core types shared across all Twill pallets and the runtime.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

// ---------------------------------------------------------------------------
// Token Constants
// ---------------------------------------------------------------------------

/// Total hard cap: 50,000,000 TWL
pub const TOTAL_SUPPLY: u128 = 50_000_000 * TWILL;

/// One TWL in base units (12 decimals)
pub const TWILL: u128 = 1_000_000_000_000;

/// Ticker symbol
pub const TOKEN_SYMBOL: &str = "TWL";

/// Decimal places
pub const TOKEN_DECIMALS: u8 = 12;

// ---------------------------------------------------------------------------
// Supply Allocation (in base units)
// ---------------------------------------------------------------------------

/// Mining pool: 100% = 50,000,000 TWL (mined via PoC + PoSe over ~20 years)
/// No founder pre-mine. No ICO. No dev fund. Every TWL is earned.
pub const MINING_POOL: u128 = TOTAL_SUPPLY;

// ---------------------------------------------------------------------------
// Block & Timing Constants
// ---------------------------------------------------------------------------

/// Target block time in milliseconds (6 seconds)
pub const BLOCK_TIME_MS: u64 = 6_000;

/// Blocks per year at 6s block time
pub const BLOCKS_PER_YEAR: u64 = 5_256_000;

/// Halving interval in blocks (4 years)
pub const HALVING_INTERVAL: u64 = BLOCKS_PER_YEAR * 4; // 21,024,000 blocks

/// Settlement timeout in blocks (~2 minutes at 6s)
pub const SETTLEMENT_TIMEOUT_BLOCKS: u32 = 20;

// ---------------------------------------------------------------------------
// Mining Constants
// ---------------------------------------------------------------------------

/// Initial block reward (Epoch 1): ~1.1891 TWL per block
/// 25,000,000 TWL * 10^12 / 21,024,000 blocks = 1,189,117,199,390.554...
/// Floor value. Undershoot < 0.001 TWL total. Mining pool cap enforced.
pub const INITIAL_BLOCK_REWARD: u128 = 1_189_117_199_390;

// Block rewards go 100% to the miner who solves the PoC puzzle.
// Stakers earn from settlement fees. Community pool accumulates the rest.
// No new TWL is created — fees are redistributed from existing circulation.

/// Fee distribution: 80% to PoSe stakers, 20% to community pool.
/// Community pool accumulates even when no stakers are active.
pub const FEE_STAKER_SHARE_BPS: u16 = 8_000;  // 80%
pub const FEE_COMMUNITY_SHARE_BPS: u16 = 2_000; // 20%

/// Bootstrap threshold: first 10M TWL mined with no transaction fee.
/// After this, miners pay a small fee (they'll already have TWL by then).
/// 10M = 20% of total supply — enough to build a robust mining economy
/// before fees are required.
pub const BOOTSTRAP_THRESHOLD: u128 = 10_000_000 * TWILL;

/// Slashing: inactivity threshold (~3 days at 6s blocks)
pub const SLASH_INACTIVITY_BLOCKS: u64 = 43_800;

/// First slash: 50% of stake (5000 bps)
pub const SLASH_FIRST_BPS: u16 = 5000;

/// Subsequent slashes: 100% of stake (10000 bps)
pub const SLASH_REPEAT_BPS: u16 = 10000;

/// Carbon issuance bond: 100 TWL
pub const CARBON_ISSUANCE_BOND: u128 = 100 * TWILL;

/// Carbon dispute window in blocks (~7 days)
pub const CARBON_DISPUTE_WINDOW: u32 = 100_800;

// ---------------------------------------------------------------------------
// Settlement Constants
// ---------------------------------------------------------------------------

/// Default settlement fee: 10 basis points (0.10%)
/// Atomic swaps carry no custody or counterparty risk — fees are kept low.
/// Community can adjust via governance proposal.
pub const SETTLEMENT_FEE_BPS: u16 = 10;

/// Exchange spread: 10 basis points (0.1%)
pub const EXCHANGE_SPREAD_BPS: u16 = 10;

// ---------------------------------------------------------------------------
// Asset Domain
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum AssetDomain {
    Crypto,
    Carbon,
    /// Fiat rails — activated by community governance runtime upgrade.
    /// Settlement records are on-chain; payment confirmation comes from oracle nodes
    /// monitoring the respective payment network (SEPA, ACH, SWIFT, UPI, Faster Payments).
    Fiat,
}

// ---------------------------------------------------------------------------
// Rail Kind — settlement rail types
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum RailKind {
    // Crypto
    Bitcoin,
    Ethereum,
    Solana,
    // Carbon
    Verra,
    GoldStandard,
    // Native
    TwillInternal,
    // Fiat — activated by community governance runtime upgrade.
    // Each variant represents a specific fiat payment network.
    // Off-chain settlement is confirmed by oracle nodes monitoring
    // the respective payment network before releasing the TWL leg.
    Sepa,   // EU SEPA credit transfer
    Ach,    // US ACH (Automated Clearing House)
    Swift,  // International wire (SWIFT)
    Upi,    // India Unified Payments Interface
    Faster, // UK Faster Payments
}

impl RailKind {
    pub fn domain(&self) -> AssetDomain {
        match self {
            Self::Bitcoin | Self::Ethereum | Self::Solana => AssetDomain::Crypto,
            Self::Verra | Self::GoldStandard => AssetDomain::Carbon,
            Self::TwillInternal => AssetDomain::Crypto,
            Self::Sepa | Self::Ach | Self::Swift | Self::Upi | Self::Faster => AssetDomain::Fiat,
        }
    }

}

// ---------------------------------------------------------------------------
// Settlement Types
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum LegSide {
    Debit,
    Credit,
}

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum SettlementStatus {
    Proposed,
    Locked,
    Settled,
    Refunded,
    Expired,
}

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum LegStatus {
    Pending,
    Locked,
    Claimed,
    Refunded,
}

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum HoldStatus {
    Held,
    Claimed,
    Refunded,
    Expired,
}

// ---------------------------------------------------------------------------
// Settlement Leg (bounded)
// ---------------------------------------------------------------------------

/// A single leg of an atomic settlement.
/// `Payload` is a bounded byte array for rail-specific data (account IDs, addresses, etc.)
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct SettlementLeg {
    pub leg_id: H256,
    pub exchange_id: H256,
    pub domain: AssetDomain,
    pub rail: RailKind,
    pub side: LegSide,
    /// Amount in smallest unit of the currency
    pub amount: u128,
    /// Currency code hash (keccak of "BTC", "ETH", "tCO2e", etc.)
    pub currency_hash: H256,
    pub status: LegStatus,
}

// ---------------------------------------------------------------------------
// Hashlock / HTLC
// ---------------------------------------------------------------------------

/// HTLC hashlock: SHA256 of the secret preimage
pub type Hashlock = H256;

/// Verify a preimage against a hashlock
pub fn verify_hashlock(preimage: &[u8], hashlock: &Hashlock) -> bool {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(preimage);
    let result = hasher.finalize();
    H256::from_slice(&result) == *hashlock
}

/// Compute hashlock from preimage
pub fn compute_hashlock(preimage: &[u8]) -> Hashlock {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(preimage);
    H256::from_slice(&hasher.finalize())
}

// ---------------------------------------------------------------------------
// Merkle Proof
// ---------------------------------------------------------------------------

/// Compute a SHA256 Merkle root from a list of leaf hashes
pub fn compute_merkle_root(leaves: &[H256]) -> H256 {
    use sha2::{Digest, Sha256};

    if leaves.is_empty() {
        return H256::zero();
    }
    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current_level: Vec<H256> = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        for chunk in current_level.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(chunk[0].as_bytes());
            if chunk.len() > 1 {
                hasher.update(chunk[1].as_bytes());
            } else {
                // Duplicate last element for odd count
                hasher.update(chunk[0].as_bytes());
            }
            next_level.push(H256::from_slice(&hasher.finalize()));
        }
        current_level = next_level;
    }

    current_level[0]
}

// ---------------------------------------------------------------------------
// Mining Types
// ---------------------------------------------------------------------------

/// Compute the block reward for a given block number (handles halvings)
pub fn block_reward_at(block_number: u64) -> u128 {
    let epoch = block_number / HALVING_INTERVAL;
    if epoch >= 20 {
        return 0; // Mining pool exhausted
    }
    INITIAL_BLOCK_REWARD >> epoch // Right shift = halving
}


// ---------------------------------------------------------------------------
// Reserve Types
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum ReserveAssetKind {
    BTC,
    ETH,
    SOL,
    CarbonCredit,
    Other,
}

// ---------------------------------------------------------------------------
// Carbon Types
// ---------------------------------------------------------------------------

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum CarbonRegistry {
    Verra,
    GoldStandard,
    OnChain,
}

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum CarbonStatus {
    Issued,
    Locked,
    Retired,
    Transferred,
    /// Bond was slashed by governance — credit is invalid. Permanently unusable.
    Slashed,
}

// ---------------------------------------------------------------------------
// Pallet Coupling Traits
// ---------------------------------------------------------------------------

/// Interface for the mining pallet — called by settlement pallet
/// to update settlement Merkle root and track validator activity.
/// Maximum share of block reward that governance can redirect to the treasury.
/// 1000 = 10%. Default at genesis is 0 (miners keep 100%).
pub const MINING_TREASURY_SHARE_MAX_BPS: u16 = 1_000;

pub trait MiningInterface<AccountId> {
    /// Update the settlement Merkle root used in PoC puzzle generation
    fn update_settlement_root(merkle_root: H256);

    /// Record that a validator processed a settlement (for activity tracking / slashing)
    fn record_validator_activity(validator: &AccountId);

    /// Accumulate settlement fees for distribution to validators
    fn accumulate_fee(amount: u128);

    /// Set the treasury share of block rewards (in BPS). Capped at MINING_TREASURY_SHARE_MAX_BPS.
    /// Called by governance on proposal enactment.
    fn set_treasury_mining_share(bps: u16);
}

/// No-op implementation for testing or when mining is disabled
impl<AccountId> MiningInterface<AccountId> for () {
    fn update_settlement_root(_: H256) {}
    fn record_validator_activity(_: &AccountId) {}
    fn accumulate_fee(_: u128) {}
    fn set_treasury_mining_share(_: u16) {}
}

/// Validator status check — used by oracle pallet to verify
/// that a price submitter is an active staked validator.
/// No admin needed — staking IS authorization.
pub trait ValidatorOracle<AccountId> {
    /// Check if an account is an active PoSe validator
    fn is_active_validator(who: &AccountId) -> bool;

    /// Get a validator's stake amount (for weighted operations)
    fn validator_stake(who: &AccountId) -> Option<u128>;
}

/// No-op implementation for testing
impl<AccountId> ValidatorOracle<AccountId> for () {
    fn is_active_validator(_: &AccountId) -> bool { false }
    fn validator_stake(_: &AccountId) -> Option<u128> { None }
}

/// Interface for the reserve pallet — called by settlement pallet
/// to record external asset deposits.
pub trait ReserveInterface {
    /// Record an asset deposit into the reserve vault.
    /// `original_amount` is in the asset's native units.
    /// The reserve pallet converts to TWL value via oracle internally.
    fn record_deposit(
        settlement_id: H256,
        asset_kind: ReserveAssetKind,
        original_amount: u128,
    );
}

/// No-op implementation for testing
impl ReserveInterface for () {
    fn record_deposit(_: H256, _: ReserveAssetKind, _: u128) {}
}

/// Interface for the carbon pallet — called by settlement pallet
/// to lock, transfer, and unlock on-chain carbon credits during atomic settlements.
/// The currency_hash field of a carbon leg is treated as the credit_id.
pub trait CarbonInterface<AccountId> {
    /// Lock a carbon credit for settlement escrow. Returns true on success.
    fn lock_for_settlement(credit_id: H256, owner: &AccountId) -> bool;
    /// Transfer a locked carbon credit to a new owner after settlement completes.
    fn transfer_settled(credit_id: H256, to: &AccountId) -> bool;
    /// Restore a locked carbon credit to Issued status on refund or expiry.
    fn unlock_refund(credit_id: H256) -> bool;
}

/// No-op implementation (used when carbon pallet is absent or in tests)
impl<AccountId> CarbonInterface<AccountId> for () {
    fn lock_for_settlement(_: H256, _: &AccountId) -> bool { false }
    fn transfer_settled(_: H256, _: &AccountId) -> bool { false }
    fn unlock_refund(_: H256) -> bool { false }
}

// ---------------------------------------------------------------------------
// Oracle Types
// ---------------------------------------------------------------------------

/// Asset pairs for price feeds
#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub enum AssetPair {
    BtcTwl,
    EthTwl,
    SolTwl,
    CarbonTwl,
}

/// Oracle interface — called by reserve pallet to get asset prices
pub trait OracleInterface {
    /// Get the canonical price for an asset pair (None if no data)
    fn get_price(pair: AssetPair) -> Option<u128>;

    /// Check if a price feed is stale
    fn is_stale(pair: AssetPair) -> bool;
}

/// No-op oracle for testing
impl OracleInterface for () {
    fn get_price(_: AssetPair) -> Option<u128> { None }
    fn is_stale(_: AssetPair) -> bool { true }
}

// ---------------------------------------------------------------------------
// Safety Wallets
// ---------------------------------------------------------------------------

/// Deterministic wallet address derivation for protocol-controlled accounts.
/// These use known seeds with no private key (or derived deterministically).
///
/// Safety wallet categories:
/// - Burn: permanently destroys tokens (0xFF fill)
/// - Treasury: fee collection and protocol revenue
/// - Emergency: frozen reserve for protocol emergencies
/// - Timelock: holds tokens subject to governance timelocks

/// Derive a deterministic 32-byte account ID from a purpose string.
/// Uses SHA256(b"twill_safety_wallet:" || purpose) to produce a
/// collision-resistant, deterministic address.
pub fn derive_safety_wallet(purpose: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"twill_safety_wallet:");
    hasher.update(purpose);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Well-known safety wallet purposes
pub const SAFETY_WALLET_BURN: &[u8] = b"burn";
/// Fee pool: holds settlement fees in transit before distribution to stakers.
/// Keyless — no private key, no governance control.
/// 100% in, 100% out. It is a buffer, not a treasury.
pub const SAFETY_WALLET_FEE_POOL: &[u8] = b"fee_pool";
/// Treasury: receives 20% of settlement fees and optionally a % of block rewards.
/// Governed by on-chain proposals — no individual can spend from it.
pub const SAFETY_WALLET_TREASURY: &[u8] = b"treasury";
pub const SAFETY_WALLET_EMERGENCY: &[u8] = b"emergency_reserve";
pub const SAFETY_WALLET_TIMELOCK: &[u8] = b"governance_timelock";


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_supply_is_correct() {
        assert_eq!(MINING_POOL, TOTAL_SUPPLY);
    }

    #[test]
    fn hashlock_verification_works() {
        let secret = b"twill_secret_preimage_2024";
        let hashlock = compute_hashlock(secret);
        assert!(verify_hashlock(secret, &hashlock));
        assert!(!verify_hashlock(b"wrong_secret", &hashlock));
    }

    #[test]
    fn halving_schedule_correct() {
        // Epoch 0: full reward
        assert_eq!(block_reward_at(0), INITIAL_BLOCK_REWARD);
        // Epoch 1: halved
        assert_eq!(block_reward_at(HALVING_INTERVAL), INITIAL_BLOCK_REWARD / 2);
        // Epoch 2: quartered
        assert_eq!(block_reward_at(HALVING_INTERVAL * 2), INITIAL_BLOCK_REWARD / 4);
        // Very late: zero
        assert_eq!(block_reward_at(HALVING_INTERVAL * 20), 0);
    }


    #[test]
    fn merkle_root_single_leaf() {
        let leaf = H256::from_slice(&[1u8; 32]);
        assert_eq!(compute_merkle_root(&[leaf]), leaf);
    }

    #[test]
    fn merkle_root_deterministic() {
        let leaves = vec![
            H256::from_slice(&[1u8; 32]),
            H256::from_slice(&[2u8; 32]),
        ];
        let root1 = compute_merkle_root(&leaves);
        let root2 = compute_merkle_root(&leaves);
        assert_eq!(root1, root2);
    }

    #[test]
    fn rail_kind_domain_mapping() {
        assert_eq!(RailKind::Bitcoin.domain(), AssetDomain::Crypto);
        assert_eq!(RailKind::Ethereum.domain(), AssetDomain::Crypto);
        assert_eq!(RailKind::Verra.domain(), AssetDomain::Carbon);
        assert_eq!(RailKind::TwillInternal.domain(), AssetDomain::Crypto);
    }

    #[test]
    fn safety_wallet_derivation_deterministic() {
        let w1 = derive_safety_wallet(SAFETY_WALLET_BURN);
        let w2 = derive_safety_wallet(SAFETY_WALLET_BURN);
        assert_eq!(w1, w2);
    }

    #[test]
    fn safety_wallets_are_distinct() {
        let burn = derive_safety_wallet(SAFETY_WALLET_BURN);
        let fee_pool = derive_safety_wallet(SAFETY_WALLET_FEE_POOL);
        let emergency = derive_safety_wallet(SAFETY_WALLET_EMERGENCY);
        let timelock = derive_safety_wallet(SAFETY_WALLET_TIMELOCK);
        assert_ne!(burn, fee_pool);
        assert_ne!(burn, emergency);
        assert_ne!(burn, timelock);
        assert_ne!(fee_pool, emergency);
        assert_ne!(fee_pool, timelock);
        assert_ne!(emergency, timelock);
    }

    #[test]
    fn block_reward_matches_spec() {
        // Epoch 1 = half the mining pool = 25,000,000 TWL (100% mined, 50M pool)
        let total_epoch1: u128 = INITIAL_BLOCK_REWARD * HALVING_INTERVAL as u128;
        let expected = 25_000_000 * TWILL;
        // Must be <= expected (never overshoot)
        assert!(total_epoch1 <= expected, "Epoch 1 overshoots: {} > {}", total_epoch1, expected);
        // Undershoot < 1 TWL across 21M blocks
        let undershoot = expected - total_epoch1;
        assert!(undershoot < TWILL, "Epoch 1 undershoot too large: {} planck", undershoot);
    }
}
