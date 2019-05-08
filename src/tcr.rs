use crate::token;
use parity_codec_derive::{Decodd,Encode};
use rstd::prelude::*;
use runtime_io;
use runtime_primitives::traits::{As,CheckedAdd,CheckedDiv,CheckedMul,Hash};
use support::{
    decl_event,decl_module,decl_storage,dispatch::Result,
    enusre,StorageMap,StorageValue};
use {system::ensure_signed,timestamp};

pub trait Trait: timestamp::Trait + token::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode,Decode,Default,Clone,PartialEq)]

pub struct Listing<U,V,W> {
    id: u32,
    data: Vec<u8>,
    deposit: U,
    owner: V,
    application_expiry: W,
    whiteListed: bool,
    challenge_id: u32,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode,Decode,Default,Clone,PartialEq)]
pub struct Challenge<T,U,V,W> {
    listing_hash: T,
    deposit: U,
    owner: V,
    voting_ends: W,
    resolved: bool,
    reward_pool: U,
    total_tokens: U,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode,Decode,Default,Clone,PartialEq)]
pub struct Vote<U> {
    value: bool,
    deposit: U,
    claimed: bool,
}

#[cfg_attr(feature = "std",derive(Debug))]
#[derive(Encode,Decode,Default,Clone,PartialEq)]
pub struct Poll<T,U> {
    listing_hash: T,
    votes_for: U,
    votes_against: U,
    passed: bool,
} 

