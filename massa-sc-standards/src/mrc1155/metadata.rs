//! MRC1155 metadata extension: per-token URIs with a base URI, falling back
//! to the collection URI.

use massa_sc_sdk::prelude::*;

use super::internals::{uri as collection_uri, URI_EVENT};

pub const BASE_URI_KEY: &str = "BASE_URI";
pub const TOKENS_URI_KEY: &str = "TOKENS_URI";

fn token_uri_key(id: U256) -> Vec<u8> {
    let mut k = Vec::with_capacity(TOKENS_URI_KEY.len() + 32);
    k.extend_from_slice(TOKENS_URI_KEY.as_bytes());
    k.extend_from_slice(&id.to_le_bytes());
    k
}

fn read_string(key: impl AsRef<[u8]>) -> String {
    storage::try_get(key).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

pub fn set_base_uri(base_uri: &str) {
    storage::set(BASE_URI_KEY, base_uri);
}

pub fn base_uri() -> String {
    read_string(BASE_URI_KEY)
}

pub fn token_uri(id: U256) -> String {
    read_string(token_uri_key(id))
}

/// Set the URI suffix of `id` and emit `URI:<uri>,<id>`.
pub fn set_uri(id: U256, uri: &str) {
    storage::set(token_uri_key(id), uri);
    generate_event(&create_event(URI_EVENT, &[uri, &id.to_string()]));
}

/// `baseURI ++ tokenURI(id)` when a token URI is set, else the collection URI.
pub fn uri(id: U256) -> String {
    let token = token_uri(id);
    if token.is_empty() {
        collection_uri(id)
    } else {
        let mut s = base_uri();
        s.push_str(&token);
        s
    }
}

pub mod entrypoints {
    use super::*;

    /// `uri(id: u256) -> string bytes`
    pub fn uri(mut args: Args) -> String {
        let id: U256 = args.must_next("id argument is missing or invalid");
        super::uri(id)
    }
}

/// Export the base MRC1155 entry points with the metadata-aware `uri(id)`
/// (replaces [`crate::export_mrc1155!`], do not use both).
#[macro_export]
macro_rules! export_mrc1155_metadata {
    () => {
        $crate::__export_mrc1155_common!();
        #[::massa_sc_sdk::export(name = "uri")]
        pub fn __std_uri(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::String {
            $crate::mrc1155::metadata::entrypoints::uri(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn fallback_and_override() {
        mock::reset();
        super::super::internals::set_uri("ipfs://collection");
        assert_eq!(uri(U256::ONE), "ipfs://collection");
        set_base_uri("https://x/");
        set_uri(U256::ONE, "1.json");
        assert_eq!(uri(U256::ONE), "https://x/1.json");
        assert_eq!(uri(U256::from(2u64)), "ipfs://collection");
    }
}
