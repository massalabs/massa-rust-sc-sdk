//! `cargo massa build`: compile every contract of the workspace (or the
//! selected packages) for `wasm32v1-none` with the right flags, strip and
//! optimize the output, validate it against the node limits and copy it to
//! the output directory.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::{MetadataCommand, Package};

use crate::inspect;

pub const TARGET: &str = "wasm32v1-none";

pub struct BuildOptions {
    pub packages: Vec<String>,
    pub features: Vec<String>,
    pub all_features: bool,
    pub no_default_features: bool,
    pub debug: bool,
    pub out_dir: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub stack_size: u32,
    pub extra_rustflags: Vec<String>,
    pub wasm_opt: bool,
    pub wasm_opt_level: String,
    pub verbose: bool,
    pub quiet: bool,
    pub cargo_args: Vec<String>,
}

const SEP: char = '\x1f';

fn rustflags(opts: &BuildOptions, workspace_root: &Path) -> String {
    let mut flags: Vec<String> = vec![
        "-C".into(),
        "target-feature=+bulk-memory,+sign-ext".into(),
        "-C".into(),
        format!("link-arg=-zstack-size={}", opts.stack_size),
    ];
    // Keep absolute paths (panic locations) out of the binary. Longest prefixes first.
    let mut remaps: Vec<(PathBuf, &str)> = Vec::new();
    if let Ok(home) = env::var("CARGO_HOME") {
        remaps.push((PathBuf::from(home), "/cargo"));
    } else if let Some(home) = env::var_os("HOME") {
        remaps.push((Path::new(&home).join(".cargo"), "/cargo"));
    }
    if let Ok(rustup) = env::var("RUSTUP_HOME") {
        remaps.push((PathBuf::from(rustup), "/rustup"));
    } else if let Some(home) = env::var_os("HOME") {
        remaps.push((Path::new(&home).join(".rustup"), "/rustup"));
    }
    remaps.push((workspace_root.to_path_buf(), "/src"));
    if let Some(home) = env::var_os("HOME") {
        remaps.push((PathBuf::from(home), "~"));
    }
    remaps.sort_by_key(|(p, _)| std::cmp::Reverse(p.as_os_str().len()));
    for (from, to) in remaps {
        flags.push(format!("--remap-path-prefix={}={}", from.display(), to));
    }
    if let Ok(extra) = env::var("MASSA_RUSTFLAGS") {
        flags.extend(extra.split_whitespace().map(String::from));
    }
    flags.extend(opts.extra_rustflags.iter().cloned());
    flags.join(&SEP.to_string())
}

fn target_installed() -> bool {
    Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == TARGET))
        .unwrap_or(true) // no rustup: let cargo tell us
}

