use alloc::string::ToString;
use massa_proto_rs::massa::abi::v1::{
    NativeAddress, NativeAddressToStringRequest, NativeAddressToStringResponse,
};

use crate::{
    alloc::string::String,
    allocator::{get_parameters, EncodeLengthPrefixed},
    massa_abi,
};
use prost::Message;

// ****************************************************************************
// Function from the abi used by the SC

massa_abi!(abi_native_address_to_string);

// ****************************************************************************

pub fn native_address_to_string(
    address_to_convert: NativeAddress,
) -> Result<String, String> {
    // serialize the arguments with protobuf
    let arg_ptr = NativeAddressToStringRequest {
        address_to_convert: Some(address_to_convert),
    }
    .encode_length_prefixed();

    // call the function from the abi
    let resp_ptr = unsafe { abi_native_address_to_string(arg_ptr) };

    // deserialize the returned value with protobuf
    Ok(NativeAddressToStringResponse::decode(
        get_parameters(resp_ptr).as_slice(),
    )
    .map_err(|_| "Error decoding NativeAddressToStringResponse".to_string())?
    .converted_address)
}

#[derive(Clone)]
pub struct Address(NativeAddress);

impl From<NativeAddress> for Address {
    fn from(value: NativeAddress) -> Self {
        Address(value)
    }
}

impl Into<NativeAddress> for Address {
    fn into(self) -> NativeAddress {
        self.0
    }
}

impl TryInto<String> for Address {
    type Error = String;

    fn try_into(self) -> Result<String, Self::Error> {
        Ok(native_address_to_string(self.0)?)
    }
}
