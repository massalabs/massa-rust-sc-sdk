//! Command-line front end of [`massa_sc_runner`], shared with the C-Layer flavour (`clayer/`).

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};

use crate::runner::{CallOptions, Runner, DEFAULT_CONTRACT};

const USAGE: &str = "\
usage: massa-sc-runner <module.wasm> [global options] <step> [\\; <step> ...]

The module is deployed at the default contract address (no constructor call);
each step is a function call on it, `main` runs it as an ExecuteSC operation.

global options:
  --contract <name>=<file.wasm>  deploy another module; `@name` then stands for its address
  --chain-id <N>                 chain id reported to the contracts (default 77, sandbox)
  --opds <key>=<value>           operation datastore entry (values: hex, s:text, args:<spec>, @file)
  --ds <key>=<value>             initial datastore entry of the main module
  --balance <nanoMAS>            initial balance of the main module (default 0)
  --gas <N>                      gas limit per call (default 1_000_000_000)
  --quiet                        do not print the final state

steps:
  <function> [call options]      call an entry point
  --swap <@name|address>=<file>  replace the bytecode at an address, keeping its datastore

call options:
  --at <@name|address>           target contract (default: the main module)
  --args <spec>                  typed arguments: s:text,u64:42,u256:1000,bool:true,bytes:<hex>,addr:@name
  --hex <hex>                    raw serialized arguments
  --coins <nanoMAS>              coins sent with the call
  --caller <@name|address>       caller address (default: a funded AU address)
  --deploy                       the caller has write access to the target (deployment context)
";

