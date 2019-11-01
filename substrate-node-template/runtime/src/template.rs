/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use crate::token;
use support::{ensure, decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result};
use system::ensure_signed;
use codec::{Encode, Decode};
use rstd::prelude::*;

/// The module's configuration trait.
pub trait Trait: system::Trait + token::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, BlockNumber, TokenBalance> {
	proposer: AccountId,
	applicant: AccountId,
	shares_requested: TokenBalance,
	starting_period: BlockNumber,
	yes_votes: u32,
	no_votes: u32,
	processed: bool,
	did_pass: bool,
	aborted: bool,
}

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Member {
	exists: bool,
	highest_index_yes_vote: u32,
}

type ProposalIndex = u32;

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as Template {
        StartingPeriod get(voting_starting_period) config(): T::BlockNumber;

		VotingPeriod get(voting_period_length) config(): T::BlockNumber; 

		MinimumDeposit get(minimum_deposit) config(): T::TokenBalance;

		ProcessingReward get(processing_reward) config(): T::TokenBalance;

		Owner get(owner) config(): T::AccountId;

		TotalShares get(total_shares): T::TokenBalance;

		TotalSharesRequested get(total_requested_shares): T::TokenBalance;
		
		Members get(member): map T::AccountId => Member;

		ProposalDeposit get(proposit_deposit): map (ProposalIndex, T::AccountId) => T::TokenBalance;

		MemberVoting: map (T::AccountId, ProposalIndex) => Option<u8>;
		
		TotalProposalSubmitted get(total_submitted_proposals): u32 = 0;

		Proposals get(proposal): map u32 => Proposal<T::AccountId, T::BlockNumber, T::TokenBalance>;
	}
}

// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event() = default;

		fn init(origin, init_value: T::TokenBalance) {
			let sender = ensure_signed(origin)?;
			ensure!(sender == Self::owner(), "Only the owner in genesis config can initialize the Token");
			<token::Module<T>>::init(sender.clone(), init_value)?;
			let member = Member {
				exists: true,
				highest_index_yes_vote: 0,
			};
			<TotalShares<T>>::mutate(|n| *n += init_value);
			<Members<T>>::insert(sender, member);
		}

		pub fn submit_proposal(origin, applicant: T::AccountId, shares_requested: T::TokenBalance) -> Result {
			let sender = ensure_signed(origin)?;
			let num_of_proposals_submitted = TotalProposalSubmitted::get();

			ensure!(<token::Module<T>>::balance_of(sender.clone()) >= Self::minimum_deposit(), "Proposal Deposit cannot be smaller than _processingReward");

			let starting_period;
			if Self::total_submitted_proposals() == 0 {
				starting_period = <system::Module<T>>::block_number() + Self::voting_starting_period();
			} else {
				let num_of_proposals = Self::total_submitted_proposals();
				let proposal = Self::proposal(num_of_proposals);

				// TODO: Refactor
				if <system::Module<T>>::block_number() > proposal.starting_period {
					starting_period = <system::Module<T>>::block_number() + Self::voting_starting_period();
				} else {
					starting_period = proposal.starting_period + Self::voting_starting_period();
				}
			}
			
			let new_proposal = Proposal {
				proposer: sender.clone(),
				applicant: applicant.clone(),
				shares_requested: shares_requested.clone(),
				starting_period: starting_period,
				yes_votes: 0,
				no_votes: 0,
				processed: false,
				did_pass: false,
				aborted: false,
			};

			<token::Module<T>>::lock(sender.clone(), Self::minimum_deposit())?;
			<ProposalDeposit<T>>::insert((num_of_proposals_submitted, sender.clone()), Self::minimum_deposit());
			<Proposals<T>>::insert(num_of_proposals_submitted, new_proposal);
			TotalProposalSubmitted::mutate(|n| *n += 1);
			<TotalSharesRequested<T>>::mutate(|n| *n += shares_requested.clone());
			Self::deposit_event(RawEvent::SubmitProposal(num_of_proposals_submitted, sender, applicant, shares_requested, starting_period));

			Ok(())
		}

		pub fn submit_vote(origin, proposal_index: u32, unit_vote: u8) -> Result {
			let sender = ensure_signed(origin)?;
			let mut proposal = Self::proposal(proposal_index);
			let voting_expired_period = proposal.starting_period + Self::voting_period_length();
			let mut member = <Members<T>>::get(sender.clone());
			let vote = <MemberVoting<T>>::get((sender.clone(), proposal_index));

			ensure!(<Proposals<T>>::exists(proposal_index), "This proposal does not exist");
			ensure!(unit_vote == 0 || unit_vote == 1, "Vote must be either 0(Yes) or 1(No)");
			ensure!(<system::Module<T>>::block_number() > proposal.starting_period, "Voting period has not started");
			ensure!(voting_expired_period > <system::Module<T>>::block_number(), "Proposal voting period has expired");
			ensure!(vote.is_none(), "Member has already voted on this proposal");
			ensure!(!proposal.aborted, "Proposal has been aborted");

			// TODO: Member Checking

			if unit_vote == 0 {
				proposal.yes_votes += 1;
				if proposal_index >= member.highest_index_yes_vote {
					member.highest_index_yes_vote = proposal_index;
				}
			} else {
				proposal.no_votes += 1;
			};
			
			<Proposals<T>>::insert(proposal_index, proposal);
			<MemberVoting<T>>::insert((sender.clone(), proposal_index), unit_vote);

			Self::deposit_event(RawEvent::SubmitVote(proposal_index, sender, unit_vote));

			Ok(())
		}

		pub fn process_proposal(origin, proposal_index: u32) -> Result {
			let sender = ensure_signed(origin)?;
			let mut proposal = Self::proposal(proposal_index);
			let voting_expired_period = proposal.starting_period + Self::voting_period_length();
			
			ensure!(<Proposals<T>>::exists(proposal_index), "This proposal does not exist");
			ensure!(<system::Module<T>>::block_number() > voting_expired_period , "Proposal is not ready to be processed");
			ensure!(proposal.processed == false, "Proposal has already been processed");
			ensure!(proposal_index == 0 || Self::proposal(proposal_index-1).processed, "Pevious proposal must be processed");

			// TODO: Member Checking
			
        	let did_pass: bool = proposal.yes_votes > proposal.no_votes;
			let is_exist = <Members<T>>::exists(proposal.applicant.clone());
						
			// TODO: Refactor
			if did_pass && !proposal.aborted {
				if !is_exist {
					let new_member = Member {
						exists: true,
						highest_index_yes_vote: proposal_index,
					};
					<Members<T>>::insert(proposal.applicant.clone(), new_member);
				}
			
				<token::Module<T>>::mint(proposal.applicant.clone(), proposal.shares_requested.clone())?;
				proposal.did_pass = true;
				<TotalShares<T>>::mutate(|n| *n += proposal.shares_requested.clone());
			} else {
				proposal.did_pass = false;
			}
			proposal.processed = true;

			<token::Module<T>>::unlock(proposal.proposer.clone(), Self::minimum_deposit() - Self::processing_reward())?;
			<token::Module<T>>::balance_transfer(proposal.proposer.clone(), sender.clone(), Self::processing_reward())?;
			<Proposals<T>>::insert(proposal_index, &proposal);

			Self::deposit_event(RawEvent::ProcessProposal(proposal_index, proposal.applicant, proposal.proposer,
            proposal.shares_requested, proposal.did_pass));

			Ok(())
		}
		// TODO: rage_quit & abort implementation

	}
}

decl_event!(
	pub enum Event<T> where 
		AccountId = <T as system::Trait>::AccountId,
		BlockNumber = <T as system::Trait>::BlockNumber,
		TokenBalance = <T as token::Trait>::TokenBalance
	{
		SomethingStored(u32, AccountId),
		SubmitProposal(u32, AccountId, AccountId, TokenBalance, BlockNumber),
		SubmitVote(u32, AccountId, u8),
		ProcessProposal(u32, AccountId, AccountId, TokenBalance, bool),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use primitives::H256;
	use support::{impl_outer_origin, assert_ok, parameter_types};
	use sr_primitives::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, weights::Weight, Perbill,
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}
	impl Trait for Test {
		type Event = ();
	}
	type TemplateModule = Module<Test>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> runtime_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		new_test_ext().execute_with(|| {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
