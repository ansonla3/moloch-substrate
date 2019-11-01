use support::{decl_module, decl_storage, decl_event, ensure, Parameter, dispatch::Result};
use system::ensure_signed;
use sr_primitives::traits::{CheckedSub, CheckedAdd, Member, SimpleArithmetic};
use codec::Codec;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + From<Self::BlockNumber>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Token {
        Init get(is_init): bool;

        TotalSupply get(total_supply) config(): T::TokenBalance;

        BalanceOf get(balance_of): map T::AccountId => T::TokenBalance;

        Allowance get(allowance): map (T::AccountId, T::AccountId) => T::TokenBalance;

        LockedDeposits get(locked_deposits): map T::AccountId => T::TokenBalance;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn transfer(origin, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::balance_transfer(sender, to, value)
        }
    }
}

decl_event!(
    pub enum Event<T> where 
        AccountId = <T as system::Trait>::AccountId,
        TokenBalance = <T as self::Trait>::TokenBalance {
        SomethingStored(u32, AccountId),
        Transfer(AccountId, AccountId, TokenBalance),
        Approval(AccountId, AccountId, TokenBalance),
    }
);

impl<T: Trait> Module<T> {
    pub fn init(sender: T::AccountId, balance: T::TokenBalance) -> Result {
        ensure!(Self::is_init() == false, "Token already initialized.");

        <BalanceOf<T>>::insert(sender, balance);
        <Init>::put(true);

        Ok(())
    }

    pub fn mint(recipient: T::AccountId, shares_to_mint: T::TokenBalance) -> Result {
        // TODO Add Checking
        let current_balance = Self::balance_of(recipient.clone());
        let new_balance = current_balance.checked_add(&shares_to_mint).ok_or("Overflow minting new shares")?;

        <BalanceOf<T>>::insert(recipient, new_balance);

        Ok(())
    }

    pub fn lock(sender: T::AccountId, balance: T::TokenBalance) -> Result {
        ensure!(<BalanceOf<T>>::exists(sender.clone()), "Account does not own this token");

        let sender_balance = Self::balance_of(sender.clone());
        ensure!(sender_balance > balance, "Not enough balance");
        let updated_from_balance = sender_balance.checked_sub(&balance).ok_or("Overflow in calculating balance")?;
        let deposit = Self::locked_deposits(sender.clone());
        let updated_deposit = deposit.checked_add(&balance).ok_or("Overflow in calculating deposit")?;

        <BalanceOf<T>>::insert(sender.clone(), updated_from_balance);

        <LockedDeposits<T>>::insert(sender, updated_deposit);

        Ok(())
    }

    pub fn unlock(sender: T::AccountId, balance: T::TokenBalance) -> Result {
        let to_balance = Self::balance_of(sender.clone());
        let updated_to_balance = to_balance.checked_add(&balance).ok_or("Overflow in calculating balance")?;
        let deposit = Self::locked_deposits(sender.clone());
        let updated_deposit = deposit.checked_sub(&balance).ok_or("Overflow in calculating deposit")?;

        <BalanceOf<T>>::insert(sender.clone(), updated_to_balance);

        <LockedDeposits<T>>::insert(sender, updated_deposit);

        Ok(())
    }

    pub fn balance_transfer(
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(<BalanceOf<T>>::exists(from.clone()), "Account does not own this token");
        let sender_balance = Self::balance_of(from.clone());
        ensure!(sender_balance >= value, "Not enough balance.");
        let updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
        let receiver_balance = Self::balance_of(to.clone());
        let updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

        <BalanceOf<T>>::insert(from.clone(), updated_from_balance);

        <BalanceOf<T>>::insert(to.clone(), updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }

}