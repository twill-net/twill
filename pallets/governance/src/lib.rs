//! # Governance Pallet
//!
//! Two-layer governance for the Twill Network:
//!
//! **Board** (5-7 members): Elected by TWL holders every 5 years.
//! Handles code maintenance and operational decisions.
//!
//! **Community** (all TWL holders): Votes on major decisions.
//! Runtime upgrades, parameter changes, reserve actions, board elections.
//! 1 TWL = 1 vote. Board cannot unilaterally change the protocol.
//!
//! Board proposes. Community approves.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency, Get},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Saturating, Zero};
    use twill_primitives::MiningInterface;

    type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Currency: ReservableCurrency<Self::AccountId>;
        /// Mining pallet interface — used to enact SetMiningTreasuryShare proposals.
        type MiningProvider: twill_primitives::MiningInterface<Self::AccountId>;

        /// Maximum board members (5-7)
        #[pallet::constant]
        type MaxBoardMembers: Get<u32>;

        /// Board term in blocks (~5 years = 26,280,000 blocks)
        #[pallet::constant]
        type BoardTermBlocks: Get<BlockNumberFor<Self>>;

        /// Voting period in blocks (~7 days = 50,400 blocks)
        #[pallet::constant]
        type VotingPeriodBlocks: Get<BlockNumberFor<Self>>;

        /// Delay after approval before enactment (~7 days)
        #[pallet::constant]
        type EnactmentDelayBlocks: Get<BlockNumberFor<Self>>;

        /// Deposit required to nominate for board (2nd election onwards).
        /// Genesis election has no deposit — TWL may not be circulating yet.
        #[pallet::constant]
        type NominationDeposit: Get<BalanceOf<Self>>;

        /// Maximum number of nominees in an election
        #[pallet::constant]
        type MaxNominees: Get<u32>;

        /// Maximum active proposals at once
        #[pallet::constant]
        type MaxActiveProposals: Get<u32>;

        /// Treasury account — board pay is transferred from here each block.
        /// No payment if treasury balance is insufficient (skips silently, no debt).
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    #[derive(Clone, PartialEqNoBound, EqNoBound, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub enum ProposalKind<T: Config> {
        /// Runtime upgrade — new WASM blob (submitted as hash, code stored off-chain)
        RuntimeUpgrade { code_hash: sp_core::H256 },
        /// Board recall — emergency removal of a board member
        BoardRecall { member: T::AccountId },
        /// Text proposal — non-binding resolution (for signaling)
        TextProposal,
        /// Set the share of block rewards redirected to the treasury (in BPS, max 1000 = 10%).
        /// Default at genesis is 0 — miners keep 100%. Community votes to activate.
        SetMiningTreasuryShare { bps: u16 },
        /// Set board pay per block, paid from the treasury equally to all seated members.
        /// Default at genesis is 0. If treasury has insufficient funds, payment skips silently.
        SetBoardPay { amount_per_block: BalanceOf<T> },
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ProposalStatus {
        Voting,
        Approved,
        Rejected,
        Enacted,
        Expired,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum VoteDirection {
        Aye,
        Nay,
        Abstain,
    }

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Proposal<T: Config> {
        pub id: u32,
        pub proposer: T::AccountId,
        pub kind: ProposalKind<T>,
        pub status: ProposalStatus,
        pub voting_ends: BlockNumberFor<T>,
        pub enactment_block: BlockNumberFor<T>,
    }

    #[derive(Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Tally<T: Config> {
        pub aye: BalanceOf<T>,
        pub nay: BalanceOf<T>,
        pub abstain: BalanceOf<T>,
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Current board members
    #[pallet::storage]
    pub type BoardMembers<T: Config> =
        StorageValue<_, BoundedVec<T::AccountId, T::MaxBoardMembers>, ValueQuery>;

    /// Block when current board term started
    #[pallet::storage]
    pub type BoardTermStart<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    /// Whether a board has been seated (false at genesis until first election)
    #[pallet::storage]
    pub type BoardSeated<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// How many board elections have completed. 0 = genesis election not yet held.
    /// Genesis election (ElectionCount == 0): no deposit, 1-person-1-vote.
    /// All subsequent elections: normal TWL-weighted rules with NominationDeposit.
    #[pallet::storage]
    pub type ElectionCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// TWL paid per block from the treasury to each seated board member equally.
    /// Default: 0 (unpaid at genesis). Community votes to set via SetBoardPay proposal.
    /// Payment skips silently if treasury is insufficient — no debt, no halt.
    #[pallet::storage]
    pub type BoardPayPerBlock<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    /// Active proposals
    #[pallet::storage]
    pub type Proposals<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, Proposal<T>>;

    /// Proposal counter
    #[pallet::storage]
    pub type ProposalCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// IDs of proposals currently in Voting status (bounded — avoids unbounded iteration)
    #[pallet::storage]
    pub type ActiveProposalIds<T: Config> =
        StorageValue<_, BoundedVec<u32, T::MaxActiveProposals>, ValueQuery>;

    /// Votes on proposals
    #[pallet::storage]
    pub type Votes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, VoteDirection>;

    /// Vote tallies
    #[pallet::storage]
    pub type Tallies<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, Tally<T>>;

    /// Board election: nominees and their deposits
    #[pallet::storage]
    pub type Nominees<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>>;

    /// Board election: votes for nominees (nominee → voter → weight)
    #[pallet::storage]
    pub type ElectionVotes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, T::AccountId, BalanceOf<T>>;

    /// Whether an election is currently active
    #[pallet::storage]
    pub type ElectionActive<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Block when current election started
    #[pallet::storage]
    pub type ElectionStartBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Proposal submitted for community vote
        ProposalSubmitted { proposal_id: u32, proposer: T::AccountId },
        /// Vote cast on a proposal
        VoteCast { proposal_id: u32, voter: T::AccountId, direction: VoteDirection, weight: BalanceOf<T> },
        /// Proposal approved by community
        ProposalApproved { proposal_id: u32, aye: BalanceOf<T>, nay: BalanceOf<T> },
        /// Proposal rejected by community
        ProposalRejected { proposal_id: u32, aye: BalanceOf<T>, nay: BalanceOf<T> },
        /// Proposal expired (quorum not met)
        ProposalExpired { proposal_id: u32 },
        /// Proposal enacted
        ProposalEnacted { proposal_id: u32, block_number: BlockNumberFor<T> },
        /// Board member recalled
        BoardMemberRecalled { member: T::AccountId },
        /// Board election started
        ElectionStarted { block_number: BlockNumberFor<T> },
        /// Nominee registered for board election
        NomineeRegistered { nominee: T::AccountId, deposit: BalanceOf<T> },
        /// Board election completed — new board seated
        BoardElected { members: sp_std::vec::Vec<T::AccountId>, term_start: BlockNumberFor<T> },
        /// Board pay distributed this block (amount per member, number of members)
        BoardPayDistributed { per_member: BalanceOf<T>, member_count: u32 },
        /// Board pay skipped — treasury insufficient
        BoardPaySkipped,
    }

    #[pallet::error]
    pub enum Error<T> {
        ProposalNotFound,
        VotingPeriodEnded,
        VotingPeriodActive,
        AlreadyVoted,
        InsufficientDeposit,
        MaxProposalsReached,
        NotBoardMember,
        AlreadyNominated,
        ElectionNotActive,
        ElectionAlreadyActive,
        NoNominees,
        NotANominee,
    }

    // -----------------------------------------------------------------------
    // Hooks
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            // Check if board term expired → trigger election
            if BoardSeated::<T>::get() && !ElectionActive::<T>::get() {
                let term_start = BoardTermStart::<T>::get();
                let term_length = T::BoardTermBlocks::get();
                if now.saturating_sub(term_start) >= term_length {
                    ElectionActive::<T>::put(true);
                    ElectionStartBlock::<T>::put(now);
                    Self::deposit_event(Event::ElectionStarted { block_number: now });
                }
            }

            // Check if election voting period ended → seat winners
            if ElectionActive::<T>::get() {
                let election_start = ElectionStartBlock::<T>::get();
                let voting_period = T::VotingPeriodBlocks::get();
                if now.saturating_sub(election_start) >= voting_period {
                    Self::finalize_election(now);
                }
            }

            // Distribute board pay from treasury (if set and board is seated)
            Self::distribute_board_pay();

            // Check proposal voting periods
            Self::process_proposals(now);

            Weight::from_parts(10_000_000, 0)
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a proposal for community vote. No deposit required.
        /// The 10% quorum requirement is the spam filter — proposals without
        /// genuine community interest simply expire.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(50_000_000, 0))]
        pub fn submit_proposal(
            origin: OriginFor<T>,
            kind: ProposalKind<T>,
        ) -> DispatchResult {
            let proposer = ensure_signed(origin)?;

            let now = frame_system::Pallet::<T>::block_number();
            let id = ProposalCount::<T>::get();
            ProposalCount::<T>::put(id + 1);

            let voting_ends = now.saturating_add(T::VotingPeriodBlocks::get());
            let enactment_block = voting_ends.saturating_add(T::EnactmentDelayBlocks::get());

            Proposals::<T>::insert(id, Proposal {
                id,
                proposer: proposer.clone(),
                kind,
                status: ProposalStatus::Voting,
                voting_ends,
                enactment_block,
            });

            Tallies::<T>::insert(id, Tally {
                aye: BalanceOf::<T>::zero(),
                nay: BalanceOf::<T>::zero(),
                abstain: BalanceOf::<T>::zero(),
            });

            // Track in active set for bounded on_initialize iteration
            ActiveProposalIds::<T>::mutate(|ids| {
                let _ = ids.try_push(id);
            });

            Self::deposit_event(Event::ProposalSubmitted { proposal_id: id, proposer });
            Ok(())
        }

        /// Vote on an active proposal. 1 TWL = 1 vote (free + locked balance).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(30_000_000, 0))]
        pub fn vote(
            origin: OriginFor<T>,
            proposal_id: u32,
            direction: VoteDirection,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;

            let proposal = Proposals::<T>::get(proposal_id)
                .ok_or(Error::<T>::ProposalNotFound)?;
            ensure!(proposal.status == ProposalStatus::Voting, Error::<T>::VotingPeriodEnded);

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now < proposal.voting_ends, Error::<T>::VotingPeriodEnded);

            ensure!(!Votes::<T>::contains_key(proposal_id, &voter), Error::<T>::AlreadyVoted);

            // Vote weight = total balance (free + reserved)
            let weight = T::Currency::total_balance(&voter);

            Votes::<T>::insert(proposal_id, &voter, direction);

            Tallies::<T>::mutate(proposal_id, |tally_opt| {
                if let Some(ref mut tally) = tally_opt {
                    match direction {
                        VoteDirection::Aye => tally.aye = tally.aye.saturating_add(weight),
                        VoteDirection::Nay => tally.nay = tally.nay.saturating_add(weight),
                        VoteDirection::Abstain => tally.abstain = tally.abstain.saturating_add(weight),
                    }
                }
            });

            Self::deposit_event(Event::VoteCast {
                proposal_id, voter, direction, weight,
            });
            Ok(())
        }

        /// Nominate yourself for the board election.
        /// Genesis election (first ever): no deposit required — TWL may not yet be circulating.
        /// All subsequent elections require NominationDeposit (returned after election).
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(30_000_000, 0))]
        pub fn nominate_for_board(origin: OriginFor<T>) -> DispatchResult {
            let nominee = ensure_signed(origin)?;
            ensure!(ElectionActive::<T>::get(), Error::<T>::ElectionNotActive);
            ensure!(!Nominees::<T>::contains_key(&nominee), Error::<T>::AlreadyNominated);

            let is_genesis_election = ElectionCount::<T>::get() == 0;
            let deposit = if is_genesis_election {
                BalanceOf::<T>::zero()
            } else {
                let d = T::NominationDeposit::get();
                T::Currency::reserve(&nominee, d)
                    .map_err(|_| Error::<T>::InsufficientDeposit)?;
                d
            };

            Nominees::<T>::insert(&nominee, deposit);
            Self::deposit_event(Event::NomineeRegistered { nominee, deposit });
            Ok(())
        }

        /// Vote for a board nominee during an active election.
        /// Weight = your total TWL balance. You can vote for multiple nominees.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(30_000_000, 0))]
        pub fn vote_board_election(
            origin: OriginFor<T>,
            nominee: T::AccountId,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            ensure!(ElectionActive::<T>::get(), Error::<T>::ElectionNotActive);
            ensure!(Nominees::<T>::contains_key(&nominee), Error::<T>::NotANominee);

            // Genesis election: 1 address = 1 vote (TWL may not be circulating yet).
            // All subsequent elections: 1 TWL = 1 vote (balance-weighted).
            let weight = if ElectionCount::<T>::get() == 0 {
                BalanceOf::<T>::from(1u32)
            } else {
                T::Currency::total_balance(&voter)
            };
            ElectionVotes::<T>::insert(&nominee, &voter, weight);

            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Distribute board pay from the treasury to each seated member equally.
        /// Skips silently if: pay is 0, no board is seated, or treasury is insufficient.
        /// Never mints new TWL — treasury-only source.
        fn distribute_board_pay() {
            let pay_per_block = BoardPayPerBlock::<T>::get();
            if pay_per_block.is_zero() { return; }
            if !BoardSeated::<T>::get() { return; }

            let members = BoardMembers::<T>::get();
            let count = members.len() as u32;
            if count == 0 { return; }

            // Integer division — dust stays in treasury
            let per_member = pay_per_block / count.into();
            if per_member.is_zero() { return; }

            let treasury = T::TreasuryAccount::get();
            for member in members.iter() {
                if T::Currency::transfer(
                    &treasury,
                    member,
                    per_member,
                    frame_support::traits::ExistenceRequirement::KeepAlive,
                ).is_err() {
                    // Treasury insufficient — skip remaining payments this block
                    Self::deposit_event(Event::BoardPaySkipped);
                    return;
                }
            }
            Self::deposit_event(Event::BoardPayDistributed { per_member, member_count: count });
        }

        /// Process proposals whose voting periods have ended.
        /// Only iterates ActiveProposalIds — bounded, O(active) not O(all-time).
        fn process_proposals(now: BlockNumberFor<T>) {
            let active_ids = ActiveProposalIds::<T>::get();
            let mut to_deactivate: sp_std::vec::Vec<u32> = sp_std::vec::Vec::new();

            for &id in active_ids.iter() {
                let Some(mut proposal) = Proposals::<T>::get(id) else { continue };
                if proposal.status != ProposalStatus::Voting { continue; }
                if now < proposal.voting_ends { continue; }

                // Voting period ended — tally
                let Some(tally) = Tallies::<T>::get(id) else { continue };

                let total_participating = tally.aye
                    .saturating_add(tally.nay)
                    .saturating_add(tally.abstain);

                // Quorum: 10% of total issuance must participate
                let total_issuance = T::Currency::total_issuance();
                let quorum = total_issuance / 10u32.into();

                if total_participating < quorum {
                    // Quorum not met — proposal expires silently. No penalty.
                    proposal.status = ProposalStatus::Expired;
                    Self::deposit_event(Event::ProposalExpired { proposal_id: id });
                } else if tally.aye > tally.nay {
                    proposal.status = ProposalStatus::Approved;
                    Self::deposit_event(Event::ProposalApproved {
                        proposal_id: id, aye: tally.aye, nay: tally.nay,
                    });

                    // Board recall: take effect immediately, no enactment delay
                    if let ProposalKind::BoardRecall { ref member } = proposal.kind {
                        BoardMembers::<T>::mutate(|members| {
                            members.retain(|m| m != member);
                        });
                        Self::deposit_event(Event::BoardMemberRecalled {
                            member: member.clone(),
                        });
                        proposal.status = ProposalStatus::Enacted;
                    }

                    // Mining treasury share: take effect immediately on approval
                    if let ProposalKind::SetMiningTreasuryShare { bps } = proposal.kind {
                        T::MiningProvider::set_treasury_mining_share(bps);
                        proposal.status = ProposalStatus::Enacted;
                    }

                    // Board pay: take effect immediately on approval, capped at protocol maximum
                    if let ProposalKind::SetBoardPay { amount_per_block } = proposal.kind {
                        let max: BalanceOf<T> = twill_primitives::MAX_BOARD_PAY_PER_BLOCK
                            .try_into().unwrap_or_else(|_| BalanceOf::<T>::zero());
                        let capped = amount_per_block.min(max);
                        BoardPayPerBlock::<T>::put(capped);
                        proposal.status = ProposalStatus::Enacted;
                    }
                } else {
                    proposal.status = ProposalStatus::Rejected;
                    Self::deposit_event(Event::ProposalRejected {
                        proposal_id: id, aye: tally.aye, nay: tally.nay,
                    });
                }

                Proposals::<T>::insert(id, proposal);
                to_deactivate.push(id);
            }

            // Remove finalized proposals from active set
            if !to_deactivate.is_empty() {
                ActiveProposalIds::<T>::mutate(|ids| {
                    ids.retain(|id| !to_deactivate.contains(id));
                });
            }
        }

        /// Finalize board election — seat top N nominees by vote weight.
        fn finalize_election(now: BlockNumberFor<T>) {
            // Collect nominee vote totals
            let mut nominee_scores: sp_std::vec::Vec<(T::AccountId, BalanceOf<T>)> = sp_std::vec::Vec::new();

            // Iterate nominees
            for (nominee, deposit) in Nominees::<T>::iter() {
                // Sum all votes for this nominee
                let mut total_votes = BalanceOf::<T>::zero();
                for (_, weight) in ElectionVotes::<T>::iter_prefix(&nominee) {
                    total_votes = total_votes.saturating_add(weight);
                }
                // Return nomination deposit regardless of outcome
                T::Currency::unreserve(&nominee, deposit);
                nominee_scores.push((nominee, total_votes));
            }

            // If no one ran, keep existing board and cancel election silently
            if nominee_scores.is_empty() {
                ElectionActive::<T>::put(false);
                let _ = Nominees::<T>::clear(u32::MAX, None);
                let _ = ElectionVotes::<T>::clear(u32::MAX, None);
                return;
            }

            // Sort by votes (descending)
            nominee_scores.sort_by(|a, b| b.1.cmp(&a.1));

            // Take top MaxBoardMembers
            let max_members = T::MaxBoardMembers::get() as usize;
            let winners: sp_std::vec::Vec<T::AccountId> = nominee_scores
                .into_iter()
                .take(max_members)
                .map(|(account, _)| account)
                .collect();

            // Seat the board
            let bounded: BoundedVec<T::AccountId, T::MaxBoardMembers> =
                winners.clone().try_into().unwrap_or_default();
            BoardMembers::<T>::put(bounded);
            BoardTermStart::<T>::put(now);
            BoardSeated::<T>::put(true);
            ElectionCount::<T>::mutate(|c| *c = c.saturating_add(1));

            // Clean up election state
            ElectionActive::<T>::put(false);
            let _ = Nominees::<T>::clear(u32::MAX, None);
            let _ = ElectionVotes::<T>::clear(u32::MAX, None);

            Self::deposit_event(Event::BoardElected {
                members: winners,
                term_start: now,
            });
        }

        /// Check if an account is a board member
        pub fn is_board_member(who: &T::AccountId) -> bool {
            BoardMembers::<T>::get().contains(who)
        }

        /// Get the current board
        pub fn board() -> sp_std::vec::Vec<T::AccountId> {
            BoardMembers::<T>::get().into_inner()
        }
    }
}
