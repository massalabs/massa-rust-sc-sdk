// ****************************************************************************
// As we go no_std we need to import alloc and use a global allocator
// Using dlmalloc allocator, the default allocator when target is
// wasm32-unknown-unknown
// ****************************************************************************
extern crate alloc;

#[global_allocator]
static A: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

// ****************************************************************************
// may try lol_alloc allocator
// ****************************************************************************
// extern crate alloc;

// use lol_alloc::{AssumeSingleThreaded, FreeListAllocator};

// // SAFETY: This application is single threaded, so using AssumeSingleThreaded
// is // allowed.
// #[global_allocator]
// static ALLOCATOR: AssumeSingleThreaded<FreeListAllocator> =
//     unsafe { AssumeSingleThreaded::new(FreeListAllocator::new()) };
// ****************************************************************************

// ****************************************************************************
// __alloc function used by to host to allocate memory for for exchange with the
// wasm module
// ****************************************************************************

use alloc::vec::Vec;

static mut SHARED_MEM: Vec<u8> = Vec::new();

#[cfg(feature = "testing")]
static mut IS_SHARED_MEM_CONSUMED: bool = true;

#[no_mangle]
#[export_name = "__alloc"]
fn myalloc(size: u32) -> u32 {
    unsafe {
        #[cfg(feature = "testing")]
        {
            use log::warn;

            if !IS_SHARED_MEM_CONSUMED {
                warn!("SHARED_MEM has not been consumed yet, possible memory leak");
            }
            IS_SHARED_MEM_CONSUMED = false;
        }

        // MUST BE FILLED or at least respect the assert bellow else the size is
        // lost between the host and the wasm module
        SHARED_MEM = alloc::vec![0; size as usize];
        assert_eq!(size as usize, SHARED_MEM.len());

        SHARED_MEM.as_ptr() as u32
    }
}

fn get_shared_mem_as_u32() -> u32 {
    unsafe { SHARED_MEM.as_ptr() as u32 }
}

pub(super) fn get_parameters(arg_ptr: u32) -> Vec<u8> {
    // This is a check to ensure that the memory is not consumed twice.
    // Only useful for tests while developing.
    // The assert below has a broader scope but is less explicit.
    #[cfg(feature = "testing")]
    unsafe {
        if IS_SHARED_MEM_CONSUMED {
            panic!(
                "SHARED_MEM has been consumed, get_parameters() called twice \
                without new memory allocation."
            );
        }
    }

    assert_eq!(arg_ptr, get_shared_mem_as_u32());

    // take the parameter
    unsafe {
        #[cfg(feature = "testing")]
        {
            IS_SHARED_MEM_CONSUMED = true;
        }

        core::mem::take(&mut SHARED_MEM)
    }
}

pub(super) fn encode_length_prefixed(data: Vec<u8>) -> u32 {
    let data_len: u32 = data.len().try_into().expect("size fit in u32");
    let buf_len: u32 = data_len + 4;

    unsafe {
        // allocate memory and bind it to our global buffer
        myalloc(buf_len);

        // MUST CLEAR HERE because myalloc() fill the buffer
        SHARED_MEM.clear();

        SHARED_MEM.extend(data_len.to_le_bytes());
        SHARED_MEM.extend(data);

        // return the pointer to the global buffer
        SHARED_MEM.as_ptr() as u32
    }
}

pub(super) trait EncodeLengthPrefixed {
    fn encode_length_prefixed(self) -> u32;
}

impl<T> EncodeLengthPrefixed for T
where
    T: prost::Message,
{
    fn encode_length_prefixed(self) -> u32 {
        // FIXME see encode_length_delimited_to_vec(&self) -> Vec<u8> in prost
        // to use self.encoded_len(); to prevent copies
        encode_length_prefixed(self.encode_to_vec())
    }
}

#[cfg(feature = "testing")]
// The below functions will only be compiled and available during tests,
pub(super) mod test {
    use super::alloc::vec::Vec;
    use crate::allocator::{get_parameters, myalloc, SHARED_MEM};

    // Function that writes the [u8] argument to SHARED_MEM
    pub(crate) fn host_write_buffer(data: &[u8]) -> u32 {
        let buf_ptr = myalloc(data.len().try_into().expect("size fit in u32"));

        unsafe {
            SHARED_MEM.clear();
            SHARED_MEM.extend_from_slice(data);
        }
        buf_ptr
    }

    // Function that reads the [u8] argument from SHARED_MEM
    pub(crate) fn host_read_buffer(arg_ptr: u32) -> Vec<u8> {
        let arg = get_parameters(arg_ptr);

        // get the first 4 bytes of arg in a array of u8
        let arg_len: [u8; 4] = arg[0..4].try_into().expect(
            "First 4 bytes of arg must contain the length of the argument",
        );

        // The first 4 bytes of arg are the length of the argument in little
        // endian
        let arg_len = u32::from_le_bytes(arg_len);

        // verify that the length is correct
        assert_eq!(arg_len + 4, arg.len() as u32);

        // return the argument
        arg[4..].to_vec()
    }
}
