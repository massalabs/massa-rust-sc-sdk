//! `cargo massa new`: create a ready-to-build contract project.

use std::path::Path;

use anyhow::{bail, Context, Result};

pub const DEFAULT_SDK_DEP: &str = r#"{ git = "https://github.com/massalabs/massa-rust-sc-sdk.git" }"#;
pub const DEFAULT_STANDARDS_DEP: &str = r#"{ git = "https://github.com/massalabs/massa-rust-sc-sdk.git" }"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    Counter,
    Token,
    Empty,
}

impl std::str::FromStr for Template {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "counter" => Template::Counter,
            "token" => Template::Token,
            "empty" => Template::Empty,
            other => bail!("unknown template `{other}` (counter, token, empty)"),
        })
    }
}

fn cargo_toml(name: &str, template: Template, sdk: &str, standards: &str) -> String {
    let standards_dep = match template {
        Template::Token => format!("massa-sc-standards = {standards}\n"),
        _ => format!("# massa-sc-standards = {standards}\n"),
    };
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
publish = false

# Standalone project, even when created inside another cargo workspace.
[workspace]

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
massa-sc-sdk = {sdk}
{standards_dep}
# Small, gas-efficient wasm. `overflow-checks` keeps arithmetic safe in release.
[profile.release]
opt-level = "s"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
overflow-checks = true
"#
    )
}

const CARGO_CONFIG: &str = r#"# `cargo massa build` is the recommended way to build (it also optimizes and
# validates the module). `cargo wasm` is a plain cargo equivalent; `cargo test`
# runs the unit tests natively against the SDK's in-memory mock VM.
[alias]
wasm = "build --release --target wasm32v1-none"

[target.wasm32v1-none]
rustflags = ["-C", "target-feature=+bulk-memory,+sign-ext", "-C", "link-arg=-zstack-size=262144"]
"#;

const GITIGNORE: &str = "/target\n/build\n.env\nnode_modules\n";

const DEPLOY_PACKAGE_JSON: &str = r#"{
  "private": true,
  "type": "module",
  "scripts": {
    "deploy": "tsx deploy.ts"
  },
  "dependencies": {
    "@massalabs/massa-web3": "^5",
    "dotenv": "^16"
  },
  "devDependencies": {
    "@types/node": "^22",
    "tsx": "^4",
    "typescript": "^5"
  }
}
"#;

const ENV_EXAMPLE: &str = r#"# Secret key of a funded account (buildnet coins: https://discord.gg/massa, #buildnet-faucet)
PRIVATE_KEY=S1...
# Buildnet (default). Mainnet: https://mainnet.massa.net/api/v2, local sandbox: http://127.0.0.1:33035
JSON_RPC_URL=https://buildnet.massa.net/api/v2
"#;

fn deploy_script(name: &str, template: Template) -> String {
    let crate_name = name.replace('-', "_");
    let (ctor_args, after_deploy) = match template {
        Template::Token => (
            "new Args().addString('My Token').addString('MTK').addU8(9n).addU256(1_000_000n * 10n ** 9n)",
            r#"const balance = await contract.read('balanceOf', new Args().addString(provider.address));
console.log('owner balance:', U256.fromBytes(balance.value));"#,
        ),
        Template::Counter => (
            "new Args().addU64(10n)",
            r#"const op = await contract.call('increment', new Args().addU64(5n));
await op.waitSpeculativeExecution();
for (const e of await op.getSpeculativeEvents()) console.log('event:', e.data);
console.log('get():', U64.fromBytes((await contract.read('get')).value));"#,
        ),
        Template::Empty => (
            "new Args()",
            r#"const result = await contract.read('hello', new Args().addString('Massa'));
console.log('hello():', bytesToStr(result.value));"#,
        ),
    };
    format!(
        r#"// Deploy build/{crate_name}.wasm with massa-web3, call its constructor, then exercise it.
//   cargo massa build && npm install && npm run deploy
import 'dotenv/config';
import {{ readFileSync }} from 'node:fs';
import {{ Account, Args, JsonRpcProvider, Mas, SmartContract, U64, U256, bytesToStr }} from '@massalabs/massa-web3';

const account = await Account.fromEnv();
const provider = JsonRpcProvider.fromRPCUrl(process.env.JSON_RPC_URL ?? 'https://buildnet.massa.net/api/v2', account);
console.log(`deploying from ${{account.address}} (${{Mas.toString(await provider.balance(false))}} MAS)`);

const bytecode = readFileSync(new URL('../build/{crate_name}.wasm', import.meta.url));
const contract = await SmartContract.deploy(provider, new Uint8Array(bytecode), {ctor_args}, {{
  // Coins kept by the contract to pay the storage its constructor writes.
  coins: Mas.fromString('0.1'),
  waitFinalExecution: true,
}});
console.log('deployed at', contract.address);

{after_deploy}
"#
    )
}

const COUNTER_LIB: &str = r#"#![cfg_attr(target_arch = "wasm32", no_std)]

use massa_sc_sdk::prelude::*;

const COUNTER_KEY: &[u8] = b"counter";

/// Called once by the deployer right after the contract is created.
#[export]
pub fn constructor(mut args: Args) {
    assert!(context::is_deploying_contract(), "constructor can only be called at deployment");
    let initial: u64 = args.next().unwrap_or(0);
    storage::set_value(COUNTER_KEY, &initial);
    generate_event(&format!("Counter initialized to {initial} by {}", context::caller()));
}

