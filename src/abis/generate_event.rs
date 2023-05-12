use crate::{
    abi::proto::massa::abi::v1::GenerateEventRequest, alloc::string::String,
    allocator::EncodeLengthPrefixed,
};

// use cfg_if::cfg_if;

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
    fn abi_generate_event(arg: u32) -> u32;
}

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
//         fn mock_generate_event(event: String)  {
//             println!("{}", event);
//         }
//     }
// }

pub fn generate_event(event: String) {
    // cfg_if! {
    //     if #[cfg(feature = "testing")]    {
    //         mock_generate_event(event)
    //     }
    //      else {
            impl_generate_event(event)
    //     }
    // }
}
