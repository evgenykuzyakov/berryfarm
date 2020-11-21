use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json;
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Balance, Promise};

mod token;
use token::*;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

uint::construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    pub last_near_per_cucumber_numer: Balance,
    pub near_balance: Balance,
    pub cucumber_balance: Balance,
    pub near_claimed: Balance,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanAccount {
    pub near_balance: U128,
    pub cucumber_balance: U128,
    pub near_claimed: U128,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct HumanStats {
    pub total_cucumber_balance: U128,
    pub total_near_claimed: U128,
    pub total_near_received: U128,
}

pub const NEAR_PER_CUCUMBER_DENOM: Balance = 1_000_000_000_000_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Farm {
    pub accounts: LookupMap<ShortAccountHash, Account>,

    pub banana_token_account_id: AccountId,

    pub near_per_cucumber_numer: Balance,

    pub total_cucumber_balance: Balance,

    pub total_near_received: Balance,

    pub total_near_claimed: Balance,

    pub vaults: LookupMap<VaultId, Vault>,

    pub next_vault_id: VaultId,
}

impl Default for Farm {
    fn default() -> Self {
        panic!("Contract should be initialized before usage")
    }
}

#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq)]
pub struct ShortAccountHash(pub [u8; 20]);

impl From<&AccountId> for ShortAccountHash {
    fn from(account_id: &AccountId) -> Self {
        let mut buf = [0u8; 20];
        buf.copy_from_slice(&env::sha256(account_id.as_bytes())[..20]);
        Self(buf)
    }
}

#[ext_contract(ext_token)]
pub trait ExtVaultFungibleToken {
    fn withdraw_from_vault(&mut self, vault_id: VaultId, receiver_id: AccountId, amount: U128);
    fn register_account(&mut self, account_id: AccountId);
    fn transfer_unsafe(&mut self, receiver_id: AccountId, amount: U128);
}

#[derive(Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum OnReceiverPayload {
    DepositAndStake,
}

/// Implements a trait to receiver_id
pub trait VaultFungibleTokenReceiver {
    fn on_receive_with_vault(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        vault_id: VaultId,
        payload: String,
    ) -> Promise;
}

#[near_bindgen]
impl Farm {
    #[init]
    pub fn new(banana_token_account_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        // Registering the account for banana token to be able to withdraw.
        ext_token::register_account(
            env::current_account_id(),
            banana_token_account_id.as_ref(),
            NO_DEPOSIT,
            GAS_FOR_ACCOUNT_REGISTRATION,
        );
        Self {
            accounts: LookupMap::new(b"a".to_vec()),
            banana_token_account_id: banana_token_account_id.into(),
            near_per_cucumber_numer: 0,
            total_cucumber_balance: 0,
            total_near_received: 0,
            total_near_claimed: 0,
            vaults: LookupMap::new(b"v".to_vec()),
            next_vault_id: VaultId(0),
        }
    }

    #[payable]
    pub fn take_my_near(&mut self) {
        assert!(
            self.total_cucumber_balance >= NEAR_PER_CUCUMBER_DENOM,
            "Not enough cucumbers"
        );
        let attached_deposit = env::attached_deposit();
        let near_per_cucumber = (U256::from(attached_deposit)
            * U256::from(NEAR_PER_CUCUMBER_DENOM)
            / U256::from(self.total_cucumber_balance))
        .as_u128();
        self.near_per_cucumber_numer += near_per_cucumber;
        self.total_near_received += attached_deposit;
    }

    pub fn register_account(&mut self) {
        let (account_id_hash, account) = self.get_mut_account(&env::predecessor_account_id());
        self.save_account(&account_id_hash, &account);
    }

    pub fn account_exists(&self, account_id: ValidAccountId) -> bool {
        self.get_internal_account(account_id.as_ref()).1.is_some()
    }

