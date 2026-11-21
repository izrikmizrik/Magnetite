//! Magnetite iOS: the app's executable. iOS starts it in `/` inside a read-only bundle and gives
//! it no arguments, so it hands the shared core the app's own folders, then the core makes the
//! process's one event loop on this, the main thread, as on a desktop.
//!
//! Stage 0 of the port (docs/unification/ios-entry.md): the game starts and draws. Signing in, the
//! game's prepared files (and the boot screen that keeps a long first start inside iOS's launch
//! time), touch controls and the in-game launcher come in later stages. Without prepared files
//! the core shows its "no resources" window, which is the smoke test here.

#[cfg(target_os = "ios")]
fn main() {
    use std::path::PathBuf;
    // Every kept core log line also to stderr, which Xcode's and devicectl's consoles show; the
    // core's own log file stays in Documents/logs.
    horizon_client::logging::set_platform_sink(|level, target, line| {
        eprintln!("[{level}] {target}: {line}");
    });
    let Some(container) = std::env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("Magnetite: iOS gave the app no container folder");
        return;
    };
    // Documents is the game folder: the Files app shows it (UIFileSharingEnabled).
    let game_dir = container.join("Documents");
    let resources = game_dir.join("resources");
    if let Err(error) = std::fs::create_dir_all(&game_dir) {
        eprintln!("Magnetite: cannot make {}: {error}", game_dir.display());
        return;
    }
    let arguments = vec![
        "--game-dir".into(),
        game_dir.to_string_lossy().into_owned(),
        "--resources".into(),
        resources.to_string_lossy().into_owned(),
        "--name".into(),
        "Player".into(),
    ];
    // Not horizon_game::main: iOS's main thread can't take the guest stack (the core calls
    // guests from its own guest-sized threads), and there is no compiler child or argv.
    let startup = horizon_client::entry::prepare_with_args(arguments);
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
}

/// The executable is iOS's only; elsewhere it says so.
#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("magnetite-ios runs on iOS only: build it with --target aarch64-apple-ios");
}
