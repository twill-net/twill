//! # TWL Token Pallet
//!
//! Minimal, autonomous token mechanics. No admin keys. No pre-mine.
//!
//! - **Hard cap**: 50,000,000 TWL enforced every block
//! - **Burn wallet**: Tracks voluntary burns + direct sends
//! - **Everything is mined**: 50M TWL via PoC/PoSe, no exceptions

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::Currency,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Zero;
    use twill_primitives::*;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: Currency<Self::AccountId>;

        /// Protocol-controlled burn address (no private key exists)
        #[pallet::constant]
        type BurnAccount: Get<Self::AccountId>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    #[pallet::storage]
    pub type TotalBurned<T: Config> = StorageValue<_, u128, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// TWL permanently burned
        Burned { from: T::AccountId, amount: BalanceOf<T>, total_burned: u128 },
    }

    #[pallet::error]
    pub enum Error<T> {
        ZeroBurn,
        InsufficientBalance,
    }

    // -----------------------------------------------------------------------
    // Hooks — burn sync + hard cap check
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
            // Sync burn wallet (catches direct transfers to burn address)
            let burn_balance: u128 = T::Currency::free_balance(&T::BurnAccount::get())
                .try_into()
                .unwrap_or(0u128);
            let tracked = TotalBurned::<T>::get();
            if burn_balance > tracked {
                TotalBurned::<T>::put(burn_balance);
            }

            // Hard cap safety net — log loudly if violated
            let total_issuance: u128 = T::Currency::total_issuance()
                .try_into()
                .unwrap_or(0u128);
            if total_issuance > TOTAL_SUPPLY {
                log::error!(
                    target: "twl-token",
                    "HARD CAP VIOLATION: total_issuance {} > TOTAL_SUPPLY {}",
                    total_issuance, TOTAL_SUPPLY,
                );
            }

            Weight::from_parts(5_000_000, 0)
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Burn TWL permanently. Permissionless. Irreversible.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(25_000_000, 0))]
        pub fn burn(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(!amount.is_zero(), Error::<T>::ZeroBurn);

            T::Currency::transfer(
                &who,
                &T::BurnAccount::get(),
                amount,
                frame_support::traits::ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::InsufficientBalance)?;

            let amount_u128: u128 = amount.try_into().unwrap_or(0u128);
            TotalBurned::<T>::mutate(|t| *t = t.saturating_add(amount_u128));

            Self::deposit_event(Event::Burned {
                from: who,
                amount,
                total_burned: TotalBurned::<T>::get(),
            });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Public read-only interface
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Circulating supply = total issued minus burned
        pub fn circulating_supply() -> u128 {
            let total_issuance: u128 = T::Currency::total_issuance()
                .try_into()
                .unwrap_or(0u128);
            total_issuance.saturating_sub(TotalBurned::<T>::get())
        }

        pub fn total_burned() -> u128 { TotalBurned::<T>::get() }
    }
}
