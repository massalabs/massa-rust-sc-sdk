//! Client for an MRC20 token deployed at some address.

use massa_sc_sdk::prelude::*;

use super::external::get_balance_entry_cost;
use super::internals::read_u256;

/// Typed calls to a remote MRC20 token.
///
/// ```ignore
/// let token = Mrc20::new(token_address);
/// let coins = token.balance_entry_cost(&recipient); // storage the caller must fund
/// token.transfer(&recipient, U256::from(10u64), coins);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mrc20 {
    pub address: Address,
}

impl Mrc20 {
    pub const fn new(address: Address) -> Self {
        Self { address }
    }

    fn call(&self, function: &str, args: Args, coins: u64) -> Vec<u8> {
        contract::call(&self.address, function, args, coins)
    }

    /// Call the token `constructor` (only meaningful right after creating it).
    pub fn init(&self, name: &str, symbol: &str, decimals: u8, supply: U256, coins: u64) {
        self.call("constructor", Args::new().add(name).add(symbol).add(decimals).add(supply), coins);
    }

    pub fn version(&self) -> String {
        String::from_utf8_lossy(&self.call("version", Args::new(), 0)).into_owned()
    }

    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.call("name", Args::new(), 0)).into_owned()
    }

    pub fn symbol(&self) -> String {
        String::from_utf8_lossy(&self.call("symbol", Args::new(), 0)).into_owned()
    }

    pub fn decimals(&self) -> u8 {
        self.call("decimals", Args::new(), 0).first().copied().unwrap_or(0)
    }

    pub fn total_supply(&self) -> U256 {
        read_u256(&self.call("totalSupply", Args::new(), 0))
    }

    pub fn balance_of(&self, account: &Address) -> U256 {
        read_u256(&self.call("balanceOf", Args::new().add(account), 0))
    }

    /// Coins to send with [`Self::transfer`] / [`Self::transfer_from`] so the
    /// token can pay a new balance entry for `recipient` (0 when it exists).
    pub fn balance_entry_cost(&self, recipient: &Address) -> u64 {
        get_balance_entry_cost(&self.address, recipient)
    }

    pub fn transfer(&self, to: &Address, amount: U256, coins: u64) {
        self.call("transfer", Args::new().add(to).add(amount), coins);
    }

    pub fn allowance(&self, owner: &Address, spender: &Address) -> U256 {
        read_u256(&self.call("allowance", Args::new().add(owner).add(spender), 0))
    }

    pub fn increase_allowance(&self, spender: &Address, amount: U256, coins: u64) {
        self.call("increaseAllowance", Args::new().add(spender).add(amount), coins);
    }

    pub fn decrease_allowance(&self, spender: &Address, amount: U256) {
        self.call("decreaseAllowance", Args::new().add(spender).add(amount), 0);
    }

    pub fn transfer_from(&self, owner: &Address, recipient: &Address, amount: U256, coins: u64) {
        self.call("transferFrom", Args::new().add(owner).add(recipient).add(amount), coins);
    }

    pub fn mint(&self, to: &Address, amount: U256, coins: u64) {
        self.call("mint", Args::new().add(to).add(amount), coins);
    }

    pub fn burn(&self, amount: U256) {
        self.call("burn", Args::new().add(amount), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn client_calls() {
        mock::reset();
        let token = Mrc20::new(mock::new_sc_address());
        mock::mock_call(&token.address, "balanceOf", U256::from(42u64).to_le_bytes());
        mock::mock_call(&token.address, "transfer", Vec::new());
        let me = context::callee();
        assert_eq!(token.balance_of(&me), U256::from(42u64));
        token.transfer(&me, U256::ONE, 7);
        let calls = mock::recorded_calls();
        assert_eq!(calls[1].function, "transfer");
        assert_eq!(calls[1].coins, 7);
        assert_eq!(calls[1].args, Args::new().add(&me).add(U256::ONE).into_bytes());
        // no balance entry for `me` in the (mocked) token storage
        assert_eq!(
            token.balance_entry_cost(&me),
            super::super::external::balance_entry_cost(me.as_bytes().len())
        );
    }
}
