//! Magnetite Android: the stock NativeActivity loads this library and android-activity calls
//! [`android_main`] on its own thread. It starts the shared core the way the desktop executable
//! does, with the activity's event loop and paths instead of argv and a working directory.
//!
//! Stage 0 of the port (docs/unification/android-entry.md): the game starts and draws; signing
//! in, the game's prepared files, touch controls and the in-game launcher come in later stages.
//! Without prepared files the core shows its "no resources" window, which is the smoke test here.
#![cfg(target_os = "android")]

use std::path::Path;
use std::sync::OnceLock;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;

/// The logcat sink. The core's own logger stays the process logger (its log file and crash
/// reports); every line it keeps is also written here (`logging::set_platform_sink`).
static LOGCAT: OnceLock<android_logger::AndroidLogger> = OnceLock::new();

fn logcat() -> &'static android_logger::AndroidLogger {
    LOGCAT.get_or_init(|| {
        android_logger::AndroidLogger::new(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("Magnetite"),
        )
    })
}

/// A kept core log line, to logcat.
fn to_logcat(level: log::Level, target: &str, line: &str) {
    use log::Log;
    logcat().log(
        &log::Record::builder()
            .level(level)
            .target(target)
            .args(format_args!("{line}"))
            .build(),
    );
}

/// A line of this crate's own, before the core's logger exists.
fn early(level: log::Level, line: &str) {
    to_logcat(level, "magnetite_android", line);
}

/// The arguments the desktop launcher would pass: the game folder, the prepared files and an
/// offline name until signing in arrives.
fn arguments(game_dir: &Path, resources: &Path) -> Vec<String> {
    vec![
        "--game-dir".into(),
        game_dir.to_string_lossy().into_owned(),
        "--resources".into(),
        resources.to_string_lossy().into_owned(),
        "--name".into(),
        "Player".into(),
    ]
}

/// Ends the process when the game returns. winit allows one event loop per process and Android
/// keeps a finished activity's process for the next start, which would then fail to make its
/// loop; a fresh process starts clean.
struct ExitOnReturn;

impl Drop for ExitOnReturn {
    fn drop(&mut self) {
        std::process::exit(0);
    }
}

/// android-activity's entry point.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    horizon_client::logging::set_platform_sink(to_logcat);
    let Some(home) = app.internal_data_path() else {
        early(log::Level::Error, "Android gave the app no data folder");
        return;
    };
    let game_dir = home.join("game");
    let resources = home.join("resources");
    if let Err(error) = std::fs::create_dir_all(&game_dir) {
        early(
            log::Level::Error,
            &format!("Cannot make the game folder {}: {error}", game_dir.display()),
        );
        return;
    }
    early(
        log::Level::Info,
        &format!("Starting Magnetite in {}", game_dir.display()),
    );
    let activity = app.clone();
    horizon_client::window::set_event_loop_factory(Box::new(move || {
        EventLoop::builder().with_android_app(activity).build()
    }));
    let _exit = ExitOnReturn;
    // Not horizon_game::main: there is no compiler child or main-thread stack to prepare on
    // Android, and the arguments come from here, not argv. The game's guest calls run on the
    // core's own guest-sized threads; this thread's stack is android-activity's.
    let startup = horizon_client::entry::prepare_with_args(arguments(&game_dir, &resources));
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
}
