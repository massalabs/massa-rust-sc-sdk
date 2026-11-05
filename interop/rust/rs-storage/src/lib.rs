//! Rust counterpart of `as/assembly/storage.ts`: same entries, same helpers.
#![no_std]
use massa_sc_sdk::collections::PersistentMap;
use massa_sc_sdk::helpers::{ConstantManager, KeyIncrementer, MapManager};
use massa_sc_sdk::prelude::*;
use massa_sc_sdk::security::AccessControl;

const USER: &str = "AU12UBnqTHDQALpocVBnkPNy7y5CndUJQTLutaVDDFgMJcq5kQiKq";

#[derive(Serializable)]
struct Item {
    name: String,
    qty: u32,
}

#[export]
pub fn write() {
    storage::set("str", "hello");
    storage::set("bytes", [1u8, 2, 3]);
    storage::set(Args::new().add("k"), Args::new().add(5u32).add("v"));
    storage::set_value("u64", &42u64);
    storage::set_value("u32", &7u32);
    storage::set_value("bool", &true);

    PersistentMap::<String, String>::new("ms").set(&"k".into(), "v".into());
    PersistentMap::<String, u32>::new("mi").set(&"k".into(), 7);
    PersistentMap::<String, u64>::new("mu").set(&"big".into(), u64::MAX);
    PersistentMap::<String, bool>::new("mb").set(&"k".into(), true);
    PersistentMap::<String, f64>::new("mf").set(&"k".into(), 1.5);
    PersistentMap::<Address, u64>::new("ma").set(&Address::from(USER), 9);
    PersistentMap::<u32, Vec<u8>>::new("mk").set(&12, vec![9, 8]);
    PersistentMap::<String, Item>::new("mo").set(&"it".into(), Item { name: "apple".into(), qty: 3 });

    let mut seq = KeyIncrementer::<u8>::new(0);
    ConstantManager::<u64>::new(&mut seq).set(&123);
    ConstantManager::<String>::new(&mut seq).set(&"constant".into());
    MapManager::<String, u32>::new(&mut seq).set(&"x".into(), &5);
    MapManager::<u64, String>::new(&mut seq).set(&77, &"seventy-seven".into());

    let mut acl = AccessControl::<u8>::new(9);
    let admin = acl.new_permission("admin");
    let user = acl.new_permission("user");
    acl.grant_permission(admin, &Address::from(USER));
    acl.grant_permission(user, &Address::from(USER));
    acl.grant_permission(user, &context::caller());
}

#[export]
pub fn read() -> Args {
    let mut out = Args::new();
    out.push(storage::get_value::<String>("str"));
    out.push(storage::get("bytes"));
    let mut args = storage::get_value::<Args>(Args::new().add("k"));
    out.push(args.must_next::<u32>("u32")).push(args.must_next::<String>("string"));
    out.push(storage::get_value::<u64>("u64"));
    out.push(storage::get_value::<u32>("u32"));
    out.push(storage::get_value::<bool>("bool"));

    out.push(PersistentMap::<String, String>::new("ms").get_some(&"k".into(), "ms::k"));
    out.push(PersistentMap::<String, u32>::new("mi").get_some(&"k".into(), "mi::k"));
    out.push(PersistentMap::<String, u64>::new("mu").get_some(&"big".into(), "mu::big"));
    out.push(PersistentMap::<String, bool>::new("mb").get_some(&"k".into(), "mb::k"));
    out.push(PersistentMap::<String, f64>::new("mf").get_some(&"k".into(), "mf::k"));
    out.push(PersistentMap::<Address, u64>::new("ma").get_some(&Address::from(USER), "ma"));
    out.push(PersistentMap::<u32, Vec<u8>>::new("mk").get_some(&12, "mk::12"));
    let item = PersistentMap::<String, Item>::new("mo").get_some(&"it".into(), "mo::it");
    out.push(item.name).push(item.qty);
    out.push(PersistentMap::<String, String>::new("ms").contains(&"missing".into()));

    let mut seq = KeyIncrementer::<u8>::new(0);
    out.push(ConstantManager::<u64>::new(&mut seq).must_value());
    out.push(ConstantManager::<String>::new(&mut seq).must_value());
    out.push(MapManager::<String, u32>::new(&mut seq).must_value(&"x".into()));
    out.push(MapManager::<u64, String>::new(&mut seq).must_value(&77));

    let mut acl = AccessControl::<u8>::new(9);
    let admin = acl.new_permission("admin");
    let user = acl.new_permission("user");
    out.push(acl.has_permission(admin, &Address::from(USER)));
    out.push(acl.has_permission(user, &Address::from(USER)));
    out.push(acl.has_permission(admin, &context::caller()));
    out.push(acl.has_permission(user, &context::caller()));
    out
}

#[export]
pub fn keys() -> Args {
    let all = storage::get_keys("");
    let mut out = Args::new().add(all.len() as u32);
    for k in all {
        out.push(k);
    }
    out
}
