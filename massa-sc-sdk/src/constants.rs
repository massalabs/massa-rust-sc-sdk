//! Network and protocol constants.

/// 1 MAS, in nanoMAS (the unit used by every coin-related ABI).
pub const ONE_MAS: u64 = 1_000_000_000;

/// Massa MainNet chain id.
pub const MASSA_MAINNET_CHAIN_ID: u64 = 77658377;
/// Massa BuildNet chain id.
pub const MASSA_BUILDNET_CHAIN_ID: u64 = 77658366;
/// Massa SecureNet chain id.
pub const MASSA_SECURENET_CHAIN_ID: u64 = 77658383;
/// Massa LabNet chain id.
pub const MASSA_LABNET_CHAIN_ID: u64 = 77658376;
/// Chain id of a node built with the `sandbox` feature (Massa and C-Layer).
pub const SANDBOX_CHAIN_ID: u64 = 77;

/// C-Layer MainNet chain id.
pub const CLAYER_MAINNET_CHAIN_ID: u64 = 67766589;
/// C-Layer TestNet chain id.
pub const CLAYER_TESTNET_CHAIN_ID: u64 = 67848384;
/// C-Layer BuildNet chain id.
pub const CLAYER_BUILDNET_CHAIN_ID: u64 = 67667668;

/// Chain id reported by the native [`crate::mock`] VM by default (same value
/// as the AssemblyScript vm-mock).
pub const MOCK_CHAIN_ID: u64 = 9123;

/// Address whose coins can never be spent.
pub const BURN_ADDRESS: &str = "AU1111111111111111111111111111111112m1s9K";

/// Largest number of keys returned by one datastore key query from execution
/// version 2 (MIP-0002): the maximum `count` of
/// [`crate::storage::get_keys_page`], and the most keys
/// [`crate::storage::get_keys`] can match before failing.
pub const MAX_DATASTORE_KEYS_PAGE: u32 = 500;

/// Maximum length of a datastore key, in bytes.
pub const MAX_DATASTORE_KEY_LENGTH: usize = 255;

/// Maximum number of threads (a slot is a `(period, thread)` pair).
pub const THREAD_COUNT: u8 = 32;

/// Ledger cost of one byte of storage, in nanoMAS (0.0001 MAS).
pub const STORAGE_BYTE_COST: u64 = 100_000;
/// Ledger cost of creating a new address entry, in nanoMAS (0.001 MAS).
pub const NEW_LEDGER_ENTRY_COST: u64 = 1_000_000;
/// Base size accounted for every datastore entry, in bytes.
pub const DATASTORE_ENTRY_BASE_SIZE: usize = 4;
