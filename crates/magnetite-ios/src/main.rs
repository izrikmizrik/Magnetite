//! Magnetite iOS: the app's executable. iOS starts it in `/` inside a read-only bundle and gives
//! it no arguments, so it hands the shared core the app's own folders and runs the boot loop on
//! this, the main thread (`docs/unification/mobile-boot.md` §1.4). The loop opens at once and
//! draws while the sign-in and the game files run on threads beside it, which is what keeps a
//! long first start inside iOS's launch time.
//!
//! Stage 1 of the port (docs/unification/ios-entry.md): the screen runs at once — signing in, the
//! game files, then the game itself, handed the session file and the prepared resources. Touch
//! controls and the in-game launcher come in later stages.

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
    // Documents is the game folder: the Files app shows it (UIFileSharingEnabled). The account and
    // the session file sit in Application Support, which the Files app does not show; the
    // download's staging is a cache, which the system may purge.
    let game_dir = container.join("Documents");
    let resources = game_dir.join("resources");
    let support = container
        .join("Library")
        .join("Application Support")
        .join("horizon");
    let work = container
        .join("Library")
        .join("Caches")
        .join("horizon-setup");
    for dir in [game_dir.as_path(), support.as_path(), work.as_path()] {
        if let Err(error) = std::fs::create_dir_all(dir) {
            eprintln!("Magnetite: cannot make {}: {error}", dir.display());
            return;
        }
    }
    let config = horizon_client::boot::BootConfig {
        game_dir,
        resources: resources.clone(),
        work: work.clone(),
        session_dir: support.clone(),
        secrets: Box::new(horizon_account::FileStore::new(
            support.join("account.json"),
        )),
        open_link,
        keep_awake,
        install: horizon_client::boot::installer_from_build(resources, work),
        // Not horizon_game::main: iOS's main thread can't take the guest stack (the core calls
        // guests from its own guest-sized threads), and there is no compiler child or argv.
        start: Box::new(|args: Vec<String>| {
            let startup = horizon_client::entry::prepare_with_args(args);
            horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
        }),
    };
    if let Err(error) = horizon_client::boot::run(config) {
        eprintln!("Magnetite: the boot loop stopped: {error}");
    }
}

/// Opens `url` in Safari (`UIApplication.openURL:options:completionHandler:`). Called on the boot
/// loop's thread, which is the main thread here, as UIKit requires.
#[cfg(target_os = "ios")]
fn open_link(url: &str) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{MainThreadMarker, NSDictionary, NSString, NSURL};
    use objc2_ui_kit::{UIApplication, UIApplicationOpenExternalURLOptionsKey};
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("Magnetite: a link can only be opened from the main thread");
        return;
    };
    let text: Retained<NSString> = NSString::from_str(url);
    // SAFETY: Foundation's URL parser on a string this call owns; `None` for a string that is not
    // a URL, which ends the call before the URL is used.
    let Some(link) = (unsafe { NSURL::URLWithString(&text) }) else {
        eprintln!("Magnetite: not a URL: {url}");
        return;
    };
    let options: Retained<NSDictionary<UIApplicationOpenExternalURLOptionsKey, AnyObject>> =
        NSDictionary::new();
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented method with the argument types it takes, on the main thread; no
    // completion handler means there is no block to outlive the call.
    unsafe {
        app.openURL_options_completionHandler(&link, &options, None);
    }
}

/// Keeps the screen on while the files download (`UIApplication.idleTimerDisabled`).
#[cfg(target_os = "ios")]
fn keep_awake(on: bool) {
    use objc2_foundation::MainThreadMarker;
    use objc2_ui_kit::UIApplication;
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented property, on the main thread.
    unsafe {
        app.setIdleTimerDisabled(on);
    }
}

/// The executable is iOS's only; elsewhere it says so.
#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("magnetite-ios runs on iOS only: build it with --target aarch64-apple-ios");
}
