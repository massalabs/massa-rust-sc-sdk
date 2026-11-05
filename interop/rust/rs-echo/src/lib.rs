//! Rust counterpart of `as/assembly/echo.ts`: same functions, same values.
#![no_std]
use massa_sc_sdk::prelude::*;

#[derive(Serializable)]
pub struct Pair {
    pub key: String,
    pub value: u64,
}

#[export]
pub fn roundtrip(mut a: Args) -> Args {
    let mut out = Args::new();
    out.push(a.must_next::<bool>("bool"));
    out.push(a.must_next::<u8>("u8"));
    out.push(a.must_next::<u16>("u16"));
    out.push(a.must_next::<i16>("i16"));
    out.push(a.must_next::<u32>("u32"));
    out.push(a.must_next::<i32>("i32"));
    out.push(a.must_next::<u64>("u64"));
    out.push(a.must_next::<i64>("i64"));
    out.push(a.must_next::<f32>("f32"));
    out.push(a.must_next::<f64>("f64"));
    out.push(a.must_next::<u128>("u128"));
    out.push(a.must_next::<i128>("i128"));
    out.push(a.must_next::<U256>("u256"));
    out.push(a.must_next::<String>("string"));
    out.push(a.must_next::<Vec<u8>>("bytes"));
    out.push(a.must_next::<Vec<String>>("string[]"));
    out.push(a.must_next::<Vec<u64>>("u64[]"));
    out.push(a.must_next::<Vec<U256>>("u256[]"));
    out.push(a.must_next::<Pair>("Pair"));
    out.push(a.must_next::<Vec<Pair>>("Pair[]"));
    out.push(a.must_next::<Address>("Address"));
    assert!(a.is_exhausted(), "trailing bytes after the last argument");
    out
}

#[export]
pub fn golden() -> Args {
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
        .add(vec![U256::from(10u64), U256::MAX])
        .add(Pair { key: "p".into(), value: 7 })
        .add(vec![Pair { key: "x".into(), value: 1 }, Pair { key: "y".into(), value: 2 }])
        .add(Address::from("AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq"))
}

#[export(name = "retString")]
pub fn ret_string() -> &'static str {
    "héllo"
}

#[export(name = "retU64")]
pub fn ret_u64() -> u64 {
    42
}

#[export(name = "retBool")]
pub fn ret_bool() -> bool {
    true
}

#[export(name = "retU256")]
pub fn ret_u256() -> U256 {
    U256::from(7u64)
}

#[export(name = "retArgs")]
pub fn ret_args() -> Args {
    Args::new().add("a").add(1u32)
}

#[export(name = "retVoid")]
pub fn ret_void(_a: Args) {}

#[export(name = "retEmpty")]
pub fn ret_empty() -> Vec<u8> {
    Vec::new()
}

#[export(name = "noParam")]
pub fn no_param() -> String {
    "ok".into()
}
