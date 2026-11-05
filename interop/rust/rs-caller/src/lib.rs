//! Rust counterpart of `as/assembly/caller.ts`: calls into other contracts,
//! generically and through the `Mrc20` client of massa-sc-standards.
#![no_std]
use massa_sc_sdk::prelude::*;
use massa_sc_standards::mrc20::Mrc20;

#[export]
pub fn forward(mut a: Args) -> Vec<u8> {
    let target: Address = a.must_next("target");
    let function: String = a.must_next("function");
    let args: Vec<u8> = a.must_next("args");
    let coins: u64 = a.must_next("coins");
    contract::call(&target, &function, args, coins)
}

#[export(name = "forwardLocal")]
pub fn forward_local(mut a: Args) -> Vec<u8> {
    let target: Address = a.must_next("target");
    let function: String = a.must_next("function");
    let args: Vec<u8> = a.must_next("args");
    contract::local_call(&target, &function, args)
}

#[export]
pub fn exists(mut a: Args) -> bool {
    let target: Address = a.must_next("target");
    let function: String = a.must_next("function");
    contract::function_exists(&target, &function)
}

#[export(name = "tokenInfo")]
pub fn token_info(mut a: Args) -> Args {
    let token = Mrc20::new(a.must_next("token"));
    Args::new().add(token.name()).add(token.symbol()).add(token.decimals()).add(token.total_supply())
}

#[export(name = "tokenBalance")]
pub fn token_balance(mut a: Args) -> U256 {
    let token = Mrc20::new(a.must_next("token"));
    token.balance_of(&a.must_next("account"))
}

#[export(name = "tokenTransfer")]
pub fn token_transfer(mut a: Args) {
    let token = Mrc20::new(a.must_next("token"));
    let to: Address = a.must_next("to");
    let amount: U256 = a.must_next("amount");
    let coins: u64 = a.must_next("coins");
    token.transfer(&to, amount, coins);
    generate_event(&format!("rs-caller transferred {amount} to {to}"));
}

#[export(name = "tokenPull")]
pub fn token_pull(mut a: Args) {
    let token = Mrc20::new(a.must_next("token"));
    let owner: Address = a.must_next("owner");
    let amount: U256 = a.must_next("amount");
    let coins: u64 = a.must_next("coins");
    token.transfer_from(&owner, &context::callee(), amount, coins);
}

#[export(name = "tokenAllowance")]
pub fn token_allowance(mut a: Args) -> U256 {
    let token = Mrc20::new(a.must_next("token"));
    let owner: Address = a.must_next("owner");
    let spender: Address = a.must_next("spender");
    token.allowance(&owner, &spender)
}

/// Storage cost the caller must send for a fresh balance entry of `account`
/// in `token`, read directly from the token's datastore (layout compat).
#[export(name = "tokenEntryCost")]
pub fn token_entry_cost(mut a: Args) -> u64 {
    let token = Mrc20::new(a.must_next("token"));
    token.balance_entry_cost(&a.must_next("account"))
}

#[export]
pub fn ping(mut a: Args) -> String {
    let who: String = a.must_next("who");
    generate_event(&format!("rs-caller pinged by {} as {who}", context::caller()));
    format!("pong {who}")
}

#[export]
pub fn paid() -> u64 {
    let coins = context::transferred_coins();
    storage::set("paid", coins.to_string());
    coins
}

#[export(name = "whoCalls")]
pub fn who_calls() -> Address {
    context::caller()
}
