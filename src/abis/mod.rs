// abi_abort is "wrapped" by panic!() in the sdk, as such it is not used
// directly and this module is not public.
mod abort;
pub(crate) mod call;
pub(crate) mod create_sc;
pub(crate) mod echo;
pub(crate) mod generate_event;
pub(crate) mod log;
pub(crate) mod native_address;
pub(crate) mod native_amount;
pub(crate) mod transfer_coins;

// re-export the functions from the abi
pub use self::log::log;
pub use call::call;
pub use create_sc::create_sc;
pub use echo::echo;
pub use generate_event::generate_event;
pub use native_address::native_address_to_string;
pub use transfer_coins::transfer_coins;

// re-export the types from the abi
pub use massa_proto::massa::abi::v1::NativeAddress;
pub use massa_proto::massa::abi::v1::NativeAmount;
pub use native_address::Address;
pub use native_amount::Amount;


#[macro_export]
macro_rules! massa_abi {
    ($func:ident) => {
        #[link(wasm_import_module = "massa")]
        extern "C" {
            fn $func(arg: u32) -> u32;
        }
    };
}

