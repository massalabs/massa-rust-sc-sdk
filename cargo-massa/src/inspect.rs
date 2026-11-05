//! Validation of a wasm module against what the Massa runtime accepts
//! (wasmer 4.3.6 with a restricted feature set, node limits, host imports)
//! and a human-readable report.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use wasm_encoder::RawSection;
use wasmparser::{
    CompositeInnerType, ExternalKind, Parser, Payload, TypeRef, ValType, Validator, WasmFeatures,
};

use crate::abi::{self, limits, Network};

/// Features enabled by `massa-sc-runtime` (`as_execution/mod.rs`): bulk
/// memory only, plus the proposals wasmparser always accepts in wasmer 4.3.6
/// (mutable globals, sign extension, saturating float-to-int).
pub fn massa_features() -> WasmFeatures {
    WasmFeatures::MVP
        | WasmFeatures::MUTABLE_GLOBAL
        | WasmFeatures::SIGN_EXTENSION
        | WasmFeatures::SATURATING_FLOAT_TO_INT
        | WasmFeatures::BULK_MEMORY
}

#[derive(Debug, Default)]
pub struct Report {
    pub size: usize,
    pub code_size: usize,
    pub data_size: usize,
    pub imports: Vec<(String, String)>,
    pub defined_functions: usize,
    pub exports: Vec<(String, String)>,
    pub custom_sections: Vec<(String, usize)>,
    pub initial_pages: u64,
    pub tables: usize,
    pub globals: usize,
    pub data_segments: usize,
    pub element_segments: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

fn val(v: &ValType) -> char {
    match v {
        ValType::I32 => 'i',
        ValType::I64 => 'I',
        ValType::F32 => 'g',
        ValType::F64 => 'f',
        ValType::V128 => 'v',
        ValType::Ref(_) => 'r',
    }
}

fn sig_string(params: &[ValType], results: &[ValType]) -> String {
    let p: String = params.iter().map(val).collect();
    let r: String = results.iter().map(val).collect();
    format!("({p})->({r})")
}

/// Inspect `bytes` and collect everything a node of `network` would complain about.
pub fn inspect(bytes: &[u8], network: Option<Network>) -> Result<Report> {
    let mut report = Report { size: bytes.len(), ..Default::default() };

    if bytes.len() > limits::MAX_BYTECODE_LENGTH {
        report.errors.push(format!(
            "bytecode is {} bytes, more than the {} bytes limit",
            bytes.len(),
            limits::MAX_BYTECODE_LENGTH
        ));
    }

    // 1. Validation with Massa's feature set.
    if let Err(e) = Validator::new_with_features(massa_features()).validate_all(bytes) {
        report.errors.push(format!(
            "module rejected by the Massa runtime validator: {e}\n  \
             (build for `wasm32v1-none`; `wasm32-unknown-unknown` output of Rust >= 1.82 \
             uses reference-types encodings Massa does not accept)"
        ));
    }

    // 2. Structure.
    let mut types: Vec<(Vec<ValType>, Vec<ValType>)> = Vec::new();
    let mut func_types: Vec<u32> = Vec::new(); // type index of every function (imports first)
    let mut import_funcs = 0usize;
    let mut exports: Vec<(String, ExternalKind, u32)> = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.context("malformed wasm")? {
            Payload::TypeSection(reader) => {
                for group in reader {
                    for sub in group?.into_types() {
                        match &sub.composite_type.inner {
                            CompositeInnerType::Func(ft) => {
                                types.push((ft.params().to_vec(), ft.results().to_vec()))
                            }
                            _ => types.push((Vec::new(), Vec::new())),
                        }
                    }
                }
            }
            Payload::ImportSection(reader) => {
                for import in reader {
                    let import = import?;
                    match import.ty {
                        TypeRef::Func(ty) => {
                            import_funcs += 1;
                            func_types.push(ty);
                            report.imports.push((import.module.to_string(), import.name.to_string()));
                        }
                        TypeRef::Memory(_) => report.errors.push(format!(
                            "imported memory {}.{}: Massa modules must define their own memory",
                            import.module, import.name
                        )),
                        other => report.errors.push(format!(
                            "unsupported import {}.{} ({other:?})",
                            import.module, import.name
                        )),
                    }
                }
            }
            Payload::FunctionSection(reader) => {
                for ty in reader {
                    func_types.push(ty?);
                    report.defined_functions += 1;
                }
            }
            Payload::TableSection(reader) => report.tables = reader.count() as usize,
            Payload::MemorySection(reader) => {
                for mem in reader {
                    let mem = mem?;
                    report.initial_pages = mem.initial;
                    if mem.initial > limits::MAX_MEMORY_PAGES {
                        report.errors.push(format!(
                            "initial memory is {} pages, the node allows at most {} (4 MiB)",
                            mem.initial,
                            limits::MAX_MEMORY_PAGES
                        ));
                    }
                }
            }
            Payload::GlobalSection(reader) => report.globals = reader.count() as usize,
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export?;
                    exports.push((export.name.to_string(), export.kind, export.index));
                }
            }
            Payload::ElementSection(reader) => report.element_segments = reader.count() as usize,
            Payload::DataSection(reader) => {
                report.data_segments = reader.count() as usize;
                report.data_size = reader.range().len();
            }
            Payload::CodeSectionStart { range, .. } => report.code_size = range.len(),
            Payload::CustomSection(reader) => {
                report.custom_sections.push((reader.name().to_string(), reader.data().len()));
            }
            _ => {}
        }
    }

    // 3. Imports must be known host functions with the right signature.
    for (i, (module, name)) in report.imports.iter().enumerate() {
        let ty = func_types.get(i).and_then(|&t| types.get(t as usize));
        let actual =
            ty.map(|(p, r)| (p.iter().map(val).collect::<String>(), r.iter().map(val).collect::<String>()));
        match abi::lookup(module, name) {
            None => report.errors.push(format!(
                "unknown host import `{module}.{name}`: the Massa runtime will fail to instantiate the module"
            )),
            Some(h) => {
                if let Some((p, r)) = &actual {
                    if p != h.params || r != h.results {
                        report.errors.push(format!(
                            "import `{module}.{name}` has signature ({p})->({r}), the host expects ({})->({})",
                            h.params, h.results
                        ));
                    }
                }
                if let Some(note) = h.availability.note() {
                    match network {
                        Some(net) if !h.availability.runs_on(net) => report
                            .errors
                            .push(format!("import `{name}` is not available on the {net:?} network: {note}")),
                        _ => report.warnings.push(format!("import `{name}`: {note}")),
                    }
                }
            }
        }
    }

    // 4. Exports: entry points must look like (i32)->(i32), ()->(i32), (i32)->() or ()->().
    let mut has_memory_export = false;
    let mut has_new = false;
    for (name, kind, index) in &exports {
        match kind {
            ExternalKind::Func => {
                let ty = func_types.get(*index as usize).and_then(|&t| types.get(t as usize));
                let sig = ty.map(|(p, r)| sig_string(p, r)).unwrap_or_default();
                report.exports.push((name.clone(), sig.clone()));
                if name == "__new" {
                    has_new = true;
                }
                if let Some((p, r)) = ty {
                    let ok_params = p.is_empty() || (p.len() == 1 && p[0] == ValType::I32);
                    let ok_results = r.is_empty() || (r.len() == 1 && r[0] == ValType::I32);
                    if !(name.starts_with("__") || (ok_params && ok_results)) {
                        report.warnings.push(format!(
                            "export `{name}` has signature {sig}: the host can only call (i32)->(i32), ()->(i32), (i32)->() or ()->()"
                        ));
                    }
                }
                if name.len() > limits::MAX_FUNCTION_NAME_LEN {
                    report.errors.push(format!(
                        "export name `{name}` is longer than {} bytes",
                        limits::MAX_FUNCTION_NAME_LEN
                    ));
                }
            }
            ExternalKind::Memory => has_memory_export = true,
            _ => {}
        }
    }
    if !has_memory_export {
        report.errors.push("the module does not export its `memory`".into());
    }
    if !has_new {
        report
            .errors
            .push("the module does not export `__new`: the host cannot pass parameters to it".into());
    }

    // 5. Node limits.
    let total_functions = import_funcs + report.defined_functions;
    if total_functions > limits::MAX_FUNCTIONS {
        report.errors.push(format!(
            "{total_functions} functions (imports included), the node allows at most {}",
            limits::MAX_FUNCTIONS
        ));
    }
    if report.imports.len() > limits::MAX_IMPORTS {
        report.errors.push(format!(
            "{} imports, the node allows at most {}",
            report.imports.len(),
            limits::MAX_IMPORTS
        ));
    }
    if exports.len() > limits::MAX_EXPORTS {
        report.errors.push(format!(
            "{} exports, the node allows at most {}",
            exports.len(),
            limits::MAX_EXPORTS
        ));
    }
    for (p, r) in &types {
        if p.len() + r.len() > limits::MAX_SIGNATURE_LEN {
            report.errors.push(format!(
                "a function signature has {} params/results, the node allows at most {}",
                p.len() + r.len(),
                limits::MAX_SIGNATURE_LEN
            ));
        }
    }
    if report.custom_sections.len() > limits::MAX_CUSTOM_SECTIONS {
        report.errors.push(format!(
            "{} custom sections ({}), the node allows at most {} (strip them, e.g. with `cargo massa build` or `strip = true`)",
            report.custom_sections.len(),
            report.custom_sections.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", "),
            limits::MAX_CUSTOM_SECTIONS
        ));
    }
    let custom_data: usize = report.custom_sections.iter().map(|(_, s)| s).sum();
    if custom_data > limits::MAX_CUSTOM_SECTIONS_DATA {
        report.errors.push(format!(
            "custom sections hold {custom_data} bytes, the node allows at most {}",
            limits::MAX_CUSTOM_SECTIONS_DATA
        ));
    }
    if report.tables > limits::MAX_TABLES {
        report.errors.push(format!(
            "{} tables, the node allows at most {}",
            report.tables,
            limits::MAX_TABLES
        ));
    }
    if report.globals > limits::MAX_GLOBALS {
        report.errors.push(format!(
            "{} globals, the node allows at most {}",
            report.globals,
            limits::MAX_GLOBALS
        ));
    }
    if report.data_segments > limits::MAX_PASSIVE_DATA {
        report.errors.push(format!(
            "{} data segments, the node allows at most {}",
            report.data_segments,
            limits::MAX_PASSIVE_DATA
        ));
    }
    if report.element_segments > limits::MAX_PASSIVE_ELEMENTS {
        report.errors.push(format!(
            "{} element segments, the node allows at most {}",
            report.element_segments,
            limits::MAX_PASSIVE_ELEMENTS
        ));
    }
    if total_functions > limits::MAX_FUNCTIONS * 3 / 4 {
        report.warnings.push(format!(
            "{total_functions} functions: getting close to the {} limit",
            limits::MAX_FUNCTIONS
        ));
    }

    Ok(report)
}

