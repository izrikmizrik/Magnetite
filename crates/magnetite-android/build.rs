//! How the Android library is linked: complete, or not at all.

use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }
    // A GNU build id, so a tombstone's frames can be symbolized against the unstripped build (the
    // Android port's tombstone reports).
    println!("cargo:rustc-cdylib-link-arg=-Wl,--build-id=sha1");
    // A shared library may be linked with symbols nobody defines, and then the system refuses to
    // load it: the app dies at its start. Such a build fails here instead.
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    // The C compiler's own runtime: C code calls into it (LuaJIT's `__clear_cache`), and rustc links
    // with `-nodefaultlibs`, which leaves it out.
    println!("cargo:rerun-if-env-changed=CC_aarch64_linux_android");
    match runtime() {
        Ok(path) => println!("cargo:rustc-cdylib-link-arg={path}"),
        // Nothing is linked by `cargo check`; a build that needs it stops at the linker.
        Err(why) => println!("cargo:warning=the NDK's compiler runtime is not linked: {why}"),
    }
}

/// The path of `libclang_rt.builtins` for the target, as the NDK's clang names it.
fn runtime() -> Result<String, String> {
    let clang = std::env::var("CC_aarch64_linux_android")
        .map_err(|_| "CC_aarch64_linux_android does not name the NDK's clang".to_owned())?;
    let output = Command::new(&clang)
        .args(["-rtlib=compiler-rt", "--print-libgcc-file-name"])
        .output()
        .map_err(|e| format!("{clang}: {e}"))?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if output.status.success() && std::path::Path::new(&path).is_file() {
        Ok(path)
    } else {
        Err(format!("{clang} names no runtime ({path:?})"))
    }
}
