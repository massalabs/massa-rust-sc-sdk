use crate::{alloc::string::String, allocator::EncodeLengthPrefixed, massa_abi};
use massa_proto_rs::massa::abi::v1::GenerateEventRequest;

// ****************************************************************************
// Function from the abi used by the SC

massa_abi!(abi_generate_event);

// ****************************************************************************

// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_generate_event(event: String) {
    // serialize the arguments with protobuf then length prefix it
    let arg_ptr = GenerateEventRequest { event }.encode_length_prefixed();

    // call the function from the abi
    unsafe { abi_generate_event(arg_ptr) };
}

pub fn generate_event(event: String) {
    impl_generate_event(event)
}
