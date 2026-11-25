//! A guest-sized stack for a thread whose own stack the platform fixes: iOS's main thread, where
//! UIKit runs the event loop and with it the whole game, has 1 MiB, which the game's start
//! overflows (`Game::new` needs more than 2 MiB, which is why Android gives its loop thread the
//! guest stack). The thread can't be swapped for a bigger one (winit runs `UIApplicationMain` on
//! it), and `-Wl,-stack_size` is not something iOS is known to honour, so the thread itself moves
//! onto a mapped stack of [`REQUIRED`] bytes with a guard page below, and back when `body` returns.

use super::REQUIRED;
use std::cell::Cell;

thread_local! {
    /// Whether this thread runs on an [`OwnStack`] now: its stack then has the guest's room
    /// whatever the thread's own reservation says ([`super::check_current`]).
    static ON_OWN_STACK: Cell<bool> = const { Cell::new(false) };
}

/// Whether the calling thread runs on an [`OwnStack`].
pub(super) fn on_own_stack() -> bool {
    ON_OWN_STACK.with(Cell::get)
}

/// A mapped stack of [`REQUIRED`] bytes (address space; pages are taken as they are touched),
/// with a guard page below it.
pub struct OwnStack {
    mapping: *mut libc::c_void,
    /// The whole mapping: the guard page and the stack above it.
    len: usize,
    guard: usize,
}

impl OwnStack {
    /// Maps the stack and its guard page.
    pub fn map() -> std::io::Result<Self> {
        // SAFETY: sysconf only reads a system setting.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        let guard = usize::try_from(page)
            .ok()
            .filter(|page| page.is_power_of_two())
            .ok_or_else(|| std::io::Error::other("sysconf(_SC_PAGESIZE) failed"))?;
        let len = REQUIRED
            .div_ceil(guard)
            .checked_mul(guard)
            .and_then(|stack| stack.checked_add(guard))
            .ok_or_else(|| std::io::Error::other("the stack's size overflows"))?;
        // SAFETY: a fresh private anonymous mapping; nothing else refers to its range.
        let mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if mapping == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        let stack = Self {
            mapping,
            len,
            guard,
        };
        // The stack grows down: its lowest page faults instead of running into whatever lies
        // below the mapping.
        // SAFETY: the first page of the mapping made above.
        if unsafe { libc::mprotect(mapping, guard, libc::PROT_NONE) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(stack)
    }

    /// Runs `body` on this stack, on the calling thread, and comes back to the thread's own stack
    /// when it returns; a panic in `body` resumes there. While `body` runs, [`super::check_current`]
    /// accepts this thread for guests. A `body` that never returns (an event loop that ends the
    /// process) keeps the stack for the process's life.
    pub fn run<R>(self, body: impl FnOnce() -> R) -> R {
        let size = self.len - self.guard;
        // SAFETY: the stack's lowest usable address, page aligned, inside the mapping.
        let base = unsafe { self.mapping.cast::<u8>().add(self.guard) };
        let was = ON_OWN_STACK.with(|on| on.replace(true));
        // SAFETY: `base` and `size` are page aligned (a multiple of the 16 bytes AArch64 and
        // x86-64 align a stack to), the guard page sits below `base`, and the callback can't
        // unwind past the switch: a panic is caught on the new stack and resumed below, back on
        // the thread's own one.
        let outcome = unsafe {
            psm::on_stack(base, size, || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(body))
            })
        };
        ON_OWN_STACK.with(|on| on.set(was));
        drop(self);
        match outcome {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

impl Drop for OwnStack {
    fn drop(&mut self) {
        // SAFETY: the mapping made in `map`, which nothing runs on any more: `run` only drops it
        // once back on the thread's own stack.
        unsafe { libc::munmap(self.mapping, self.len) };
    }
}

#[cfg(test)]
mod tests {
    use super::OwnStack;

    /// A 1 MiB thread, as iOS's main thread is: refused for guests and unable to hold a 4 MiB
    /// frame on its own stack, it takes both once on its own one, and is its old self after.
    #[test]
    fn a_small_thread_runs_deep_work_on_its_own_stack() {
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| {
                assert!(crate::check_current().is_err());
                let stack = OwnStack::map().unwrap();
                let (sum, admitted) = stack.run(|| {
                    // More than the thread's whole stack, written through so it's really used.
                    let frame = std::hint::black_box([1u8; 4 * 1024 * 1024]);
                    let sum: u64 = frame.iter().map(|&b| u64::from(b)).sum();
                    (sum, crate::check_current().is_ok())
                });
                assert_eq!(sum, 4 * 1024 * 1024);
                assert!(admitted, "a thread on its own stack has the guest's room");
                assert!(crate::check_current().is_err());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// A panic on the stack comes back to the caller with its payload, on the thread's own stack.
    #[test]
    fn a_panic_resumes_on_the_threads_own_stack() {
        let payload = std::panic::catch_unwind(|| {
            OwnStack::map().unwrap().run(|| std::panic::panic_any(7u32));
        })
        .unwrap_err();
        assert_eq!(payload.downcast_ref::<u32>(), Some(&7));
        assert!(!super::on_own_stack());
    }
}
