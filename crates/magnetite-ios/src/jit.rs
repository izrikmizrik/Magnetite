//! Whether this run may compile code at all (`BootConfig::jit`): iOS allows a JIT only to a
//! process a debugger — or a JIT enabler that attaches like one — is attached to. The kernel says
//! so through `csops`'s status flags, whose `CS_DEBUGGED` bit is set exactly in that case.
//!
//! The two functions are declared here rather than taken from a crate: libSystem exports them,
//! and `csops` has no Rust binding in anything this crate depends on.

use std::ffi::c_void;

/// `CS_DEBUGGED`: the process is being debugged (or a JIT enabler has made the kernel treat it so).
const CS_DEBUGGED: u32 = 0x1000_0000;
/// `CS_OPS_STATUS`, the operation that fills the flags out-parameter.
const CS_OPS_STATUS: u32 = 0;
/// The flags fit in a `u32`; `csops` is handed its byte count.
const FLAGS_SIZE: usize = std::mem::size_of::<u32>();

// SAFETY: libSystem's declarations, as `<sys/codesign.h>` and `<unistd.h>` give them.
unsafe extern "C" {
    fn csops(pid: i32, ops: u32, useraddr: *mut c_void, usersize: usize) -> i32;
    fn getpid() -> i32;
}

/// Whether the system lets this process compile code: a debugger attached, or a JIT enabler
/// standing in for one.
///
/// This is a syscall, so it works off the main thread and costs nothing to call.
pub fn enabled() -> bool {
    let mut flags: u32 = 0;
    // SAFETY: the flags out-parameter is this call's own `u32`, whose address and byte count are
    // what `csops` is documented to write through; both pointers are valid for the call.
    let status = unsafe {
        csops(
            getpid(),
            CS_OPS_STATUS,
            &mut flags as *mut u32 as *mut c_void,
            FLAGS_SIZE,
        )
    };
    status == 0 && (flags & CS_DEBUGGED) != 0
}
