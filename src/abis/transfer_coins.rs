use crate::{allocator::EncodeLengthPrefixed, massa_abi};
use massa_proto_rs::massa::abi::v1::{
    NativeAddress, NativeAmount, TransferCoinsRequest,
};

use super::{Address, Amount};


// ****************************************************************************
// Function from the abi used by the SC

massa_abi!(abi_transfer_coins);

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_transfer_coins(address: NativeAddress, amount: NativeAmount) {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = TransferCoinsRequest {
        target_address: Some(address),
        amount_to_transfer: Some(amount),
    }
    .encode_length_prefixed();

    // call the function from the abi
    unsafe { abi_transfer_coins(arg_ptr) };
}

pub fn transfer_coins(to_address: Address, raw_amount: Amount) {
    impl_transfer_coins(to_address.into(), raw_amount.into())
}
