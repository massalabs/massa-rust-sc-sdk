use massa_proto_rs::massa::abi::v1::{
    CallRequest, CallResponse, NativeAddress, NativeAmount,
};

use crate::{
    abis::{Address, Amount},
    alloc::{
        string::{String, ToString},
        vec::Vec,
    },
    allocator::{get_parameters, EncodeLengthPrefixed},
    massa_abi,
};
use prost::Message;

// ****************************************************************************
// Function from the abi used by the SC

massa_abi!(abi_call);

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_call(
    target_sc_address: NativeAddress,
    target_function_name: String,
    function_arg: Vec<u8>,
    call_coins: NativeAmount,
) -> Result<Vec<u8>, String> {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = CallRequest {
        target_sc_address: Some(target_sc_address),
        target_function_name,
        function_arg: function_arg,
        call_coins: Some(call_coins),
    }
    .encode_length_prefixed();

    // call the function from the abi
    let ret_ptr = unsafe { abi_call(arg_ptr) };

    let ret = get_parameters(ret_ptr);

    Ok(CallResponse::decode(ret.as_slice())
        .map_err(|_| "Create SC response decode error".to_string())?
        .data)
}

pub fn call(
    address: Address,
    function: String,
    arg: Vec<u8>,
    call_coins: Amount,
) -> Result<Vec<u8>, String> {
    impl_call(address.into(), function, arg, call_coins.into())
}
