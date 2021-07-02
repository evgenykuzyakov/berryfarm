use crate::*;
use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::core_impl::ext_fungible_token_receiver;
use near_sdk::{assert_one_yocto, log, Gas, PromiseOrValue, PromiseResult};

const GAS_FOR_RESOLVE_TRANSFER: Gas = 5_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL: Gas = 25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER;

const NO_DEPOSIT: Balance = 0;

#[near_bindgen]
impl FungibleTokenCore for Farm {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: ValidAccountId, amount: U128, memo: Option<String>) {
        assert_one_yocto();
        let amount = amount.into();
        let sender_id = self.withdraw_from_sender(receiver_id.as_ref(), amount);
        self.deposit_to_account(receiver_id.as_ref(), amount);
        log!(
            "Transfer 🥒{} from {} to {}",
            amount,
            sender_id,
            receiver_id
        );
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: ValidAccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_one_yocto();
        let amount = amount.into();
        let sender_id = self.withdraw_from_sender(receiver_id.as_ref(), amount);
        self.deposit_to_account(receiver_id.as_ref(), amount);
        log!(
            "Transfer 🥒{} from {} to {}",
            amount,
            sender_id,
            receiver_id
        );
        if let Some(memo) = memo {
            log!("Memo: {}", memo);
        }
        // Initiating receiver's call and the callback
        ext_fungible_token_receiver::ft_on_transfer(
            sender_id.clone(),
            amount.into(),
            msg,
            receiver_id.as_ref(),
            NO_DEPOSIT,
            env::prepaid_gas() - GAS_FOR_FT_TRANSFER_CALL,
        )
        .then(ext_ft_self::ft_resolve_transfer(
            sender_id,
            receiver_id.into(),
            amount.into(),
            &env::current_account_id(),
            NO_DEPOSIT,
            GAS_FOR_RESOLVE_TRANSFER,
        ))
        .into()
    }

    fn ft_total_supply(&self) -> U128 {
        self.total_cucumber_balance.into()
    }

    fn ft_balance_of(&self, account_id: ValidAccountId) -> U128 {
        self.get_internal_account(account_id.as_ref())
            .1
            .map(|account| account.cucumber_balance)
            .unwrap_or(0)
            .into()
    }
}

#[ext_contract(ext_ft_self)]
trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[near_bindgen]
impl FungibleTokenResolver for Farm {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let amount: Balance = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let (receiver_account_id_hash, mut receiver_account) =
                self.get_mut_account(&receiver_id);

            let receiver_balance = receiver_account.cucumber_balance;
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                receiver_account.cucumber_balance -= refund_amount;
                self.save_account(&receiver_account_id_hash, &receiver_account);

                let (sender_account_id_hash, mut sender_account) = self.get_mut_account(&sender_id);
                sender_account.cucumber_balance += refund_amount;
                self.save_account(&sender_account_id_hash, &sender_account);
                log!(
                    "Refund 🥒{} from {} to {}",
                    refund_amount,
                    receiver_id,
                    sender_id
                );
                return (amount - refund_amount).into();
            }
        }
        amount.into()
    }
}
