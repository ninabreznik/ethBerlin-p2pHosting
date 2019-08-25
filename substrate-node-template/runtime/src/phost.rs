/// A runtime module template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references


/// For more guidance on Substrate modules, see the example module
/// https://github.com/paritytech/substrate/blob/master/srml/example/src/lib.rs

use support::{decl_module, decl_storage, decl_event, StorageValue, StorageMap, dispatch::Result};
use core::convert::TryInto;
use system::ensure_signed;
use codec::{Encode, Decode};
use rstd::vec::Vec;
use primitives::{ed25519, Hasher, Blake2Hasher};

type Public = ed25519::Public;
type Signature = ed25519::Signature;

/// The module's configuration trait.
pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[derive(Decode, PartialEq, Eq, Encode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Node {
	index: u64,
	hash: Vec<u8>,
	size: u64
}

#[derive(Decode, PartialEq, Eq, Encode, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proof {
	index: u64,
	nodes: Vec<Node>,
	signature: Option<Signature>
}

type DatIdIndex = u64;
type UserIdIndex = u64;

decl_event!(
	pub enum Event<T> 
	where
	AccountId = <T as system::Trait>::AccountId,
	BlockNumber = <T as system::Trait>::BlockNumber 
	{
		SomethingStored(DatIdIndex, Public),
		Challenge(
			AccountId,
			BlockNumber
		),
		NewPin(AccountId),
	}
);

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as TemplateModule {
		// Each dat archive gets an id
		DatId get(next_id): DatIdIndex;
		// Each dat archive has a public key
		DatKey get(public_key): map u64 => Public;
		// Each dat archive has a tree size
		TreeSize get(tree_size): map u64 => u64;
		// each dat archive has a merkle root
		MerkleRoot get(merkle_root): map u64 => Signature;
		// users are put into an "array"
		UsersCount: u64;
		Users get(user): map UserIdIndex => T::AccountId;
		// each user has a vec of data items they manage
		UsersStorage: map T::AccountId => Vec<Public>;

		// current check condition
		SelectedUser: T::AccountId;
		SelectedDat: Public;
		TimeLimit get(time_limit): T::BlockNumber;
		Nonce: u64;
	}
}


// The module's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event<T>() = default;

		fn on_initialize(n: T::BlockNumber) {
			// if no one is currently selected to give proof, select someone
			if !<SelectedUser<T>>::exists() && <UsersCount>::get() > 0 {
				let nonce = <Nonce>::get();
				let new_random = (<system::Module<T>>::random_seed(), nonce)
					.using_encoded(|b| Blake2Hasher::hash(b))
					.using_encoded(|mut b| u64::decode(&mut b))
					.expect("Hash must be bigger than 8 bytes; Qed");
				let new_time_limit = new_random % <DatId>::get();
				let future_block = 
					n + T::BlockNumber::from(new_time_limit.try_into().unwrap_or(2));
				let random_user_index = new_random % <UsersCount>::get();
				let random_user = <Users<T>>::get(random_user_index);
				let users_dats = <UsersStorage<T>>::get(random_user.clone());
				let users_dats_len = users_dats.len();
				let random_dat = users_dats.get(new_random as usize % users_dats_len)
					.expect("Users must not have empty storage; Qed");
				<SelectedUser<T>>::put(&random_user);
				<SelectedDat>::put(random_dat);
				<TimeLimit<T>>::put(future_block);
				<Nonce>::mutate(|m| *m += 1);
				Self::deposit_event(RawEvent::Challenge(random_user, future_block));
			}
		}

		fn submit_proof(origin, proof: Proof) {
			// if proof okay
				//reward and unselect user
				<SelectedUser<T>>::kill();
			// else let the user try again until time limit
		}

		// Submit a new piece of data that you want to have users copy
		fn register_data(origin, merkle_root: T::Hash, tree_size: u64) {
			
		}

		// owner of data updates blockchain with new merkle root and tree size
		fn update_data(origin, merkle_root: T::Hash, tree_size: u64) {

		}

		// User requests a dat for them to pin
		fn register_backup(origin) {
			let account = ensure_signed(origin)?;
			let nonce = <Nonce>::get();
			let new_random = (<system::Module<T>>::random_seed(), nonce)
					.using_encoded(|b| Blake2Hasher::hash(b))
					.using_encoded(|mut b| u64::decode(&mut b))
					.expect("Hash must be bigger than 8 bytes; Qed");
			let random_index = new_random % <DatId>::get();
			let random_dat = DatKey::get(random_index);
			let mut current_user_dats = <UsersStorage<T>>::get(&account);
			current_user_dats.push(random_dat);
			<UsersStorage<T>>::insert(&account.clone(), &current_user_dats);
			<Nonce>::mutate(|m| *m += 1);
			if(current_user_dats.len() == 1){
				<Users<T>>::insert(<UsersCount>::get(), &account);
				<UsersCount>::mutate(|m| *m += 1);
			}
			Self::deposit_event(RawEvent::NewPin(account));
		}

		fn on_finalize(n: T::BlockNumber) {
			if (n == Self::time_limit()) {
				let user = <SelectedUser<T>>::take();
				//(todo) calculate some punishment
				//(todo) punish user
				<UsersStorage<T>>::remove(user);
				<UsersCount>::mutate(|m| *m -= 1);
			}
		}
	}
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok, parameter_types};
	use sr_primitives::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};
	use sr_primitives::weights::Weight;
	use sr_primitives::Perbill;

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
		type WeightMultiplierUpdate = ();
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
	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			// Just a dummy test for the dummy funtion `do_something`
			// calling the `do_something` function with a value 42
			assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
			// asserting that the stored value is equal to what we stored
			assert_eq!(TemplateModule::something(), Some(42));
		});
	}
}
