#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use frame_support::sp_runtime::traits::{AccountIdConversion, Saturating, Zero, Hash};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_system::pallet_prelude::*;
	use frame_support::pallet_prelude::*;
	use frame_support::inherent::Vec;
    use frame_support::BoundedVec;
    use frame_support::traits::{ConstU32, Currency};
    use integer_sqrt::IntegerSquareRoot;

    pub type ProposalNo = u32;
    pub type RoundNo = u32;

    #[derive(Encode, Decode, Default, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct ProposalDetails<AccountId> { //struct for a proposal
        name: BoundedVec<u8, ConstU32<32>>,
        owner: AccountId,
        vote_value: u128
    }

    #[derive(Encode, Decode, Default, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
    pub struct VoteDetails<AccountId, Balance> { //struct for a vote
        account: AccountId,
        balance: Balance
    }

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Currency: Currency<Self::AccountId>;
        #[pallet::constant]
        type MaxProposals: Get<u32>;
        #[pallet::constant]
        type MaxVotes: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::type_value]
    pub fn DefaultNo() -> u32 { 1 }

    #[pallet::storage]
    #[pallet::getter(fn next_round_no)]
	//for going through rounds and proposals
    pub type NextRoundNo<T: Config> = StorageValue<_, RoundNo, ValueQuery, DefaultNo>;
    #[pallet::storage]
    #[pallet::getter(fn next_proposal_no)]
    pub type NextProposalNo<T: Config> = StorageValue<_, ProposalNo, ValueQuery, DefaultNo>;
	//admin for round control and vote counting
    #[pallet::storage]
    pub type Admin<T: Config> = StorageMap<_, Identity, <T as frame_system::Config>::AccountId, u32>;
    #[pallet::storage]
    pub type Round<T: Config> = StorageMap<_, Identity, RoundNo, bool>;
	#[pallet::storage]
    pub type Vote<T: Config> = StorageNMap<_,
	(NMapKey<Blake2_128Concat, RoundNo>,NMapKey<Blake2_128Concat, ProposalNo>),
	BoundedVec<VoteDetails<T::AccountId, BalanceOf<T>>, T::MaxVotes>,ValueQuery>;
	//proposals have owners and a collection of them
    #[pallet::storage]
    pub type Proposal<T: Config> = StorageMap<_, Identity, ProposalNo, ProposalDetails<T::AccountId>, OptionQuery>;
    #[pallet::storage]
    pub type ProposalOwner<T: Config> = StorageMap<_, Identity, <T as frame_system::Config>::AccountId, ProposalNo>;
    #[pallet::storage]
    pub type Proposals<T: Config> = StorageMap<_, Blake2_128Concat, u32, BoundedVec<u32, T::MaxProposals>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        NewVotingRound(RoundNo),
        NewProposalCreated(ProposalNo),
        ProposalRegistered(RoundNo, ProposalNo),
        LogErrNoProposalsForRound(),
        LogConversionFailed()
	}

	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
        InvalidRoundNo,
        InvalidProposalNo,
        DuplicateProposal,
        ProposalNotRegistered,
        ProposalsOverflow,
        VotesOverflow,
        NotAdmin,
        RoundInactive,
	}

	// Extrinsic Functions
	#[pallet::call]
	impl<T: Config> Pallet<T> {

        #[pallet::weight(10_000)] //start a round for proposal creation and voting
        pub fn start_round(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?; //whoever starts round will have to end
            let round_no = Self::new_round_no();

            <Admin<T>>::insert(&who, round_no);
            <Round<T>>::insert(round_no, true);
            Self::deposit_event(Event::NewVotingRound(round_no));
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn end_round(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?; //same who started
            let result = <Admin<T>>::try_get(&who);
            if result.is_err() {
                Err(Error::<T>::NotAdmin)?
            }
            let round_no = result.unwrap_or(0);
            if round_no == 0 {
                Err(Error::<T>::InvalidRoundNo)?
            }
			//remove all points of interaction with the round
            <Admin<T>>::remove(&who);
            <Round<T>>::remove(round_no);
            //count the votes
            Self::tally_votes(round_no);
            Ok(())
        }

		//if there is an active round, create a proposal (have to input round no)
        #[pallet::weight(10_000)]
        pub fn create_proposal(origin: OriginFor<T>, name: BoundedVec<u8, ConstU32<32>>, round_no: u32) -> DispatchResult {
			match <Round<T>>::try_get(round_no) {
                Ok(is_active) => {
                    if !is_active {
                        Err(Error::<T>::RoundInactive)?
                    }
                },
                Err(_) => Err(Error::<T>::InvalidRoundNo)?
            }
            let who = ensure_signed(origin)?;
            let proposal_no = Self::new_proposal_no();
            let proposal_details = ProposalDetails { owner: who, name, vote_value: 0_u128 };
			if Self::is_proposal_registered(proposal_no, round_no) {
				Err(<Error<T>>::DuplicateProposal)?
			}
            <Proposal<T>>::insert(proposal_no, proposal_details);

			<Proposals<T>>::try_mutate(0, |proposals_vec| {
                proposals_vec.try_push(proposal_no)
            }).map_err(|_| <Error<T>>::ProposalsOverflow)?;
            Self::deposit_event(Event::ProposalRegistered(proposal_no,round_no));
            Ok(())
        }


        #[pallet::weight(10_000)] //make a vote
        pub fn vote(origin: OriginFor<T>, round_no: u32, proposal_no: u32, balance: BalanceOf<T>) -> DispatchResult {
            let who = ensure_signed(origin)?; //verify voter
            if !<Round<T>>::contains_key(round_no) { //checking if round is correct
                Err(Error::<T>::InvalidRoundNo)?
            } else if <Round<T>>::get(round_no).unwrap_or(false) == false {
                Err(Error::<T>::InvalidRoundNo)?
            }
            //checking if the proposal exists
            if !Self::is_proposal_registered(proposal_no, round_no) {
                Err(<Error<T>>::ProposalNotRegistered)?
            }
            let details = VoteDetails { account: who, balance };
            <Vote<T>>::try_mutate(Self::create_vote_key(round_no, proposal_no), |votes_vec| {
                votes_vec.try_push(details)
            }).map_err(|_| <Error<T>>::VotesOverflow)?;
            Ok(())
        }

	}


    //Private Functions
    impl<T: Config> Pallet<T> {
        fn new_round_no() -> RoundNo {
            let round_no = Self::next_round_no();
            let next_round_no: RoundNo = round_no.wrapping_add(1);
            NextRoundNo::<T>::put(next_round_no);
            round_no
        }

        fn new_proposal_no() -> ProposalNo {
            let proposal_no = Self::next_proposal_no();
            let next_proposal_no: ProposalNo = proposal_no.wrapping_add(1);
            NextProposalNo::<T>::put(next_proposal_no);
            proposal_no
        }

        fn create_vote_key(round_no: RoundNo, proposal_no: ProposalNo) -> (RoundNo, ProposalNo) {
            (round_no, proposal_no)
        }

        fn is_proposal_registered(proposal_no: u32, round_no: u32) -> bool {
            match <Proposals<T>>::try_get(round_no) {
                Ok(proposals_vec) => {
                    if proposals_vec.contains(&proposal_no) {
                        return true;
                    }
                },
                Err(_) => {}
            }
            return false;
        }

        fn get_votes_for_proposal(proposal_no: ProposalNo, round_no: RoundNo) -> Vec<VoteDetails<T::AccountId, BalanceOf<T>>> {
            let mut votes: Vec<VoteDetails<T::AccountId, BalanceOf<T>>> = Vec::new();
            match <Vote<T>>::try_get(Self::create_vote_key(round_no, proposal_no)) {
                Ok(votes_vec) => { votes = votes_vec.into_inner() },
                Err(_) => {}
            }
            votes
        }

        fn calculate_vote_value(proposal_no: u32, round_no: u32) -> u128 {
            match <Proposal<T>>::try_get(proposal_no)  {
                Ok(mut details) => {
                    if details.vote_value == 0_u128 {
                        let mut vote_value = 0_u128;
                        for vote in Self::get_votes_for_proposal(proposal_no, round_no) {
                            let balance128 = Self::balance_as_128(vote.balance);
                            vote_value += balance128.integer_sqrt();
                        }
                        vote_value = u128::pow(vote_value, 2);
                        details.vote_value = vote_value;
                        <Proposal<T>>::insert(proposal_no, details);
                        return vote_value;
                    } else {
                        details.vote_value
                    }
                },
                Err(_) => { 0_u128 }
            }
        }

        // distribute funds into project accounts
        fn tally_votes(round_no: RoundNo) {
            // get list of projects for this round_id
            let proposals: Vec<u32>;
            match <Proposals<T>>::try_get(round_no) {
                Ok(proposals_vec) => { proposals = proposals_vec.into_inner() },
                Err(_) => {
                    Self::deposit_event(Event::LogErrNoProposalsForRound());
                    return;
                }
            }
            //tally votes for each project
			let mut vote_tally: Vec<u128> = vec![];
            let mut total_vote_value = 0_u128;
            for proposal_no in &proposals {
                total_vote_value += Self::calculate_vote_value(*proposal_no, round_no);
				vote_tally.push(total_vote_value.clone());
            }
            //return vote_tally
        }

        fn balance_as_128(balance: BalanceOf<T>) -> u128 {
            let b128_option: Option<u128> = balance.try_into().ok();
            match b128_option {
                Some(b128) => { b128 },
                None => { 0 }
            }
        }
    }
}