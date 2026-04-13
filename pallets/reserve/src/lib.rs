//! # Reserve Pallet
//!
//! Manages the Twill Reserve Vault — protocol-owned backing for TWL.
//! Fully autonomous — no admin extrinsics. Deposits happen only via
//! the settlement pallet's trait interface.
//!
//! ## BTC Model
//!
//! No human can manually deposit into or withdraw from the reserve.
//! The settlement engine is the only path in. The reserve grows
//! organically from real economic activity. Snapshots are automatic.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::Saturating;
    use twill_primitives::*;

    /// Snapshot interval: every 100 blocks (~10 minutes). Hardcoded, not configurable.
    const SNAPSHOT_INTERVAL: u32 = 100;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::constant]
        type MaxReserveAssets: Get<u32>;

        /// Oracle for price feed queries (asset valuation)
        type Oracle: twill_primitives::OracleInterface;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    #[pallet::storage]
    pub type TotalReserveValue<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type ReserveByAsset<T: Config> =
        StorageMap<_, Blake2_128Concat, ReserveAssetKind, u128, ValueQuery>;
    #[pallet::storage]
    pub type DepositCount<T: Config> = StorageValue<_, u64, ValueQuery>;
    #[pallet::storage]
    pub type ReserveSnapshots<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, u128>;
    #[pallet::storage]
    pub type LastSnapshotBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;
    #[pallet::storage]
    pub type Deposits<T: Config> = StorageMap<
        _, Blake2_128Concat, H256, ReserveDeposit<T>,
    >;

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct ReserveDeposit<T: Config> {
        pub settlement_id: H256,
        pub asset_kind: ReserveAssetKind,
        pub value_twl: u128,
        pub original_amount: u128,
        pub deposited_at: BlockNumberFor<T>,
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
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
        /// Reserve revalued at current oracle prices (mark-to-market).
        /// Permissionless — anyone can call revalue() at any time.
        ReserveRevalued {
            old_total: u128,
            new_total: u128,
            block_number: BlockNumberFor<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ZeroValue,
        ArithmeticOverflow,
        DuplicateDeposit,
    }

    // -----------------------------------------------------------------------
    // Hooks — automatic snapshots
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let interval: BlockNumberFor<T> = SNAPSHOT_INTERVAL.into();
            let last = LastSnapshotBlock::<T>::get();

            if now.saturating_sub(last) >= interval {
                let total = TotalReserveValue::<T>::get();
                ReserveSnapshots::<T>::insert(now, total);
                LastSnapshotBlock::<T>::put(now);
                Self::deposit_event(Event::ReserveSnapshot {
                    block_number: now, total_value: total,
                });
                return Weight::from_parts(10_000_000, 0);
            }

            Weight::zero()
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Permissionless mark-to-market: revalue all reserve deposits at current oracle prices.
        ///
        /// Any account can call this. It is O(all deposits) but has no state risk —
        /// it only reads oracle prices and updates the stored TWL valuations.
        ///
        /// Useful after a large oracle price move to bring `TotalReserveValue` up to date
        /// so that `floor_price()` returns an accurate current reading.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(500_000_000, 0))]
        pub fn revalue(origin: OriginFor<T>) -> DispatchResult {
            ensure_signed(origin)?; // Anyone may call
            Self::revalue_reserves();
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // ReserveInterface — the ONLY way deposits enter
    // -----------------------------------------------------------------------

    impl<T: Config> twill_primitives::ReserveInterface for Pallet<T> {
        fn record_deposit(
            settlement_id: H256,
            asset_kind: ReserveAssetKind,
            original_amount: u128,
        ) {
            if original_amount == 0 || Deposits::<T>::contains_key(settlement_id) {
                return;
            }

            let value_twl = Self::oracle_value_twl(asset_kind, original_amount);
            if value_twl == 0 {
                return;
            }

            let now = frame_system::Pallet::<T>::block_number();

            Deposits::<T>::insert(settlement_id, ReserveDeposit {
                settlement_id, asset_kind, value_twl, original_amount, deposited_at: now,
            });

            TotalReserveValue::<T>::mutate(|t| *t = t.saturating_add(value_twl));
            ReserveByAsset::<T>::mutate(asset_kind, |s| *s = s.saturating_add(value_twl));
            DepositCount::<T>::mutate(|n| *n = n.saturating_add(1));

            let total_reserve = TotalReserveValue::<T>::get();
            Self::deposit_event(Event::ReserveDeposited {
                settlement_id, asset_kind, value_twl, total_reserve,
            });
        }
    }

    // -----------------------------------------------------------------------
    // Public read-only interface
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        pub fn total_reserve() -> u128 { TotalReserveValue::<T>::get() }

        pub fn reserve_for_asset(asset: ReserveAssetKind) -> u128 {
            ReserveByAsset::<T>::get(asset)
        }

        pub fn floor_price(circulating_supply: u128) -> u128 {
            if circulating_supply == 0 { return 0; }
            TotalReserveValue::<T>::get().saturating_mul(TWILL) / circulating_supply
        }

        pub fn deposit_count() -> u64 { DepositCount::<T>::get() }

        /// Revalue all reserve deposits at current oracle prices (mark-to-market).
        /// Called by the permissionless `revalue` extrinsic.
        pub fn revalue_reserves() {
            let old_total = TotalReserveValue::<T>::get();
            let now = frame_system::Pallet::<T>::block_number();

            // Rebuild totals from scratch
            let mut new_total: u128 = 0;
            let mut new_by_asset: sp_std::collections::btree_map::BTreeMap<u8, u128> =
                sp_std::collections::btree_map::BTreeMap::new();

            // Iterate all deposits and recompute current oracle value
            for (_, mut deposit) in Deposits::<T>::iter() {
                let new_value = Self::oracle_value_twl(deposit.asset_kind, deposit.original_amount);
                // Store updated value back into the deposit record
                deposit.value_twl = new_value;
                Deposits::<T>::insert(deposit.settlement_id, deposit.clone());

                new_total = new_total.saturating_add(new_value);
                let asset_key = deposit.asset_kind as u8;
                let entry = new_by_asset.entry(asset_key).or_insert(0u128);
                *entry = entry.saturating_add(new_value);
            }

            TotalReserveValue::<T>::put(new_total);

            // Update per-asset totals
            let assets = [
                ReserveAssetKind::BTC,
                ReserveAssetKind::ETH,
                ReserveAssetKind::SOL,
                ReserveAssetKind::CarbonCredit,
                ReserveAssetKind::Other,
            ];
            for asset in assets {
                let val = *new_by_asset.get(&(asset as u8)).unwrap_or(&0u128);
                ReserveByAsset::<T>::insert(asset, val);
            }

            Self::deposit_event(Event::ReserveRevalued {
                old_total,
                new_total,
                block_number: now,
            });
        }

        /// Oracle-based asset valuation. Falls back to raw amount.
        pub fn oracle_value_twl(asset_kind: ReserveAssetKind, original_amount: u128) -> u128 {
            let pair = match asset_kind {
                ReserveAssetKind::BTC => Some(AssetPair::BtcTwl),
                ReserveAssetKind::ETH => Some(AssetPair::EthTwl),
                ReserveAssetKind::SOL => Some(AssetPair::SolTwl),
                ReserveAssetKind::CarbonCredit => Some(AssetPair::CarbonTwl),
                ReserveAssetKind::Other => None,
            };

            if let Some(p) = pair {
                if let Some(price) = T::Oracle::get_price(p) {
                    if price > 0 {
                        return original_amount.saturating_mul(price) / TWILL;
                    }
                }
                // Oracle stale or price zero — return 0, not raw amount.
                // A missing price must not inflate the reserve valuation.
                return 0;
            }
            0
        }

        pub fn composition() -> (u16, u16, u16, u16) {
            let total = TotalReserveValue::<T>::get();
            if total == 0 { return (0, 0, 0, 0); }
            let to_bps = |v: u128| -> u16 { ((v.saturating_mul(10_000)) / total) as u16 };
            (
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::BTC)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::ETH)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::SOL)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::CarbonCredit)),
            )
        }
    }
}
