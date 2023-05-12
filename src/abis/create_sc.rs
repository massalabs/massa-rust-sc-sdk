use crate::{
    abi::proto::massa::abi::v1::{CreateScRequest, CreateScResponse},
    alloc::{
        string::{String, ToString},
        vec::Vec,
    },
    allocator::{get_parameters, EncodeLengthPrefixed},
};
// use anyhow::{anyhow, Result};
// use cfg_if::cfg_if;
use prost::Message;

// ****************************************************************************
// Function from the abi used by the SC

// Interface between the sdk and the abi

// specify the "namespace" of the function being imported i.e. "massa"
#[link(wasm_import_module = "massa")]
// specify the function name as it is in the abi
// CHECK: extern "C" implies no_mangle
extern "C" {
    // may be use to "rename" a function
    // #[link_name = "actual_symbol_name"]
    fn abi_create_sc(arg: u32) -> u32;
}

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_create_sc(bytecode: Vec<u8>) -> Result<String, String> {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = CreateScRequest { bytecode }.encode_length_prefixed();

    // call the function from the abi
    let ret_ptr = unsafe { abi_create_sc(arg_ptr) };

    let ret = get_parameters(ret_ptr);

    let Ok(response) = CreateScResponse::decode(ret.as_slice()) else {
        return Err("Create SC response decode error".to_string())
    };

    Ok(response.address.ok_or("no_address".to_string())?.address)
}

// ****************************************************************************
// mocked version of the abi so one can dev and write tests without the need
// to call the host
// cfg_if! {
//     if #[cfg(feature = "testing")] {
//         extern crate std;
//         use std::println;
//         use std::string::ToString;

//         // Should we leave it up to the user to implement the mock?
//         // Should we mock at the abi_level?
//         // Can mockall do the job?
//         fn mock_create_sc(_bytecode: Vec<u8>) -> Result<String>  {
//             println!("SC created");
//             Ok("fake_sc_address".to_string())
//         }
//     }
// }

pub fn create_sc(bytecode: Vec<u8>) -> Result<String, String> {
    // cfg_if! {
    //     if #[cfg(feature = "testing")]    {
    //         mock_create_sc(bytecode)
    //     }
    //      else {
    impl_create_sc(bytecode)
    //     }
    // }
}
