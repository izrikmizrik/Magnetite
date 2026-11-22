//! Magnetite Android: the stock NativeActivity loads this library and android-activity calls
//! [`android_main`] on its own thread. It starts the boot loop with the activity's event loop and
//! the app's private folders instead of argv and a working directory
//! (`docs/unification/mobile-boot.md` §1.4).
//!
//! Stage 1 of the port (docs/unification/android-entry.md): the screen runs at once — signing in,
//! the game files, then the game itself, handed the session file and the prepared resources. Touch
//! controls and the in-game launcher come in later stages.
#![cfg(target_os = "android")]

mod clipboard;
mod ime;

use std::sync::OnceLock;
use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::{AndroidApp, OnCreateState, WindowManagerFlags};

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

/// The activity, kept for the boot loop's two plain `fn` hooks (`open_link`, `keep_awake`): they
/// take no context, and `AndroidApp` is reference counted, cheap and safe to keep.
static ACTIVITY: OnceLock<AndroidApp> = OnceLock::new();

/// Opens `url` in the system's browser: an `android.intent.action.VIEW` intent started from the
/// activity, as the game's own link opener does. Called on the boot loop's thread, which is the
/// activity's thread here.
fn open_link(url: &str) {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return;
    }
    let Some(app) = ACTIVITY.get() else {
        early(
            log::Level::Warn,
            "Android has no activity to open a link from",
        );
        return;
    };
    // SAFETY: `AndroidApp` keeps the VM and the activity alive for this call; this thread holds
    // the JVM only for the attach, and the activity for this call.
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        early(log::Level::Warn, "Android gave no JVM to open a link with");
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        early(
            log::Level::Warn,
            &format!("Android: cannot attach for {url}"),
        );
        return;
    };
    let opened = env.with_local_frame(16, |env| -> jni::errors::Result<()> {
        let address = jni::objects::JObject::from(env.new_string(url)?);
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[jni::objects::JValue::Object(&address)],
            )?
            .l()?;
        let action = jni::objects::JObject::from(env.new_string("android.intent.action.VIEW")?);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;Landroid/net/Uri;)V",
            &[
                jni::objects::JValue::Object(&action),
                jni::objects::JValue::Object(&uri),
            ],
        )?;
        // SAFETY: the pointer is the activity `AndroidApp` holds alive; it is used for this call
        // only and not deleted with the local frame.
        let activity = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
        env.call_method(
            &activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[jni::objects::JValue::Object(&intent)],
        )?;
        Ok(())
    });
    if let Err(error) = opened {
        // A device without a browser must not leave a pending Java exception.
        let _ = env.exception_clear();
        early(
            log::Level::Warn,
            &format!("Android would not open {url}: {error}"),
        );
    }
}

/// Keeps the screen on while the files download (`Window`'s flag, the boot loop's switch).
fn keep_awake(on: bool) {
    let Some(app) = ACTIVITY.get() else {
        return;
    };
    let (add, remove) = if on {
        (
            WindowManagerFlags::KEEP_SCREEN_ON,
            WindowManagerFlags::empty(),
        )
    } else {
        (
            WindowManagerFlags::empty(),
            WindowManagerFlags::KEEP_SCREEN_ON,
        )
    };
    app.set_window_flags(add, remove);
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

/// android-activity's optional onCreate entry point: the Java UI thread, before `android_main`.
/// The keyboard's hidden EditText belongs to that thread (View operations check their thread),
/// so its initialize runs here, as the fork's `android_on_create` does. A panic inside this JNI
/// callback would abort the process, so it is caught and logged instead.
#[unsafe(no_mangle)]
fn android_on_create(state: &OnCreateState<'_>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        ime::initialize(state.vm_as_ptr().cast(), state.activity_as_ptr().cast())
    }));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => early(
            log::Level::Error,
            &format!("Android system keyboard initialization: {error}"),
        ),
        Err(_) => early(
            log::Level::Error,
            "Android system keyboard initialization panicked",
        ),
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
    // The game folder is the player's (`--game-dir`): saves, options.txt, logs. The prepared game
    // files sit beside it; the account and the session file share their own private folder, and
    // the download's staging is kept between runs so an interrupted fetch continues.
    let game_dir = home.join("game");
    let resources = home.join("resources");
    let horizon = home.join("horizon");
    let work = home.join("setup-work");
    for dir in [game_dir.as_path(), horizon.as_path(), work.as_path()] {
        if let Err(error) = std::fs::create_dir_all(dir) {
            early(
                log::Level::Error,
                &format!("Cannot make {}: {error}", dir.display()),
            );
            return;
        }
    }
    early(
        log::Level::Info,
        &format!("Starting Magnetite in {}", game_dir.display()),
    );
    let _ = ACTIVITY.set(app.clone());
    let activity = app.clone();
    horizon_client::window::set_event_loop_factory(Box::new(move || {
        EventLoop::builder().with_android_app(activity).build()
    }));
    let _exit = ExitOnReturn;
    let config = horizon_client::boot::BootConfig {
        game_dir,
        resources: resources.clone(),
        work: work.clone(),
        session_dir: horizon.clone(),
        secrets: Box::new(horizon_account::FileStore::new(
            horizon.join("account.json"),
        )),
        open_link,
        keep_awake,
        install: horizon_client::boot::installer_from_build(resources, work),
        // Not horizon_game::main: there is no compiler child or main-thread stack to prepare on
        // Android, and the arguments come from the boot loop, not argv. The game's guest calls run
        // on the core's own guest-sized threads; this thread's stack is android-activity's.
        start: Box::new(|args: Vec<String>| {
            let startup = horizon_client::entry::prepare_with_args(args);
            horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
        }),
    };
    // The platform's keyboard and clipboard for the core, as the iOS host gives them: the
    // hidden EditText the system keyboard types into, and the ClipboardManager.
    horizon_client::window::set_platform_keyboard(horizon_client::window::PlatformKeyboard {
        show: ime::show,
        hide: ime::hide,
        text: ime::text,
        set_text: ime::set_text,
    });
    horizon_client::clipboard::set_platform_clipboard(clipboard::get, clipboard::set);
    if let Err(error) = horizon_client::boot::run(config) {
        early(
            log::Level::Error,
            &format!("The boot loop stopped: {error}"),
        );
    }
}
