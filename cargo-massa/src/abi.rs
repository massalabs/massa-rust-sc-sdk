//! Host imports known to the Massa runtime (`massa-sc-runtime`,
//! `src/as_execution/context.rs`) with their wasm signatures.
//! `i` = i32, `I` = i64, `f` = f64.

/// Networks a module can target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Network {
    /// Massa (mainnet, buildnet...).
    Massa,
    /// C-Layer (a Massa fork with extra host functions, without MIP-0002 yet).
    Clayer,
    /// Both: every network-specific import is an error.
    Any,
}

/// Where a host function is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Every network.
    All,
    /// Massa nodes running execution version >= 2 (MIP-0002); not C-Layer yet.
    MassaMip0002,
    /// C-Layer nodes only.
    ClayerOnly,
}

impl Availability {
    /// Human-readable restriction, `None` when available everywhere.
    pub fn note(self) -> Option<&'static str> {
        match self {
            Availability::All => None,
            Availability::MassaMip0002 => {
                Some("requires execution version >= 2 (MIP-0002); not available on C-Layer yet")
            }
            Availability::ClayerOnly => Some("C-Layer only; the module cannot be instantiated on Massa"),
        }
    }

    /// Whether a module importing such a function can run on `network`.
    pub fn runs_on(self, network: Network) -> bool {
        match (self, network) {
            (Availability::All, _) => true,
            (_, Network::Any) => false,
            (Availability::MassaMip0002, Network::Massa) => true,
            (Availability::ClayerOnly, Network::Clayer) => true,
            _ => false,
        }
    }
}

pub struct HostFn {
    pub module: &'static str,
    pub name: &'static str,
    pub params: &'static str,
    pub results: &'static str,
    pub availability: Availability,
}

const fn f(module: &'static str, name: &'static str, params: &'static str, results: &'static str) -> HostFn {
    HostFn { module, name, params, results, availability: Availability::All }
}

const fn n(
    module: &'static str,
    name: &'static str,
    params: &'static str,
    results: &'static str,
    availability: Availability,
) -> HostFn {
    HostFn { module, name, params, results, availability }
}

