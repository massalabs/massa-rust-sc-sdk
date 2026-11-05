//! Rust counterpart of `as/assembly/token.ts` (MRC20 mintable, burnable, ownable).
#![no_std]
use massa_sc_sdk::prelude::*;
use massa_sc_standards::{export_mrc20, export_mrc20_burnable, export_mrc20_mintable, export_ownable, mrc20};

#[export]
pub fn constructor(mut a: Args) {
    let name: String = a.must_next("name");
    let symbol: String = a.must_next("symbol");
    let decimals: u8 = a.must_next("decimals");
    let supply: U256 = a.must_next("totalSupply");
    mrc20::constructor(&name, &symbol, decimals, supply);
}

export_mrc20!();
export_mrc20_mintable!();
export_mrc20_burnable!();
export_ownable!();
