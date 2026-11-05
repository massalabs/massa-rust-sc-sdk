//! `massa-sc-runner` command line (Massa runtime). The C-Layer flavour in
//! `clayer/` compiles the same `cli` module against `c-layer-sc-runtime`.
//!
//! ```text
//! massa-sc-runner build/counter.wasm constructor --args u64:10 --deploy \; increment --args u64:5 \; get
//! ```

mod cli;
use massa_sc_runner as runner;

fn main() -> anyhow::Result<()> {
    cli::main()
}
