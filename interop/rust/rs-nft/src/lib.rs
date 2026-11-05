//! Rust counterpart of `as/assembly/nft.ts` (MRC721 enumerable + metadata, ownable).
#![no_std]
use massa_sc_sdk::prelude::*;
use massa_sc_standards::mrc721::{enumerable, metadata};
use massa_sc_standards::{export_mrc721_enumerable, export_mrc721_metadata, export_ownable, ownership};

#[export]
pub fn constructor(mut a: Args) {
    let name: String = a.must_next("name");
    let symbol: String = a.must_next("symbol");
    let base_uri: String = a.must_next("baseURI");
    enumerable::constructor(&name, &symbol);
    metadata::set_base_uri(&base_uri);
}

#[export(name = "setTokenURI")]
pub fn set_token_uri(mut a: Args) {
    ownership::only_owner();
    let id: U256 = a.must_next("tokenId");
    let uri: String = a.must_next("uri");
    metadata::set_uri(id, &uri);
}

export_mrc721_enumerable!();
export_mrc721_metadata!();
export_ownable!();
