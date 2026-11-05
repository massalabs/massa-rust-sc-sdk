#!/usr/bin/env bash
# Build every fixture of the interoperability suite into interop/build/:
#   as_<name>.wasm  AssemblyScript (asc + @massalabs/massa-as-sdk + @massalabs/sc-standards)
#   rs_<name>.wasm  Rust (cargo massa build)
# Set SC_STANDARDS=/path/to/massa-standards/smart-contracts to compile the
# AssemblyScript fixtures against a checkout instead of the published package.
# Extra arguments go to `cargo massa build` (e.g. --network any).
set -euo pipefail
cd "$(dirname "$0")"

( cd as
  # A checkout and the published package share a version number: always start clean.
  rm -rf node_modules/@massalabs/sc-standards package-lock.json
  npm install --silent --no-audit --no-fund
  if [[ -n "${SC_STANDARDS:-}" ]]; then npm install --silent --no-save --no-audit --no-fund "$SC_STANDARDS"; fi
  npm run --silent build )

( cd rust
  cargo run -q --manifest-path ../../Cargo.toml -p cargo-massa -- massa build --out-dir ../build --no-wasm-opt "$@" )
