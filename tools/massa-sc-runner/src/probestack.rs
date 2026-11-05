//! `__rust_probestack` for wasmer-vm 4.x on x86_64 Linux.
//!
//! wasmer-vm 4 wires Cranelift's out-of-line stack probe to the Rust runtime
//! symbol `__rust_probestack`. Rust 1.90+ no longer exports that symbol under
//! its plain name (`compiler_builtins` mangles it), so recent toolchains fail
//! to link with `undefined symbol: __rust_probestack` whenever the reference
//! is not dead-stripped. This is the same routine as `compiler_builtins`'
//! (touch the stack one page at a time so that a large frame cannot jump over
//! the guard page), declared weak so that a toolchain which still provides the
//! symbol wins.
//!
//! Register convention (set by the generated code): `rax` holds the frame
//! size; the routine preserves every register except `r11`.

core::arch::global_asm!(
    ".pushsection .text.__rust_probestack,\"ax\",@progbits",
    ".weak __rust_probestack",
    ".type __rust_probestack,@function",
    ".p2align 4",
    "__rust_probestack:",
    ".cfi_startproc",
    "push rbp",
    ".cfi_adjust_cfa_offset 8",
    ".cfi_offset rbp, -16",
    "mov rbp, rsp",
    ".cfi_def_cfa_register rbp",
    "mov r11, rax",
    "cmp r11, 0x1000",
    "jna 3f",
    "2:",
    "sub rsp, 0x1000",
    "test qword ptr [rsp + 8], rsp",
    "sub r11, 0x1000",
    "cmp r11, 0x1000",
    "ja 2b",
    "3:",
    "sub rsp, r11",
    "test qword ptr [rsp + 8], rsp",
    "add rsp, rax",
    "leave",
    ".cfi_def_cfa_register rsp",
    ".cfi_adjust_cfa_offset -8",
    "ret",
    ".cfi_endproc",
    ".size __rust_probestack, . - __rust_probestack",
    ".popsection",
);
