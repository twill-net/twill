//! # Twill Network Runtime
//!
//! The runtime is the state transition function of the Twill blockchain.
//! Standalone L1 — composes all pallets (settlement, reserve, mining,
//! carbon, token) into a working chain.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  Twill Runtime                       │
//! ├─────────────────────────────────────────────────────┤
//! │  pallet-settlement  │ pallet-reserve │ pallet-carbon │
//! │  (HTLC atomic swaps)│ (reserve vault)│ (tCO2e mgmt)  │
//! ├─────────────────────────────────────────────────────┤
//! │  pallet-mining (PoC + PoSe unified mining)           │
//! │  pallet-twl-token (cap, vesting, burn)               │
//! ├─────────────────────────────────────────────────────┤
//! │  pallet-balances │ pallet-timestamp │ pallet-tx-pay  │
//! ├─────────────────────────────────────────────────────┤
//! │  frame-system (accounts, block production, events)   │
//! └─────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU32, ConstU8, ConstU128},
    weights::{
        constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
        Weight,
    },
};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    create_runtime_str, generic,
    traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
use twill_primitives::*;

/// Opaque types for the node. The runtime is opaque to the node.
pub mod opaque {
    use super::*;
    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;

    // No session keys — Twill uses permissionless PoC mining, not authority-based consensus.
    sp_runtime::impl_opaque_keys! {
        pub struct SessionKeys {}
    }
}

/// WASM binary — available when compiled with `std` feature
#[cfg(feature = "std")]
pub fn wasm_binary() -> Option<&'static [u8]> {
    WASM_BINARY
}

/// Alias for the signature scheme used by the runtime.
pub type Signature = MultiSignature;

/// AccountId — derived from the signature scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance type — u128 for TWL (12 decimals, 100M cap fits easily).
pub type Balance = u128;

/// Block number type.
pub type BlockNumber = u32;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data.
pub type Hash = H256;

/// Block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Signed extras for transaction validation.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<sp_runtime::MultiAddress<AccountId, ()>, RuntimeCall, Signature, SignedExtra>;

/// Block type used by the runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

// ---------------------------------------------------------------------------
// Runtime Version
// ---------------------------------------------------------------------------

#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
    spec_name: create_runtime_str!("twill"),
    impl_name: create_runtime_str!("twill-node"),
    authoring_version: 1,
    spec_version: 101,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Block time: 6 seconds
const MILLISECS_PER_BLOCK: u64 = 6000;
const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

/// Block weights: ~2 seconds of compute per block
const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX);

/// Block length: 5 MiB
const MAXIMUM_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Parameter Types
// ---------------------------------------------------------------------------

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: sp_version::RuntimeVersion = VERSION;
    pub const SS58Prefix: u8 = 42;

    // Block limits
    pub RuntimeBlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::builder()
            .base_block(Weight::from_parts(5_000_000, 0))
            .for_class(frame_support::dispatch::DispatchClass::all(), |weights| {
                weights.base_extrinsic = Weight::from_parts(125_000, 0);
            })
            .for_class(frame_support::dispatch::DispatchClass::Normal, |weights| {
                weights.max_total = Some(
                    MAXIMUM_BLOCK_WEIGHT.saturating_mul(3).saturating_div(4)
                );
            })
            .for_class(frame_support::dispatch::DispatchClass::Operational, |weights| {
                weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            })
            .build_or_panic();

    pub RuntimeBlockLength: frame_system::limits::BlockLength =
        frame_system::limits::BlockLength::max(MAXIMUM_BLOCK_LENGTH);

    // Balances
    pub const ExistentialDeposit: Balance = TWILL / 100; // 0.01 TWL

    // Timestamp
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;

    // Settlement
    pub const MaxLegsPerSettlement: u32 = 10;
    pub const MaxPayloadSize: u32 = 1024; // 1 KiB per leg payload
    pub const SettlementTimeout: BlockNumber = SETTLEMENT_TIMEOUT_BLOCKS;
    pub const FeeBps: u16 = SETTLEMENT_FEE_BPS;
    pub const MinFee: Balance = TWILL / 10; // 0.1 TWL minimum fee
    pub const MaxExpiryPerBlock: u32 = 50;

    // Mining
    pub const MaxPoseValidators: u32 = 100;
    pub const MinPoseStake: Balance = 1000 * TWILL; // 1,000 TWL minimum stake

    // Carbon
    pub const MaxProjectIdLength: u32 = 128;
    pub const MaxIssuanceAmount: u128 = 1_000_000 * 1_000_000; // 1M tCO2e in micro units
    pub const IssuanceBond: Balance = CARBON_ISSUANCE_BOND;
    pub const DisputeWindow: BlockNumber = CARBON_DISPUTE_WINDOW;

    // Reserve
    pub const MaxReserveAssets: u32 = 20;

    // Governance
    pub const MaxBoardMembers: u32 = 7;
    /// ~5 years at 6s blocks (5 * 365.25 * 24 * 600)
    pub const BoardTermBlocks: BlockNumber = 26_280_000;
    /// ~7 days at 6s blocks (7 * 24 * 3600 / 6)
    pub const VotingPeriodBlocks: BlockNumber = 100_800;
    pub const EnactmentDelayBlocks: BlockNumber = 100_800;
    pub const NominationDeposit: Balance = 100 * TWILL; // 2nd election onwards only
    pub const MaxNominees: u32 = 100;
    pub const MaxActiveProposals: u32 = 200;

    // Safety wallets — deterministic, derived from SHA256 of purpose string.
    // No private key exists for any of these. They are protocol-controlled.
    /// Fee pool: keyless buffer that holds settlement fees before distribution to stakers.
    pub FeePoolAccount: AccountId = AccountId::new(
        twill_primitives::derive_safety_wallet(twill_primitives::SAFETY_WALLET_FEE_POOL)
    );
    /// Community pool: accumulates 20% of all settlement fees.
    /// Governed by community proposals. No private key. Protocol-controlled.
    pub CommunityPoolAccount: AccountId = AccountId::new(
        twill_primitives::derive_safety_wallet(twill_primitives::SAFETY_WALLET_COMMUNITY_POOL)
    );
    pub BurnAccount: AccountId = AccountId::new(
        twill_primitives::derive_safety_wallet(twill_primitives::SAFETY_WALLET_BURN)
    );
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type AccountId = AccountId;
    type RuntimeCall = RuntimeCall;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type Nonce = Nonce;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
    type Version = Version;
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

