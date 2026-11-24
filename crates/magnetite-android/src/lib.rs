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
use winit::platform::android::activity::{AndroidApp, OnCreateState};

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

/// The system's language as a game language code (`ru_ru`): `java.util.Locale.getDefault()`'s
/// language and country, lower case. `None` when Java can't be reached.
fn system_language() -> Option<String> {
    let app = ACTIVITY.get()?;
    // SAFETY: `AndroidApp` keeps the VM alive for this call.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let code = env.with_local_frame(8, |env| -> jni::errors::Result<String> {
        let locale = env
            .call_static_method(
                "java/util/Locale",
                "getDefault",
                "()Ljava/util/Locale;",
                &[],
            )?
            .l()?;
        let mut part = |name: &str| -> jni::errors::Result<String> {
            let value = env
                .call_method(&locale, name, "()Ljava/lang/String;", &[])?
                .l()?;
            Ok(env.get_string(&jni::objects::JString::from(value))?.into())
        };
        let language = part("getLanguage")?;
        let country = part("getCountry")?;
        Ok(format!("{language}_{country}").to_lowercase())
    });
    match code {
        Ok(code) if code.len() > 1 && !code.ends_with('_') => Some(code),
        Ok(_) => None,
        Err(error) => {
            let _ = env.exception_clear();
            early(
                log::Level::Warn,
                &format!("Android gave no language: {error}"),
            );
            None
        }
    }
}

/// Keeps the screen on while the files download (`Window`'s flag, the boot loop's switch).
fn keep_awake(on: bool) {
    // Through the UI thread (`ime::keep_screen_on`): the window flags' own call deadlocks when
    // made from the event loop's callbacks, which is where the boot loop runs.
    if !ime::keep_screen_on(on) {
        early(
            log::Level::Warn,
            "Android cannot keep the screen on: no UI thread",
        );
    }
}

/// The launcher upright, the game on its side (`ime::set_upright`, on the UI thread).
fn set_orientation(_window: &winit::window::Window, upright: bool) {
    if !ime::set_upright(upright) {
        early(
            log::Level::Warn,
            "Android cannot turn the screen: no UI thread",
        );
    }
}

/// The game's log to the system's share sheet: an `ACTION_SEND` of its text, in a chooser.
fn share_log(_path: &std::path::Path, text: &str) {
    let Some(app) = ACTIVITY.get() else {
        return;
    };
    // SAFETY: `AndroidApp` keeps the VM and the activity alive for this call.
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    let shared = env.with_local_frame(16, |env| -> jni::errors::Result<()> {
        use jni::objects::{JObject, JValue};
        let action = JObject::from(env.new_string("android.intent.action.SEND")?);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        let mime = JObject::from(env.new_string("text/plain")?);
        env.call_method(
            &intent,
            "setType",
            "(Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&mime)],
        )?;
        for (key, value) in [
            ("android.intent.extra.TEXT", text),
            ("android.intent.extra.SUBJECT", "Magnetite latest.log"),
        ] {
            let key = JObject::from(env.new_string(key)?);
            let value = JObject::from(env.new_string(value)?);
            env.call_method(
                &intent,
                "putExtra",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
                &[JValue::Object(&key), JValue::Object(&value)],
            )?;
        }
        let chooser = env
            .call_static_method(
                "android/content/Intent",
                "createChooser",
                "(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;",
                &[JValue::Object(&intent), JValue::Object(&JObject::null())],
            )?
            .l()?;
        // SAFETY: the activity `AndroidApp` holds alive, used for this call only.
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        env.call_method(
            &activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&chooser)],
        )?;
        Ok(())
    });
    if let Err(error) = shared {
        let _ = env.exception_clear();
        early(
            log::Level::Warn,
            &format!("Android would not share the log: {error}"),
        );
    }
}

/// The game folder in the system's Files app: a `VIEW` of this app's documents root, through the
/// Storage Access Framework provider the APK registers (`MagnetiteDocumentsProvider`). The path
/// is not used: the provider always shows `getFilesDir()/game`, whatever the boot loop calls the
/// folder. A Files app that takes no root view gets the root document itself.
fn open_folder(_path: &std::path::Path) -> bool {
    let Some(app) = ACTIVITY.get() else {
        early(
            log::Level::Warn,
            "Android has no activity to open a folder from",
        );
        return false;
    };
    // SAFETY: `AndroidApp` keeps the VM and the activity alive for this call.
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        early(log::Level::Warn, "Android gave no JVM to open a folder");
        return false;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        early(log::Level::Warn, "Android: cannot attach to open a folder");
        return false;
    };
    let opened = env.with_local_frame(32, |env| -> jni::errors::Result<()> {
        use jni::objects::{JObject, JString, JValue};
        // SAFETY: the activity `AndroidApp` holds alive, used for these calls only.
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let package = env
            .call_method(&activity, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let package: String = env.get_string(&JString::from(package))?.into();
        // The provider's authority is the app's package name plus ".documents" (AndroidManifest).
        let authority = JObject::from(env.new_string(format!("{package}.documents"))?);
        let root_id = JObject::from(env.new_string("magnetite")?);
        let root_uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "buildRootUri",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&authority), JValue::Object(&root_id)],
            )?
            .l()?;
        let action = JObject::from(env.new_string("android.intent.action.VIEW")?);
        let intent = env.new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )?;
        // The root first: a Files app that lists roots opens the folder there.
        let root_type = JObject::from(env.new_string("vnd.android.document/root")?);
        env.call_method(
            &intent,
            "setDataAndType",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&root_uri), JValue::Object(&root_type)],
        )?;
        // Read and write: the Files app may copy files in and out of the folder.
        env.call_method(
            &intent,
            "addFlags",
            "(I)Landroid/content/Intent;",
            &[JValue::Int(1 | 2)],
        )?;
        let took_root = env
            .call_method(
                &activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )
            .is_ok();
        if took_root {
            return Ok(());
        }
        // No app took the root; the root document itself may still open. The failed start left an
        // exception pending, which would hide the second attempt's own failure.
        let _ = env.exception_clear();
        let empty = JObject::from(env.new_string("")?);
        let document_uri = env
            .call_static_method(
                "android/provider/DocumentsContract",
                "buildDocumentUri",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&authority), JValue::Object(&empty)],
            )?
            .l()?;
        let directory_type = JObject::from(env.new_string("vnd.android.document/directory")?);
        env.call_method(
            &intent,
            "setDataAndType",
            "(Landroid/net/Uri;Ljava/lang/String;)Landroid/content/Intent;",
            &[
                JValue::Object(&document_uri),
                JValue::Object(&directory_type),
            ],
        )?;
        env.call_method(
            &activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    });
    if let Err(error) = opened {
        let _ = env.exception_clear();
        early(
            log::Level::Warn,
            &format!("Android would not open the game folder: {error}"),
        );
        return false;
    }
    true
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

