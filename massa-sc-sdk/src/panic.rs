//! Panic handling: a panic aborts the execution through the host `abort` ABI so
//! that the message shows up in the operation's error event
//! (`error: <message> at <file>:<line> col: <col>`), exactly like an
//! AssemblyScript `assert`/`throw`.

/// Abort the current execution with a message. The host reverts every state
/// change made during the execution (storage, coins, events).
#[cold]
#[inline(never)]
pub fn abort(message: &str) -> ! {
    abort_at(message, "", 0, 0)
}

#[cold]
#[inline(never)]
pub(crate) fn abort_at(message: &str, file: &str, line: u32, col: u32) -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        let msg = crate::memory::HostBuf::string(message);
        let file = crate::memory::HostBuf::string(file);
        unsafe { crate::abi::env::abort(msg.ptr(), file.ptr(), line as i32, col as i32) };
        // The host turns `abort` into a trap: this is never reached.
        core::arch::wasm32::unreachable()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if file.is_empty() {
            panic!("{message}")
        } else {
            panic!("{message} at {file}:{line} col: {col}")
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "panic-handler"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let (file, line, col) = match info.location() {
        Some(l) => (l.file(), l.line(), l.column()),
        None => ("", 0, 0),
    };
    #[cfg(feature = "panic-message")]
    {
        use core::fmt::Write;
        let mut msg = alloc::string::String::new();
        let _ = write!(msg, "{}", info.message());
        abort_at(&msg, file, line, col)
    }
    #[cfg(not(feature = "panic-message"))]
    {
        abort_at(info.message().as_str().unwrap_or("panic"), file, line, col)
    }
}
