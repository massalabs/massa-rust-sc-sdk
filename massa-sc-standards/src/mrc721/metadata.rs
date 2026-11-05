//! MRC721 metadata extension: `uri(tokenId) = baseURI ++ tokenURI`.

use massa_sc_sdk::prelude::*;

/// Datastore key of the base URI.
pub const BASE_URI_KEY: &str = "BASE_URI";
/// Prefix of per-token URI entries: `TOKENS_URI ++ tokenId`.
pub const TOKENS_URI_KEY: &str = "TOKENS_URI";
/// Event key of URI changes.
pub const URI_EVENT: &str = "URI";

fn token_uri_key(id: U256) -> Vec<u8> {
    let mut k = Vec::with_capacity(TOKENS_URI_KEY.len() + 32);
    k.extend_from_slice(TOKENS_URI_KEY.as_bytes());
    k.extend_from_slice(&id.to_le_bytes());
    k
}

fn read_string(key: impl AsRef<[u8]>) -> String {
    storage::try_get(key).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

/// Set the base URI prepended to every token URI.
pub fn set_base_uri(base_uri: &str) {
    storage::set(BASE_URI_KEY, base_uri);
}

/// The base URI (empty when unset).
pub fn base_uri() -> String {
    read_string(BASE_URI_KEY)
}

/// The URI suffix of `id` (empty when unset).
pub fn token_uri(id: U256) -> String {
    read_string(token_uri_key(id))
}

/// Set the URI suffix of `id` and emit `URI:<uri>,<id>`.
pub fn set_uri(id: U256, uri: &str) {
    storage::set(token_uri_key(id), uri);
    generate_event(&create_event(URI_EVENT, &[uri, &id.to_string()]));
}

/// Full URI of `id`: base URI followed by the token URI.
pub fn uri(id: U256) -> String {
    let mut s = base_uri();
    s.push_str(&token_uri(id));
    s
}

/// Entry points taking serialized arguments.
pub mod entrypoints {
    use super::*;

    /// `uri(tokenId: u256) -> string bytes`
    pub fn uri(mut args: Args) -> String {
        let id: U256 = args.must_next("id argument is missing or invalid");
        super::uri(id)
    }
}

/// Export `uri(tokenId)`.
#[macro_export]
macro_rules! export_mrc721_metadata {
    () => {
        #[::massa_sc_sdk::export(name = "uri")]
        pub fn __std_uri(args: ::massa_sc_sdk::Args) -> ::massa_sc_sdk::prelude::String {
            $crate::mrc721::metadata::entrypoints::uri(args)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use massa_sc_sdk::mock;

    #[test]
    fn uris() {
        mock::reset();
        set_base_uri("ipfs://base/");
        set_uri(U256::from(3u64), "3.json");
        assert_eq!(uri(U256::from(3u64)), "ipfs://base/3.json");
        assert_eq!(entrypoints::uri(Args::new().add(U256::from(4u64))), "ipfs://base/");
        assert_eq!(mock::events(), vec!["URI:3.json,3"]);
    }
}
