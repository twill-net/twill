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

    /// Per-relayer confirmation records: (exchange_id, relayer) → deposit info
    #[pallet::storage]
    pub type RelayerConfirmations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat, H256,
        Blake2_128Concat, T::AccountId,
        BridgeDepositInfo<T>,
    >;

    /// exchange_id → confirmed (true once threshold reached)
    #[pallet::storage]
    pub type ConfirmedDeposits<T: Config> = StorageMap<_, Blake2_128Concat, H256, bool, ValueQuery>;

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
            let bounded: BoundedVec<T::AccountId, T::MaxRelayers> =
                self.relayers.clone().try_into().expect("too many genesis relayers");
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
        /// A relayer submitted a deposit confirmation
        PartialConfirmation {
            exchange_id: H256,
            relayer: T::AccountId,
            confirmations: u8,
            threshold: u8,
        },
        /// Deposit reached threshold — settlement may proceed
        DepositConfirmed { exchange_id: H256, asset: BridgeAsset, amount: u128 },
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
        /// Relayer confirms an off-chain deposit.
        /// All relayers must submit the same `chain_txid` and `amount` — mismatches
        /// are rejected to prevent a single malicious relayer from poisoning the set.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(60_000_000, 0))]
        pub fn confirm_deposit(
            origin: OriginFor<T>,
            exchange_id: H256,
            asset: BridgeAsset,
            chain_txid: H256,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let relayers = Relayers::<T>::get();
            ensure!(relayers.contains(&who), Error::<T>::NotRelayer);

            ensure!(
                !RelayerConfirmations::<T>::contains_key(exchange_id, &who),
                Error::<T>::AlreadyConfirmed
            );

            // If other relayers have already confirmed, ensure txid+amount match
            let existing: Vec<_> = RelayerConfirmations::<T>::iter_prefix(exchange_id).collect();
            if let Some((_, first)) = existing.first() {
                ensure!(
                    first.chain_txid == chain_txid && first.amount == amount,
                    Error::<T>::ConflictingConfirmation
                );
            }

            let now = frame_system::Pallet::<T>::block_number();
            RelayerConfirmations::<T>::insert(
                exchange_id,
                &who,
                BridgeDepositInfo { asset, chain_txid, amount, confirmed_at: now },
            );

            let threshold = ConfirmationThreshold::<T>::get();
            let count = (existing.len() as u8).saturating_add(1);

            if count >= threshold {
                ConfirmedDeposits::<T>::insert(exchange_id, true);
                Self::deposit_event(Event::DepositConfirmed { exchange_id, asset, amount });
            } else {
                Self::deposit_event(Event::PartialConfirmation {
                    exchange_id,
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
        fn is_deposit_confirmed(exchange_id: H256) -> bool {
            ConfirmedDeposits::<T>::get(exchange_id)
        }
    }
}
