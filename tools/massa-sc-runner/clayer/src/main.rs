//! `massa-sc-runner` on the C-Layer runtime: the same program, compiled
//! against `c-layer-sc-runtime` (which adds `assembly_script_hash_mimc`).

#[path = "../../src/cli.rs"]
mod cli;
#[allow(dead_code)]
#[path = "../../src/lib.rs"]
mod runner;

fn main() -> anyhow::Result<()> {
    cli::main()
}
