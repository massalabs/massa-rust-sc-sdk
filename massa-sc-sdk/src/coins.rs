//! Native coin (MAS) balances and transfers. All amounts are in nanoMAS
//! (`1 MAS = 10^9 nanoMAS`, see [`crate::constants::ONE_MAS`]).

use crate::{context, env, Address};

pub use crate::constants::ONE_MAS;

/// Balance of the current contract.
#[inline]
pub fn balance() -> u64 {
    env::balance()
}

/// Balance of `address` (0 when the address does not exist).
#[inline]
pub fn balance_of(address: &Address) -> u64 {
    env::balance_of(address.as_str())
}

/// Send `amount` nanoMAS from the current contract to `to`.
/// Aborts when the balance is insufficient or the address is invalid.
#[inline]
pub fn transfer_coins(to: &Address, amount: u64) {
    env::transfer_coins(to.as_str(), amount)
}

/// Send `amount` nanoMAS from `from` to `to`. Only possible when the current
/// execution has write access to `from` (e.g. an address it just created).
#[inline]
pub fn transfer_coins_of(from: &Address, to: &Address, amount: u64) {
    env::transfer_coins_of(from.as_str(), to.as_str(), amount)
}

/// Refund the caller with the coins it sent that were not needed.
///
/// Call it at the end of a payable function, with the contract balance
/// measured at its very beginning (`let initial = coins::balance();`):
/// whatever the execution spent (e.g. storage costs of new datastore
/// entries) is deducted from the coins the caller sent, `caller_debit` is kept
/// by the contract, and the rest is sent back to `refund_to` (default: the
/// caller).
///
/// Aborts with `INSUFFICIENT_COINS_TRANSFERRED` when the caller sent less than
/// `caller_debit`, and with `SPENT_MORE_COINS_THAN_SENT` when the execution
/// consumed more than what the caller sent.
///
/// Must be the single, outermost reconciliation of an execution: nesting two
/// `transfer_remaining` in one call misaccounts (both read the same
/// [`context::transferred_coins`]).
pub fn transfer_remaining(initial_balance: u64, caller_debit: u64, refund_to: Option<&Address>) {
    let final_balance = balance();
    let sent = context::transferred_coins();
    assert!(sent >= caller_debit, "INSUFFICIENT_COINS_TRANSFERRED");

    let recipient;
    let refund_to = match refund_to {
        Some(a) => a,
        None => {
            recipient = context::caller();
            &recipient
        }
    };

    if initial_balance >= final_balance {
        // The execution spent coins (e.g. storage).
        let spent = initial_balance - final_balance;
        let to_spend = spent.checked_add(caller_debit).expect("overflow");
        assert!(to_spend <= sent, "SPENT_MORE_COINS_THAN_SENT");
        if to_spend < sent {
            transfer_coins(refund_to, sent - to_spend);
        }
    } else {
        // The execution released coins (e.g. deleted storage).
        let received = final_balance - initial_balance;
        let to_send = (sent - caller_debit).checked_add(received).expect("overflow");
        if to_send > 0 {
            transfer_coins(refund_to, to_send);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn refund_logic() {
        mock::reset();
        let caller = context::caller();
        let callee = context::callee();
        mock::set_balance(&callee, 1_000);
        mock::set_balance(&caller, 0);
        // caller sends 300, contract spends 100 and keeps a 50 debit: refund 150
        mock::set_transferred_coins(300);
        let initial = balance();
        mock::set_balance(&callee, 900);
        transfer_remaining(initial, 50, None);
        assert_eq!(balance_of(&caller), 150);
        assert_eq!(balance(), 750);
    }

    #[test]
    #[should_panic(expected = "SPENT_MORE_COINS_THAN_SENT")]
    fn refund_detects_overspending() {
        mock::reset();
        let callee = context::callee();
        mock::set_balance(&callee, 1_000);
        mock::set_transferred_coins(10);
        let initial = balance();
        mock::set_balance(&callee, 900);
        transfer_remaining(initial, 0, None);
    }
}
