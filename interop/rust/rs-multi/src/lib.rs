//! Rust counterpart of `as/assembly/multi.ts` (MRC1155 metadata, mintable, burnable, roles, ownable).
#![no_std]
use massa_sc_sdk::prelude::*;
use massa_sc_standards::mrc1155::{self, metadata, mintable::MINTER_ROLE};
use massa_sc_standards::{
    access_control, export_access_control, export_mrc1155_burnable, export_mrc1155_metadata,
    export_mrc1155_mintable, export_ownable, ownership,
};

#[export]
pub fn constructor(mut a: Args) {
    let uri: String = a.must_next("uri");
    mrc1155::constructor(&uri);
    access_control::grant_role(MINTER_ROLE, &context::caller());
}

#[export(name = "setTokenURI")]
pub fn set_token_uri(mut a: Args) {
    ownership::only_owner();
    let id: U256 = a.must_next("id");
    let uri: String = a.must_next("uri");
    metadata::set_uri(id, &uri);
}

#[export(name = "setBaseURI")]
pub fn set_base_uri(mut a: Args) {
    ownership::only_owner();
    let uri: String = a.must_next("uri");
    metadata::set_base_uri(&uri);
}

export_mrc1155_metadata!();
export_mrc1155_mintable!();
export_mrc1155_burnable!();
export_access_control!();
export_ownable!();