/// Drop every custom section (`name`, `producers`, `target_features`,
/// DWARF...): the node allows at most one and they only add bytes.
pub fn strip_custom_sections(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut module = wasm_encoder::Module::new();
    let mut stripped = false;
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload?;
        if matches!(payload, Payload::CustomSection(_)) {
            stripped = true;
            continue;
        }
        if let Some((id, range)) = payload.as_section() {
            module.section(&RawSection { id, data: &bytes[range] });
        }
    }
    if !stripped {
        return Ok(bytes.to_vec());
    }
    Ok(module.finish())
}

/// Human-readable summary.
pub fn print_report(report: &Report, verbose: bool) {
    println!(
        "  size: {} bytes (code {}, data {}), {} functions ({} imports + {} defined), memory {} pages",
        report.size,
        report.code_size,
        report.data_size,
        report.imports.len() + report.defined_functions,
        report.imports.len(),
        report.defined_functions,
        report.initial_pages
    );
    let entry_points: Vec<&str> = report
        .exports
        .iter()
        .filter(|(n, _)| !n.starts_with("__") && n != "memory")
        .map(|(n, _)| n.as_str())
        .collect();
    println!("  entry points: {}", entry_points.join(", "));
    let mut by_module: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (m, n) in &report.imports {
        by_module.entry(m.as_str()).or_default().push(n.strip_prefix("assembly_script_").unwrap_or(n));
    }
    for (m, names) in by_module {
        println!("  imports from `{m}`: {}", names.join(", "));
    }
    if verbose {
        for (n, sig) in &report.exports {
            println!("    export {n} {sig}");
        }
        for (n, s) in &report.custom_sections {
            println!("    custom section `{n}` ({s} bytes)");
        }
    }
    for w in &report.warnings {
        println!("  warning: {w}");
    }
    for e in &report.errors {
        println!("  ERROR: {e}");
    }
}
