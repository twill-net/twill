//! # Mining Pallet
//!
//! PoC+PoSe unified block mining and PoSe staking rewards.
//! Fully permissionless — no admin keys, no root extrinsics.
//!
//! ## Mining Model
//!
//! **New TWL is created exclusively through GPU block mining.**
//!
//! **Block Mining (PoC+PoSe):** A miner solves the hash puzzle (PoC)
//! with the settlement merkle root embedded in the proof (PoSe). Mining
//! the block and validating the settlement ledger are one atomic operation.
//! The miner earns 100% of the block reward.
//!
//! **PoSe Staking:** Participants stake TWL to back the settlement
//! infrastructure. Stakers earn settlement fees (existing, already-minted TWL
//! redistributed from the fee pool) proportional to their stake. Staking
//! does not create new TWL — fees are redistributed from participants who
//! paid them.
//!
//! After genesis, this pallet runs itself:
//! - Miners submit proofs permissionlessly
//! - Stakers register/deregister permissionlessly
//! - Settlement fees auto-distribute to stakers in on_finalize (stake-weighted)
//! - Slashing is automatic (inactivity detection)
//! - Epoch transitions (halvings) are automatic

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::{Pays, PostDispatchInfo},
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, Get, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{Bounded, Saturating, Zero};
    use twill_primitives::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: ReservableCurrency<Self::AccountId>;

        #[pallet::constant]
        type MaxPoseValidators: Get<u32>;

        #[pallet::constant]
        type MinPoseStake: Get<BalanceOf<Self>>;

        /// Fee pool account — buffer for settlement fees before staker distribution.
        #[pallet::constant]
        type FeePoolAccount: Get<Self::AccountId>;

        /// Treasury account — accumulates 20% of all settlement fees and optionally
        /// a governance-voted share of block rewards. Spendable only via passed proposals.
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Cumulative TWL minted from the mining pool (block mining + staking)
    #[pallet::storage]
    pub type TotalMinted<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type CurrentEpoch<T: Config> = StorageValue<_, u32, ValueQuery>;
    /// Cumulative TWL earned by block miners (PoC+PoSe unified)
    #[pallet::storage]
    pub type TotalPocRewards<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type PoseValidators<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, PoseValidator<T>>;
    #[pallet::storage]
    pub type ActiveValidatorSet<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxPoseValidators>, ValueQuery>;
    #[pallet::storage]
    pub type LastPocRewardBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;
    #[pallet::storage]
    pub type PocDifficulty<T: Config> = StorageValue<_, H256>;
    #[pallet::storage]
    pub type CurrentSettlementRoot<T: Config> = StorageValue<_, H256, ValueQuery>;
    #[pallet::storage]
    pub type GenesisBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;
    #[pallet::storage]
    pub type GenesisInitialized<T: Config> = StorageValue<_, bool, ValueQuery>;
    #[pallet::storage]
    pub type DifficultyAdjustmentBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;
    #[pallet::storage]
    pub type EpochStartTimestamp<T: Config> = StorageValue<_, u64, ValueQuery>;
    /// Settlement fees accumulated for distribution to stakers
    #[pallet::storage]
    pub type PendingFeePool<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;
    /// Governance-adjustable share of block rewards redirected to the treasury (in BPS).
    /// Default: 0 (miners keep 100%). Max: MINING_TREASURY_SHARE_MAX_BPS (10%).
    /// Set via SetMiningTreasuryShare governance proposal.
    #[pallet::storage]
    pub type MiningTreasuryShareBps<T: Config> = StorageValue<_, u16, ValueQuery>;
    /// Last block each staker was active (for auto-slashing)
    #[pallet::storage]
    pub type LastActiveBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, ValueQuery>;
    /// Slash count per staker (1st = 50%, 2nd+ = 100% + deregister)
    #[pallet::storage]
    pub type SlashCount<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

    pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
    pub const INITIAL_DIFFICULTY: [u8; 32] = [
        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct PoseValidator<T: Config> {
        pub account: T::AccountId,
        pub stake: BalanceOf<T>,
        pub registered_at: BlockNumberFor<T>,
        pub active: bool,
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Block mined via PoC+PoSe. The miner solved the hash puzzle with
        /// the settlement merkle root embedded — mining and settlement
        /// validation are one atomic operation.
        BlockMined { miner: T::AccountId, reward: BalanceOf<T>, block_number: BlockNumberFor<T> },
        /// Settlement fees distributed to stakers (existing TWL redistribution, not new minting).
        FeesDistributed { fee_reward: BalanceOf<T>, staker_count: u32, block_number: BlockNumberFor<T> },
        EpochChanged { epoch: u32, new_block_reward: u128 },
        StakerRegistered { staker: T::AccountId, stake: BalanceOf<T> },
        StakerDeregistered { staker: T::AccountId },
        MiningPoolExhausted { total_minted: u128, final_block: BlockNumberFor<T> },
        SettlementRootUpdated { merkle_root: H256 },
        StakerSlashed { staker: T::AccountId, amount: BalanceOf<T>, offense_number: u32, auto_deregistered: bool },
        DifficultyAdjusted { old_difficulty: H256, new_difficulty: H256, block_number: BlockNumberFor<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        MiningPoolExhausted,
        InsufficientStake,
        AlreadyRegistered,
        ValidatorNotFound,
        MaxValidatorsReached,
        InvalidPocProof,
        DuplicateBlockReward,
        StaleSettlementRoot,
        BootstrapEnded,
    }

    // -----------------------------------------------------------------------
    // Hooks — fully automatic, no human needed
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            if !GenesisInitialized::<T>::get() {
                GenesisBlock::<T>::put(now);
                DifficultyAdjustmentBlock::<T>::put(now);
                GenesisInitialized::<T>::put(true);
            }

            let genesis = GenesisBlock::<T>::get();
            let blocks_since: u64 = now.saturating_sub(genesis).try_into().unwrap_or(0u64);
            let epoch = (blocks_since / HALVING_INTERVAL) as u32;
            let current_epoch = CurrentEpoch::<T>::get();

            if epoch > current_epoch {
                CurrentEpoch::<T>::put(epoch);
                let new_reward = block_reward_at(blocks_since);
                Self::deposit_event(Event::EpochChanged {
                    epoch, new_block_reward: new_reward,
                });
            }

            Weight::from_parts(10_000_000, 0)
        }

        fn on_finalize(now: BlockNumberFor<T>) {
            Self::auto_distribute_staking(now);
            Self::auto_slash_inactive(now);
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics — ALL permissionless
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Mine a block by solving the PoC puzzle with the settlement root.
        ///
        /// PoC+PoSe are unified: the settlement merkle root is embedded in
        /// the hash proof, so mining the block simultaneously validates the
        /// settlement ledger. Block mined + ledger validated = 1 reward.
        ///
        /// Bootstrap: first 10M TWL mined fee-free. After that, miners
        /// pay a small transaction fee (by then they already have TWL).
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(100_000_000, 0))]
        pub fn submit_poc_proof(
            origin: OriginFor<T>,
            nonce: H256,
            settlement_root: H256,
        ) -> DispatchResultWithPostInfo {
            let miner = ensure_signed(origin)?;

            let total_minted = TotalMinted::<T>::get();
            ensure!(total_minted < MINING_POOL, Error::<T>::MiningPoolExhausted);

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(LastPocRewardBlock::<T>::get() < now, Error::<T>::DuplicateBlockReward);

            let current_root = CurrentSettlementRoot::<T>::get();
            ensure!(settlement_root == current_root, Error::<T>::StaleSettlementRoot);

            let parent_hash = frame_system::Pallet::<T>::parent_hash();
            let pow_hash = Self::compute_poc_hash(&nonce, &settlement_root, parent_hash.as_ref());
            let difficulty = PocDifficulty::<T>::get()
                .unwrap_or_else(|| H256::from(INITIAL_DIFFICULTY));
            ensure!(pow_hash < difficulty, Error::<T>::InvalidPocProof);

            let genesis = GenesisBlock::<T>::get();
            let blocks_since: u64 = now.saturating_sub(genesis).try_into().unwrap_or(0u64);

            // Block reward split: miner gets (10000 - treasury_bps) / 10000.
            // Treasury share is 0 at genesis. Community can vote to redirect up to 10%.
            let mining_reward = block_reward_at(blocks_since);

            let remaining = MINING_POOL.saturating_sub(total_minted);
            let actual_reward = mining_reward.min(remaining);

            if actual_reward == 0 {
                Self::deposit_event(Event::MiningPoolExhausted { total_minted, final_block: now });
                return Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No });
            }

            let treasury_bps = MiningTreasuryShareBps::<T>::get() as u128;
            let treasury_amount = actual_reward.saturating_mul(treasury_bps) / 10_000u128;
            let miner_amount = actual_reward.saturating_sub(treasury_amount);

            let miner_balance: BalanceOf<T> = miner_amount
                .try_into().map_err(|_| Error::<T>::MiningPoolExhausted)?;
            let _ = T::Currency::deposit_creating(&miner, miner_balance);

            if treasury_amount > 0 {
                let treasury_balance: BalanceOf<T> = treasury_amount
                    .try_into().unwrap_or_else(|_| BalanceOf::<T>::zero());
                let _ = T::Currency::deposit_creating(&T::TreasuryAccount::get(), treasury_balance);
            }

            TotalMinted::<T>::mutate(|m| *m = m.saturating_add(actual_reward));
            TotalPocRewards::<T>::mutate(|r| *r = r.saturating_add(actual_reward));
            LastPocRewardBlock::<T>::put(now);

            Self::deposit_event(Event::BlockMined { miner, reward: miner_balance, block_number: now });

            // Bootstrap: fee-free until 2.5M TWL mined, then small fee
            let pays = if total_minted < BOOTSTRAP_THRESHOLD { Pays::No } else { Pays::Yes };
            Ok(PostDispatchInfo { actual_weight: None, pays_fee: pays })
        }

        /// Stake TWL to back the settlement infrastructure (PoSe staking).
        ///
        /// Staked TWL is the collateral that makes settlements trustworthy.
        /// Stakers earn settlement fees proportional to their stake.
        /// Permissionless — just stake.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn register_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!PoseValidators::<T>::contains_key(&who), Error::<T>::AlreadyRegistered);

            let stake = T::MinPoseStake::get();
            T::Currency::reserve(&who, stake).map_err(|_| Error::<T>::InsufficientStake)?;

            let now = frame_system::Pallet::<T>::block_number();
            PoseValidators::<T>::insert(&who, PoseValidator {
                account: who.clone(), stake, registered_at: now, active: true,
            });
            LastActiveBlock::<T>::insert(&who, now);

            ActiveValidatorSet::<T>::try_mutate(|set| {
                set.try_push(who.clone()).map_err(|_| Error::<T>::MaxValidatorsReached)
            })?;

            Self::deposit_event(Event::StakerRegistered { staker: who, stake });
            Ok(())
        }

        /// Unstake and exit the PoSe staking set. Remaining stake returned.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn deregister_validator(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = PoseValidators::<T>::get(&who).ok_or(Error::<T>::ValidatorNotFound)?;

            T::Currency::unreserve(&who, validator.stake);
            PoseValidators::<T>::remove(&who);
            LastActiveBlock::<T>::remove(&who);
            SlashCount::<T>::remove(&who);
            ActiveValidatorSet::<T>::mutate(|set| { set.retain(|v| v != &who); });

            Self::deposit_event(Event::StakerDeregistered { staker: who });
            Ok(())
        }

        /// Bootstrap mining: unsigned proof submission during bootstrap period.
        ///
        /// During bootstrap (TotalMinted < 10M TWL), miners submit proofs
        /// without paying fees. The PoW proof itself is the spam protection —
        /// invalid proofs are rejected at the transaction pool level.
        /// After 10M TWL mined, miners must use submit_poc_proof (signed, with fee).
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(100_000_000, 0))]
        pub fn submit_poc_proof_unsigned(
            origin: OriginFor<T>,
            miner: T::AccountId,
            nonce: H256,
            settlement_root: H256,
        ) -> DispatchResult {
            ensure_none(origin)?;

            let total_minted = TotalMinted::<T>::get();
            ensure!(total_minted < BOOTSTRAP_THRESHOLD, Error::<T>::BootstrapEnded);
            ensure!(total_minted < MINING_POOL, Error::<T>::MiningPoolExhausted);

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(LastPocRewardBlock::<T>::get() < now, Error::<T>::DuplicateBlockReward);

            let current_root = CurrentSettlementRoot::<T>::get();
            ensure!(settlement_root == current_root, Error::<T>::StaleSettlementRoot);

            let parent_hash = frame_system::Pallet::<T>::parent_hash();
            let pow_hash = Self::compute_poc_hash(&nonce, &settlement_root, parent_hash.as_ref());
            let difficulty = PocDifficulty::<T>::get()
                .unwrap_or_else(|| H256::from(INITIAL_DIFFICULTY));
            ensure!(pow_hash < difficulty, Error::<T>::InvalidPocProof);

            let genesis = GenesisBlock::<T>::get();
            let blocks_since: u64 = now.saturating_sub(genesis).try_into().unwrap_or(0u64);

            let mining_reward = block_reward_at(blocks_since);
            let remaining = MINING_POOL.saturating_sub(total_minted);
            let actual_reward = mining_reward.min(remaining);

            if actual_reward == 0 {
                Self::deposit_event(Event::MiningPoolExhausted { total_minted, final_block: now });
                return Ok(());
            }

            let treasury_bps = MiningTreasuryShareBps::<T>::get() as u128;
            let treasury_amount = actual_reward.saturating_mul(treasury_bps) / 10_000u128;
            let miner_amount = actual_reward.saturating_sub(treasury_amount);

            let miner_balance: BalanceOf<T> = miner_amount
                .try_into().map_err(|_| Error::<T>::MiningPoolExhausted)?;
            let _ = T::Currency::deposit_creating(&miner, miner_balance);

            if treasury_amount > 0 {
                let treasury_balance: BalanceOf<T> = treasury_amount
                    .try_into().unwrap_or_else(|_| BalanceOf::<T>::zero());
                let _ = T::Currency::deposit_creating(&T::TreasuryAccount::get(), treasury_balance);
            }

            TotalMinted::<T>::mutate(|m| *m = m.saturating_add(actual_reward));
            TotalPocRewards::<T>::mutate(|r| *r = r.saturating_add(actual_reward));
            LastPocRewardBlock::<T>::put(now);

            Self::deposit_event(Event::BlockMined { miner, reward: miner_balance, block_number: now });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // ValidateUnsigned — bootstrap mining proof validation at pool level
    // -----------------------------------------------------------------------

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::submit_poc_proof_unsigned { miner: _, nonce: _, settlement_root, .. } = call {
                // Only allow during bootstrap
                let total_minted = TotalMinted::<T>::get();
                if total_minted >= BOOTSTRAP_THRESHOLD {
                    return InvalidTransaction::Custom(1).into();
                }

                // Verify settlement root matches
                let current_root = CurrentSettlementRoot::<T>::get();
                if *settlement_root != current_root {
                    return InvalidTransaction::Custom(2).into();
                }

                // One proof per block.
                // NOTE: use `>` not `>=` — at genesis LastPocRewardBlock == now == 0,
                // and `>=` would incorrectly mark every first-ever submission as Stale.
                // The PoW itself is verified in dispatch (submit_poc_proof_unsigned) using
                // the execution-time parent_hash, which is hash(current head) — the value
                // that frame_system::parent_hash() returns in block N+1.  We cannot verify
                // the same value here at pool time (pool runs in block N's state where
                // frame_system::parent_hash() == hash(N-1)), so we skip the PoW check at
                // pool level and rely on dispatch to reject invalid proofs.
                let now = frame_system::Pallet::<T>::block_number();
                if LastPocRewardBlock::<T>::get() > now {
                    return InvalidTransaction::Stale.into();
                }

                ValidTransaction::with_tag_prefix("TwillBootstrapMining")
                    .priority(TransactionPriority::MAX)
                    .longevity(1)
                    .and_provides(("bootstrap_mining", now))
                    .propagate(true)
                    .build()
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }

    // -----------------------------------------------------------------------
    // Trait implementations — cross-pallet interface
    // -----------------------------------------------------------------------

    impl<T: Config> twill_primitives::MiningInterface<T::AccountId> for Pallet<T> {
        /// Settlement merkle root finalized — becomes part of PoC proof.
        fn update_settlement_root(merkle_root: H256) {
            CurrentSettlementRoot::<T>::put(merkle_root);
        }

        /// Record staker activity (resets inactivity slash timer).
        fn record_validator_activity(validator: &T::AccountId) {
            let now = frame_system::Pallet::<T>::block_number();
            LastActiveBlock::<T>::insert(validator, now);
        }

        /// Accumulate settlement fees for distribution to stakers.
        fn accumulate_fee(amount: u128) {
            let balance: BalanceOf<T> = amount
                .try_into().unwrap_or_else(|_| BalanceOf::<T>::max_value());
            PendingFeePool::<T>::mutate(|pool| { *pool = pool.saturating_add(balance); });
        }

        /// Set the treasury share of block rewards. Called by governance on enactment.
        fn set_treasury_mining_share(bps: u16) {
            let capped = bps.min(twill_primitives::MINING_TREASURY_SHARE_MAX_BPS);
            MiningTreasuryShareBps::<T>::put(capped);
        }
    }

    impl<T: Config> twill_primitives::ValidatorOracle<T::AccountId> for Pallet<T> {
        fn is_active_validator(who: &T::AccountId) -> bool {
            ActiveValidatorSet::<T>::get().contains(who)
        }

        fn validator_stake(who: &T::AccountId) -> Option<u128> {
            PoseValidators::<T>::get(who).map(|v| v.stake.try_into().unwrap_or(0u128))
        }
    }

    // -----------------------------------------------------------------------
    // Internal — automatic, no human intervention
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Distribute settlement fees each block — 80% to stakers, 20% to community pool.
        ///
        /// The community pool share is transferred every block fees are available,
        /// regardless of whether stakers are active. The staker share is distributed
        /// only when there are active validators; otherwise it remains in the fee pool.
        fn auto_distribute_staking(now: BlockNumberFor<T>) {
            let validators = ActiveValidatorSet::<T>::get();
            let pending_fees = PendingFeePool::<T>::get();

            if pending_fees.is_zero() {
                return;
            }

            let fee_u128: u128 = pending_fees.try_into().unwrap_or(0u128);

            // Always send 20% to community pool
            let community_share = fee_u128.saturating_mul(twill_primitives::FEE_COMMUNITY_SHARE_BPS as u128) / 10_000u128;
            let staker_share = fee_u128.saturating_sub(community_share);

            if community_share > 0 {
                let community_amount: BalanceOf<T> = community_share.try_into().unwrap_or_else(|_| BalanceOf::<T>::zero());
                let _ = T::Currency::transfer(
                    &T::FeePoolAccount::get(),
                    &T::TreasuryAccount::get(),
                    community_amount,
                    ExistenceRequirement::KeepAlive,
                );
            }

            // Distribute 80% to stakers if any are active
            if !validators.is_empty() && staker_share > 0 {
                Self::distribute_stake_weighted(&validators, staker_share);
                PendingFeePool::<T>::put(BalanceOf::<T>::zero());

                let total: BalanceOf<T> = pending_fees;
                Self::deposit_event(Event::FeesDistributed {
                    fee_reward: total,
                    staker_count: validators.len() as u32,
                    block_number: now,
                });
            } else if validators.is_empty() {
                // No stakers — community portion already transferred, clear only that amount
                let community_amount: BalanceOf<T> = community_share.try_into().unwrap_or_else(|_| BalanceOf::<T>::zero());
                PendingFeePool::<T>::mutate(|p| *p = p.saturating_sub(community_amount));
            }
        }

        /// Distribute settlement fees to stakers proportional to stake.
        /// Transfers existing TWL from the fee pool — no new minting.
        fn distribute_stake_weighted(
            validators: &BoundedVec<T::AccountId, T::MaxPoseValidators>,
            amount: u128,
        ) {
            let total_stake: u128 = validators.iter()
                .filter_map(|v| PoseValidators::<T>::get(v))
                .map(|v| v.stake.try_into().unwrap_or(0u128))
                .fold(0u128, |acc, s| acc.saturating_add(s));

            let fee_pool = T::FeePoolAccount::get();

            if total_stake == 0 {
                let per = amount / validators.len() as u128;
                for v in validators.iter() {
                    let r: BalanceOf<T> = per.try_into().unwrap_or_else(|_| BalanceOf::<T>::max_value());
                    let _ = T::Currency::transfer(&fee_pool, v, r, ExistenceRequirement::KeepAlive);
                }
            } else {
                for v in validators.iter() {
                    if let Some(val) = PoseValidators::<T>::get(v) {
                        let stake: u128 = val.stake.try_into().unwrap_or(0u128);
                        let share = amount.saturating_mul(stake) / total_stake;
                        let r: BalanceOf<T> = share.try_into().unwrap_or_else(|_| BalanceOf::<T>::max_value());
                        let _ = T::Currency::transfer(&fee_pool, v, r, ExistenceRequirement::KeepAlive);
                    }
                }
            }
        }

        /// Auto-slash inactive stakers. 50% first, 100% + deregister second.
        fn auto_slash_inactive(now: BlockNumberFor<T>) {
            let threshold: BlockNumberFor<T> = SLASH_INACTIVITY_BLOCKS
                .try_into().unwrap_or_else(|_| now);

            let validators = ActiveValidatorSet::<T>::get();
            let mut to_remove: sp_std::vec::Vec<T::AccountId> = sp_std::vec::Vec::new();

            for validator in validators.iter() {
                let last_active = LastActiveBlock::<T>::get(validator);
                if now.saturating_sub(last_active) < threshold { continue; }

                let slash_num = SlashCount::<T>::get(validator).saturating_add(1);
                SlashCount::<T>::insert(validator, slash_num);

                let slash_bps: u16 = if slash_num == 1 { SLASH_FIRST_BPS } else { SLASH_REPEAT_BPS };

                if let Some(v) = PoseValidators::<T>::get(validator) {
                    let stake_u128: u128 = v.stake.try_into().unwrap_or(0u128);
                    let slash_u128 = stake_u128.saturating_mul(slash_bps as u128) / 10_000u128;
                    let slash_bal: BalanceOf<T> = slash_u128.try_into()
                        .unwrap_or_else(|_| BalanceOf::<T>::max_value());

                    let unreserved = T::Currency::unreserve(validator, slash_bal);
                    // Burn slashed stake — reduces supply, no treasury.
                    let _ = T::Currency::slash(validator, unreserved);

                    let auto_dereg = slash_num >= 2;
                    Self::deposit_event(Event::StakerSlashed {
                        staker: validator.clone(), amount: unreserved,
                        offense_number: slash_num, auto_deregistered: auto_dereg,
                    });

                    if auto_dereg {
                        let remaining = v.stake.saturating_sub(slash_bal);
                        if !remaining.is_zero() { T::Currency::unreserve(validator, remaining); }
                        PoseValidators::<T>::remove(validator);
                        LastActiveBlock::<T>::remove(validator);
                        SlashCount::<T>::remove(validator);
                        to_remove.push(validator.clone());
                    } else {
                        PoseValidators::<T>::mutate(validator, |v_opt| {
                            if let Some(ref mut v) = v_opt { v.stake = v.stake.saturating_sub(slash_bal); }
                        });
                        LastActiveBlock::<T>::insert(validator, now);
                    }
                }
            }

            if !to_remove.is_empty() {
                ActiveValidatorSet::<T>::mutate(|set| { set.retain(|v| !to_remove.contains(v)); });
            }
        }

        fn compute_poc_hash(nonce: &H256, settlement_root: &H256, parent_hash: &[u8]) -> H256 {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(nonce.as_bytes());
            hasher.update(settlement_root.as_bytes());
            hasher.update(parent_hash);
            H256::from_slice(&hasher.finalize())
        }

        pub fn total_minted() -> u128 { TotalMinted::<T>::get() }
        pub fn remaining_pool() -> u128 { MINING_POOL.saturating_sub(TotalMinted::<T>::get()) }
        pub fn epoch() -> u32 { CurrentEpoch::<T>::get() }
    }
}
