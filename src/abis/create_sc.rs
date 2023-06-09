use massa_proto_rs::massa::abi::v1::{
    CreateScRequest, CreateScResponse, NativeAddress,
};

use crate::{
    alloc::{
        string::{String, ToString},
        vec::Vec,
    },
    allocator::{get_parameters, EncodeLengthPrefixed},
    massa_abi,
};
use prost::Message;

use super::Address;

// ****************************************************************************
// Function from the abi used by the SC

massa_abi!(abi_create_sc);

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_create_sc(bytecode: Vec<u8>) -> Result<NativeAddress, String> {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = CreateScRequest { bytecode }.encode_length_prefixed();

    // call the function from the abi
    let ret_ptr = unsafe { abi_create_sc(arg_ptr) };

    let ret = get_parameters(ret_ptr);

    let Ok(response) = CreateScResponse::decode(ret.as_slice()) else {
        return Err("Create SC response decode error".to_string())
    };

    response.sc_address.ok_or("No address return".to_string())
}

pub fn create_sc(bytecode: Vec<u8>) -> Result<Address, String> {
    impl_create_sc(bytecode).map(|addr| addr.into())
}
