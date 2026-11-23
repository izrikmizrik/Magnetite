//! Gives the Windows executable Horizon's icon and its stack, as horizon-client's
//! (`packaging/windows/exe.rs`), and Magnetite's PC executable the same stack: its main thread
//! runs the game's guests too (`horizon_guest_stack::prepare_main`).
#[path = "../../packaging/windows/exe.rs"]
mod windows_exe;

fn main() {
    windows_exe::resources("horizon-client");
    // `magnetite` is built with the desktop features only (its `required-features`).
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        let target_os =
            std::env::var("CARGO_CFG_TARGET_OS").expect("Cargo must provide the build target OS");
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        for arg in windows_exe::executable_link::arguments(&target_os, &target_env) {
            println!("cargo:rustc-link-arg-bin=magnetite={arg}");
        }
    }
}
