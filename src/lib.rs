// As of may 2023, a good documentation can be found here:
// https://surma.dev/things/rust-to-webassembly/
// and https://rustwasm.github.io/docs/book/reference/debugging.html
//
// ** /!\ ** wee_alloc has a leak and is more or less deprecated, lol_alloc
// seems to be the best option for small size.
// For now, I will use the default allocator which is
// dlmalloc: https://docs.rs/dlmalloc/latest/dlmalloc/

// ****************************************************************************
// Main target of this crate is wasm32-unknown-unknown and we want to override
// panic. For this purpose we need to use the no_std attribute. But for testing
// it is easier to stay on the default target, hence we go no_std by default but
// add crate std (revert no_std) for testing.
// ****************************************************************************
#![no_std]

// #[cfg(not(target_arch = "wasm32"))]
// extern crate std;

// ****************************************************************************
// Reexport alloc crate and anyhow crate
// ****************************************************************************
extern crate alloc;

pub use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec::Vec,
    vec,
};

// pub use anyhow::{anyhow, Result};
// ****************************************************************************

mod abi;
pub mod abis;
mod allocator;

pub fn get_parameters(arg_ptr: u32) -> Vec<u8> {
    allocator::get_parameters(arg_ptr)
}

pub fn encode_length_prefixed(data: Vec<u8>) -> u32 {
    allocator::encode_length_prefixed(data)
}

// ****************************************************************************
// a bunch of functions to simulate the host
// ****************************************************************************
#[cfg(feature = "testing")]
pub mod test {
    // extern crate std;

    use super::alloc::vec::Vec;
    use super::allocator;

    /// Simulate arguments passed by the host to the SC
    pub fn host_write_buffer(data: &[u8]) -> u32 {
        allocator::test::host_write_buffer(data)
    }

    /// Simulate reading arguments passed by the host to the SC
    pub fn host_read_buffer(arg_ptr: u32) -> Vec<u8> {
        allocator::test::host_read_buffer(arg_ptr)
    }
}

#[test]
fn test_basic_build() {
    extern crate std;
    use std::println;

    println!("test_basic: ok");
}