pub const HOST_FUNCTIONS: &[HostFn] = &[
    // AssemblyScript runtime imports
    f("env", "abort", "iiii", ""),
    f("env", "seed", "", "f"),
    f("env", "Date.now", "", "f"),
    f("env", "console.log", "i", ""),
    f("env", "console.info", "i", ""),
    f("env", "console.warn", "i", ""),
    f("env", "console.error", "i", ""),
    f("env", "console.debug", "i", ""),
    f("env", "trace", "iifffff", ""),
    f("env", "process.exit", "i", ""),
    // Massa ABI
    f("massa", "assembly_script_print", "i", ""),
    f("massa", "assembly_script_call", "iiiI", "i"),
    f("massa", "assembly_script_get_remaining_gas", "", "I"),
    f("massa", "assembly_script_create_sc", "i", "i"),
    f("massa", "assembly_script_set_data", "ii", ""),
    f("massa", "assembly_script_set_data_for", "iii", ""),
    f("massa", "assembly_script_get_data", "i", "i"),
    f("massa", "assembly_script_get_data_for", "ii", "i"),
    f("massa", "assembly_script_delete_data", "i", ""),
    f("massa", "assembly_script_delete_data_for", "ii", ""),
    f("massa", "assembly_script_append_data", "ii", ""),
    f("massa", "assembly_script_append_data_for", "iii", ""),
    f("massa", "assembly_script_has_data", "i", "i"),
    f("massa", "assembly_script_has_data_for", "ii", "i"),
    f("massa", "assembly_script_get_owned_addresses", "", "i"),
    f("massa", "assembly_script_get_call_stack", "", "i"),
    f("massa", "assembly_script_generate_event", "i", ""),
    f("massa", "assembly_script_transfer_coins", "iI", ""),
    f("massa", "assembly_script_transfer_coins_for", "iiI", ""),
    f("massa", "assembly_script_get_balance", "", "I"),
    f("massa", "assembly_script_get_balance_for", "i", "I"),
    f("massa", "assembly_script_hash", "i", "i"),
    f("massa", "assembly_script_hash_sha256", "i", "i"),
    f("massa", "assembly_script_keccak256_hash", "i", "i"),
    n("massa", "assembly_script_hash_mimc", "i", "i", Availability::ClayerOnly),
    f("massa", "assembly_script_signature_verify", "iii", "i"),
    f("massa", "assembly_script_evm_signature_verify", "iii", "i"),
    f("massa", "assembly_script_evm_get_address_from_pubkey", "i", "i"),
    f("massa", "assembly_script_evm_get_pubkey_from_signature", "ii", "i"),
    f("massa", "assembly_script_is_address_eoa", "i", "i"),
    f("massa", "assembly_script_address_from_public_key", "i", "i"),
    f("massa", "assembly_script_validate_address", "i", "i"),
    f("massa", "assembly_script_unsafe_random", "", "I"),
    f("massa", "assembly_script_get_call_coins", "", "I"),
    f("massa", "assembly_script_get_time", "", "I"),
    f("massa", "assembly_script_send_message", "iiIiIiIIIiii", ""),
    f("massa", "assembly_script_get_origin_operation_id", "", "i"),
    f("massa", "assembly_script_get_current_period", "", "I"),
    f("massa", "assembly_script_get_current_thread", "", "i"),
    f("massa", "assembly_script_set_bytecode", "i", ""),
    f("massa", "assembly_script_set_bytecode_for", "ii", ""),
    f("massa", "assembly_script_get_op_keys", "", "i"),
    f("massa", "assembly_script_get_op_keys_prefix", "i", "i"),
    f("massa", "assembly_script_get_keys", "i", "i"),
    f("massa", "assembly_script_get_keys_for", "ii", "i"),
    n("massa", "assembly_script_get_keys_paginated", "iii", "i", Availability::MassaMip0002),
    n("massa", "assembly_script_get_keys_for_paginated", "iiii", "i", Availability::MassaMip0002),
    f("massa", "assembly_script_has_op_key", "i", "i"),
    f("massa", "assembly_script_get_op_data", "i", "i"),
    f("massa", "assembly_script_get_bytecode", "", "i"),
    f("massa", "assembly_script_get_bytecode_for", "i", "i"),
    f("massa", "assembly_script_local_call", "iii", "i"),
    f("massa", "assembly_script_local_execution", "iii", "i"),
    f("massa", "assembly_script_caller_has_write_access", "", "i"),
    f("massa", "assembly_script_function_exists", "ii", "i"),
    f("massa", "assembly_script_chain_id", "", "I"),
    f("massa", "assembly_script_get_deferred_call_quote", "IiII", "I"),
    f("massa", "assembly_script_deferred_call_register", "iiIiIiI", "i"),
    f("massa", "assembly_script_deferred_call_exists", "i", "i"),
    f("massa", "assembly_script_deferred_call_cancel", "i", ""),
];

pub fn lookup(module: &str, name: &str) -> Option<&'static HostFn> {
    HOST_FUNCTIONS.iter().find(|h| h.module == module && h.name == name)
}

/// Node limits (massa `massa-models/src/config/constants.rs`).
#[allow(dead_code)]
pub mod limits {
    pub const MAX_DEFINED_FUNCTIONS: usize = 512;
    pub const MAX_FUNCTION_IMPORTS: usize = 256;
    pub const MAX_FUNCTIONS: usize = MAX_DEFINED_FUNCTIONS + MAX_FUNCTION_IMPORTS;
    pub const MAX_IMPORTS: usize = MAX_FUNCTION_IMPORTS + 1;
    pub const MAX_EXPORTS: usize = MAX_DEFINED_FUNCTIONS + 512;
    pub const MAX_SIGNATURE_LEN: usize = 64 + 8;
    pub const MAX_NAME_LEN: usize = 256;
    pub const MAX_FUNCTION_NAME_LEN: usize = 256;
    pub const MAX_CUSTOM_SECTIONS: usize = 1;
    pub const MAX_CUSTOM_SECTIONS_DATA: usize = 1_000_000;
    pub const MAX_TABLE_INITIALIZERS: usize = 512;
    pub const MAX_PASSIVE_ELEMENTS: usize = 512;
    pub const MAX_PASSIVE_DATA: usize = 512;
    pub const MAX_GLOBALS: usize = 512;
    pub const MAX_TABLES: usize = 16;
    pub const MAX_MEMORIES: usize = 1;
    pub const MAX_MEMORY_PAGES: u64 = 64;
    pub const MAX_BYTECODE_LENGTH: usize = 10_000_000;
}
