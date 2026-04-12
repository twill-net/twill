//! # Oracle Pallet
//!
//! Permissionless price oracle for the Twill Network.
//! No admin keys — staking IS authorization.
//!
//! ## How It Works
//!
//! Any active PoSe validator can submit price feeds. No root approval
//! needed. The oracle checks validator status via the ValidatorOracle
//! trait (implemented by the mining pallet).
//!
//! Canonical price = median of non-stale submissions from active validators.
//! Settlement-derived prices (from actual trades) take priority.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;
    use sp_std::vec::Vec;
    use twill_primitives::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum price submissions tracked per pair
        #[pallet::constant]
        type MaxSubmitters: Get<u32>;

        /// Blocks after which a price is stale
        #[pallet::constant]
        type StalenessThreshold: Get<BlockNumberFor<Self>>;

        /// Validator status check — permissionless authorization.
        /// If you're staked as a validator, you can submit prices.
        type ValidatorCheck: twill_primitives::ValidatorOracle<Self::AccountId>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Price submissions: (AssetPair, AccountId) -> (price, block)
    #[pallet::storage]
    pub type PriceFeeds<T: Config> = StorageDoubleMap<
        _, Blake2_128Concat, AssetPair, Blake2_128Concat, T::AccountId,
        (u128, BlockNumberFor<T>),
    >;

    /// Canonical price per asset pair (median of valid submissions)
    #[pallet::storage]
    pub type CanonicalPrices<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetPair, CanonicalPrice<T>>;

    /// Settlement-derived prices (from actual trades)
    #[pallet::storage]
    pub type SettlementPrices<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetPair, CanonicalPrice<T>>;

    /// Track which validators have submitted for median calculation
    #[pallet::storage]
    pub type ActiveSubmitters<T: Config> = StorageMap<
        _, Blake2_128Concat, AssetPair,
        BoundedVec<T::AccountId, T::MaxSubmitters>,
        ValueQuery,
    >;

    /// Last known canonical price per asset pair, used for stale-price fallback.
    /// Stores (price, block_number_when_computed).
    #[pallet::storage]
    pub type LastKnownPrice<T: Config> =
        StorageMap<_, Blake2_128Concat, AssetPair, (u128, BlockNumberFor<T>), OptionQuery>;

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct CanonicalPrice<T: Config> {
        pub price: u128,
        pub updated_at: BlockNumberFor<T>,
        pub source_count: u8,
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PriceSubmitted { submitter: T::AccountId, pair: AssetPair, price: u128 },
        CanonicalPriceUpdated { pair: AssetPair, price: u128, source_count: u8 },
        SettlementPriceUpdated { pair: AssetPair, price: u128 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Caller is not an active staked validator
        NotActiveValidator,
        /// Price cannot be zero
        ZeroPrice,
    }

    // -----------------------------------------------------------------------
    // Extrinsics — ALL permissionless
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a price feed. Permissionless for any staked validator.
        /// No admin approval needed — your stake IS your authorization.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn submit_price(
            origin: OriginFor<T>,
            pair: AssetPair,
            price: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(price > 0, Error::<T>::ZeroPrice);

            // Permissionless check: are you a staked validator?
            ensure!(
                T::ValidatorCheck::is_active_validator(&who),
                Error::<T>::NotActiveValidator
            );

            let now = frame_system::Pallet::<T>::block_number();
            PriceFeeds::<T>::insert(pair, &who, (price, now));

            // Track this submitter for the pair
            ActiveSubmitters::<T>::try_mutate(pair, |submitters| -> Result<(), ()> {
                if !submitters.contains(&who) {
                    let _ = submitters.try_push(who.clone()); // Bounded — silently drops if full
                }
                Ok(())
            }).ok();

            Self::deposit_event(Event::PriceSubmitted { submitter: who, pair, price });

            Self::recalculate_median(pair);

            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Recalculate canonical price as median of non-stale validator submissions.
        fn recalculate_median(pair: AssetPair) {
            let now = frame_system::Pallet::<T>::block_number();
            let threshold = T::StalenessThreshold::get();
            let submitters = ActiveSubmitters::<T>::get(pair);

            let mut prices: Vec<u128> = Vec::new();
            for submitter in submitters.iter() {
                // Only count submissions from CURRENTLY active validators
                if !T::ValidatorCheck::is_active_validator(submitter) {
                    continue;
                }
                if let Some((price, block)) = PriceFeeds::<T>::get(pair, submitter) {
                    if now.saturating_sub(block) <= threshold {
                        prices.push(price);
                    }
                }
            }

            if prices.is_empty() { return; }

            prices.sort_unstable();
            let median = if prices.len() % 2 == 0 {
                let mid = prices.len() / 2;
                (prices[mid - 1].saturating_add(prices[mid])) / 2
            } else {
                prices[prices.len() / 2]
            };

            CanonicalPrices::<T>::insert(pair, CanonicalPrice {
                price: median, updated_at: now, source_count: prices.len() as u8,
            });

            // Snapshot for stale-price fallback
            LastKnownPrice::<T>::insert(pair, (median, now));

            Self::deposit_event(Event::CanonicalPriceUpdated {
                pair, price: median, source_count: prices.len() as u8,
            });
        }

        /// Record a settlement-derived price.
        pub fn record_settlement_price(pair: AssetPair, price: u128) {
            let now = frame_system::Pallet::<T>::block_number();
            SettlementPrices::<T>::insert(pair, CanonicalPrice {
                price, updated_at: now, source_count: 1,
            });
            Self::deposit_event(Event::SettlementPriceUpdated { pair, price });
        }
    }

    // -----------------------------------------------------------------------
    // OracleInterface — used by reserve pallet
    // -----------------------------------------------------------------------

    /// One era in blocks (14,400 blocks ~ 24 hours at 6s block time)
    const ERA_BLOCKS: u32 = 14_400;
    /// Maximum eras of staleness before fallback returns None
    const MAX_STALE_ERAS: u32 = 10;

    impl<T: Config> twill_primitives::OracleInterface for Pallet<T> {
        fn get_price(pair: AssetPair) -> Option<u128> {
            let now = frame_system::Pallet::<T>::block_number();
            let threshold = T::StalenessThreshold::get();

            // Settlement-derived price first (most trustworthy)
            if let Some(sp) = SettlementPrices::<T>::get(pair) {
                if now.saturating_sub(sp.updated_at) <= threshold {
                    return Some(sp.price);
                }
            }

            // Validator oracle fallback
            if let Some(cp) = CanonicalPrices::<T>::get(pair) {
                if now.saturating_sub(cp.updated_at) <= threshold {
                    return Some(cp.price);
                }
            }

            // Stale-price fallback: use LastKnownPrice with a 5% discount per
            // era of staleness, capped at MAX_STALE_ERAS (after that, None).
            if let Some((price, recorded_at)) = LastKnownPrice::<T>::get(pair) {
                let age = now.saturating_sub(recorded_at);
                let era_len: BlockNumberFor<T> = ERA_BLOCKS.into();
                let max_age: BlockNumberFor<T> = (ERA_BLOCKS * MAX_STALE_ERAS).into();

                if age > max_age {
                    return None;
                }

                // Count how many full eras have elapsed
                let mut eras_elapsed = 0u32;
                let mut cursor = era_len;
                while cursor <= age && eras_elapsed < MAX_STALE_ERAS {
                    eras_elapsed += 1;
                    cursor = cursor.saturating_add(era_len);
                }

                // Apply 5% discount per era: price * (95/100)^eras_elapsed
                let mut discounted = price;
                for _ in 0..eras_elapsed {
                    discounted = discounted.saturating_mul(95) / 100;
                }

                if discounted == 0 {
                    return None;
                }

                return Some(discounted);
            }

            None
        }

        fn is_stale(pair: AssetPair) -> bool {
            Self::get_price(pair).is_none()
        }
    }
}