fn wasm_opt_available() -> bool {
    Command::new("wasm-opt").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn run_wasm_opt(input: &[u8], level: &str) -> Result<Vec<u8>> {
    let dir = env::temp_dir().join(format!("cargo-massa-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let in_path = dir.join("in.wasm");
    let out_path = dir.join("out.wasm");
    std::fs::write(&in_path, input)?;
    let status = Command::new("wasm-opt")
        .arg(level)
        // Only the proposals the Massa runtime enables.
        .args([
            "--mvp-features",
            "--enable-mutable-globals",
            "--enable-sign-ext",
            "--enable-nontrapping-float-to-int",
            "--enable-bulk-memory",
            "--strip-debug",
            "--strip-producers",
            "--strip-target-features",
        ])
        .arg(&in_path)
        .arg("-o")
        .arg(&out_path)
        .status()
        .context("running wasm-opt")?;
    if !status.success() {
        bail!("wasm-opt failed");
    }
    let out = std::fs::read(&out_path)?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(out)
}

fn is_contract(pkg: &Package) -> bool {
    pkg.targets.iter().any(|t| t.crate_types.contains(&cargo_metadata::CrateType::CDyLib))
}

pub fn run(opts: BuildOptions) -> Result<()> {
    let mut cmd = MetadataCommand::new();
    if let Some(m) = &opts.manifest_path {
        cmd.manifest_path(m);
    }
    let metadata = cmd.exec().context("reading cargo metadata (is this a cargo project?)")?;
    let workspace_root = metadata.workspace_root.as_std_path().to_path_buf();

    // Packages to build: explicit, else the root package, else every workspace
    // member that produces a cdylib.
    let members: Vec<&Package> =
        metadata.workspace_packages().into_iter().filter(|p| is_contract(p)).collect();
    let selected: Vec<&Package> = if !opts.packages.is_empty() {
        opts.packages
            .iter()
            .map(|name| {
                members.iter().copied().find(|p| &p.name == name).ok_or_else(|| {
                    anyhow!("package `{name}` not found or not a contract (cdylib) in this workspace")
                })
            })
            .collect::<Result<_>>()?
    } else if let Some(root) = metadata.root_package().filter(|p| is_contract(p)) {
        vec![root]
    } else {
        members.clone()
    };
    if selected.is_empty() {
        bail!("no contract found: a contract is a package with `crate-type = [\"cdylib\"]`");
    }

    if !target_installed() {
        bail!("the `{TARGET}` target is not installed: run `rustup target add {TARGET}`");
    }

    let profile = if opts.debug { "debug" } else { "release" };
    let mut cargo = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cargo.arg("build").arg("--target").arg(TARGET);
    if !opts.debug {
        cargo.arg("--release");
    }
    if let Some(m) = &opts.manifest_path {
        cargo.arg("--manifest-path").arg(m);
    }
    for p in &selected {
        cargo.arg("-p").arg(&p.name);
    }
    if !opts.features.is_empty() {
        cargo.arg("--features").arg(opts.features.join(","));
    }
    if opts.all_features {
        cargo.arg("--all-features");
    }
    if opts.no_default_features {
        cargo.arg("--no-default-features");
    }
    if opts.quiet {
        cargo.arg("--quiet");
    }
    cargo.args(&opts.cargo_args);
    cargo.env("CARGO_ENCODED_RUSTFLAGS", rustflags(&opts, &workspace_root));
    if opts.verbose {
        eprintln!("running: {cargo:?}");
    }
    let status = cargo.status().context("running cargo build")?;
    if !status.success() {
        bail!("cargo build failed");
    }

    let target_dir = metadata.target_directory.as_std_path().join(TARGET).join(profile);
    let out_dir =
        if opts.out_dir.is_absolute() { opts.out_dir.clone() } else { workspace_root.join(&opts.out_dir) };
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let use_wasm_opt = opts.wasm_opt && wasm_opt_available();
    if opts.wasm_opt && !use_wasm_opt && !opts.quiet {
        eprintln!("note: `wasm-opt` not found, skipping post-optimization (install binaryen or `cargo install wasm-opt` for ~10-20% smaller modules)");
    }

    let mut failed = false;
    for pkg in &selected {
        for target in
            pkg.targets.iter().filter(|t| t.crate_types.contains(&cargo_metadata::CrateType::CDyLib))
        {
            let file_name = format!("{}.wasm", target.name.replace('-', "_"));
            let artifact = target_dir.join(&file_name);
            let bytes = std::fs::read(&artifact)
                .with_context(|| format!("reading {} (did the build produce it?)", artifact.display()))?;
            let raw_size = bytes.len();
            let mut bytes = inspect::strip_custom_sections(&bytes)?;
            if use_wasm_opt {
                match run_wasm_opt(&bytes, &opts.wasm_opt_level) {
                    Ok(optimized) => bytes = inspect::strip_custom_sections(&optimized)?,
                    Err(e) => eprintln!("warning: wasm-opt failed ({e}), keeping the unoptimized module"),
                }
            }
            let out_path = out_dir.join(&file_name);
            std::fs::write(&out_path, &bytes).with_context(|| format!("writing {}", out_path.display()))?;
            let report = inspect::inspect(&bytes)?;
            println!(
                "{} -> {} ({} bytes{})",
                pkg.name,
                out_path.display(),
                bytes.len(),
                if raw_size != bytes.len() {
                    format!(", {raw_size} before post-processing")
                } else {
                    String::new()
                }
            );
            inspect::print_report(&report, opts.verbose);
            if !report.errors.is_empty() {
                failed = true;
            }
        }
    }
    if failed {
        bail!("at least one module would be rejected by a Massa node");
    }
    Ok(())
}
