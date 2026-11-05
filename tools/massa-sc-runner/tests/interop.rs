//! Rust <-> AssemblyScript interoperability suite.
//!
//! Runs AssemblyScript contracts (compiled with `asc` from
//! `@massalabs/massa-as-sdk` / `@massalabs/sc-standards`) and their Rust
//! counterparts (`massa-sc-sdk` / `massa-sc-standards`) side by side in the
//! real Massa runtime, and checks that:
//!
//! * `Args` bytes are identical in both directions, for every supported type;
//! * entry points return the same bytes for the same idioms;
//! * contracts call each other transparently (generic calls, the MRC20 client
//!   of each SDK against the token of the other, coins, call stack);
//! * the same operations leave byte-identical datastores and events, so the
//!   bytecode of one implementation can replace the other on top of its data
//!   (MRC20, MRC721, MRC1155, ownership, roles, storage helpers).
//!
//! Build the fixtures first: `interop/build.sh` (needs npm and the
//! `wasm32v1-none` target). The reference is `massa-standards` main
//! (`SC_STANDARDS=path/to/massa-standards/smart-contracts interop/build.sh`);
//! the published `@massalabs/sc-standards` 1.3.0 still stores exhausted
//! balances as 32 zero bytes where main and Rust delete the entry (both read
//! zero), set `INTEROP_LEGACY_STANDARDS=1` to tolerate that difference.

use std::collections::BTreeMap;
use std::path::PathBuf;

use massa_sc_runner::{CallOptions, Runner, DEFAULT_CALLER};
use massa_sc_sdk::{Address, Args, ArgsDecode, ArgsEncode, U256};

const USER: &str = "AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq";
const OTHER: &str = "AU1qzBYHnMW6WwiLwRxSJnnbSmbidbMdpFGVEWUVqRSnnGJCcUPZ";
/// Enough coins to pay any storage the standards create in one call.
const COINS: u64 = 100_000_000;

fn fixture(name: &str) -> Vec<u8> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../interop/build").join(format!("{name}.wasm"));
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!("{}: {e}\nbuild the interoperability fixtures first: interop/build.sh", path.display())
    })
}

fn owner() -> Address {
    Address::from(DEFAULT_CALLER)
}

fn user() -> Address {
    Address::from(USER)
}

/// Serialize an `Args` to pass it as a call parameter.
fn bytes(args: &Args) -> Vec<u8> {
    args.as_bytes().to_vec()
}

fn u256(v: u64) -> U256 {
    U256::from(v)
}

/// Keys whose values differ between two datastores, ignoring (when
/// `INTEROP_LEGACY_STANDARDS` is set) entries that one side deleted and the
/// other left as an all-zero u256.
fn datastore_diff(a: &Datastore, b: &Datastore) -> Vec<String> {
    let legacy = std::env::var_os("INTEROP_LEGACY_STANDARDS").is_some();
    let zero = |v: Option<&Vec<u8>>| v.is_none_or(|v| v.len() == 32 && v.iter().all(|b| *b == 0));
    a.keys()
        .chain(b.keys())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .filter(|k| !(legacy && zero(a.get(*k)) && zero(b.get(*k))))
        .map(|k| format!("{:?}: as={:?} rs={:?}", String::from_utf8_lossy(k), a.get(k), b.get(k)))
        .collect()
}

fn as_u256(ret: &[u8]) -> U256 {
    U256::decode_bytes(ret).expect("u256 return")
}

fn as_str(ret: &[u8]) -> String {
    String::from_utf8(ret.to_vec()).expect("utf-8 return")
}