fn parse_hex(s: &str) -> Result<Vec<u8>> {
    let s = s.trim_start_matches("0x");
    if s.len() & 1 == 1 {
        bail!("odd hex length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| anyhow!("bad hex: {e}")))
        .collect()
}

/// Encode a comma-separated list of typed values with the `Args` layout.
fn encode_args(spec: &str, names: &HashMap<String, String>) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    if spec.trim().is_empty() {
        return Ok(out);
    }
    for item in spec.split(',') {
        let (ty, val) =
            item.split_once(':').ok_or_else(|| anyhow!("argument `{item}` must be `type:value`"))?;
        match ty.trim() {
            "s" | "str" | "string" | "addr" | "address" => {
                let val = resolve(val, names);
                out.extend_from_slice(&(val.len() as u32).to_le_bytes());
                out.extend_from_slice(val.as_bytes());
            }
            "bool" => out.push(matches!(val, "true" | "1") as u8),
            "u8" => out.push(val.parse::<u8>()?),
            "i8" => out.push(val.parse::<i8>()? as u8),
            "u16" => out.extend_from_slice(&val.parse::<u16>()?.to_le_bytes()),
            "i16" => out.extend_from_slice(&val.parse::<i16>()?.to_le_bytes()),
            "u32" => out.extend_from_slice(&val.parse::<u32>()?.to_le_bytes()),
            "i32" => out.extend_from_slice(&val.parse::<i32>()?.to_le_bytes()),
            "u64" => out.extend_from_slice(&val.parse::<u64>()?.to_le_bytes()),
            "i64" => out.extend_from_slice(&val.parse::<i64>()?.to_le_bytes()),
            "u128" => out.extend_from_slice(&val.parse::<u128>()?.to_le_bytes()),
            "i128" => out.extend_from_slice(&val.parse::<i128>()?.to_le_bytes()),
            "u256" => out.extend_from_slice(&val.parse::<ethnum::U256>()?.to_le_bytes()),
            "f32" => out.extend_from_slice(&val.parse::<f32>()?.to_le_bytes()),
            "f64" => out.extend_from_slice(&val.parse::<f64>()?.to_le_bytes()),
            "bytes" => {
                let b = parse_hex(val)?;
                out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                out.extend_from_slice(&b);
            }
            "hex" => out.extend_from_slice(&parse_hex(val)?),
            other => bail!("unknown argument type `{other}`"),
        }
    }
    Ok(out)
}

/// `@name` -> address of a `--contract`.
fn resolve(s: &str, names: &HashMap<String, String>) -> String {
    match s.strip_prefix('@') {
        Some(name) => names.get(name).cloned().unwrap_or_else(|| s.to_string()),
        None => s.to_string(),
    }
}

fn parse_value(s: &str, names: &HashMap<String, String>) -> Result<Vec<u8>> {
    if let Some(rest) = s.strip_prefix("s:") {
        Ok(rest.as_bytes().to_vec())
    } else if let Some(rest) = s.strip_prefix("hex:") {
        parse_hex(rest)
    } else if let Some(rest) = s.strip_prefix("args:") {
        encode_args(rest, names)
    } else if let Some(rest) = s.strip_prefix('@') {
        std::fs::read(rest).with_context(|| format!("reading {rest}"))
    } else {
        parse_hex(s)
    }
}

/// Call parameter: raw bytes, or a typed spec encoded once every `@name` is known.
enum Param {
    Raw(Vec<u8>),
    Spec(String),
}

enum Step {
    Call { function: String, target: String, param: Param, opts: CallOptions },
    Swap { target: String, file: String },
}

pub fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 || args[1] == "-h" || args[1] == "--help" {
        eprint!("{USAGE}");
        std::process::exit(2);
    }
    let wasm = std::fs::read(&args[1]).with_context(|| format!("reading {}", args[1]))?;

    let mut runner = Runner::new();
    runner.trace = false;
    let mut names: HashMap<String, String> = HashMap::new();
    names.insert("main".into(), DEFAULT_CONTRACT.into());
    let mut quiet = false;
    let mut steps: Vec<Step> = Vec::new();
    let mut extra_contracts: Vec<(String, String)> = Vec::new();
    let mut initial_ds: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut raw_opds: Vec<(String, String)> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        let a = args[i].as_str();
        let next = |i: usize| -> Result<&String> {
            args.get(i + 1).ok_or_else(|| anyhow!("missing value after {a}"))
        };
        fn last_call<'s>(
            steps: &'s mut [Step],
            option: &str,
        ) -> Result<(&'s mut String, &'s mut Param, &'s mut CallOptions)> {
            match steps.last_mut() {
                Some(Step::Call { target, param, opts, .. }) => Ok((target, param, opts)),
                _ => bail!("{option} must follow a function name"),
            }
        }
        match a {
            "--contract" => {
                let (name, file) =
                    next(i)?.split_once('=').ok_or_else(|| anyhow!("--contract expects name=file"))?;
                extra_contracts.push((name.to_string(), file.to_string()));
                i += 2;
            }
            "--chain-id" => {
                runner.set_chain_id(next(i)?.parse()?);
                i += 2;
            }
            "--gas" => {
                runner.gas_limit = next(i)?.parse()?;
                i += 2;
            }
            "--balance" => {
                runner.set_balance(DEFAULT_CONTRACT, next(i)?.parse()?);
                i += 2;
            }
            "--quiet" => {
                quiet = true;
                i += 1;
            }
            "--opds" | "--ds" => {
                let (k, v) = next(i)?.split_once('=').ok_or_else(|| anyhow!("{a} expects key=value"))?;
                if a == "--opds" {
                    raw_opds.push((k.to_string(), v.to_string()));
                } else {
                    initial_ds.push((parse_value(k, &names)?, parse_value(v, &names)?));
                }
                i += 2;
            }
            "--swap" => {
                let (target, file) =
                    next(i)?.split_once('=').ok_or_else(|| anyhow!("--swap expects address=file"))?;
                steps.push(Step::Swap { target: target.to_string(), file: file.to_string() });
                i += 2;
            }
            "--at" => {
                let v = next(i)?.clone();
                last_call(&mut steps, a)?.0.clone_from(&v);
                i += 2;
            }
            "--args" => {
                let v = next(i)?.clone();
                *last_call(&mut steps, a)?.1 = Param::Spec(v);
                i += 2;
            }
            "--hex" => {
                let v = parse_hex(next(i)?)?;
                *last_call(&mut steps, a)?.1 = Param::Raw(v);
                i += 2;
            }
            "--coins" => {
                let v: u64 = next(i)?.parse()?;
                last_call(&mut steps, a)?.2.coins = v;
                i += 2;
            }
            "--caller" => {
                let v = next(i)?.clone();
                last_call(&mut steps, a)?.2.caller = v;
                i += 2;
            }
            "--deploy" => {
                last_call(&mut steps, a)?.2.deploy_context = true;
                i += 1;
            }
            ";" => i += 1,
            other if other.starts_with("--") => bail!("unknown option {other}"),
            other => {
                steps.push(Step::Call {
                    function: other.to_string(),
                    target: "@main".to_string(),
                    param: Param::Raw(Vec::new()),
                    opts: CallOptions::default(),
                });
                i += 1;
            }
        }
    }
    if steps.is_empty() {
        bail!("no step to run");
    }

    // Deploy the modules (no constructor call: steps decide what to run).
    runner.set_bytecode(DEFAULT_CONTRACT, &wasm).map_err(|e| anyhow!("{}: {e}", args[1]))?;
    for (k, v) in initial_ds {
        runner.set_datastore(DEFAULT_CONTRACT, k, v);
    }
    for (name, file) in &extra_contracts {
        let bytes = std::fs::read(file).with_context(|| format!("reading {file}"))?;
        let address = runner.new_address();
        runner.set_bytecode(&address, &bytes).map_err(|e| anyhow!("{file}: {e}"))?;
        names.insert(name.clone(), address);
    }
    for (k, v) in raw_opds {
        runner.set_op_datastore(parse_value(&k, &names)?, parse_value(&v, &names)?);
    }
    eprintln!("contracts: {}", names.iter().map(|(n, a)| format!("@{n}={a}")).collect::<Vec<_>>().join(" "));

    let mut failed = false;
    for step in steps {
        match step {
            Step::Swap { target, file } => {
                let bytes = std::fs::read(&file).with_context(|| format!("reading {file}"))?;
                let target = resolve(&target, &names);
                runner.set_bytecode(&target, &bytes).map_err(|e| anyhow!("{file}: {e}"))?;
                println!("== swapped bytecode of {target} for {file} ({} bytes)", bytes.len());
            }
            Step::Call { function, target, param, mut opts } => {
                let param = match param {
                    Param::Raw(bytes) => bytes,
                    Param::Spec(spec) => encode_args(&spec, &names)?,
                };
                let target = resolve(&target, &names);
                opts.caller = resolve(&opts.caller, &names);
                println!(
                    "== {function}({}) at {target} coins={} caller={}",
                    hex(&param),
                    opts.coins,
                    opts.caller
                );
                let t = std::time::Instant::now();
                let res = if function == "main" {
                    let caller = opts.caller.clone();
                    runner.execute(&caller, &runner.bytecode(&target))
                } else {
                    runner.call_with(&target, &function, &param, opts)
                };
                match res {
                    Ok(out) => {
                        println!(
                            "   ok: return={} {:?} gas={} ({:.1?})",
                            hex(&out.ret),
                            String::from_utf8_lossy(&out.ret),
                            out.gas,
                            t.elapsed()
                        );
                        for e in &out.events {
                            println!("   event: {e}");
                        }
                        for m in &out.async_messages {
                            println!("   async message: {m}");
                        }
                    }
                    Err(e) => {
                        failed = true;
                        println!("   ERROR: {e}");
                    }
                }
            }
        }
    }
    if !quiet {
        println!("== final state");
        for acc in runner.accounts() {
            let addr = &acc.address;
            let name =
                names.iter().find(|(_, a)| *a == addr).map(|(n, _)| format!(" (@{n})")).unwrap_or_default();
            println!(
                "{addr}{name}: balance={} bytecode={}B datastore={} keys",
                acc.balance,
                acc.bytecode_len,
                acc.datastore.len()
            );
            for (k, v) in &acc.datastore {
                let ks = String::from_utf8_lossy(k);
                let kd = if !k.is_empty() && ks.chars().all(|c| !c.is_control() && c != '\u{FFFD}') {
                    format!("{ks:?}")
                } else {
                    format!("0x{}", hex(k))
                };
                println!("   {kd} => {}", if v.len() <= 64 { hex(v) } else { format!("{} bytes", v.len()) });
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
