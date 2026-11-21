//! A GNU build id in the library, so a tombstone's frames can be symbolized against the
//! unstripped build (the Android port's tombstone reports).

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,--build-id=sha1");
    }
}
