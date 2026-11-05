//! `cargo massa`: build, check and scaffold Massa smart contracts in Rust.

mod abi;
mod build_cmd;
mod inspect;
mod scaffold;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    /// Build, check and scaffold Massa smart contracts written in Rust.
    #[command(subcommand)]
    Massa(Command),
}

#[derive(Subcommand)]
enum Command {
    /// Compile contracts to `build/<name>.wasm`, optimized and validated for the node.
    Build(BuildArgs),
    /// Validate an existing `.wasm` against what a Massa node accepts and print a report.
    Check(CheckArgs),
    /// Create a new contract project.
    New(NewArgs),
}

#[derive(Args)]
struct BuildArgs {
    /// Package(s) to build (default: the current package, or every cdylib of the workspace).
    #[arg(short, long = "package", value_name = "SPEC")]
    packages: Vec<String>,
    /// Cargo features to enable.
    #[arg(short = 'F', long, value_delimiter = ',')]
    features: Vec<String>,
    #[arg(long)]
    all_features: bool,
    #[arg(long)]
    no_default_features: bool,
    /// Build with the `dev` profile (still stripped and validated).
    #[arg(long)]
    debug: bool,
    /// Where to write the final modules.
    #[arg(long, default_value = "build")]
    out_dir: PathBuf,
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    /// Shadow stack size in bytes (Massa modules get 4 MiB of memory in total).
    #[arg(long, default_value_t = 256 * 1024)]
    stack_size: u32,
    /// Extra rustc flags (also read from `MASSA_RUSTFLAGS`).
    #[arg(long = "rustflag", value_name = "FLAG")]
    rustflags: Vec<String>,
    /// Skip the `wasm-opt` post-optimization.
    #[arg(long)]
    no_wasm_opt: bool,
    /// `wasm-opt` level: -Os (default), -Oz, -O3...
    #[arg(long, default_value = "-Os")]
    wasm_opt_level: String,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short, long)]
    quiet: bool,
    /// Fail when the module imports a host function this network lacks
    /// (`massa`: no `mimc`; `clayer`: no paginated datastore keys; `any`: neither).
    #[arg(long, value_enum)]
    network: Option<abi::Network>,
    /// Extra arguments passed to `cargo build` (after `--`).
    #[arg(last = true)]
    cargo_args: Vec<String>,
}

#[derive(Args)]
struct CheckArgs {
    /// Module(s) to inspect.
    #[arg(required = true)]
    files: Vec<PathBuf>,
    /// Fail when the module imports a host function this network lacks.
    #[arg(long, value_enum)]
    network: Option<abi::Network>,
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Args)]
struct NewArgs {
    /// Project (and crate) name.
    name: String,
    /// `counter` (default), `token` (MRC20 with massa-sc-standards) or `empty`.
    #[arg(long, default_value = "counter")]
    template: scaffold::Template,
    /// `massa-sc-sdk` dependency spec, e.g. `{ version = "0.2" }` or `{ path = "../massa-sc-sdk" }`.
    #[arg(long)]
    sdk: Option<String>,
    /// `massa-sc-standards` dependency spec.
    #[arg(long)]
    standards: Option<String>,
}

fn main() -> Result<()> {
    let Cargo::Massa(cmd) = Cargo::parse();
    match cmd {
        Command::Build(a) => build_cmd::run(build_cmd::BuildOptions {
            packages: a.packages,
            features: a.features,
            all_features: a.all_features,
            no_default_features: a.no_default_features,
            debug: a.debug,
            out_dir: a.out_dir,
            manifest_path: a.manifest_path,
            stack_size: a.stack_size,
            extra_rustflags: a.rustflags,
            wasm_opt: !a.no_wasm_opt,
            wasm_opt_level: a.wasm_opt_level,
            verbose: a.verbose,
            quiet: a.quiet,
            cargo_args: a.cargo_args,
            network: a.network,
        }),
        Command::Check(a) => {
            let mut failed = false;
            for file in &a.files {
                let bytes = std::fs::read(file).with_context(|| format!("reading {}", file.display()))?;
                let report = inspect::inspect(&bytes, a.network)?;
                println!("{}", file.display());
                inspect::print_report(&report, a.verbose);
                failed |= !report.errors.is_empty();
            }
            if failed {
                std::process::exit(1);
            }
            Ok(())
        }
        Command::New(a) => scaffold::run(&a.name, a.template, a.sdk.as_deref(), a.standards.as_deref()),
    }
}
