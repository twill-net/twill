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
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
    };
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::{traits::{Saturating, Zero}, SaturatedConversion};
    use twill_primitives::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

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

        /// TWL currency — used to lock and burn tokens on redemption
        type Currency: ReservableCurrency<Self::AccountId>;
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

    /// Pending and completed redemption requests
    #[pallet::storage]
    pub type RedemptionRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, u64, RedemptionRequest<T>>;

    /// Monotonic redemption ID counter
    #[pallet::storage]
    pub type NextRedemptionId<T: Config> = StorageValue<_, u64, ValueQuery>;
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

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum RedemptionStatus {
        /// TWL locked, awaiting board fulfillment
        Pending,
        /// Off-chain asset sent; TWL burned
        Fulfilled,
        /// Cancelled; TWL returned to holder
        Cancelled,
    }

    /// A request to redeem TWL for a pro-rata share of reserve assets.
    ///
    /// Flow:
    ///   1. Holder calls `request_redemption(desired_asset, twl_amount)`
    ///      → TWL is reserved (locked), request stored as Pending
    ///   2. Board processes off-chain: sends BTC/ETH/SOL to holder's external address
    ///   3. Board calls `fulfill_redemption(request_id)`
    ///      → TWL burned, status set to Fulfilled
    ///   4. Alternatively, holder calls `cancel_redemption(request_id)`
    ///      → TWL unreserved, status Cancelled
    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct RedemptionRequest<T: Config> {
        /// Account that requested redemption
        pub who: T::AccountId,
        /// Which reserve asset they want to receive
        pub desired_asset: ReserveAssetKind,
        /// Amount of TWL locked (to be burned on fulfillment)
        pub twl_locked: BalanceOf<T>,
        /// Expected asset amount calculated at request time (informational, in asset's smallest unit)
        pub expected_asset_amount: u128,
        pub requested_at: BlockNumberFor<T>,
        pub status: RedemptionStatus,
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
        /// A redemption request was submitted (TWL locked).
        RedemptionRequested {
            request_id: u64,
            who: T::AccountId,
            desired_asset: ReserveAssetKind,
            twl_locked: BalanceOf<T>,
            expected_asset_amount: u128,
        },
        /// Board fulfilled a redemption — TWL burned, off-chain asset sent.
        RedemptionFulfilled {
            request_id: u64,
            who: T::AccountId,
            twl_burned: BalanceOf<T>,
            asset: ReserveAssetKind,
            asset_amount: u128,
        },
        /// Redemption cancelled — TWL returned to holder.
        /// The requester cancelled their own pending redemption.
        RedemptionCancelled { request_id: u64, who: T::AccountId, twl_returned: BalanceOf<T> },
        /// Governance (root-origin via runtime upgrade) cancelled a pending
        /// redemption — typically because the off-chain transfer is impossible.
        RedemptionForceCancelled { request_id: u64, who: T::AccountId, twl_returned: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        ZeroValue,
        ArithmeticOverflow,
        DuplicateDeposit,
        /// Redemption request not found
        RedemptionNotFound,
        /// Redemption is not in Pending state (already fulfilled or cancelled)
        RedemptionNotPending,
        /// Only the requester or root can cancel a redemption
        NotRequester,
        /// The desired asset has no reserve backing (cannot redeem for it)
        AssetNotInReserve,
        /// Oracle price unavailable — cannot compute redemption amount
        OraclePriceUnavailable,
        /// TWL amount must be greater than zero
        ZeroAmount,
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

        /// Request to redeem TWL for a pro-rata share of a reserve asset.
        ///
        /// TWL is locked (reserved) immediately. The board processes the off-chain
        /// transfer and calls `fulfill_redemption` to complete the burn. The caller
        /// can cancel at any time before fulfillment to retrieve their locked TWL.
        ///
        /// The `expected_asset_amount` is calculated at request time from the current
        /// oracle price and floor price — it is stored informatively and does not bind
        /// the fulfillment amount (which is determined at the time of off-chain transfer).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(80_000_000, 0))]
        pub fn request_redemption(
            origin: OriginFor<T>,
            desired_asset: ReserveAssetKind,
            twl_amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!twl_amount.is_zero(), Error::<T>::ZeroAmount);

            let total_reserve = TotalReserveValue::<T>::get();
            ensure!(total_reserve > 0, Error::<T>::AssetNotInReserve);

            let asset_reserve = ReserveByAsset::<T>::get(desired_asset);
            ensure!(asset_reserve > 0, Error::<T>::AssetNotInReserve);

            // Compute expected asset amount for informational purposes.
            // Pro-rata formula (safe — no u128 overflow):
            //   step 1: user's share of supply in basis points
            //   step 2: their entitlement in TWL-value
            //   step 3: convert to asset units via oracle
            let twl_u128: u128 = twl_amount.saturated_into();
            let circulating_u128: u128 = T::Currency::total_issuance().saturated_into();

            let expected_asset_amount = if circulating_u128 > 0 {
                // share_bps = twl / circulating, scaled to 10_000
                let share_bps = twl_u128.saturating_mul(10_000) / circulating_u128;
                // entitlement in TWL-value units
                let entitlement_twl = asset_reserve.saturating_mul(share_bps) / 10_000;

                // Convert TWL-value to asset native units via oracle
                let oracle_pair = match desired_asset {
                    ReserveAssetKind::BTC  => Some(AssetPair::BtcTwl),
                    ReserveAssetKind::ETH  => Some(AssetPair::EthTwl),
                    ReserveAssetKind::SOL  => Some(AssetPair::SolTwl),
                    ReserveAssetKind::USDC => Some(AssetPair::UsdcTwl),
                    ReserveAssetKind::CarbonCredit => Some(AssetPair::CarbonTwl),
                    ReserveAssetKind::Other => None,
                };

                if let Some(pair) = oracle_pair {
                    if let Some(price) = T::Oracle::get_price(pair) {
                        if price > 0 {
                            entitlement_twl.saturating_mul(TWILL) / price
                        } else { 0 }
                    } else { 0 }
                } else { 0 }
            } else { 0 };

            // Lock the TWL
            T::Currency::reserve(&who, twl_amount)?;

            let request_id = NextRedemptionId::<T>::get();
            NextRedemptionId::<T>::put(request_id.saturating_add(1));

            let now = frame_system::Pallet::<T>::block_number();
            RedemptionRequests::<T>::insert(request_id, RedemptionRequest {
                who: who.clone(),
                desired_asset,
                twl_locked: twl_amount,
                expected_asset_amount,
                requested_at: now,
                status: RedemptionStatus::Pending,
            });

            Self::deposit_event(Event::RedemptionRequested {
                request_id,
                who,
                desired_asset,
                twl_locked: twl_amount,
                expected_asset_amount,
            });

            Ok(())
        }

        /// Fulfill a pending redemption request. Root only.
        ///
        /// Called by the board after the off-chain asset transfer has been completed.
        /// Burns the requester's locked TWL and marks the request Fulfilled.
        ///
        /// `asset_amount_sent` is the actual asset amount sent off-chain (recorded on-chain
        /// for transparency; may differ from expected due to price movement between request
        /// and fulfillment).
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(60_000_000, 0))]
        pub fn fulfill_redemption(
            origin: OriginFor<T>,
            request_id: u64,
            asset_amount_sent: u128,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut request = RedemptionRequests::<T>::get(request_id)
                .ok_or(Error::<T>::RedemptionNotFound)?;
            ensure!(request.status == RedemptionStatus::Pending, Error::<T>::RedemptionNotPending);

            // Burn the locked TWL (slash_reserved reduces total issuance)
            let twl_burned = request.twl_locked;
            let (imbalance, _) = T::Currency::slash_reserved(&request.who, twl_burned);
            drop(imbalance); // Dropping the imbalance burns it (reduces total issuance)

            request.status = RedemptionStatus::Fulfilled;
            RedemptionRequests::<T>::insert(request_id, &request);

            Self::deposit_event(Event::RedemptionFulfilled {
                request_id,
                who: request.who,
                twl_burned,
                asset: request.desired_asset,
                asset_amount: asset_amount_sent,
            });

            Ok(())
        }

        /// Cancel a pending redemption and return the locked TWL.
        ///
        /// The requester can cancel their own pending request at any time before
        /// fulfillment. Root can also cancel any pending request (e.g. if the
        /// off-chain transfer cannot be completed).
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(40_000_000, 0))]
        pub fn cancel_redemption(
            origin: OriginFor<T>,
            request_id: u64,
        ) -> DispatchResult {
            // Accept either root (board override) or a signed account (self-cancel)
            let is_root = ensure_root(origin.clone()).is_ok();
            let caller_opt: Option<T::AccountId> = if is_root {
                None
            } else {
                Some(ensure_signed(origin)?)
            };

            let mut request = RedemptionRequests::<T>::get(request_id)
                .ok_or(Error::<T>::RedemptionNotFound)?;
            ensure!(request.status == RedemptionStatus::Pending, Error::<T>::RedemptionNotPending);

            // Signed callers may only cancel their own requests
            if let Some(ref caller) = caller_opt {
                ensure!(caller == &request.who, Error::<T>::NotRequester);
            }

            // Return the locked TWL
            let twl_returned = request.twl_locked;
            T::Currency::unreserve(&request.who, twl_returned);

            request.status = RedemptionStatus::Cancelled;
            let who = request.who.clone();
            RedemptionRequests::<T>::insert(request_id, &request);

            if is_root {
                Self::deposit_event(Event::RedemptionForceCancelled {
                    request_id,
                    who,
                    twl_returned,
                });
            } else {
                Self::deposit_event(Event::RedemptionCancelled {
                    request_id,
                    who,
                    twl_returned,
                });
            }

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
                ReserveAssetKind::USDC,
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
                ReserveAssetKind::USDC => Some(AssetPair::UsdcTwl),
                ReserveAssetKind::CarbonCredit => Some(AssetPair::CarbonTwl),
                ReserveAssetKind::Other => None,
            };

            if let Some(p) = pair {
                if let Some(price) = T::Oracle::get_price(p) {
                    if price > 0 {
                        // `original_amount * price` can exceed u128 for large reserves
                        // at high oracle prices (both factors can approach 2^66 in
                        // planck units). Promote to U256 for the multiply, then divide
                        // back to u128. Same pattern as the retarget and fee share.
                        let wide = sp_core::U256::from(original_amount)
                            .saturating_mul(sp_core::U256::from(price))
                            / sp_core::U256::from(TWILL);
                        return if wide > sp_core::U256::from(u128::MAX) {
                            u128::MAX
                        } else {
                            wide.low_u128()
                        };
                    }
                }
                // Oracle stale or price zero — return 0, not raw amount.
                // A missing price must not inflate the reserve valuation.
                return 0;
            }
            0
        }

        /// Returns reserve composition as basis points per first-class asset:
        /// (BTC, ETH, SOL, USDC, CarbonCredit). The `Other` bucket is excluded
        /// from the tuple — it can be inspected directly via `ReserveByAsset`.
        pub fn composition() -> (u16, u16, u16, u16, u16) {
            let total = TotalReserveValue::<T>::get();
            if total == 0 { return (0, 0, 0, 0, 0); }
            // bps is mathematically bounded by 10_000 because v <= total, but
            // we clamp explicitly so the u16 cast can never silently truncate
            // even if a future change to total accounting introduces drift.
            let to_bps = |v: u128| -> u16 {
                let bps = v.saturating_mul(10_000) / total;
                bps.min(10_000) as u16
            };
            (
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::BTC)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::ETH)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::SOL)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::USDC)),
                to_bps(ReserveByAsset::<T>::get(ReserveAssetKind::CarbonCredit)),
            )
        }
    }
}