/// Add `by` (default 1) to the counter and return the new value.
#[export]
pub fn increment(mut args: Args) -> u64 {
    let by: u64 = args.next().unwrap_or(1);
    let value = current().checked_add(by).expect("counter overflow");
    storage::set_value(COUNTER_KEY, &value);
    generate_event(&format!("Counter is now {value}"));
    value
}

/// Read the counter (also works through a read-only call).
#[export]
pub fn get() -> u64 {
    current()
}

fn current() -> u64 {
    storage::try_get_value(COUNTER_KEY).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn increments() {
        mock::reset();
        mock::set_deploy_context(None);
        constructor(Args::new().add(10u64));
        mock::set_admin_context(false);
        assert_eq!(increment(Args::new()), 11);
        assert_eq!(increment(Args::new().add(5u64)), 16);
        assert_eq!(get(), 16);
        assert_eq!(mock::events().last().map(String::as_str), Some("Counter is now 16"));
    }

    #[test]
    #[should_panic(expected = "constructor can only be called at deployment")]
    fn constructor_is_protected() {
        mock::reset();
        constructor(Args::new());
    }
}
"#;

const TOKEN_LIB: &str = r#"#![cfg_attr(target_arch = "wasm32", no_std)]

use massa_sc_sdk::prelude::*;
use massa_sc_standards::{export_mrc20, export_mrc20_burnable, export_mrc20_mintable, export_ownable, mrc20};

/// Deploy with: name (string), symbol (string), decimals (u8), total supply (u256).
#[export]
pub fn constructor(mut args: Args) {
    let name: String = args.must_next("name");
    let symbol: String = args.must_next("symbol");
    let decimals: u8 = args.must_next("decimals");
    let total_supply: U256 = args.must_next("totalSupply");
    mrc20::constructor(&name, &symbol, decimals, total_supply);
}

// Standard entry points: name, symbol, decimals, totalSupply, balanceOf,
// transfer, allowance, increaseAllowance, decreaseAllowance, transferFrom,
// mint (owner), burn, burnFrom, setOwner, ownerAddress, isOwner.
export_mrc20!();
export_mrc20_mintable!();
export_mrc20_burnable!();
export_ownable!();

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn deploy_and_transfer() {
        mock::reset();
        mock::set_deploy_context(None);
        constructor(Args::new().add("My Token").add("MTK").add(9u8).add(U256::from(1_000u64)));
        mock::set_admin_context(false);
        let me = context::caller();
        let bob = mock::new_user_address();
        mrc20::entrypoints::transfer(Args::new().add(&bob).add(U256::from(10u64)));
        assert_eq!(mrc20::internals::balance(&bob), U256::from(10u64));
        assert_eq!(mrc20::internals::balance(&me), U256::from(990u64));
    }
}
"#;

const EMPTY_LIB: &str = r#"#![cfg_attr(target_arch = "wasm32", no_std)]

use massa_sc_sdk::prelude::*;

#[export]
pub fn constructor(_args: Args) {
    assert!(context::is_deploying_contract(), "constructor can only be called at deployment");
}

#[export]
pub fn hello(mut args: Args) -> String {
    let name: String = args.next().unwrap_or_else(|_| "world".into());
    let message = format!("Hello, {name}!");
    generate_event(&message);
    message
}
"#;

fn readme(name: &str) -> String {
    format!(
        r#"# {name}

A [Massa](https://massa.net) smart contract written in Rust with
[massa-sc-sdk](https://github.com/massalabs/massa-rust-sc-sdk).

```sh
cargo test                 # unit tests, natively, against the in-memory mock VM
cargo massa build          # build/<crate>.wasm, optimized and validated for the node

cd scripts && cp .env.example .env   # PRIVATE_KEY of a funded account
npm install && npm run deploy        # deploy with massa-web3 and call the contract
```

The contract follows the same conventions as AssemblyScript ones (single
`Args` parameter, `constructor` called by the deployer, same storage and event
formats), so any Massa tooling and any other contract interoperate with it.
See the [examples repository](https://github.com/massalabs/massa-rust-sc-examples)
for more.
"#
    )
}

pub fn run(name: &str, template: Template, sdk: Option<&str>, standards: Option<&str>) -> Result<()> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        bail!("invalid project name `{name}`");
    }
    let dir = Path::new(name);
    if dir.exists() {
        bail!("`{}` already exists", dir.display());
    }
    let sdk = sdk.unwrap_or(DEFAULT_SDK_DEP);
    let standards = standards.unwrap_or(DEFAULT_STANDARDS_DEP);
    let lib = match template {
        Template::Counter => COUNTER_LIB,
        Template::Token => TOKEN_LIB,
        Template::Empty => EMPTY_LIB,
    };
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::create_dir_all(dir.join(".cargo"))?;
    std::fs::create_dir_all(dir.join("scripts"))?;
    let write = |p: &str, c: &str| -> Result<()> {
        std::fs::write(dir.join(p), c).with_context(|| format!("writing {p}"))
    };
    write("Cargo.toml", &cargo_toml(name, template, sdk, standards))?;
    write(".cargo/config.toml", CARGO_CONFIG)?;
    write(".gitignore", GITIGNORE)?;
    write("src/lib.rs", lib)?;
    write("README.md", &readme(name))?;
    write("scripts/package.json", DEPLOY_PACKAGE_JSON)?;
    write("scripts/deploy.ts", &deploy_script(name, template))?;
    write("scripts/.env.example", ENV_EXAMPLE)?;
    println!("created `{name}` ({template:?} template)");
    println!("  cd {name} && cargo test && cargo massa build");
    println!("  cd scripts && cp .env.example .env && npm install && npm run deploy");
    Ok(())
}