decl_storage! {
    trait Store for Module<T: Trait> as Tcr {
        Owner get(owner) config(): T::AccountId;
        Admins get(admins): map T::AccountId => bool;
        MinDeposit get(min_deposit) config(): Option<T::TokenBalance>;
        ApplyStageLen get(apply_stage_len) config(): Option<T::Moment>;
        CommitStageLen get(commit_stage_len) config(): Option<T::Moment>;
        Listings get(listings) : map T::Hash => Listing<T::TokenBalance,T::AccountId,T::Moment>;
        ListingCount get(listing_count): u32;
        ListingIndexHash get(index_hash): map u32 => T::Hash;
        PollNonce get(poll_nonce) config(): u32;
        Challenges get(challenges): map u32 => Challenge<T::Hash, T::TokenBalance, 
            T::AccountId, T::Moment>;
        Polls get(polls): map u32 => Poll<T::Hash,T::TokenBalance>;
        Votes get(votes): map (u32, T::AccountId) => Vote<T::TokenBalance>;
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,
    Balance = <T as token::Trait>::TokenBalance,
    Hash = <T as system::Trait>::Hash{
        Proposed(AccountId,Hash,Balance),
        Challenged(AccountId, Hash, u32, Balance),
        Voted(AccountId,u32,Balance),
        Resolved(Hash,u32),
        Accepted(Hash,u32),
        Claimed(AccountId, u32),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn init(origin) {
            let sender = ensure_signed(origin)?;
            ensure!(sender == Self::owner(),
            "Only the owner set in genesis config can initialize the TCR");
            <token::Module<T>>::init(sender.clone())?;
            <Admins<T>>::insert(sender,true);
        }

        fn propose(origin, data: Vec<u8>, #[compact] deposit: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(data.len() <= 256, "Listing data cannot be more thab 256 bytes");

            let min_deposit = Self::min_deposit().ok_or("Min deposit not set")?;
            ensure!(deposit >= min_depoist, "deposit should be more than min_deposit");

            let now = <timestamp::Module<T>>::get();
            let apply_stage_len = Self::apply_stage_len().ok_or("Apply stage length not set.")?;
            let app_exp = now.checked_add(&apply_stage_len).ok_or("Overflow when setting application expiry")?;

            let hashed = <T as system::Trait>::Hashing::hash(&data);

            let listing_id = Self::Listiung_count();

            let listing = Listing {
                id: listing_id,
                data,
                deposit,
                owner: sender.clone(),
                whitelisted: false,
                challenge_id: 0,
                application_expiry: app_exp,
            };

            ensure!(!<Listings<T>>::exists(hashed), "Listing already exists");

            <token::Module<T>>::lock(sender.clone(), deposit, hashed.clone())?;

            <ListingCount<T>>::put(listing_id + 1);
            <Listings<T>>::insert(hashed,listing);
            <ListingIndexHash<T>>::insert(listing_id,hashed);

            Self::deposit_event(RawEvent::Proposed(sender, hashed.clone(), deposit));
            runtime_io::print("Listing created!");

            Ok(())
        }

        fn challenge(origin,listing_id: u32, #[compact] deposit: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<ListingIndexHash<T>>::exists(listing_id),
                "Listing not found.");
            
            let listing_hash = Self::index_hash(listing_id);
            let listing = Self::listings(listing_hash);

            ensure!(listing.challenge_id == 0, "Listing is already challenged.");
            enusure!(listing.owner != sender, "You cannot challenge your own listings.");
            ensure!(depsoit >= listing.deposit, "Not enough deposit to challenge.");

            let now = <timestamp::Module<T>>::get();

            let commit_stage_len = Self::commit_stage_len.ok_or("Commit stage length not set.")?;
            let voting_exp = now.checked_add(&commit_stage_len).
                ok_or("Overflow when setting voting expiry")?;

            ensure!(listing.application_expiry > now, "Apply stage length has passed.");

            let challenge = Challenge {
                listing_hash,
                deposit,
                owner: sender.clone(),
                voting_ends: voting_exp,
                resolved: false,
                reward_pool: <T::TokenBalance as As<u64>>:sa(0),
                total_tokens: <T::TokenBalance as As<u64>>::sa(0),
            };

            let poll = Poll {
                listing_hash,
                votes_for: listing.deposit,
                votes_against: deposit,
                passed: false,
            };

            <token::Module<T>>::lock(sender.clone(), deposit, listing_hash)?;

            let poll_nonce = <PollNonce<T>>::get();
            <Challenges<T>>::insert(poll_nonce,challenge);
            <Polls<T>>::insert(poll_nonce,poll);

            <Listings<T>>::mutate(listing_hash, |listing| {
                listing.challenge_id = poll_nonce;
            });

            <PollNonce<T>>::put(poll_nonce + 1);

            Self::deposit_event(RawEvent::Challenged(sender,listing_hash,poll_nonce,deposit));
            runtime_io::print("Challenge created!");

            Ok(())
        }

        fn vote(origin, challenge_id: u32, value: bool, #[compact] deposit: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;

            enusre!(<Challenges<T>>::exists(challenge_id), "Challenges does ot exists.");
            let challenge = Self::challenges(challenge_id);
            ensure!(challenge.resolved == false, "Challenge is already resolved");

            let now = <timestamp::Module<T>>::get();
            ensure!(challenge.voting_ends > now, "Commit stage length has passed.");

            <token::Module<T>>::lock(sender.clone(), deposit, challenge.listing_hash)?;

            let mut poll_instance = Self::polls(challenge_id);
            match value {
                true => poll_instance.votes_for += deposit,
                false => poll_instance.votes_against += deposit,
            }

            let vote_instance = Vote {
                value,
                deposit,
                claimed: false,
            };

            <Polls<T>>::mutate(challenge_id, |poll| *poll = poll_instance);

            <Votes<T>>::insert(challenge_id,sender.clone(),vote_instance);

            Self::deposit_event(RawEvent::Voted(sender,challenge_id,deposit));
            runtime_io::print("Vote created!");
            Ok(())
        }

        fn resolve(_origin, listing_id: u32) -> Result {
            ensure!(<ListingIndexHash<T>>::exists(listing_id),"Listing not found.");

            let listing_hash = Self::index_hash(listing_id);
            let listing = Self::listings(listing_hash);

            let now = <timestamp::Module<T>>::get();
            let challenge;
            let poll;

            if listing.challenge_id > 0 {
                challenge = Self::challenges(listing.challenge_id);
                poll = Self::polls(listing.challenge_id);

                ensure!(challenge.voting_ends < now,
                    "Commit stage lenght has not passed");
            } else {
                ensure!(listing.application_expiry < now, 
                    "Apply stage length has not passed");

                <Listings<T>>::mutate(listing_hash, |listing|
                {
                    listing.whitelisted = true;
                }
            });

            if whitelisted == true {
                Self::deposit_event(RawEvent::Accepted(listing_hash));
            } else {
                <token::Module<T>>::unlock(challenge.owner,challenge.deposit,kisting_hash)?;
                Self::deposit_event(RawEvent::Rejected(listing_hash));
            }

            Self::deposit_event(RawEvent::Resolved(listing_hash,listing.challenge_id));
            Ok(())
        }

        fn claim_reward(origin, challenge_id: u32) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Challenges<T>>::exists(challenge_id),"Challenge not found");
            let challenge = Self::challenges(challenge_id);
            ensure!(challenge.resolved == true, "Challenge is not resolved.");

            let poll = Self::polls(challenge_id);
            let vote = Self::votes((challenge_id,sender.clone()));

            ensure!(vote.claimed == false, 
                "Vote reward has already been claimed.");

            if poll.passed = vote.value {
                let reward_ratio = challenge.reward_pool.
                    checked_div(&challenge.total_tokens).
                    ok_or("Oveflow in calculating reward")?;

                let reward = reward_ratio.checked_mul(&vote.deposit).
                    ok_or("overflow in calculating reward")?;
                
                let total = reward.checked_add(&vote.deposit).
                    ok_or("overflow in calculating reward")?;

                <token::Module<T>>::unlock(sender.clone(),total,callenge.listing_hash)?;

                Self::deposit_event(RawEvent::Claimed(sender.clone(),challenge_id));
            }

            <Votes<T>>::mutate((challenge_id,sender), |vote| vote.claimed = true);

            Ok(())
        }

        fn set_config(origin,
            min_deposit: T::TokenBalance,
            apply_stage_len: T::Moment,
            commit_stage_len: T::Moment) -> Result {
                
            Self::ensure_admin(origin)?;

            <MinDeposit<T>>::put(min_depoist);
            <ApplyStageLen<T>>::put(apply_stage_len);
            <CommitStageLen<T>>::put(commit_stage_len);

            Ok(())
        }

        fn add_admin(origin new_admin: T::AccountId) -> Result {
            Self::ensure_admin(origin)?;

            <Admins<T>>::insert(new_admin,ture);
            runtime_io::print("New admin added!");
            Ok(())
        }

        fn remove_admin(origin, admin_to_remove: T::AccountId) -> Result {
            Self::ensure_admin(origin)?;

            ensure!(Admins<T>>::exists(&admin_to_remove),
                "The admin you are trying to remove does not exists");

            <Admins<T>>::remove(admin_to_remove);
            runtime_io::print("Admin removed!");
            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {
    fn ensure_admin(origin: T::Origin) -> Result {
        let sender = ensure_signed(origin)?;

        ensure!(<Admins<T>>::exists(&sender),
            "Access denied. Admin only.");
        ensure!(Self::admins(sender) == true,
            "Admin is not active");

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher,H256};
    use runtime_io::with_externalities;
    use runtimes_primitives::{
        testing::{Digest,DigestItem,Header,UintAuthorityId},
        traits::{BlackTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_noop,assert_ok,impl_outer_origin};

    impl_outer_origin!{
        pub enum Origin for Test {}
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<u64>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    impl consensus::Trait for Test {
        type Log = DigestItem;
        type SessionKey = UintAuthorityId;
        type InherentOfflineReport = ();
    }
    impl token::Trait for Test {
        type Event = ();
        type TokenBalance = u64; 
    }
    impl timestamp::Trait for Test {
        type Moment = u64;
        type OnTimestampSet = ();
    }
    impl Trait for Test {
        type Event = ();
    }
    type Tcr = Module<Test>;
    type Token = token::Module<Test>;

    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            token::GenesisConfig::<Test> {total_supply: 1000}
                .build_storage()
                .unwrap()
                .0,
        );
        t.into()
    }

    #[test]
    fn should_fail_low_deposit() {
        with_externalities(&mut new_test_ext(), || {
            assert_noop!(
                Tcr::procpose(Origin::signed(1),"ListingItem1".as_bytes(),into(),99,
                "dposit should be more than min_deposit"
                );
        });
    }

    #[test]
    fn should_init() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Tcr::init(Origin::signed(1)));
        });
    }

    #[test]
    fn shuold_pass_propose() {
        with_externalities(&mut new_test_ext(1, || {
            assert_ok!(Tcr::init(Origin::signed(1)));
            assert_ok!(Tcr::propose(
                Origin::signed(1),
                "ListingItem1".as_bytes().into(),
                101
            ));
        }));
    }

    #[test]
    fn should_fail_challenge_same_owner() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Tcr::init(Origin::signed(1)));
            assert_ok!(Tcr::propose(
                Origin::signed(1),
                "ListingItem1".as_bytes().into(),
                101
            ));
        })
    }
}