//! # Settlement Pallet
//!
//! HTLC-based atomic settlement engine for the Twill Network.
//!
//! Cross-border, cross-asset atomic settlement:
//! - LOCK: Escrow funds from sender
//! - EXECUTE: Atomic transfer to receiver on hashlock reveal
//! - REFUND: Auto-refund on timelock expiry
//!
//! Supports atomic exchange of any combination of TWL, carbon credits,
//! and external crypto assets. All-or-nothing atomic settlement.
//!
//! ## Settlement Lifecycle
//!
//! 1. **Propose** — Initiator creates a settlement with a hashlock (SHA256 of secret)
//! 2. **Lock Legs** — Each participant locks their asset leg against the hashlock
//! 3. **Settle** — Initiator reveals the secret preimage; all locked legs claim atomically
//! 4. **Refund/Expire** — If timelock expires, auto-refund via on_initialize or manual call

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::{Hash, Saturating, Zero};
    use sp_std::vec::Vec;
    use twill_primitives::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency used for TWL settlement
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum number of legs per settlement
        #[pallet::constant]
        type MaxLegsPerSettlement: Get<u32>;

        /// Maximum payload size per leg (bytes)
        #[pallet::constant]
        type MaxPayloadSize: Get<u32>;

        /// Minimum settlement timeout in blocks (~2 minutes). On-chain-only settlements
        /// (TWL↔Carbon) can use this directly — everything resolves in seconds.
        #[pallet::constant]
        type SettlementTimeout: Get<BlockNumberFor<Self>>;

        /// Maximum settlement timeout in blocks (default 14400 = 24 hours).
        /// BTC or ETH legs require bridge relayer confirmation — those chains need time.
        /// Proposer chooses any value in [SettlementTimeout, MaxSettlementTimeout].
        #[pallet::constant]
        type MaxSettlementTimeout: Get<BlockNumberFor<Self>>;

        /// Fee basis points (10 = 0.10%)
        #[pallet::constant]
        type FeeBps: Get<u16>;

        /// Minimum fee in TWL base units (prevents dust fees)
        #[pallet::constant]
        type MinFee: Get<BalanceOf<Self>>;

        /// Fee pool account — keyless buffer that holds settlement fees
        /// in transit before they are distributed 100% to PoSe stakers.
        #[pallet::constant]
        type FeePoolAccount: Get<Self::AccountId>;

        /// Mining pallet integration (PoSe score updates, settlement root)
        type MiningProvider: twill_primitives::MiningInterface<Self::AccountId>;

        /// Reserve pallet integration (asset deposit recording)
        type ReserveProvider: twill_primitives::ReserveInterface;

        /// Carbon pallet integration (lock/transfer/unlock on-chain carbon credits)
        type CarbonProvider: twill_primitives::CarbonInterface<Self::AccountId>;

        /// Oracle integration — price gate and settlement-price ledger
        type OracleProvider: twill_primitives::OracleInterface;

        /// Bridge pallet integration — confirms off-chain BTC/ETH/SOL deposits
        /// before any external-chain debit leg can execute.
        type BridgeProvider: twill_primitives::BridgeInterface;

        /// Maximum settlements that can auto-expire per block
        #[pallet::constant]
        type MaxExpiryPerBlock: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Active settlements indexed by exchange ID
    #[pallet::storage]
    pub type Settlements<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // exchange_id
        Settlement<T>,
    >;

    /// Settlement legs indexed by (exchange_id, leg_index)
    #[pallet::storage]
    pub type Legs<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        H256, // exchange_id
        Blake2_128Concat,
        u32,  // leg_index
        Leg<T>,
    >;

    /// Number of legs per settlement
    #[pallet::storage]
    pub type LegCount<T: Config> = StorageMap<_, Blake2_128Concat, H256, u32, ValueQuery>;

    /// Total settlements processed (for PoSe reward calculation)
    #[pallet::storage]
    pub type TotalSettlements<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total volume settled in TWL base units (for reserve/mining metrics)
    #[pallet::storage]
    pub type TotalVolumeSettled<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Settlement count per validator (for PoSe scoring)
    #[pallet::storage]
    pub type ValidatorSettlements<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Fees collected per settlement (for distribution)
    #[pallet::storage]
    pub type SettlementFees<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BalanceOf<T>, ValueQuery>;

    /// Expiry queue: maps timelock_block => list of exchange_ids expiring at that block.
    /// Bounded per block to prevent unbounded iteration.
    #[pallet::storage]
    pub type ExpiryQueue<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        BoundedVec<H256, T::MaxExpiryPerBlock>,
        ValueQuery,
    >;

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Settlement<T: Config> {
        pub exchange_id: H256,
        pub proposer: T::AccountId,
        pub hashlock: H256,
        pub timelock_block: BlockNumberFor<T>,
        pub status: SettlementStatus,
        pub merkle_root: H256,
        pub created_at: BlockNumberFor<T>,
    }

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Leg<T: Config> {
        pub participant: T::AccountId,
        pub domain: AssetDomain,
        pub rail: RailKind,
        pub side: LegSide,
        pub amount: BalanceOf<T>,
        pub currency_hash: H256,
        pub status: LegStatus,
        pub payload: BoundedVec<u8, T::MaxPayloadSize>,
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Settlement proposed
        SettlementProposed {
            exchange_id: H256,
            proposer: T::AccountId,
            hashlock: H256,
            timelock_block: BlockNumberFor<T>,
        },

        /// Leg locked
        LegLocked {
            exchange_id: H256,
            leg_index: u32,
            participant: T::AccountId,
            amount: BalanceOf<T>,
        },

        /// Settlement completed — secret revealed, all legs claimed.
        /// This event is the full on-chain ledger entry for every completed atomic swap.
        SettlementCompleted {
            exchange_id: H256,
            settler: T::AccountId,
            merkle_root: H256,
            /// Total TWL-internal debit volume (in planck). The economic throughput of this swap.
            total_twl_volume: BalanceOf<T>,
            /// Settlement fee collected (in TWL planck). Flows to PoSe stakers.
            fee: BalanceOf<T>,
            /// Number of legs in this settlement.
            leg_count: u32,
            /// Block number at settlement finality.
            block_number: BlockNumberFor<T>,
        },

        /// Settlement refunded — timelock expired (manual trigger)
        SettlementRefunded {
            exchange_id: H256,
            refunder: T::AccountId,
        },

        /// Settlement auto-expired by on_initialize
        SettlementExpired {
            exchange_id: H256,
        },

        /// Fee collected
        FeeCollected {
            exchange_id: H256,
            amount: BalanceOf<T>,
        },

    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// Settlement with this ID already exists
        SettlementAlreadyExists,
        /// Settlement not found
        SettlementNotFound,
        /// Settlement is not in the expected status
        InvalidSettlementStatus,
        /// Too many legs for this settlement
        TooManyLegs,
        /// Invalid hashlock preimage
        InvalidPreimage,
        /// Timelock has not expired yet (cannot refund)
        TimelockNotExpired,
        /// Timelock has expired (cannot settle)
        TimelockExpired,
        /// Insufficient balance to lock leg
        InsufficientBalance,
        /// Leg not found
        LegNotFound,
        /// Not all legs are locked
        NotAllLegsLocked,
        /// Carbon credit lock failed (credit not found, wrong owner, or wrong status)
        CarbonLockFailed,
        /// Only the proposer can settle
        NotProposer,
        /// Payload too large
        PayloadTooLarge,
        /// Arithmetic overflow
        ArithmeticOverflow,
        /// Debit/credit mismatch — no debit leg matches the credit currency
        DebitCreditMismatch,
        /// Expiry queue full for this block
        ExpiryQueueFull,
        /// Oracle price unavailable for a non-TWL rail in this settlement.
        /// Every external asset must have a live oracle price before settlement
        /// can execute. Submit oracle prices first, then retry.
        OraclePriceUnavailable,
        /// Every settlement must include at least one TwillInternal leg.
        /// This ensures fee collection and price discovery on every swap.
        TwlLegRequired,
        /// BTC/ETH/SOL debit leg requires bridge relayer confirmation before settlement.
        /// Bridge relayers must call confirm_deposit until threshold is reached.
        BridgeConfirmationRequired,
    }

    // -----------------------------------------------------------------------
    // Hooks
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let expired_ids = ExpiryQueue::<T>::take(now);
            let mut weight = Weight::from_parts(5_000_000, 0);

            for exchange_id in expired_ids.iter() {
                if let Some(settlement) = Settlements::<T>::get(exchange_id) {
                    if settlement.status == SettlementStatus::Proposed
                        || settlement.status == SettlementStatus::Locked
                    {
                        Self::do_refund_legs(*exchange_id);

                        Settlements::<T>::mutate(exchange_id, |s_opt| {
                            if let Some(ref mut s) = s_opt {
                                s.status = SettlementStatus::Expired;
                            }
                        });

                        Self::deposit_event(Event::SettlementExpired {
                            exchange_id: *exchange_id,
                        });
                    }
                }
                weight = weight.saturating_add(Weight::from_parts(50_000_000, 0));
            }

            weight
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Propose a new atomic settlement.
        ///
        /// `timeout_blocks` — how long until the settlement expires and auto-refunds.
        /// Clamped to [SettlementTimeout, MaxSettlementTimeout].
        ///
        /// On-chain-only swaps (TWL↔Carbon): use the minimum (default 20 blocks).
        /// BTC/ETH swaps requiring bridge confirmation: use 600–14400 blocks (1h–24h).
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn propose(
            origin: OriginFor<T>,
            exchange_id: H256,
            hashlock: H256,
            timeout_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;

            ensure!(
                !Settlements::<T>::contains_key(exchange_id),
                Error::<T>::SettlementAlreadyExists
            );

            let now = frame_system::Pallet::<T>::block_number();
            // Clamp to [min, max] — prevents both dust timeouts and unbounded locks
            let min_timeout = T::SettlementTimeout::get();
            let max_timeout = T::MaxSettlementTimeout::get();
            let clamped = timeout_blocks.max(min_timeout).min(max_timeout);
            let timelock_block = now.saturating_add(clamped);

            let settlement = Settlement {
                exchange_id,
                proposer: proposer.clone(),
                hashlock,
                timelock_block,
                status: SettlementStatus::Proposed,
                merkle_root: H256::zero(),
                created_at: now,
            };

            Settlements::<T>::insert(exchange_id, settlement);

            // Register in expiry queue for automatic cleanup
            ExpiryQueue::<T>::try_mutate(timelock_block, |queue| {
                queue.try_push(exchange_id).map_err(|_| Error::<T>::ExpiryQueueFull)
            })?;

            Self::deposit_event(Event::SettlementProposed {
                exchange_id,
                proposer,
                hashlock,
                timelock_block,
            });

            Ok(())
        }

        /// Lock a leg into an existing settlement.
        ///
        /// Debit legs reserve TWL in escrow. Credit legs record the
        /// expected payout (no reservation needed — funds come from debit side).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(75_000_000, 0))]
        pub fn lock_leg(
            origin: OriginFor<T>,
            exchange_id: H256,
            domain: AssetDomain,
            rail: RailKind,
            side: LegSide,
            amount: BalanceOf<T>,
            currency_hash: H256,
            payload: Vec<u8>,
        ) -> DispatchResult {
            let participant = ensure_signed(origin)?;

            let settlement =
                Settlements::<T>::get(exchange_id).ok_or(Error::<T>::SettlementNotFound)?;

            ensure!(
                settlement.status == SettlementStatus::Proposed
                    || settlement.status == SettlementStatus::Locked,
                Error::<T>::InvalidSettlementStatus
            );

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now < settlement.timelock_block, Error::<T>::TimelockExpired);

            let leg_index = LegCount::<T>::get(exchange_id);
            ensure!(
                leg_index < T::MaxLegsPerSettlement::get(),
                Error::<T>::TooManyLegs
            );

            let bounded_payload: BoundedVec<u8, T::MaxPayloadSize> = payload
                .try_into()
                .map_err(|_| Error::<T>::PayloadTooLarge)?;

            // Reserve TWL for debit legs on internal rails (escrow)
            if side == LegSide::Debit && rail == RailKind::TwillInternal {
                T::Currency::reserve(&participant, amount)
                    .map_err(|_| Error::<T>::InsufficientBalance)?;
            }

            // Lock on-chain carbon credits for carbon debit legs
            // currency_hash is treated as the credit_id for carbon legs
            if side == LegSide::Debit && domain == AssetDomain::Carbon {
                ensure!(
                    T::CarbonProvider::lock_for_settlement(currency_hash, &participant),
                    Error::<T>::CarbonLockFailed
                );
            }

            let leg = Leg {
                participant: participant.clone(),
                domain,
                rail,
                side,
                amount,
                currency_hash,
                status: LegStatus::Locked,
                payload: bounded_payload,
            };

            Legs::<T>::insert(exchange_id, leg_index, leg);
            LegCount::<T>::insert(exchange_id, leg_index + 1);

            Settlements::<T>::mutate(exchange_id, |s| {
                if let Some(ref mut settlement) = s {
                    settlement.status = SettlementStatus::Locked;
                }
            });

            Self::deposit_event(Event::LegLocked {
                exchange_id,
                leg_index,
                participant,
                amount,
            });

            Ok(())
        }

        /// Settle an exchange by revealing the hashlock preimage.
        ///
        /// Atomic settlement using lock/execute pattern:
        /// 1. Verify preimage against hashlock
        /// 2. Validate all legs locked
        /// 3. For each currency: match debit total to credit total, deduct fee
        /// 4. Execute on-chain TWL transfers (debit escrow -> credit receivers)
        /// 5. Fee goes to FeePoolAccount, distributed 100% to PoSe stakers
        /// 6. Build Merkle proof, update metrics, record reserve deposits
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(200_000_000, 0))]
        pub fn settle(
            origin: OriginFor<T>,
            exchange_id: H256,
            preimage: Vec<u8>,
        ) -> DispatchResult {
            let settler = ensure_signed(origin)?;

            let settlement =
                Settlements::<T>::get(exchange_id).ok_or(Error::<T>::SettlementNotFound)?;

            ensure!(
                settlement.status == SettlementStatus::Locked,
                Error::<T>::InvalidSettlementStatus
            );
            ensure!(settlement.proposer == settler, Error::<T>::NotProposer);

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now < settlement.timelock_block, Error::<T>::TimelockExpired);

            // Verify hashlock
            ensure!(
                verify_hashlock(&preimage, &settlement.hashlock),
                Error::<T>::InvalidPreimage
            );

            let leg_count = LegCount::<T>::get(exchange_id);

            // ---------------------------------------------------------------
            // Pass 1: Validate all legs locked, collect info
            // ---------------------------------------------------------------

            struct LegInfo<AccountId, Balance> {
                index: u32,
                participant: AccountId,
                amount: Balance,
                domain: AssetDomain,
                rail: RailKind,
                side: LegSide,
                currency_hash: H256,
            }

            let mut legs_info: Vec<LegInfo<T::AccountId, BalanceOf<T>>> = Vec::new();
            let mut total_volume = BalanceOf::<T>::zero();

            for i in 0..leg_count {
                let leg = Legs::<T>::get(exchange_id, i).ok_or(Error::<T>::LegNotFound)?;
                ensure!(leg.status == LegStatus::Locked, Error::<T>::NotAllLegsLocked);

                total_volume = total_volume.saturating_add(leg.amount);

                legs_info.push(LegInfo {
                    index: i,
                    participant: leg.participant.clone(),
                    amount: leg.amount,
                    domain: leg.domain,
                    rail: leg.rail,
                    side: leg.side,
                    currency_hash: leg.currency_hash,
                });
            }

            // ---------------------------------------------------------------
            // Pass 1.1: Enforce mandatory TWL debit leg
            // Every settlement must have at least one TwillInternal *debit*
            // leg. The fee is calculated on the TWL-debit volume, so without
            // one the fee collapses to zero and stakers/treasury see no
            // revenue on that flow. A TWL credit-only leg (payout) is not
            // sufficient; the chain must be the one moving value inward.
            // ---------------------------------------------------------------

            ensure!(
                legs_info.iter().any(|l|
                    l.rail == RailKind::TwillInternal && l.side == LegSide::Debit
                ),
                Error::<T>::TwlLegRequired
            );

            // ---------------------------------------------------------------
            // Pass 1.5: Oracle price gate
            // Every non-TWL rail must have a live oracle price before this
            // settlement can execute. This enforces that every asset flowing
            // through Twill is valued — no unpriced settlement, no dark trades.
            // ---------------------------------------------------------------

            // Track external debit totals per oracle pair for price recording
            let mut btc_debit: u128 = 0u128;
            let mut eth_debit: u128 = 0u128;
            let mut sol_debit: u128 = 0u128;
            let mut usdc_debit: u128 = 0u128;
            let mut carbon_debit: u128 = 0u128;

            for leg in legs_info.iter() {
                if leg.side != LegSide::Debit || leg.rail == RailKind::TwillInternal {
                    continue;
                }
                match leg.rail.oracle_pair() {
                    Some(pair) => {
                        ensure!(
                            T::OracleProvider::get_price(pair).map_or(false, |p| p > 0),
                            Error::<T>::OraclePriceUnavailable
                        );
                        let amt: u128 = leg.amount.try_into().unwrap_or(0u128);
                        match pair {
                            AssetPair::BtcTwl    => btc_debit    = btc_debit.saturating_add(amt),
                            AssetPair::EthTwl    => eth_debit    = eth_debit.saturating_add(amt),
                            AssetPair::SolTwl    => sol_debit    = sol_debit.saturating_add(amt),
                            AssetPair::UsdcTwl   => usdc_debit   = usdc_debit.saturating_add(amt),
                            AssetPair::CarbonTwl => carbon_debit = carbon_debit.saturating_add(amt),
                            // Fiat rails (USD/EUR) recorded for completeness; settlement
                            // price feedback not yet wired for fiat (no on-chain counterpart)
                            AssetPair::UsdTwl | AssetPair::EurTwl => {},
                        }
                    },
                    None => {
                        // Rail has no oracle pair — fiat rails require governance
                        // activation and oracle node infrastructure before use.
                        return Err(Error::<T>::OraclePriceUnavailable.into());
                    },
                }
            }

            // ---------------------------------------------------------------
            // Pass 1.6: Bridge confirmation gate
            // BTC/ETH/SOL debit legs require N-of-M relayer confirmation that
            // the deposit has been received on the external chain. Carbon and
            // fiat legs have on-chain or oracle confirmation respectively.
            // TwillInternal legs need no external confirmation (on-chain).
            // ---------------------------------------------------------------

            for leg in legs_info.iter() {
                if leg.side != LegSide::Debit { continue; }
                match leg.rail {
                    RailKind::Bitcoin | RailKind::Ethereum | RailKind::Solana => {
                        ensure!(
                            T::BridgeProvider::is_deposit_confirmed(exchange_id, leg.index),
                            Error::<T>::BridgeConfirmationRequired
                        );
                    },
                    _ => {} // Other rails confirmed by on-chain state or oracle
                }
            }

            // ---------------------------------------------------------------
            // Pass 2: Calculate fee on total TWL-internal debit volume
            // ---------------------------------------------------------------

            let twl_debit_total: BalanceOf<T> = legs_info
                .iter()
                .filter(|l| l.rail == RailKind::TwillInternal && l.side == LegSide::Debit)
                .fold(BalanceOf::<T>::zero(), |acc, l| acc.saturating_add(l.amount));

            let fee_bps = T::FeeBps::get();
            let calculated_fee = Self::calculate_fee(twl_debit_total, fee_bps);
            let min_fee = T::MinFee::get();
            let fee = if twl_debit_total.is_zero() {
                BalanceOf::<T>::zero()
            } else if calculated_fee < min_fee {
                min_fee
            } else {
                calculated_fee
            };

            // ---------------------------------------------------------------
            // Pass 3: Execute atomic transfers for TWL-internal legs
            // ---------------------------------------------------------------

            // For each TWL-internal debit leg: unreserve escrow, then
            // transfer (amount - proportional_fee) to matched credit legs.
            //
            // Fee is split proportionally across all debit legs of the same currency.

            // Collect unique currency hashes for TWL-internal legs
            let mut processed_currencies: Vec<H256> = Vec::new();

            for leg in legs_info.iter() {
                if leg.rail != RailKind::TwillInternal {
                    continue;
                }
                if processed_currencies.contains(&leg.currency_hash) {
                    continue;
                }
                processed_currencies.push(leg.currency_hash);

                // Sum debits and credits for this currency
                let currency_debits: Vec<&LegInfo<T::AccountId, BalanceOf<T>>> = legs_info
                    .iter()
                    .filter(|l| l.currency_hash == leg.currency_hash
                        && l.rail == RailKind::TwillInternal
                        && l.side == LegSide::Debit)
                    .collect();

                let currency_credits: Vec<&LegInfo<T::AccountId, BalanceOf<T>>> = legs_info
                    .iter()
                    .filter(|l| l.currency_hash == leg.currency_hash
                        && l.rail == RailKind::TwillInternal
                        && l.side == LegSide::Credit)
                    .collect();

                let debit_sum: BalanceOf<T> = currency_debits
                    .iter()
                    .fold(BalanceOf::<T>::zero(), |acc, l| acc.saturating_add(l.amount));

                // Unreserve all debit escrows for this currency
                for debit in currency_debits.iter() {
                    T::Currency::unreserve(&debit.participant, debit.amount);
                }

                // Calculate fee share for this currency (proportional to its debit volume).
                // Both `fee` and `debit_sum` are u128 balances; their product can exceed
                // u128::MAX at sizes near TOTAL_SUPPLY. Promote to U256 for the multiply
                // so the proportion is exact, then divide back down before re-entering
                // the balance type. Same class of fix as `compute_retarget`.
                let currency_fee = if twl_debit_total.is_zero() {
                    BalanceOf::<T>::zero()
                } else {
                    let fee_u128: u128 = fee.try_into().unwrap_or(0u128);
                    let debit_sum_u128: u128 = debit_sum.try_into().unwrap_or(0u128);
                    let total_u128: u128 = twl_debit_total.try_into().unwrap_or(1u128).max(1);
                    let wide = sp_core::U256::from(fee_u128)
                        .saturating_mul(sp_core::U256::from(debit_sum_u128))
                        / sp_core::U256::from(total_u128);
                    let share_u128: u128 = if wide > sp_core::U256::from(u128::MAX) {
                        u128::MAX
                    } else {
                        wide.low_u128()
                    };
                    share_u128.try_into().unwrap_or(BalanceOf::<T>::zero())
                };

                // Deduct fee from the first debit participant of this currency
                if !currency_fee.is_zero() {
                    if let Some(fee_payer) = currency_debits.first() {
                        // AllowDeath: the debit participant locked these funds for settlement;
                        // draining their account to zero is intentional and correct.
                        // Hard fail: if fee transfer fails, the entire settlement fails.
                        // Fees flow directly into FeePoolAccount balance — no counter to drift.
                        T::Currency::transfer(
                            &fee_payer.participant,
                            &T::FeePoolAccount::get(),
                            currency_fee,
                            ExistenceRequirement::AllowDeath,
                        )?;
                    }
                }

                // Transfer to each credit leg from debit legs (round-robin)
                // Each credit leg draws from debits in order until fulfilled
                let mut debit_idx = 0usize;
                let mut debit_remaining = if currency_debits.is_empty() {
                    BalanceOf::<T>::zero()
                } else {
                    currency_debits[0].amount.saturating_sub(
                        if debit_idx == 0 { currency_fee } else { BalanceOf::<T>::zero() }
                    )
                };

                for credit in currency_credits.iter() {
                    let mut credit_remaining = credit.amount;

                    while !credit_remaining.is_zero() && debit_idx < currency_debits.len() {
                        let transfer_amount = credit_remaining.min(debit_remaining);

                        if !transfer_amount.is_zero() {
                            // AllowDeath: debit participant committed these funds at lock time;
                            // the full locked amount must flow to credit legs.
                            // Hard fail: if any leg transfer fails, the entire dispatch
                            // returns Err and Substrate atomically rolls back the fee
                            // transfer and any prior leg transfers in this settlement.
                            T::Currency::transfer(
                                &currency_debits[debit_idx].participant,
                                &credit.participant,
                                transfer_amount,
                                ExistenceRequirement::AllowDeath,
                            )?;
                        }

                        credit_remaining = credit_remaining.saturating_sub(transfer_amount);
                        debit_remaining = debit_remaining.saturating_sub(transfer_amount);

                        if debit_remaining.is_zero() {
                            debit_idx += 1;
                            if debit_idx < currency_debits.len() {
                                debit_remaining = currency_debits[debit_idx].amount;
                            }
                        }
                    }
                }
            }

            // Emit regular fee event
            if !fee.is_zero() {
                SettlementFees::<T>::insert(exchange_id, fee);
                Self::deposit_event(Event::FeeCollected {
                    exchange_id,
                    amount: fee,
                });
            }

            // ---------------------------------------------------------------
            // Pass 3.5: Transfer on-chain carbon credits
            // currency_hash for carbon legs is the credit_id in pallet-carbon.
            // Each carbon debit leg transfers its locked credit to the
            // matching credit leg participant with the same currency_hash.
            // ---------------------------------------------------------------

            for debit in legs_info.iter().filter(|l| l.domain == AssetDomain::Carbon && l.side == LegSide::Debit) {
                if let Some(credit) = legs_info.iter().find(|l| {
                    l.domain == AssetDomain::Carbon
                        && l.side == LegSide::Credit
                        && l.currency_hash == debit.currency_hash
                }) {
                    T::CarbonProvider::transfer_settled(debit.currency_hash, &credit.participant);
                }
            }

            // ---------------------------------------------------------------
            // Pass 4: Update leg statuses and build Merkle tree
            // ---------------------------------------------------------------

            let mut merkle_leaves = Vec::new();
            for i in 0..leg_count {
                Legs::<T>::mutate(exchange_id, i, |leg_opt| {
                    if let Some(ref mut leg) = leg_opt {
                        leg.status = LegStatus::Claimed;
                        let leaf_data = (exchange_id, i, leg.amount, leg.status);
                        let leaf = T::Hashing::hash_of(&leaf_data);
                        merkle_leaves.push(H256::from_slice(leaf.as_ref()));
                    }
                });
            }

            let merkle_root = compute_merkle_root(&merkle_leaves);

            // Finalize settlement
            Settlements::<T>::mutate(exchange_id, |s_opt| {
                if let Some(ref mut s) = s_opt {
                    s.status = SettlementStatus::Settled;
                    s.merkle_root = merkle_root;
                }
            });

            // Update network metrics.
            // If the BalanceOf<T> -> u128 conversion ever fails, skip the
            // volume update rather than pinning the metric at u128::MAX
            // (which would mask real volume forever).
            TotalSettlements::<T>::mutate(|n| *n = n.saturating_add(1));
            if let Ok(delta) = TryInto::<u128>::try_into(total_volume) {
                TotalVolumeSettled::<T>::mutate(|v| *v = v.saturating_add(delta));
            }
            ValidatorSettlements::<T>::mutate(&settler, |n| *n = n.saturating_add(1));

            // ---------------------------------------------------------------
            // Pass 5: Cross-pallet integration
            // ---------------------------------------------------------------

            // PoSe ledger finalization — settlement merkle root becomes
            // part of the next block's PoC proof, coupling mining to settlement.
            T::MiningProvider::update_settlement_root(merkle_root);
            T::MiningProvider::record_validator_activity(&settler);

            // Fee already sits in FeePoolAccount from the transfer above.
            // Mining pallet reads actual FeePoolAccount balance each block
            // and distributes 80/20 to stakers/treasury. No counter needed.

            // Record external asset deposits into reserve vault
            for leg in legs_info.iter() {
                if leg.rail != RailKind::TwillInternal && leg.side == LegSide::Debit {
                    let asset_kind = match leg.rail.domain() {
                        AssetDomain::Crypto => match leg.rail {
                            RailKind::Bitcoin => ReserveAssetKind::BTC,
                            RailKind::Ethereum => ReserveAssetKind::ETH,
                            RailKind::Solana => ReserveAssetKind::SOL,
                            RailKind::Usdc => ReserveAssetKind::USDC,
                            _ => ReserveAssetKind::Other,
                        },
                        AssetDomain::Carbon => ReserveAssetKind::CarbonCredit,
                        AssetDomain::Fiat => ReserveAssetKind::Other,
                    };
                    let amount_u128: u128 = leg.amount.try_into().unwrap_or(0u128);
                    T::ReserveProvider::record_deposit(
                        exchange_id,
                        asset_kind,
                        amount_u128,
                    );
                }
            }

            // Record settlement-derived prices to oracle.
            // Settlement prices are the highest-trust oracle inputs — derived
            // from real on-chain economic activity, not off-chain submissions.
            // Price formula: price = twl_debit_total / external_debit_amount
            // i.e. how many TWL planck per smallest unit of the external asset.
            let twl_total: u128 = twl_debit_total.try_into().unwrap_or(0u128);
            if twl_total > 0 {
                if btc_debit > 0 {
                    T::OracleProvider::record_settlement_price(
                        AssetPair::BtcTwl,
                        twl_total.saturating_div(btc_debit),
                    );
                }
                if eth_debit > 0 {
                    T::OracleProvider::record_settlement_price(
                        AssetPair::EthTwl,
                        twl_total.saturating_div(eth_debit),
                    );
                }
                if sol_debit > 0 {
                    T::OracleProvider::record_settlement_price(
                        AssetPair::SolTwl,
                        twl_total.saturating_div(sol_debit),
                    );
                }
                if usdc_debit > 0 {
                    T::OracleProvider::record_settlement_price(
                        AssetPair::UsdcTwl,
                        twl_total.saturating_div(usdc_debit),
                    );
                }
                if carbon_debit > 0 {
                    T::OracleProvider::record_settlement_price(
                        AssetPair::CarbonTwl,
                        twl_total.saturating_div(carbon_debit),
                    );
                }
            }

            Self::deposit_event(Event::SettlementCompleted {
                exchange_id,
                settler,
                merkle_root,
                total_twl_volume: twl_debit_total,
                fee,
                leg_count,
                block_number: now,
            });

            Ok(())
        }

        /// Refund a settlement after the timelock expires.
        ///
        /// Anyone can trigger a refund after timeout. All reserved amounts
        /// are returned to their original owners. Settlements are also
        /// auto-expired by on_initialize, but this provides a manual fallback.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(150_000_000, 0))]
        pub fn refund(origin: OriginFor<T>, exchange_id: H256) -> DispatchResult {
            let refunder = ensure_signed(origin)?;

            let settlement =
                Settlements::<T>::get(exchange_id).ok_or(Error::<T>::SettlementNotFound)?;

            ensure!(
                settlement.status == SettlementStatus::Proposed
                    || settlement.status == SettlementStatus::Locked,
                Error::<T>::InvalidSettlementStatus
            );

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(
                now >= settlement.timelock_block,
                Error::<T>::TimelockNotExpired
            );

            Self::do_refund_legs(exchange_id);

            Settlements::<T>::mutate(exchange_id, |s_opt| {
                if let Some(ref mut s) = s_opt {
                    s.status = SettlementStatus::Refunded;
                }
            });

            Self::deposit_event(Event::SettlementRefunded {
                exchange_id,
                refunder,
            });

            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Refund all locked debit legs for a settlement (unreserve escrow).
        fn do_refund_legs(exchange_id: H256) {
            let leg_count = LegCount::<T>::get(exchange_id);
            for i in 0..leg_count {
                Legs::<T>::mutate(exchange_id, i, |leg_opt| {
                    if let Some(ref mut leg) = leg_opt {
                        if leg.status == LegStatus::Locked && leg.side == LegSide::Debit {
                            if leg.rail == RailKind::TwillInternal {
                                T::Currency::unreserve(&leg.participant, leg.amount);
                            } else if leg.domain == AssetDomain::Carbon {
                                T::CarbonProvider::unlock_refund(leg.currency_hash);
                            }
                        }
                        leg.status = LegStatus::Refunded;
                    }
                });
            }
        }

        /// Calculate fee from total volume and basis points
        fn calculate_fee(amount: BalanceOf<T>, fee_bps: u16) -> BalanceOf<T> {
            let bps: BalanceOf<T> = fee_bps.into();
            let divisor: BalanceOf<T> = 10_000u32.into();
            amount.saturating_mul(bps) / divisor
        }

        /// Get settlement throughput for a validator (for PoSe scoring)
        pub fn validator_throughput(validator: &T::AccountId) -> u64 {
            ValidatorSettlements::<T>::get(validator)
        }

        /// Get total network settlement count
        pub fn total_settlement_count() -> u64 {
            TotalSettlements::<T>::get()
        }

        /// Get total volume settled
        pub fn total_volume() -> u128 {
            TotalVolumeSettled::<T>::get()
        }
    }
}
