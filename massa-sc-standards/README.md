# massa-sc-standards

MRC20, MRC721, MRC1155, Ownable and role-based access control for Rust Massa
contracts, byte-compatible (storage layout, entry-point names, events) with
[massa-standards](https://github.com/massalabs/massa-standards).

```rust
use massa_sc_sdk::prelude::*;
use massa_sc_standards::{export_mrc20, export_ownable, mrc20};

#[export]
pub fn constructor(mut args: Args) {
    let name: String = args.must_next("name");
    let symbol: String = args.must_next("symbol");
    let decimals: u8 = args.must_next("decimals");
    let supply: U256 = args.must_next("totalSupply");
    mrc20::constructor(&name, &symbol, decimals, supply);
}
export_mrc20!();
export_ownable!();
```

See the [repository README](https://github.com/massalabs/massa-rust-sc-sdk)
and the crate documentation.
