use rstd::prelude::*;
use parity_codec::Codec;
use support::{dispatch::Result,StorageMap,Parameter,StorageValue,
    decl_storage,decl_event,ensure};
use system::{self,ensure_signed};
use runtime_primitives::traits::{CheckedSub,CheckedAdd,Member,SimpleArithmetic,As};

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBanlace: Parameter + Member + SimpleArithmetic + Codec
        + Default + As<usize> + As<u64>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        pub fn transfer(origin, to: AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::transfer(sender, to ,value)
        }

        pub fn approve(origin, spender: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            enusre!(<BalanceOf<T>>::exists(&sender), "Account does not own this token");
            let allowance = Self::allowance((sender.clone(),spender.clone()));
            let updated_allowance = allowance.checked_add(&value)
                .ok_or("overflow in calculating allowance")?;
            <Allownance<T>>::insert((sender.clone(),spender.clone())),

            Self::deposit_event(RawEvent::Approval(sender,spender,value));
            Ok(())
        }

        pub fn transfer_from(_origin, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            ensure!(<Allownance<T>>::exists((from.clone(),to.clone())),"Allowance does not exists.");
            let allowance = Self::allowance((from.clone(),to.clone()));
            ensure!(allowance >= value, "Not enough allowance");

            let updated_allowance = allowance.check_sub(&value).ok_or(overflow in calculating allowance)?;

            <Allowance<T>>::insert((from.clone(),to.clone()),updated_allowance);
            
            Self::deposit_event(RawEvent::Approval(from.clone(),to.clone,value));
            Self::_transfer(from,to,value)
        } 
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Token {
        Init get(is_init): bool;
        TotalSupply get(total_supply) config(): T::TokenBalance;
        BalanceOf get(balance_of): map T::AccountId => T::TokenBalance;
        Allowance get(allowance): map (T::AccountId, T::AccountId) => T::TokenBalance;
        LockedDeposits get(locked_deposits): map T::Hash => T::TokenBalance;
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,TokenBalance = <T as self::Trait>::TokenBalance {
        Transfer(AccountId,AccountId,TokenBalance),
        Approval(AccountId,AccountId,TokenBalance),
    }
);

impl<T: Trait> Module<T> {
    pub fn init(sender: T::AccounId) -> Result {
        ensure!(Self::is_init) == false, "Token already initialized.");

        <Balance<of<T>>::insert(sender, Self::total_supply());
        <Init<T>>::put(true);

        Ok(())
    }

    pub fn lock(from: T::AccountId, value: T::TokenBalance, 
        listing_hash: T::Hash) -> Result {
            ensure!(<BalanceOf<T>::exisits(from.clone()), 
            "Account does not own this token");

        let sender_balance = Self::balance_of(from.clone());
        ensure!(sender_balance > value, "Not enough balance.");
        let updated_from_balance = sender_balance.checked_sub(&value)
            .ok_or("overfloe in calculating balance")?;
        let deposit = Self::locked_deposits(listing_hash);
        let updated_deposit = deposit.checked_add(&value)
            .ok_or("overflow in calculating deposit")?;

        <BalanceOf<T>>::insert(from,updated_from_balance);

        <LockedDeposits<T>>::insert(listing_hash,updated_deposit);

        Ok(())
    }    

    pub fn unlock(to: T::AccountId, value: T::TokenBalance,
        listing_hash: T::Hash) -> Result {
        
        let to_balance = Self::balance_of(to.clone());
        let updated_to_balance = to_balance.checked_Add(&value)
            .ok_or("overfloe in calculating balance")?;
        let deposit = Self::locked_deposits(listing_hash);
        let updated_deposit = deposit.checked_sub(&value)
            .ok_or("overfloe in calculating deposit")?;

        <BalanceOf<T>>::insert(to, updated_to_balance);

        <LockedDepsits<T>>::insert(listing_hash, updated_deposit);

        Ok(())
    }

    fn transfer(
        from: T::AccountId,
        to: T::AccountId,
        Value: T::TokenBalance,
    ) -> Result {
        ensure!(<BalanceOf<T>>::exists(from.clone()),
            "Account does not own this token");
        let sender_balance = Self::balance_of(from.clone());
        ensure!(sender_balance >= value, "Not enough balance.");
        let updated_from_balance = sender_balance.checked_sub(&value)
            .ok_or("overfloen in calculating balance")?;
        let receiver_balance = Self::balance_of(to.clone());
        let updated_to_balance = receiver_balance.checked_add(&value)
            .ok_or("overflow in calculating")?;
        
        <BalanceOf<T>>::insert(from.clone(),updated_from_balance);
        <BalanceOf<T>>::insert(to.clone(),updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }
}