    pub fn claim_near(&mut self) -> U128 {
        let account_id = env::predecessor_account_id();
        let (account_id_hash, mut account) = self.get_mut_account(&account_id);
        let amount = account.near_balance;
        account.near_balance = 0;
        account.near_claimed += amount;
        self.save_account(&account_id_hash, &account);
        if amount > 0 {
            Promise::new(account_id).transfer(amount);
            self.total_near_claimed += amount;
        }
        amount.into()
    }

    pub fn get_near_balance(&self, account_id: ValidAccountId) -> U128 {
        self.get_internal_account(account_id.as_ref())
            .1
            .map(|mut account| {
                self.touch(&mut account);
                account.near_balance
            })
            .unwrap_or(0)
            .into()
    }

    pub fn get_account(&self, account_id: ValidAccountId) -> Option<HumanAccount> {
        self.get_internal_account(account_id.as_ref())
            .1
            .map(|mut account| {
                self.touch(&mut account);
                HumanAccount {
                    near_balance: account.near_balance.into(),
                    cucumber_balance: account.cucumber_balance.into(),
                    near_claimed: account.near_claimed.into(),
                }
            })
    }

    pub fn get_stats(&self) -> HumanStats {
        HumanStats {
            total_cucumber_balance: self.total_cucumber_balance.into(),
            total_near_claimed: self.total_near_claimed.into(),
            total_near_received: self.total_near_received.into(),
        }
    }

    pub fn get_total_near_claimed(&self) -> U128 {
        self.total_near_claimed.into()
    }

    pub fn get_total_near_received(&self) -> U128 {
        self.total_near_received.into()
    }
}

#[near_bindgen]
impl VaultFungibleTokenReceiver for Farm {
    fn on_receive_with_vault(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        vault_id: VaultId,
        payload: String,
    ) -> Promise {
        if &env::predecessor_account_id() != &self.banana_token_account_id {
            env::panic(b"This farm can only receive bananas through a contract API");
        }
        let payload: OnReceiverPayload =
            serde_json::from_str(&payload).expect("Failed to parse the payload");

        let amount: Balance = amount.into();

        match payload {
            OnReceiverPayload::DepositAndStake => {
                let (account_id_hash, mut account) = self.get_mut_account(sender_id.as_ref());
                account.cucumber_balance += amount;
                self.save_account(&account_id_hash, &account);

                self.total_cucumber_balance += amount;

                ext_token::withdraw_from_vault(
                    vault_id,
                    env::current_account_id(),
                    amount.into(),
                    &self.banana_token_account_id,
                    NO_DEPOSIT,
                    GAS_FOR_WITHDRAW_FROM_VAULT,
                )
            }
        }
    }
}

impl Farm {
    fn get_internal_account(&self, account_id: &AccountId) -> (ShortAccountHash, Option<Account>) {
        let account_id_hash: ShortAccountHash = account_id.into();
        let account = self.accounts.get(&account_id_hash);
        (account_id_hash, account)
    }

    /// Redeeming rewards and updating inner pool balances.
    fn touch(&self, account: &mut Account) {
        let near_per_cucumber_diff =
            self.near_per_cucumber_numer - account.last_near_per_cucumber_numer;
        let earned_balance = (U256::from(near_per_cucumber_diff)
            * U256::from(account.cucumber_balance)
            / U256::from(NEAR_PER_CUCUMBER_DENOM))
        .as_u128();
        account.near_balance += earned_balance;
        account.last_near_per_cucumber_numer = self.near_per_cucumber_numer;
    }

    fn get_mut_account(&mut self, account_id: &AccountId) -> (ShortAccountHash, Account) {
        let (account_id_hash, account) = self.get_internal_account(&account_id);
        let mut account = account.unwrap_or_else(|| Account {
            last_near_per_cucumber_numer: self.near_per_cucumber_numer,
            near_balance: 0,
            cucumber_balance: 0,
            near_claimed: 0,
        });
        self.touch(&mut account);
        (account_id_hash, account)
    }

    fn save_account(&mut self, account_id_hash: &ShortAccountHash, account: &Account) {
        self.accounts.insert(account_id_hash, account);
    }
}
