//! # Bridge Pallet
//!
//! N-of-M relayer confirmation for off-chain asset deposits.
//! Relayers watch Bitcoin, Ethereum, and Solana chains and submit
//! on-chain confirmations. Once `ConfirmationThreshold` relayers agree
//! on the same txid and amount, the deposit is marked confirmed.
//!
//! Settlement pallet checks `is_deposit_confirmed(exchange_id)` before
//! executing any BTC/ETH/SOL leg.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of relayers
        #[pallet::constant]
        type MaxRelayers: Get<u32>;

        /// Maximum confirmations stored per deposit
        #[pallet::constant]
        type MaxConfirmationsPerDeposit: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum BridgeAsset {
        Bitcoin,
        Ethereum,
        Solana,
    }

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct BridgeDepositInfo<T: Config> {
        pub asset: BridgeAsset,
        /// External chain transaction hash
        pub chain_txid: H256,
        /// Amount in the asset's native smallest unit
        pub amount: u128,
        pub confirmed_at: BlockNumberFor<T>,
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Authorized relayer accounts
    #[pallet::storage]
    pub type Relayers<T: Config> = StorageValue<
        _,
        BoundedVec<T::AccountId, T::MaxRelayers>,
        ValueQuery,
    >;

    /// Number of matching confirmations required
    #[pallet::storage]
    pub type ConfirmationThreshold<T: Config> = StorageValue<_, u8, ValueQuery>;

    /// Per-relayer confirmation records: ((exchange_id, leg_index), relayer) → deposit info.
    /// Each external leg is confirmed independently — a settlement with both a BTC
    /// leg and an ETH leg requires two separate confirmation sets.
    #[pallet::storage]
    pub type RelayerConfirmations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, (H256, u32),
        Blake2_128Concat, T::AccountId,
        BridgeDepositInfo<T>,
    >;

    /// (exchange_id, leg_index) → confirmed (true once threshold reached)
    #[pallet::storage]
    pub type ConfirmedDeposits<T: Config> =
        StorageMap<_, Blake2_128Concat, (H256, u32), bool, ValueQuery>;

    // -----------------------------------------------------------------------
    // Genesis
    // -----------------------------------------------------------------------

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub relayers: Vec<T::AccountId>,
        pub threshold: u8,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Genesis config is author-supplied, so exceeding MaxRelayers
            // is a chain-spec error. Fail fast with a clear message rather
            // than letting the chain come up with a silently truncated or
            // empty relayer set.
            let bounded: BoundedVec<T::AccountId, T::MaxRelayers> =
                self.relayers.clone().try_into().expect(
                    "bridge genesis: relayers Vec exceeds MaxRelayers bound \
                     — shorten the genesis relayer list or raise MaxRelayers",
                );
            Relayers::<T>::put(bounded);
            ConfirmationThreshold::<T>::put(if self.threshold == 0 { 2 } else { self.threshold });
        }
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A relayer submitted a partial confirmation for a leg
        PartialConfirmation {
            exchange_id: H256,
            leg_index: u32,
            relayer: T::AccountId,
            confirmations: u8,
            threshold: u8,
        },
        /// Leg reached confirmation threshold — settlement may proceed for this leg
        DepositConfirmed { exchange_id: H256, leg_index: u32, asset: BridgeAsset, amount: u128 },
        RelayerAdded { who: T::AccountId },
        RelayerRemoved { who: T::AccountId },
        ThresholdChanged { new_threshold: u8 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Caller is not an authorized relayer
        NotRelayer,
        /// Relayer already confirmed this deposit
        AlreadyConfirmed,
        /// Too many relayers (governance limit)
        TooManyRelayers,
        /// Relayer not found
        RelayerNotFound,
        /// Threshold cannot be zero
        ZeroThreshold,
        /// Conflicting confirmation (different txid or amount than previous relayers)
        ConflictingConfirmation,
        /// Account is already an authorized relayer
        AlreadyRelayer,
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Relayer confirms an off-chain deposit for a specific leg within a settlement.
        ///
        /// `leg_index` is the 0-based position of the BTC/ETH/SOL leg within the
        /// settlement's leg list. A settlement with two external legs (e.g. BTC and ETH)
        /// requires separate `confirm_deposit` calls for each leg.
        ///
        /// All relayers must submit the same `chain_txid` and `amount` for a given
        /// (exchange_id, leg_index) — mismatches are rejected to prevent a single
        /// malicious relayer from poisoning the confirmation set.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(60_000_000, 0))]
        pub fn confirm_deposit(
            origin: OriginFor<T>,
            exchange_id: H256,
            leg_index: u32,
            asset: BridgeAsset,
            chain_txid: H256,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let relayers = Relayers::<T>::get();
            ensure!(relayers.contains(&who), Error::<T>::NotRelayer);

            let leg_key = (exchange_id, leg_index);

            ensure!(
                !RelayerConfirmations::<T>::contains_key(leg_key, &who),
                Error::<T>::AlreadyConfirmed
            );

            // If other relayers have already confirmed this leg, ensure txid+amount match
            let existing: Vec<_> = RelayerConfirmations::<T>::iter_prefix(leg_key).collect();
            if let Some((_, first)) = existing.first() {
                ensure!(
                    first.chain_txid == chain_txid && first.amount == amount,
                    Error::<T>::ConflictingConfirmation
                );
            }

            let now = frame_system::Pallet::<T>::block_number();
            RelayerConfirmations::<T>::insert(
                leg_key,
                &who,
                BridgeDepositInfo { asset, chain_txid, amount, confirmed_at: now },
            );

            let threshold = ConfirmationThreshold::<T>::get();
            let count = (existing.len() as u8).saturating_add(1);

            if count >= threshold {
                ConfirmedDeposits::<T>::insert(leg_key, true);
                Self::deposit_event(Event::DepositConfirmed { exchange_id, leg_index, asset, amount });
            } else {
                Self::deposit_event(Event::PartialConfirmation {
                    exchange_id,
                    leg_index,
                    relayer: who,
                    confirmations: count,
                    threshold,
                });
            }

            Ok(())
        }

        /// Add an authorized relayer. Root only.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(20_000_000, 0))]
        pub fn add_relayer(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Relayers::<T>::try_mutate(|list| {
                ensure!(!list.contains(&who), Error::<T>::AlreadyRelayer);
                list.try_push(who.clone()).map_err(|_| Error::<T>::TooManyRelayers)
            })?;
            Self::deposit_event(Event::RelayerAdded { who });
            Ok(())
        }

        /// Remove a relayer. Root only.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(20_000_000, 0))]
        pub fn remove_relayer(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Relayers::<T>::try_mutate(|list| -> Result<(), DispatchError> {
                let pos = list.iter().position(|x| x == &who)
                    .ok_or(Error::<T>::RelayerNotFound)?;
                list.remove(pos);
                Ok(())
            })?;
            Self::deposit_event(Event::RelayerRemoved { who });
            Ok(())
        }

        /// Set the confirmation threshold. Root only.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000_000, 0))]
        pub fn set_threshold(origin: OriginFor<T>, threshold: u8) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(threshold > 0, Error::<T>::ZeroThreshold);
            ConfirmationThreshold::<T>::put(threshold);
            Self::deposit_event(Event::ThresholdChanged { new_threshold: threshold });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // BridgeInterface implementation
    // -----------------------------------------------------------------------

    impl<T: Config> twill_primitives::BridgeInterface for Pallet<T> {
        fn is_deposit_confirmed(exchange_id: H256, leg_index: u32) -> bool {
            ConfirmedDeposits::<T>::get((exchange_id, leg_index))
        }
    }
}
