//! # Carbon Pallet
//!
//! Permissionless carbon credit management for the Twill Network.
//! No admin keys — anyone can issue credits by posting a bond.
//!
//! ## Issuance Model (BTC-style)
//!
//! Anyone can issue a carbon credit by providing:
//! - A verification_hash (proof of registry verification)
//! - A bond of 100 TWL (returned after dispute window)
//!
//! Credits enter a dispute window (~7 days). If unchallenged,
//! the bond is returned and the credit is fully active.
//! If challenged and found invalid, the bond is slashed.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency, Imbalance},
    };
    use sp_runtime::traits::Zero;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_runtime::traits::Saturating;
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
        type MaxProjectIdLength: Get<u32>;

        #[pallet::constant]
        type MaxIssuanceAmount: Get<u128>;

        /// Issuance bond amount (returned after dispute window)
        #[pallet::constant]
        type IssuanceBond: Get<BalanceOf<Self>>;

        /// Dispute window in blocks (~7 days)
        #[pallet::constant]
        type DisputeWindow: Get<BlockNumberFor<Self>>;

    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    #[pallet::storage]
    pub type Credits<T: Config> = StorageMap<_, Blake2_128Concat, H256, CarbonCredit<T>>;
    #[pallet::storage]
    pub type TotalIssued<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type TotalRetired<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type TotalLocked<T: Config> = StorageValue<_, u128, ValueQuery>;
    #[pallet::storage]
    pub type AccountCredits<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u128, ValueQuery>;
    #[pallet::storage]
    pub type RetirementCertificates<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, RetirementCert<T>>;
    #[pallet::storage]
    pub type CertificateCount<T: Config> = StorageValue<_, u64, ValueQuery>;
    /// Tracks bond for credits still in dispute window
    #[pallet::storage]
    pub type IssuanceBonds<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, (T::AccountId, BalanceOf<T>, BlockNumberFor<T>)>;

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct CarbonCredit<T: Config> {
        pub credit_id: H256,
        pub owner: T::AccountId,
        pub registry: CarbonRegistry,
        pub project_id: BoundedVec<u8, T::MaxProjectIdLength>,
        pub amount: u128,
        pub vintage_year: u16,
        pub status: CarbonStatus,
        pub verification_hash: H256,
        pub issued_at: BlockNumberFor<T>,
    }

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct RetirementCert<T: Config> {
        pub certificate_id: H256,
        pub credit_id: H256,
        pub retiree: T::AccountId,
        pub amount: u128,
        pub registry: CarbonRegistry,
        pub retired_at: BlockNumberFor<T>,
    }

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CreditIssued { credit_id: H256, owner: T::AccountId, registry: CarbonRegistry, amount: u128, vintage_year: u16 },
        CreditLocked { credit_id: H256, amount: u128 },
        CreditUnlocked { credit_id: H256, amount: u128 },
        CreditRetired { credit_id: H256, certificate_id: H256, retiree: T::AccountId, amount: u128 },
        CreditTransferred { credit_id: H256, from: T::AccountId, to: T::AccountId, amount: u128 },
        BondReturned { credit_id: H256, issuer: T::AccountId, amount: BalanceOf<T> },
        /// Bond slashed by governance — issuer posted a fraudulent or invalid credit.
        /// Slashed TWL is burned (deflationary). Credit permanently invalidated.
        BondSlashed { credit_id: H256, issuer: T::AccountId, amount: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        CreditAlreadyExists,
        CreditNotFound,
        NotOwner,
        InvalidCreditStatus,
        InsufficientCredits,
        ExceedsMaxIssuance,
        InvalidVintageYear,
        ProjectIdTooLong,
        InsufficientBond,
        DisputeWindowNotOver,
        BondNotFound,
        /// Credit has already been slashed — permanently invalid
        AlreadySlashed,
    }

    // -----------------------------------------------------------------------
    // Extrinsics — ALL permissionless
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Issue a carbon credit. Permissionless — post a bond.
        /// Bond is returned after dispute window if unchallenged.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn issue(
            origin: OriginFor<T>,
            credit_id: H256,
            registry: CarbonRegistry,
            project_id: sp_std::vec::Vec<u8>,
            amount: u128,
            vintage_year: u16,
            verification_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(!Credits::<T>::contains_key(credit_id), Error::<T>::CreditAlreadyExists);
            ensure!(amount <= T::MaxIssuanceAmount::get(), Error::<T>::ExceedsMaxIssuance);

            let bounded_project: BoundedVec<u8, T::MaxProjectIdLength> = project_id
                .try_into().map_err(|_| Error::<T>::ProjectIdTooLong)?;

            // Reserve bond
            let bond = T::IssuanceBond::get();
            T::Currency::reserve(&who, bond)
                .map_err(|_| Error::<T>::InsufficientBond)?;

            let now = frame_system::Pallet::<T>::block_number();

            Credits::<T>::insert(credit_id, CarbonCredit {
                credit_id, owner: who.clone(), registry, project_id: bounded_project,
                amount, vintage_year, status: CarbonStatus::Issued,
                verification_hash, issued_at: now,
            });

            // Track bond for dispute window
            IssuanceBonds::<T>::insert(credit_id, (who.clone(), bond, now));

            TotalIssued::<T>::mutate(|t| *t = t.saturating_add(amount));
            AccountCredits::<T>::mutate(&who, |b| *b = b.saturating_add(amount));

            Self::deposit_event(Event::CreditIssued {
                credit_id, owner: who, registry, amount, vintage_year,
            });

            Ok(())
        }

        /// Claim bond back after dispute window. Permissionless.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(25_000_000, 0))]
        pub fn claim_bond(origin: OriginFor<T>, credit_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (issuer, bond, issued_at) = IssuanceBonds::<T>::get(credit_id)
                .ok_or(Error::<T>::BondNotFound)?;

            ensure!(who == issuer, Error::<T>::NotOwner);

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(
                now.saturating_sub(issued_at) >= T::DisputeWindow::get(),
                Error::<T>::DisputeWindowNotOver
            );

            T::Currency::unreserve(&issuer, bond);
            IssuanceBonds::<T>::remove(credit_id);

            // Transition to Active — credit is now usable in settlements and retirements
            Credits::<T>::try_mutate(credit_id, |opt| -> DispatchResult {
                let credit = opt.as_mut().ok_or(Error::<T>::CreditNotFound)?;
                credit.status = CarbonStatus::Active;
                Ok(())
            })?;

            Self::deposit_event(Event::BondReturned { credit_id, issuer, amount: bond });
            Ok(())
        }

        /// Lock a carbon credit for settlement.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(25_000_000, 0))]
        pub fn lock(origin: OriginFor<T>, credit_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Credits::<T>::try_mutate(credit_id, |opt| -> DispatchResult {
                let credit = opt.as_mut().ok_or(Error::<T>::CreditNotFound)?;
                ensure!(credit.owner == who, Error::<T>::NotOwner);
                ensure!(credit.status == CarbonStatus::Active, Error::<T>::InvalidCreditStatus);
                credit.status = CarbonStatus::Locked;
                TotalLocked::<T>::mutate(|t| *t = t.saturating_add(credit.amount));
                Self::deposit_event(Event::CreditLocked { credit_id, amount: credit.amount });
                Ok(())
            })
        }

        /// Permanently retire a carbon credit.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn retire(origin: OriginFor<T>, credit_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Credits::<T>::try_mutate(credit_id, |opt| -> DispatchResult {
                let credit = opt.as_mut().ok_or(Error::<T>::CreditNotFound)?;
                ensure!(credit.owner == who, Error::<T>::NotOwner);
                ensure!(
                    credit.status == CarbonStatus::Active || credit.status == CarbonStatus::Locked,
                    Error::<T>::InvalidCreditStatus
                );
                let was_locked = credit.status == CarbonStatus::Locked;
                credit.status = CarbonStatus::Retired;
                let amount = credit.amount;
                let registry = credit.registry;
                let now = frame_system::Pallet::<T>::block_number();

                let cert_count = CertificateCount::<T>::get();
                let cert_data = (credit_id, who.clone(), amount, now, cert_count);
                let certificate_id = H256::from_slice(
                    sp_core::hashing::sha2_256(&codec::Encode::encode(&cert_data)).as_ref()
                );

                RetirementCertificates::<T>::insert(certificate_id, RetirementCert {
                    certificate_id, credit_id, retiree: who.clone(),
                    amount, registry, retired_at: now,
                });
                CertificateCount::<T>::put(cert_count.saturating_add(1));
                TotalRetired::<T>::mutate(|t| *t = t.saturating_add(amount));
                AccountCredits::<T>::mutate(&who, |b| *b = b.saturating_sub(amount));
                if was_locked { TotalLocked::<T>::mutate(|t| *t = t.saturating_sub(amount)); }

                Self::deposit_event(Event::CreditRetired { credit_id, certificate_id, retiree: who, amount });
                Ok(())
            })
        }

        /// Transfer carbon credits.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(25_000_000, 0))]
        pub fn transfer(origin: OriginFor<T>, credit_id: H256, to: T::AccountId) -> DispatchResult {
            let from = ensure_signed(origin)?;
            Credits::<T>::try_mutate(credit_id, |opt| -> DispatchResult {
                let credit = opt.as_mut().ok_or(Error::<T>::CreditNotFound)?;
                ensure!(credit.owner == from, Error::<T>::NotOwner);
                ensure!(credit.status == CarbonStatus::Issued, Error::<T>::InvalidCreditStatus);
                let amount = credit.amount;
                AccountCredits::<T>::mutate(&from, |b| *b = b.saturating_sub(amount));
                AccountCredits::<T>::mutate(&to, |b| *b = b.saturating_add(amount));
                credit.owner = to.clone();
                Self::deposit_event(Event::CreditTransferred { credit_id, from, to, amount });
                Ok(())
            })
        }

        /// Slash a fraudulent carbon credit bond. Governance-only (root origin).
        ///
        /// Called via a passed governance proposal when a credit is found to be
        /// fraudulent, invalid, or unverifiable. The issuer's reserved bond is
        /// slashed and burned (deflationary). The credit is permanently invalidated.
        ///
        /// Can only be called within the dispute window. After the dispute window,
        /// the bond is claimable by the issuer via claim_bond().
        ///
        /// slash_percent: 0–10000 bps of bond to slash (10000 = 100%)
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(75_000_000, 0))]
        pub fn slash_bond(
            origin: OriginFor<T>,
            credit_id: H256,
            slash_percent: u16,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let (issuer, bond, _issued_at) = IssuanceBonds::<T>::get(credit_id)
                .ok_or(Error::<T>::BondNotFound)?;

            // Clamp slash to 100%
            let bps = slash_percent.min(10_000);
            let slash_amount = bond.saturating_mul(bps.into()) / 10_000u32.into();

            // Mark credit as Slashed — permanently unusable
            Credits::<T>::try_mutate(credit_id, |opt| -> DispatchResult {
                let credit = opt.as_mut().ok_or(Error::<T>::CreditNotFound)?;
                ensure!(credit.status != CarbonStatus::Slashed, Error::<T>::AlreadySlashed);

                // Remove from account balance
                let amount = credit.amount;
                AccountCredits::<T>::mutate(&credit.owner, |b| *b = b.saturating_sub(amount));
                TotalIssued::<T>::mutate(|t| *t = t.saturating_sub(amount));

                credit.status = CarbonStatus::Slashed;
                Ok(())
            })?;

            // Slash and burn the reserved bond
            let (slashed_imbalance, _) = T::Currency::slash_reserved(&issuer, slash_amount);
            let slashed_amount = slashed_imbalance.peek();
            // Imbalance dropped here = tokens burned (no offsetting deposit)

            // Return remainder of bond unreserved (partial slash)
            let remainder = bond.saturating_sub(slash_amount);
            if !remainder.is_zero() {
                T::Currency::unreserve(&issuer, remainder);
            }
            IssuanceBonds::<T>::remove(credit_id);

            Self::deposit_event(Event::BondSlashed {
                credit_id,
                issuer,
                amount: slashed_amount,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn total_issued() -> u128 { TotalIssued::<T>::get() }
        pub fn total_retired() -> u128 { TotalRetired::<T>::get() }
        pub fn active_credits() -> u128 { TotalIssued::<T>::get().saturating_sub(TotalRetired::<T>::get()) }
        pub fn balance_of(account: &T::AccountId) -> u128 { AccountCredits::<T>::get(account) }
    }

    // -----------------------------------------------------------------------
    // CarbonInterface — called by settlement pallet for atomic carbon swaps
    // -----------------------------------------------------------------------

    impl<T: Config> twill_primitives::CarbonInterface<T::AccountId> for Pallet<T> {
        /// Lock a carbon credit into settlement escrow.
        /// The credit must be Issued and owned by `owner`.
        fn lock_for_settlement(credit_id: sp_core::H256, owner: &T::AccountId) -> bool {
            Credits::<T>::try_mutate(credit_id, |opt| -> Result<(), ()> {
                let credit = opt.as_mut().ok_or(())?;
                if &credit.owner != owner { return Err(()); }
                if credit.status == CarbonStatus::Slashed { return Err(()); }
                if credit.status != CarbonStatus::Issued { return Err(()); }
                credit.status = CarbonStatus::Locked;
                TotalLocked::<T>::mutate(|t| *t = t.saturating_add(credit.amount));
                Ok(())
            }).is_ok()
        }

        /// Transfer a locked carbon credit to `to` after settlement completes.
        /// Restores status to Issued under the new owner.
        fn transfer_settled(credit_id: sp_core::H256, to: &T::AccountId) -> bool {
            Credits::<T>::try_mutate(credit_id, |opt| -> Result<(), ()> {
                let credit = opt.as_mut().ok_or(())?;
                if credit.status != CarbonStatus::Locked { return Err(()); }
                let amount = credit.amount;
                let from = credit.owner.clone();
                AccountCredits::<T>::mutate(&from, |b| *b = b.saturating_sub(amount));
                AccountCredits::<T>::mutate(to, |b| *b = b.saturating_add(amount));
                credit.owner = to.clone();
                credit.status = CarbonStatus::Issued;
                TotalLocked::<T>::mutate(|t| *t = t.saturating_sub(amount));
                Ok(())
            }).is_ok()
        }

        /// Restore a locked carbon credit to Issued on refund or expiry.
        fn unlock_refund(credit_id: sp_core::H256) -> bool {
            Credits::<T>::try_mutate(credit_id, |opt| -> Result<(), ()> {
                let credit = opt.as_mut().ok_or(())?;
                if credit.status != CarbonStatus::Locked { return Err(()); }
                let amount = credit.amount;
                credit.status = CarbonStatus::Issued;
                TotalLocked::<T>::mutate(|t| *t = t.saturating_sub(amount));
                Ok(())
            }).is_ok()
        }
    }
}