// ---------------------------------------------------------------------------
// Balances (native TWL token)
// ---------------------------------------------------------------------------

impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// ---------------------------------------------------------------------------
// Transaction Payment
// ---------------------------------------------------------------------------

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = frame_support::weights::IdentityFee<Balance>;
    type LengthToFee = frame_support::weights::IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

// ---------------------------------------------------------------------------
// Assets (wrapped crypto: wBTC, wETH, wSOL)
// ---------------------------------------------------------------------------

impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = frame_support::traits::AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<{ 100 * TWILL }>;
    type AssetAccountDeposit = ConstU128<{ TWILL / 100 }>;
    type MetadataDepositBase = ConstU128<{ TWILL }>;
    type MetadataDepositPerByte = ConstU128<{ TWILL / 100 }>;
    type ApprovalDeposit = ConstU128<{ TWILL / 100 }>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

// ---------------------------------------------------------------------------
// Twill Pallets
// ---------------------------------------------------------------------------

impl pallet_settlement::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxLegsPerSettlement = MaxLegsPerSettlement;
    type MaxPayloadSize = MaxPayloadSize;
    type SettlementTimeout = SettlementTimeout;
    type FeeBps = FeeBps;
    type MinFee = MinFee;
    type FeePoolAccount = FeePoolAccount;
    type MiningProvider = Mining;
    type ReserveProvider = Reserve;
    type CarbonProvider = Carbon;
    type MaxExpiryPerBlock = MaxExpiryPerBlock;
}

impl pallet_reserve::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxReserveAssets = MaxReserveAssets;
    type Oracle = Oracle;
}

impl pallet_mining::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxPoseValidators = MaxPoseValidators;
    type MinPoseStake = MinPoseStake;
    type FeePoolAccount = FeePoolAccount;
    type CommunityPoolAccount = CommunityPoolAccount;
}

impl pallet_carbon::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxProjectIdLength = MaxProjectIdLength;
    type MaxIssuanceAmount = MaxIssuanceAmount;
    type IssuanceBond = IssuanceBond;
    type DisputeWindow = DisputeWindow;
}

impl pallet_twl_token::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BurnAccount = BurnAccount;
}

impl pallet_oracle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type MaxSubmitters = MaxPoseValidators;
    type StalenessThreshold = ConstU32<50>; // ~5 minutes at 6s blocks
    type ValidatorCheck = Mining;
}

impl pallet_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxBoardMembers = MaxBoardMembers;
    type BoardTermBlocks = BoardTermBlocks;
    type VotingPeriodBlocks = VotingPeriodBlocks;
    type EnactmentDelayBlocks = EnactmentDelayBlocks;
    type NominationDeposit = NominationDeposit;
    type MaxNominees = MaxNominees;
    type MaxActiveProposals = MaxActiveProposals;
}

// ---------------------------------------------------------------------------
// Construct Runtime
// ---------------------------------------------------------------------------

construct_runtime!(
    pub struct Runtime {
        // Core
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,

        // Wrapped Assets
        Assets: pallet_assets,

        // Twill Protocol
        Settlement: pallet_settlement,
        Reserve: pallet_reserve,
        Mining: pallet_mining::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
        Carbon: pallet_carbon,
        TwlToken: pallet_twl_token,
        Oracle: pallet_oracle,

        // Governance
        Governance: pallet_governance,
    }
);

// ---------------------------------------------------------------------------
// Runtime API Implementation
// ---------------------------------------------------------------------------

sp_api::impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> sp_version::RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            frame_support::genesis_builder_helper::build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            frame_support::genesis_builder_helper::get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            Default::default()
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }

        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }

        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }
}
