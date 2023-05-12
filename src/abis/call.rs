use crate::{
    abi::proto::massa::abi::v1::{Address, Amount, CallRequest, CallResponse},
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
    fn abi_call(arg: u32) -> u32;
}

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_call(
    address: String,
    function: String,
    arg: Vec<u8>,
    call_coins: u64,
) -> Result<Vec<u8>, String> {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = CallRequest {
        address: Some(Address { address }),
        function,
        arg,
        call_coins: Some(Amount { amount: call_coins }),
    }
    .encode_length_prefixed();

    // call the function from the abi
    let ret_ptr = unsafe { abi_call(arg_ptr) };

    let ret = get_parameters(ret_ptr);

    let Ok(response) = CallResponse::decode(ret.as_slice()) else {
        return Err("Create SC response decode error".to_string())
    };

    Ok(response.return_data)
}

// ****************************************************************************
// mocked version of the abi so one can dev and write tests without the need
// to call the host
// cfg_if! {
//     if #[cfg(feature = "testing")] {
//         extern crate std;
//         use std::println;

//         // Should we leave it up to the user to implement the mock?
//         // Should we mock at the abi_level?
//         // Can mockall do the job?
//         fn mock_call(
//             _address: String,
//             _function: String,
//             _arg: Vec<u8>,
//             _call_coins: u64,
//         ) -> Result<Vec<u8>>  {
//             println!("SC calld");
//             Ok(Vec::new())
//         }
//     }
// }

pub fn call(
    address: String,
    function: String,
    arg: Vec<u8>,
    call_coins: u64,
) -> Result<Vec<u8>, String> {
    // cfg_if! {
    //     if #[cfg(feature = "testing")] {
    //         mock_call(address, function, arg, call_coins)
    //     }
    //      else {
    impl_call(address, function, arg, call_coins)
    //     }
    // }
}