/// `call` that must succeed.
fn ok(r: Result<massa_sc_runner::Outcome, String>, what: &str) -> massa_sc_runner::Outcome {
    match r {
        Ok(o) => o,
        Err(e) => panic!("{what} failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
struct Pair {
    key: String,
    value: u64,
}

impl ArgsEncode for Pair {
    fn encode_into(&self, out: &mut Vec<u8>) {
        self.key.encode_into(out);
        self.value.encode_into(out);
    }
}

impl ArgsDecode for Pair {
    fn decode(args: &mut Args) -> massa_sc_sdk::args::Result<Self> {
        Ok(Pair { key: args.next()?, value: args.next()? })
    }
}

/// The value set `golden()` returns in both fixtures.
fn golden_args() -> Args {
    Args::new()
        .add(true)
        .add(200u8)
        .add(65000u16)
        .add(-2i16)
        .add(0xdeadbeefu32)
        .add(-123456i32)
        .add(u64::MAX)
        .add(-1i64)
        .add(1.5f32)
        .add(-2.25f64)
        .add(0x99aabbccddeeff00_1122334455667788u128)
        .add(-5i128)
        .add(U256::from_words(0xffffffffffffffff_0000000000000003, 0x0000000000000002_0000000000000001))
        .add("héllo wörld 😀")
        .add(vec![1u8, 2, 3])
        .add(vec!["a".to_string(), "bc".to_string(), String::new()])
        .add(vec![1u64, 2, 3])
        .add(vec![u256(10), U256::MAX])
        .add(Pair { key: "p".into(), value: 7 })
        .add(vec![Pair { key: "x".into(), value: 1 }, Pair { key: "y".into(), value: 2 }])
        .add(user())
}

#[test]
fn args_bytes_are_identical_in_both_directions() {
    let mut r = Runner::new();
    let as_echo = r.deploy_new(&fixture("as_echo"), &[], 0).unwrap();
    let rs_echo = r.deploy_new(&fixture("rs_echo"), &[], 0).unwrap();
    let golden = bytes(&golden_args());

    // Both encoders produce the same bytes for the same values...
    let as_golden = ok(r.call(&as_echo, "golden", []), "as golden").ret;
    let rs_golden = ok(r.call(&rs_echo, "golden", []), "rs golden").ret;
    assert_eq!(as_golden, golden, "AssemblyScript Args differ from the Rust Args");
    assert_eq!(rs_golden, golden, "Rust contract Args differ from the native Rust Args");

    // ...and each decoder reads the other's bytes back, field by field.
    for (echo, name) in [(&as_echo, "AssemblyScript"), (&rs_echo, "Rust")] {
        let out = ok(r.call(echo, "roundtrip", &golden), &format!("{name} roundtrip")).ret;
        assert_eq!(out, golden, "{name} decode+encode changed the bytes");
    }

    // The Rust decoder reads AssemblyScript-produced values.
    let mut a = Args::from(as_golden);
    assert!(a.next_bool().unwrap());
    assert_eq!(a.next_u8().unwrap(), 200);
    assert_eq!(a.next_u16().unwrap(), 65000);
    assert_eq!(a.next_i16().unwrap(), -2);
    assert_eq!(a.next_u32().unwrap(), 0xdeadbeef);
    assert_eq!(a.next_i32().unwrap(), -123456);
    assert_eq!(a.next_u64().unwrap(), u64::MAX);
    assert_eq!(a.next_i64().unwrap(), -1);
    assert_eq!(a.next_f32().unwrap(), 1.5);
    assert_eq!(a.next_f64().unwrap(), -2.25);
    assert_eq!(a.next_u128().unwrap(), 0x99aabbccddeeff00_1122334455667788u128);
    assert_eq!(a.next_i128().unwrap(), -5);
    assert_eq!(a.next_u256().unwrap().to_le_bytes()[0], 1);
    assert_eq!(a.next_string().unwrap(), "héllo wörld 😀");
    assert_eq!(a.next_bytes().unwrap(), vec![1, 2, 3]);
    assert_eq!(a.next_string_array().unwrap(), vec!["a", "bc", ""]);
    assert_eq!(a.next_array::<u64>().unwrap(), vec![1, 2, 3]);
    assert_eq!(a.next_array::<U256>().unwrap(), vec![u256(10), U256::MAX]);
    assert_eq!(a.next::<Pair>().unwrap(), Pair { key: "p".into(), value: 7 });
    assert_eq!(a.next::<Vec<Pair>>().unwrap().len(), 2);
    assert_eq!(a.next_address().unwrap(), user());
    assert!(a.is_exhausted());

    // A wrong type on the AssemblyScript side fails like an AssemblyScript caller would see it.
    let err = r.call(&as_echo, "roundtrip", [1u8, 2]).unwrap_err();
    assert!(err.contains("u16") || err.contains("Result"), "unexpected error: {err}");
}

#[test]
fn return_value_idioms_match() {
    let mut r = Runner::new();
    let as_echo = r.deploy_new(&fixture("as_echo"), &[], 0).unwrap();
    let rs_echo = r.deploy_new(&fixture("rs_echo"), &[], 0).unwrap();
    let expected: Vec<(&str, Vec<u8>)> = vec![
        ("retString", "héllo".as_bytes().to_vec()),
        ("retU64", 42u64.to_le_bytes().to_vec()),
        ("retBool", vec![1]),
        ("retU256", u256(7).to_le_bytes().to_vec()),
        ("retArgs", bytes(&Args::new().add("a").add(1u32))),
        ("retVoid", vec![]),
        ("retEmpty", vec![]),
        ("noParam", b"ok".to_vec()),
    ];
    for (function, want) in expected {
        let as_ret = ok(r.call(&as_echo, function, []), function).ret;
        let rs_ret = ok(r.call(&rs_echo, function, []), function).ret;
        assert_eq!(as_ret, want, "AssemblyScript {function}");
        assert_eq!(rs_ret, want, "Rust {function}");
    }
}

// ---------------------------------------------------------------------------
// Cross-contract calls: every (caller SDK, token SDK) combination.
// ---------------------------------------------------------------------------

fn token_constructor_args() -> Vec<u8> {
    bytes(&Args::new().add("Interop Token").add("ITK").add(9u8).add(u256(1_000)))
}

fn cross_calls(caller_fixture: &str, token_fixture: &str) {
    let mut r = Runner::new();
    let token = r.deploy_new(&fixture(token_fixture), &token_constructor_args(), COINS).unwrap();
    let caller = r.deploy_new(&fixture(caller_fixture), &[], 0).unwrap();
    r.set_balance(&caller, 10 * COINS);
    let caller_addr = Address::from(caller.as_str());
    let token_addr = Address::from(token.as_str());
    let what = format!("{caller_fixture} -> {token_fixture}");

    // The owner gives the caller contract 100 tokens.
    ok(
        r.call_from(
            DEFAULT_CALLER,
            &token,
            "transfer",
            bytes(&Args::new().add(&caller_addr).add(u256(100))),
            COINS,
        ),
        &what,
    );

    // Metadata through the caller's MRC20 client.
    let info = ok(r.call(&caller, "tokenInfo", bytes(&Args::new().add(&token_addr))), &what).ret;
    let mut info = Args::from(info);
    assert_eq!(info.next_string().unwrap(), "Interop Token", "{what}: name");
    assert_eq!(info.next_string().unwrap(), "ITK", "{what}: symbol");
    assert_eq!(info.next_u8().unwrap(), 9, "{what}: decimals");
    assert_eq!(info.next_u256().unwrap(), u256(1_000), "{what}: totalSupply");

    let balance = |r: &mut Runner, who: &Address| {
        as_u256(
            &ok(r.call(&caller, "tokenBalance", bytes(&Args::new().add(&token_addr).add(who))), &what).ret,
        )
    };
    assert_eq!(balance(&mut r, &caller_addr), u256(100), "{what}: balance after transfer");

    // The caller contract transfers 40 of its tokens to USER, paying the
    // storage of USER's new balance entry (read from the token's datastore).
    let entry_cost = |r: &mut Runner, who: &Address| {
        u64::decode_bytes(
            &ok(r.call(&caller, "tokenEntryCost", bytes(&Args::new().add(&token_addr).add(who))), &what).ret,
        )
        .unwrap()
    };
    let cost = entry_cost(&mut r, &user());
    assert_eq!(cost, (4 + 7 + USER.len() + 32) as u64 * 100_000, "{what}: balance entry cost");
    assert_eq!(entry_cost(&mut r, &caller_addr), 0, "{what}: existing entry costs nothing");
    let out = ok(
        r.call(
            &caller,
            "tokenTransfer",
            bytes(&Args::new().add(&token_addr).add(user()).add(u256(40)).add(cost)),
        ),
        &what,
    );
    assert!(
        out.events.iter().any(|e| e == "TRANSFER SUCCESS"),
        "{what}: token event missing: {:?}",
        out.events
    );
    assert!(out.events.iter().any(|e| e.contains("transferred 40 to")), "{what}: caller event missing");
    assert_eq!(balance(&mut r, &caller_addr), u256(60), "{what}");
    assert_eq!(balance(&mut r, &user()), u256(40), "{what}");

    // Allowance flow: the owner approves the caller contract, which pulls.
    ok(
        r.call_from(
            DEFAULT_CALLER,
            &token,
            "increaseAllowance",
            bytes(&Args::new().add(&caller_addr).add(u256(50))),
            COINS,
        ),
        &what,
    );
    let allowance = |r: &mut Runner| {
        as_u256(
            &ok(
                r.call(
                    &caller,
                    "tokenAllowance",
                    bytes(&Args::new().add(&token_addr).add(owner()).add(&caller_addr)),
                ),
                &what,
            )
            .ret,
        )
    };
    assert_eq!(allowance(&mut r), u256(50), "{what}");
    ok(
        r.call(
            &caller,
            "tokenPull",
            bytes(&Args::new().add(&token_addr).add(owner()).add(u256(50)).add(0u64)),
        ),
        &what,
    );
    assert_eq!(allowance(&mut r), u256(0), "{what}");
    assert_eq!(balance(&mut r, &caller_addr), u256(110), "{what}");

    // Generic forwarding, function existence.
    let ret = ok(
        r.call(
            &caller,
            "forward",
            bytes(
                &Args::new().add(&token_addr).add("balanceOf").add(bytes(&Args::new().add(user()))).add(0u64),
            ),
        ),
        &what,
    )
    .ret;
    assert_eq!(as_u256(&ret), u256(40), "{what}: forwarded balanceOf");
    let exists = |r: &mut Runner, f: &str| {
        ok(r.call(&caller, "exists", bytes(&Args::new().add(&token_addr).add(f))), &what).ret == [1]
    };
    assert!(exists(&mut r, "transfer"), "{what}");
    assert!(!exists(&mut r, "doesNotExist"), "{what}");

    // A failing callee aborts the caller with the callee's message.
    let err = r
        .call(
            &caller,
            "forward",
            bytes(
                &Args::new()
                    .add(&token_addr)
                    .add("transfer")
                    .add(bytes(&Args::new().add(user()).add(u256(1_000_000))))
                    .add(0u64),
            ),
        )
        .unwrap_err();
    assert!(err.contains("insufficient funds"), "{what}: {err}");
}

#[test]
fn rust_caller_uses_assemblyscript_token() {
    cross_calls("rs_caller", "as_token");
}

#[test]
fn assemblyscript_caller_uses_rust_token() {
    cross_calls("as_caller", "rs_token");
}

#[test]
fn rust_caller_uses_rust_token() {
    cross_calls("rs_caller", "rs_token");
}

#[test]
fn assemblyscript_caller_uses_assemblyscript_token() {
    cross_calls("as_caller", "as_token");
}

#[test]
fn call_stack_and_coins_cross_sdk() {
    let mut r = Runner::new();
    let as_caller = r.deploy_new(&fixture("as_caller"), &[], 0).unwrap();
    let rs_caller = r.deploy_new(&fixture("rs_caller"), &[], 0).unwrap();
    r.set_balance(&as_caller, COINS);
    r.set_balance(&rs_caller, COINS);
    for (from, to, label) in [(&as_caller, &rs_caller, "as -> rs"), (&rs_caller, &as_caller, "rs -> as")] {
        let to_addr = Address::from(to.as_str());
        // ping: return value and caller address seen by the callee
        let out = ok(
            r.call(
                from,
                "forward",
                bytes(&Args::new().add(&to_addr).add("ping").add(bytes(&Args::new().add("x"))).add(0u64)),
            ),
            label,
        );
        assert_eq!(as_str(&out.ret), "pong x", "{label}");
        assert!(
            out.events.iter().any(|e| e.contains(&format!("pinged by {from}"))),
            "{label}: {:?}",
            out.events
        );
        // whoCalls: the callee sees the forwarding contract, not the originator
        let out = ok(
            r.call(
                from,
                "forward",
                bytes(&Args::new().add(&to_addr).add("whoCalls").add(Vec::<u8>::new()).add(0u64)),
            ),
            label,
        );
        assert_eq!(as_str(&out.ret), *from, "{label}");
        // paid: coins travel with the call
        let before = r.balance(to);
        let out = ok(
            r.call(
                from,
                "forward",
                bytes(&Args::new().add(&to_addr).add("paid").add(Vec::<u8>::new()).add(1234u64)),
            ),
            label,
        );
        assert_eq!(u64::decode_bytes(&out.ret).unwrap(), 1234, "{label}");
        assert_eq!(r.balance(to), before + 1234, "{label}");
        assert_eq!(r.datastore(to).get(&b"paid"[..]).map(|v| v.as_slice()), Some(&b"1234"[..]), "{label}");
        // local call: the callee's code runs in the caller's context
        let out = ok(
            r.call(
                from,
                "forwardLocal",
                bytes(&Args::new().add(&to_addr).add("whoCalls").add(Vec::<u8>::new())),
            ),
            label,
        );
        assert_eq!(as_str(&out.ret), DEFAULT_CALLER, "{label}: local call keeps the caller's stack");
    }
}

// ---------------------------------------------------------------------------
// Standards: same operations, same datastore, same events, in any order of
// implementations.
// ---------------------------------------------------------------------------

type Step = (&'static str, &'static str, Vec<u8>, u64); // (caller, function, args, coins)
type Datastore = BTreeMap<Vec<u8>, Vec<u8>>;

/// Run `steps` on a fresh deployment of `fixture_name` and return
/// (datastore, per-step events, per-step return values).
fn run_steps(fixture_name: &str, ctor: &[u8], steps: &[Step]) -> (Datastore, Vec<Vec<String>>, Vec<Vec<u8>>) {
    let mut r = Runner::new();
    let address = "AS12touvnNJy1gJBz1N77iwz79QZFJxnQuXkvUa8VfLws2mpJub6z";
    r.deploy(address, &fixture(fixture_name), ctor, COINS).unwrap();
    let mut events = Vec::new();
    let mut rets = Vec::new();
    for (caller, function, args, coins) in steps {
        let out =
            ok(r.call_from(caller, address, function, args, *coins), &format!("{fixture_name}.{function}"));
        events.push(out.events);
        rets.push(out.ret);
    }
    (r.datastore(address), events, rets)
}

/// Compare the two implementations over `steps`, then check that each one
/// keeps working on the datastore left by the other.
fn standards_agree(
    as_fixture: &str,
    rs_fixture: &str,
    ctor: &[u8],
    steps: &[Step],
    reads: &[(&str, Vec<u8>)],
) {
    let (as_ds, as_events, as_rets) = run_steps(as_fixture, ctor, steps);
    let (rs_ds, rs_events, rs_rets) = run_steps(rs_fixture, ctor, steps);
    for (i, step) in steps.iter().enumerate() {
        assert_eq!(as_rets[i], rs_rets[i], "{}: return value of step {i} ({})", rs_fixture, step.1);
        assert_eq!(as_events[i], rs_events[i], "{}: events of step {i} ({})", rs_fixture, step.1);
    }
    let diff = datastore_diff(&as_ds, &rs_ds);
    assert!(diff.is_empty(), "{rs_fixture}: datastores differ:\n{}", diff.join("\n"));

    // Swap implementations on top of each other's state and read.
    for (first, second) in [(as_fixture, rs_fixture), (rs_fixture, as_fixture)] {
        let mut r = Runner::new();
        let address = r.deploy_new(&fixture(first), ctor, COINS).unwrap();
        for (caller, function, args, coins) in steps {
            ok(r.call_from(caller, &address, function, args, *coins), function);
        }
        r.set_bytecode(&address, &fixture(second)).unwrap();
        for (function, args) in reads {
            let want = ok(r.call(&address, function, args), function).ret;
            // The reference is the pure run of `second`.
            let mut pure = Runner::new();
            let pure_address = pure.deploy_new(&fixture(second), ctor, COINS).unwrap();
            for (caller, function, args, coins) in steps {
                ok(pure.call_from(caller, &pure_address, function, args, *coins), function);
            }
            let got = ok(pure.call(&pure_address, function, args), function).ret;
            assert_eq!(want, got, "{first} then {second}: {function} differs from a pure {second} run");
        }
    }
}

#[test]
fn mrc20_implementations_are_interchangeable() {
    let caller = Address::from(DEFAULT_CALLER);
    let steps: Vec<Step> = vec![
        (DEFAULT_CALLER, "transfer", bytes(&Args::new().add(user()).add(u256(100))), COINS),
        (DEFAULT_CALLER, "increaseAllowance", bytes(&Args::new().add(user()).add(u256(50))), COINS),
        (
            USER,
            "transferFrom",
            bytes(&Args::new().add(&caller).add(Address::from(OTHER)).add(u256(20))),
            COINS,
        ),
        (DEFAULT_CALLER, "decreaseAllowance", bytes(&Args::new().add(user()).add(u256(10))), COINS),
        (DEFAULT_CALLER, "mint", bytes(&Args::new().add(user()).add(u256(5))), COINS),
        (USER, "burn", bytes(&Args::new().add(u256(3))), COINS),
        (USER, "transfer", bytes(&Args::new().add(&caller).add(u256(1))), COINS),
        (DEFAULT_CALLER, "setOwner", bytes(&Args::new().add(user())), COINS),
    ];
    let reads = vec![
        ("name", vec![]),
        ("symbol", vec![]),
        ("decimals", vec![]),
        ("totalSupply", vec![]),
        ("balanceOf", bytes(&Args::new().add(user()))),
        ("balanceOf", bytes(&Args::new().add(&caller))),
        ("balanceOf", bytes(&Args::new().add(Address::from(OTHER)))),
        ("allowance", bytes(&Args::new().add(&caller).add(user()))),
        ("ownerAddress", vec![]),
        ("isOwner", bytes(&Args::new().add(user()))),
    ];
    standards_agree("as_token", "rs_token", &token_constructor_args(), &steps, &reads);
}

#[test]
fn mrc721_implementations_are_interchangeable() {
    let ctor = bytes(&Args::new().add("Interop NFT").add("INFT").add("ipfs://base/"));
    let steps: Vec<Step> = vec![
        (DEFAULT_CALLER, "mint", bytes(&Args::new().add(owner()).add(u256(1))), COINS),
        (DEFAULT_CALLER, "mint", bytes(&Args::new().add(owner()).add(u256(2))), COINS),
        (DEFAULT_CALLER, "mint", bytes(&Args::new().add(user()).add(u256(3))), COINS),
        (DEFAULT_CALLER, "mint", bytes(&Args::new().add(user()).add(u256(4))), COINS),
        (DEFAULT_CALLER, "approve", bytes(&Args::new().add(user()).add(u256(1))), COINS),
        (DEFAULT_CALLER, "setApprovalForAll", bytes(&Args::new().add(Address::from(OTHER)).add(true)), COINS),
        (USER, "transferFrom", bytes(&Args::new().add(owner()).add(user()).add(u256(1))), COINS),
        (
            OTHER,
            "transferFrom",
            bytes(&Args::new().add(owner()).add(Address::from(OTHER)).add(u256(2))),
            COINS,
        ),
        (USER, "burn", bytes(&Args::new().add(u256(4))), COINS),
        (DEFAULT_CALLER, "setTokenURI", bytes(&Args::new().add(u256(1)).add("one.json")), COINS),
    ];
    let reads = vec![
        ("name", vec![]),
        ("symbol", vec![]),
        ("totalSupply", vec![]),
        ("ownerOf", bytes(&Args::new().add(u256(1)))),
        ("ownerOf", bytes(&Args::new().add(u256(2)))),
        ("ownerOf", bytes(&Args::new().add(u256(3)))),
        ("ownerOf", bytes(&Args::new().add(u256(4)))),
        ("balanceOf", bytes(&Args::new().add(owner()))),
        ("balanceOf", bytes(&Args::new().add(user()))),
        ("getApproved", bytes(&Args::new().add(u256(1)))),
        ("isApprovedForAll", bytes(&Args::new().add(owner()).add(Address::from(OTHER)))),
        ("uri", bytes(&Args::new().add(u256(1)))),
        ("uri", bytes(&Args::new().add(u256(2)))),
        ("ownerAddress", vec![]),
    ];
    standards_agree("as_nft", "rs_nft", &ctor, &steps, &reads);
}

#[test]
fn mrc1155_implementations_are_interchangeable() {
    let ctor = bytes(&Args::new().add("ipfs://collection/{id}.json"));
    let empty: Vec<u8> = Vec::new();
    let steps: Vec<Step> = vec![
        (
            DEFAULT_CALLER,
            "mint",
            bytes(&Args::new().add(owner()).add(u256(7)).add(u256(100)).add(&empty)),
            COINS,
        ),
        (
            DEFAULT_CALLER,
            "mintBatch",
            bytes(
                &Args::new().add(owner()).add(vec![u256(1), u256(2)]).add(vec![u256(5), u256(6)]).add(&empty),
            ),
            COINS,
        ),
        (DEFAULT_CALLER, "setApprovalForAll", bytes(&Args::new().add(user()).add(true)), COINS),
        (
            USER,
            "safeTransferFrom",
            bytes(&Args::new().add(owner()).add(user()).add(u256(7)).add(u256(40)).add(&empty)),
            COINS,
        ),
        (
            DEFAULT_CALLER,
            "safeBatchTransferFrom",
            bytes(
                &Args::new()
                    .add(owner())
                    .add(user())
                    .add(vec![u256(1), u256(2)])
                    .add(vec![u256(2), u256(3)])
                    .add(&empty),
            ),
            COINS,
        ),
        (USER, "burn", bytes(&Args::new().add(user()).add(u256(7)).add(u256(10))), COINS),
        (USER, "burnBatch", bytes(&Args::new().add(user()).add(vec![u256(1)]).add(vec![u256(1)])), COINS),
        (DEFAULT_CALLER, "setTokenURI", bytes(&Args::new().add(u256(7)).add("7.json")), COINS),
        (DEFAULT_CALLER, "setBaseURI", bytes(&Args::new().add("https://meta/")), COINS),
        (DEFAULT_CALLER, "grantRole", bytes(&Args::new().add("minter").add(user())), COINS),
        (
            USER,
            "mint",
            bytes(&Args::new().add(Address::from(OTHER)).add(u256(9)).add(u256(1)).add(&empty)),
            COINS,
        ),
    ];
    let reads = vec![
        ("balanceOf", bytes(&Args::new().add(owner()).add(u256(7)))),
        ("balanceOf", bytes(&Args::new().add(user()).add(u256(7)))),
        (
            "balanceOfBatch",
            bytes(&Args::new().add(vec![owner(), user(), user()]).add(vec![u256(1), u256(1), u256(2)])),
        ),
        ("isApprovedForAll", bytes(&Args::new().add(owner()).add(user()))),
        ("uri", bytes(&Args::new().add(u256(7)))),
        ("uri", bytes(&Args::new().add(u256(1)))),
        ("hasRole", bytes(&Args::new().add("minter").add(user()))),
        ("members", bytes(&Args::new().add("minter"))),
        ("ownerAddress", vec![]),
    ];
    standards_agree("as_multi", "rs_multi", &ctor, &steps, &reads);
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

#[test]
fn storage_helpers_share_one_layout() {
    // Each implementation writes, each implementation reads, in every combination.
    let mut outputs = Vec::new();
    let mut datastores = Vec::new();
    for writer in ["as_storage", "rs_storage"] {
        let mut r = Runner::new();
        let address = r.deploy_new(&fixture(writer), &[], 0).unwrap();
        ok(r.call(&address, "write", []), writer);
        datastores.push((writer, r.datastore(&address)));
        for reader in ["as_storage", "rs_storage"] {
            r.set_bytecode(&address, &fixture(reader)).unwrap();
            let out = ok(r.call(&address, "read", []), &format!("{writer} -> {reader} read")).ret;
            outputs.push((format!("{writer} -> {reader}"), out));
        }
    }
    let diff = datastore_diff(&datastores[0].1, &datastores[1].1);
    assert!(diff.is_empty(), "storage layouts differ:\n{}", diff.join("\n"));
    for (label, out) in &outputs[1..] {
        assert_eq!(out, &outputs[0].1, "{label} reads differently than {}", outputs[0].0);
    }

    // And the values are the expected ones.
    let mut a = Args::from(outputs[0].1.clone());
    assert_eq!(a.next_string().unwrap(), "hello");
    assert_eq!(a.next_bytes().unwrap(), vec![1, 2, 3]);
    assert_eq!(a.next_u32().unwrap(), 5);
    assert_eq!(a.next_string().unwrap(), "v");
    assert_eq!(a.next_u64().unwrap(), 42);
    assert_eq!(a.next_u32().unwrap(), 7);
    assert!(a.next_bool().unwrap());
    assert_eq!(a.next_string().unwrap(), "v");
    assert_eq!(a.next_u32().unwrap(), 7);
    assert_eq!(a.next_u64().unwrap(), u64::MAX);
    assert!(a.next_bool().unwrap());
    assert_eq!(a.next_f64().unwrap(), 1.5);
    assert_eq!(a.next_u64().unwrap(), 9);
    assert_eq!(a.next_bytes().unwrap(), vec![9, 8]);
    assert_eq!(a.next_string().unwrap(), "apple");
    assert_eq!(a.next_u32().unwrap(), 3);
    assert!(!a.next_bool().unwrap());
    assert_eq!(a.next_u64().unwrap(), 123);
    assert_eq!(a.next_string().unwrap(), "constant");
    assert_eq!(a.next_u32().unwrap(), 5);
    assert_eq!(a.next_string().unwrap(), "seventy-seven");
    assert!(a.next_bool().unwrap());
    assert!(a.next_bool().unwrap());
    assert!(!a.next_bool().unwrap());
    assert!(a.next_bool().unwrap());
    assert!(a.is_exhausted());
}

#[test]
fn deployment_context_matches_the_deployer() {
    // `constructor` only runs with write access from the creator, in both SDKs.
    let mut r = Runner::new();
    for name in ["as_token", "rs_token"] {
        let address = r.new_address();
        r.set_bytecode(&address, &fixture(name)).unwrap();
        let err = r
            .call_with(&address, "constructor", token_constructor_args(), CallOptions::default())
            .unwrap_err();
        assert!(!err.is_empty(), "{name}: constructor must fail outside of deployment");
        let out = r.call_with(
            &address,
            "constructor",
            token_constructor_args(),
            CallOptions { deploy_context: true, coins: COINS, ..Default::default() },
        );
        assert!(out.is_ok(), "{name}: {out:?}");
        assert_eq!(as_str(&ok(r.call(&address, "ownerAddress", []), name).ret), DEFAULT_CALLER, "{name}");
    }
}
