use crate::*;

use near_sdk::json_types::ValidAccountId;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{ext_contract, Balance, Gas, Promise};

/// Don't need deposits for function calls.
pub const NO_DEPOSIT: Balance = 0;

/// NOTE: These fees are going to change with the update.
/// Basic compute.
const GAS_BASE_COMPUTE: Gas = 5_000_000_000_000;
/// Fee for function call promise.
const GAS_FOR_PROMISE: Gas = 5_000_000_000_000;
/// Fee for the `.then` call.
const GAS_FOR_DATA_DEPENDENCY: Gas = 10_000_000_000_000;

/// Gas attached to the receiver for `on_receive_with_safe` call.
/// NOTE: The minimum logic is to do some very basic compute and schedule a withdrawal from safe
/// that it returns from the promise.
const MIN_GAS_FOR_RECEIVER: Gas = GAS_FOR_PROMISE + GAS_BASE_COMPUTE;
/// Gas attached to the callback to resolve safe. It only needs to do basic compute.
/// NOTE: It doesn't account for storage refunds.
const GAS_FOR_CALLBACK: Gas = GAS_BASE_COMPUTE;
/// The amount of gas required to complete the execution of `transfer_with_safe`.
/// We need to create 2 promises with a dependencies and with some basic compute to write to the
/// state.
/// NOTE: It doesn't account for storage refunds.
const GAS_FOR_REMAINING_COMPUTE: Gas =
    2 * GAS_FOR_PROMISE + GAS_FOR_DATA_DEPENDENCY + GAS_BASE_COMPUTE;

pub const GAS_FOR_ACCOUNT_REGISTRATION: Gas = GAS_BASE_COMPUTE;

pub const GAS_FOR_WITHDRAW_FROM_VAULT: Gas = GAS_BASE_COMPUTE;

/// Safe identifier.
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Copy)]
#[serde(crate = "near_sdk::serde")]
pub struct VaultId(pub u64);

impl VaultId {
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Vault {
    /// The `ShortAccountHash` of the receiver ID.
    /// This information is only needed to validate safe ownership during withdrawal.
    pub receiver_id_hash: ShortAccountHash,
    /// The remaining amount of tokens in the safe.
    pub balance: Balance,
}

#[ext_contract(ext_token_receiver)]
trait ExtTokenReceiver {
    fn on_receive_with_vault(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        vault_id: VaultId,
        payload: String,
    );
}

#[ext_contract(ext_self)]
trait ExtSelf {
    fn resolve_vault(&mut self, vault_id: VaultId, sender_id: AccountId);
}

/// NEP 122
trait VaultFungibleToken {
    /// Simple transfers
    /// Gas requirement: 5 TGas or 5000000000000 Gas
    /// Should be called by the balance owner.
    /// Requires that the sender and the receiver accounts be registered.
    ///
    /// Actions:
    /// - Transfers `amount` of tokens from `predecessor_id` to `receiver_id`.
    fn transfer_raw(&mut self, receiver_id: ValidAccountId, amount: U128);

    /// Transfer to a contract with payload
    /// Gas requirement: 40+ TGas or 40000000000000 Gas.
    /// Consumes: 30 TGas and the remaining gas is passed to the `receiver_id` (at least 10 TGas)
    /// Should be called by the balance owner.
    /// Returns a promise, that will result in the unspent balance from the transfer `amount`.
    ///
    /// Actions:
    /// - Withdraws `amount` from the `predecessor_id` account.
    /// - Creates a new local safe with a new unique `safe_id` with the following content:
    ///     `{sender_id: predecessor_id, amount: amount, receiver_id: receiver_id}`
    /// - Saves this safe to the storage.
    /// - Calls on `receiver_id` method `on_token_receive(sender_id: predecessor_id, amount, safe_id, payload)`/
    /// - Attaches a self callback to this promise `resolve_safe(safe_id, sender_id)`
    fn transfer_with_vault(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        payload: String,
    ) -> Promise;
    fn withdraw_from_vault(&mut self, vault_id: VaultId, receiver_id: ValidAccountId, amount: U128);
    fn resolve_vault(&mut self, vault_id: VaultId, sender_id: AccountId) -> U128;

    fn get_total_supply(&self) -> U128;
    fn get_balance(&self, account_id: ValidAccountId) -> U128;
}

#[near_bindgen]
impl VaultFungibleToken for Farm {
    #[payable]
    fn transfer_raw(&mut self, receiver_id: ValidAccountId, amount: U128) {
        assert_paid();
        let amount = amount.into();
        self.withdraw_from_sender(receiver_id.as_ref(), amount);
        self.deposit_to_account(receiver_id.as_ref(), amount);
    }

