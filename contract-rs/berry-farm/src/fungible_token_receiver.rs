use crate::*;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::PromiseOrValue;

#[near_bindgen]
impl FungibleTokenReceiver for Farm {
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        if &env::predecessor_account_id() != &self.banana_token_account_id {
            env::panic(b"This farm can only receive bananas through a contract API");
        }
        let payload: OnReceiverPayload =
            serde_json::from_str(&msg).expect("Failed to parse the payload");

        let amount: Balance = amount.into();

        match payload {
            OnReceiverPayload::DepositAndStake => {
                let (account_id_hash, mut account) = self.get_mut_account(sender_id.as_ref());
                account.cucumber_balance += amount;
                self.save_account(&account_id_hash, &account);

                self.total_cucumber_balance += amount;
                PromiseOrValue::Value(0.into())
            }
        }
    }
}
