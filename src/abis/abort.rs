use crate::alloc::string::String;
use crate::allocator::encode_length_prefixed;

// ****************************************************************************
// Override the panic handler to call the abort function from the abi
// ****************************************************************************

#[cfg(target_arch = "wasm32")]
#[cfg(not(feature = "testing"))]
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    use crate::alloc::format;

    impl_abort(format!("Panic occurred: {:?}", info));
    unreachable!()
}

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
    fn abi_abort(arg: u32) -> u32;
}

// ****************************************************************************
// Interface between the sdk and the SC i.e. seen by the user
// Wrapped function to "hide" unsafe and manage serialize/deserialize of the
// parameters
fn impl_abort(arg: String) {
    // FIXME: do not relly on encode_length_prefixed as it allocates memory
    // reimplment it writing directly into the memory at address 0
    let ptr = encode_length_prefixed(arg.into_bytes());

    // call the function from the abi
    unsafe { abi_abort(ptr) };
}

// ****************************************************************************
// ** mocked version of the abi **
// it has no sense for abort as for testings we will use the default
// panic_handler
// ****************************************************************************