/// Sizes the stack of every Rust thread spawned without a size, `android_main` included, as the
/// library loads: android-activity spawns `android_main` with `std::thread::spawn`, and the game's
/// loop runs there and builds the game on it (`Game::new` overflows the 2 MiB default), as a
/// desktop runs it on a thread of the guest stack's size (`horizon_guest_stack::builder`).
/// `RUST_MIN_STACK` is read once, at the first spawn, and android-activity spawns its stdio
/// forwarder before `android_on_create`: only a load-time constructor comes first. It reserves
/// address space, not memory.
extern "C" fn size_rust_threads() {
    let stack = horizon_guest_stack::GUEST_STACK_BYTES + horizon_guest_stack::HOST_HEADROOM;
    // SAFETY: runs while the dynamic loader loads this library, before any of its code or
    // threads; nothing else in the process reads this variable.
    unsafe { std::env::set_var("RUST_MIN_STACK", stack.to_string()) };
}

#[used]
#[unsafe(link_section = ".init_array")]
static SIZE_RUST_THREADS: extern "C" fn() = size_rust_threads;

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

/// The microphone's permission for mods: `RECORD_AUDIO`, asked at run time.
mod microphone {
    use super::{ACTIVITY, early};
    use horizon_client::microphone::Permission;

    const RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";

    /// `f` with the JNI environment and the activity, on this thread.
    fn with_activity<R>(
        f: impl FnOnce(&mut jni::JNIEnv, &jni::objects::JObject) -> jni::errors::Result<R>,
    ) -> Option<R> {
        let app = ACTIVITY.get()?;
        // SAFETY: `AndroidApp` keeps the VM and the activity alive for this call.
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        let result = env.with_local_frame(8, |env| {
            // SAFETY: the activity `AndroidApp` holds alive, used for these calls only.
            let activity = unsafe { jni::objects::JObject::from_raw(app.activity_as_ptr().cast()) };
            f(env, &activity)
        });
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                let _ = env.exception_clear();
                early(log::Level::Warn, &format!("Android microphone: {error}"));
                None
            }
        }
    }

    /// `Activity.checkSelfPermission(RECORD_AUDIO)`; not granted, it is denied once the player
    /// refused (`shouldShowRequestPermissionRationale`), and still to be asked before. A player
    /// who refused for good reads as not asked, and the question then shows nothing.
    pub(super) fn permission() -> Permission {
        let state = with_activity(|env, activity| {
            let name = env.new_string(RECORD_AUDIO)?;
            let name = jni::objects::JValue::Object(&name);
            let state = env
                .call_method(
                    activity,
                    "checkSelfPermission",
                    "(Ljava/lang/String;)I",
                    &[name],
                )?
                .i()?;
            // PackageManager.PERMISSION_GRANTED
            if state == 0 {
                return Ok(Permission::Granted);
            }
            let refused = env
                .call_method(
                    activity,
                    "shouldShowRequestPermissionRationale",
                    "(Ljava/lang/String;)Z",
                    &[name],
                )?
                .z()?;
            Ok(if refused {
                Permission::Denied
            } else {
                Permission::Undetermined
            })
        });
        state.unwrap_or(Permission::Unavailable)
    }

    /// `Activity.requestPermissions([RECORD_AUDIO])`: the system's dialog over the game; the
    /// answer is read back through [`permission`].
    pub(super) fn request() {
        with_activity(|env, activity| {
            let name = env.new_string(RECORD_AUDIO)?;
            let names = env.new_object_array(1, "java/lang/String", &name)?;
            env.call_method(
                activity,
                "requestPermissions",
                "([Ljava/lang/String;I)V",
                &[
                    jni::objects::JValue::Object(&names),
                    jni::objects::JValue::Int(1),
                ],
            )?;
            Ok(())
        });
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
        language: system_language(),
        launcher: true,
        set_orientation,
        share_log,
        open_folder,
        // Android apps are signed once; nothing expires.
        signature_expires: None,
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
    horizon_client::microphone::set_platform(horizon_client::microphone::Platform {
        permission: microphone::permission,
        request: microphone::request,
        prepare: || {},
    });
    if let Err(error) = horizon_client::boot::run(config) {
        early(
            log::Level::Error,
            &format!("The boot loop stopped: {error}"),
        );
    }
}
