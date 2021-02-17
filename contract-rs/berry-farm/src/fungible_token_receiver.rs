use crate::*;
use near_sdk::json_types::{ValidAccountId, U128};

trait FungibleTokenReceiver {
    /// Called by fungible token contract after `ft_transfer_call` was initiated by
    /// `sender_id` of the given `amount` with the transfer message given in `msg` field.
    /// The `amount` of tokens were already transferred to this contract account and ready to be used.
    ///
    /// The method must return the amount of tokens that are *not* used/accepted by this contract from the transferred
    /// amount. Examples:
    /// - The transferred amount was `500`, the contract completely takes it and must return `0`.
    /// - The transferred amount was `500`, but this transfer call only needs `450` for the action passed in the `msg`
    ///   field, then the method must return `50`.
    /// - The transferred amount was `500`, but the action in `msg` field has expired and the transfer must be
    ///   cancelled. The method must return `500` or panic.
    ///
    /// Arguments:
    /// - `sender_id` - the account ID that initiated the transfer.
    /// - `amount` - the amount of tokens that were transferred to this account in a decimal string representation.
    /// - `msg` - a string message that was passed with this transfer call.
    ///
    /// Returns the amount of unused tokens that should be returned to sender, in a decimal string representation.
    fn ft_on_transfer(&mut self, sender_id: ValidAccountId, amount: U128, msg: String) -> U128;
}

#[near_bindgen]
impl FungibleTokenReceiver for Farm {
    fn ft_on_transfer(&mut self, sender_id: ValidAccountId, amount: U128, msg: String) -> U128 {
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
                0.into()
            }
        }
    }
}