    #[payable]
    fn transfer_with_vault(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        payload: String,
    ) -> Promise {
        assert_paid();
        let gas_to_receiver =
            env::prepaid_gas().saturating_sub(GAS_FOR_REMAINING_COMPUTE + GAS_FOR_CALLBACK);

        if gas_to_receiver < MIN_GAS_FOR_RECEIVER {
            env::panic(b"Not enough gas attached. Attach at least 40 TGas");
        }

        let amount = amount.into();
        let sender_id = self.withdraw_from_sender(receiver_id.as_ref(), amount);

        // Creating a new vault
        let vault_id = self.next_vault_id;
        self.next_vault_id = vault_id.next();
        let vault = Vault {
            receiver_id_hash: receiver_id.as_ref().into(),
            balance: amount,
        };
        self.vaults.insert(&vault_id, &vault);

        // Calling the receiver
        ext_token_receiver::on_receive_with_vault(
            sender_id.clone(),
            amount.into(),
            vault_id,
            payload,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            gas_to_receiver,
        )
        .then(ext_self::resolve_vault(
            vault_id,
            sender_id,
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_CALLBACK,
        ))
    }

    fn withdraw_from_vault(
        &mut self,
        vault_id: VaultId,
        receiver_id: ValidAccountId,
        amount: U128,
    ) {
        let mut vault = self.vaults.get(&vault_id).expect("Vault doesn't exist");
        let vault_receiver_id = env::predecessor_account_id();
        if &ShortAccountHash::from(&vault_receiver_id) != &vault.receiver_id_hash {
            env::panic(b"The vault is not owned by the predecessor");
        }
        let amount = amount.into();
        if vault.balance < amount {
            env::panic(b"Not enough balance in the vault");
        }
        vault.balance -= amount;
        self.vaults.insert(&vault_id, &vault);

        self.deposit_to_account(receiver_id.as_ref(), amount);
    }

    fn resolve_vault(&mut self, vault_id: VaultId, sender_id: AccountId) -> U128 {
        if env::current_account_id() != env::predecessor_account_id() {
            env::panic(b"This method is private");
        }

        let vault = self.vaults.remove(&vault_id).expect("Vault doesn't exist");

        if vault.balance > 0 {
            self.deposit_to_account(&sender_id, vault.balance);
        }

        vault.balance.into()
    }

    fn get_total_supply(&self) -> U128 {
        self.total_cucumber_balance.into()
    }

    fn get_balance(&self, account_id: ValidAccountId) -> U128 {
        self.get_internal_account(account_id.as_ref())
            .1
            .map(|account| account.cucumber_balance)
            .unwrap_or(0)
            .into()
    }
}

impl Farm {
    /// Withdraws `amount` from the `predecessor_id` while comparing it to the `receiver_id`.
    /// Return `predecessor_id` and hash of the predecessor
    pub(crate) fn withdraw_from_sender(
        &mut self,
        receiver_id: &AccountId,
        amount: Balance,
    ) -> AccountId {
        if amount == 0 {
            env::panic(b"Transfer amount should be positive");
        }
        let sender_id = env::predecessor_account_id();
        if &sender_id == receiver_id {
            env::panic(b"The receiver should be different from the sender");
        }

        // Retrieving the account from the state.
        let (account_id_hash, mut account) = self.get_mut_account(&sender_id.clone());

        // Checking and updating the balance
        if account.cucumber_balance < amount {
            env::panic(b"Not enough cucumber balance");
        }
        account.cucumber_balance -= amount;

        // Saving the account back to the state.
        self.save_account(&account_id_hash, &account);

        sender_id
    }

    /// Deposits `amount` to the `account_id`
    pub(crate) fn deposit_to_account(&mut self, account_id: &AccountId, amount: Balance) {
        if amount == 0 {
            return;
        }
        // Retrieving the account from the state.
        let (account_id_hash, account) = self.get_internal_account(&account_id);
        let mut account = account.expect("Receiver account doesn't exist");
        self.touch(&mut account);

        account.cucumber_balance += amount;
        // Saving the account back to the state.
        self.save_account(&account_id_hash, &account);
    }
}

fn assert_paid() {
    assert!(
        env::attached_deposit() > 0,
        "Requires a deposit of at least 1 yoctoNEAR to prevent function access key calls"
    );
}
