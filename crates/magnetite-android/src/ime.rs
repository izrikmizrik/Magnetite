//! The system keyboard for Android's stock NativeActivity: a hidden stock `EditText` on the
//! Android UI looper, driven through JNI, no project Java classes.
//!
//! Ported from the Android fork of Horizon (MIT licence), where it is `android/native_ime.rs`,
//! trimmed to the keyboard: the document pickers, mouse capture, screen/cutout configuration and
//! viewport bounds stay behind. The core gets the four plain functions of
//! `horizon_client::window::PlatformKeyboard`: [`show`], [`hide`], [`text`], [`set_text`].
//!
//! Enter works through the multiline input type, as in the fork: the keyboard's Enter key types
//! a line break into the box, [`text`] reports it and the core reads it as Enter
//! (`system_keyboard::edits`). A secret box is multiline too, masked with
//! `PasswordTransformationMethod` instead of the password input variation: a password field's
//! keyboard offers a Done action, not an Enter key, and that action never reaches the buffer
//! (the fork's password mode had no line break either).
//!
//! [`initialize`] must run on the Java UI thread during the activity's onCreate: View operations
//! check their thread. android-activity's optional `android_on_create` entry point (`lib.rs`)
//! runs there before `android_main`; the fork initializes from the same hook. The game's thread
//! only queues commands; a Unix socket pair wakes the UI looper, whose callback applies them
//! with that thread's JNI environment.

use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_forThread() -> *mut std::ffi::c_void;
    fn ALooper_addFd(
        looper: *mut std::ffi::c_void,
        fd: i32,
        ident: i32,
        events: i32,
        callback: Option<unsafe extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32>,
        data: *mut std::ffi::c_void,
    ) -> i32;
}

enum Command {
    Show(String, horizon_client::system_keyboard::Mode),
    SetText(String),
    Hide,
    Poll(u64),
    /// Keep the screen on, or let it sleep again (`View.setKeepScreenOn` on the native view).
    KeepScreenOn(bool),
    /// `Activity.setRequestedOrientation` with this `ActivityInfo` constant.
    Orientation(i32),
}

struct Ui {
    vm: JavaVM,
    edit: GlobalRef,
    input: GlobalRef,
    native: GlobalRef,
    commands: Mutex<VecDeque<Command>>,
    reader: Mutex<UnixStream>,
    writer: Mutex<UnixStream>,
    text: Mutex<String>,
    polling: AtomicBool,
    active: AtomicBool,
    ready: AtomicBool,
    failed: AtomicBool,
    keyboard_request: Mutex<Option<std::time::Instant>>,
    /// Bumped by every [`set_text`]; a Poll queued before it must not report the old box text.
    generation: AtomicU64,
}

static UI: OnceLock<Arc<Ui>> = OnceLock::new();

/// Must run during NativeActivity onCreate, on the Java UI thread.
///
/// # Safety
/// VM and activity pointers must be supplied by android-activity and valid for
/// the duration of this call. No local JNI reference is retained.
pub unsafe fn initialize(
    vm: *mut jni::sys::JavaVM,
    activity: jni::sys::jobject,
) -> Result<(), String> {
    if UI.get().is_some() {
        return Ok(());
    }
    let vm = unsafe { JavaVM::from_raw(vm) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let (edit, input, native) = env
        .with_local_frame(32, |env| -> jni::errors::Result<_> {
            let activity = unsafe { JObject::from_raw(activity) };
            let content = env
                .call_method(
                    &activity,
                    "findViewById",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0x01020002)],
                )?
                .l()?;
            // NativeActivity's first content child owns the native input queue.
            let native = env
                .call_method(
                    &content,
                    "getChildAt",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0)],
                )?
                .l()?;
            let edit = env.new_object(
                "android/widget/EditText",
                "(Landroid/content/Context;)V",
                &[JValue::Object(&activity)],
            )?;
            env.call_method(&edit, "setInputType", "(I)V", &[JValue::Int(0x20001)])?;
            env.call_method(&edit, "setImeOptions", "(I)V", &[JValue::Int(0x12000000)])?;
            env.call_method(&edit, "setAlpha", "(F)V", &[JValue::Float(0.0)])?;
            env.call_method(&edit, "setSaveEnabled", "(Z)V", &[JValue::Bool(0)])?;
            env.call_method(&edit, "setFocusableInTouchMode", "(Z)V", &[JValue::Bool(1)])?;
            env.call_method(&edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
            let params = env.new_object(
                "android/view/ViewGroup$LayoutParams",
                "(II)V",
                &[JValue::Int(1), JValue::Int(1)],
            )?;
            env.call_method(
                &activity,
                "addContentView",
                "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
                &[JValue::Object(&edit), JValue::Object(&params)],
            )?;
            let service = JObject::from(env.new_string("input_method")?);
            let input = env
                .call_method(
                    &activity,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[JValue::Object(&service)],
                )?
                .l()?;
            env.call_method(&native, "setFocusable", "(Z)V", &[JValue::Bool(1)])?;
            env.call_method(
                &native,
                "setFocusableInTouchMode",
                "(Z)V",
                &[JValue::Bool(1)],
            )?;
            let sdk = env
                .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
                .and_then(|v| v.i())
                .unwrap_or(0);
            if sdk >= 26 {
                if env
                    .call_method(
                        &native,
                        "setDefaultFocusHighlightEnabled",
                        "(Z)V",
                        &[JValue::Bool(0)],
                    )
                    .is_err()
                {
                    let _ = env.exception_clear();
                }
            }
            env.call_method(&native, "requestFocus", "()Z", &[])?;
            Ok((
                env.new_global_ref(edit)?,
                env.new_global_ref(input)?,
                env.new_global_ref(native)?,
            ))
        })
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })?;
    drop(env);
    let (reader, writer) = UnixStream::pair().map_err(|e| e.to_string())?;
    reader.set_nonblocking(true).map_err(|e| e.to_string())?;
    writer.set_nonblocking(true).map_err(|e| e.to_string())?;
    let ui = Arc::new(Ui {
        vm,
        edit,
        input,
        native,
        commands: Mutex::new(VecDeque::new()),
        reader: Mutex::new(reader),
        writer: Mutex::new(writer),
        text: Mutex::new(String::new()),
        polling: AtomicBool::new(false),
        active: AtomicBool::new(false),
        ready: AtomicBool::new(false),
        failed: AtomicBool::new(false),
        keyboard_request: Mutex::new(None),
        generation: AtomicU64::new(0),
    });
    let looper = unsafe { ALooper_forThread() };
    if looper.is_null() {
        return Err("No Android UI looper during onCreate".into());
    }
    let fd = ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_raw_fd();
    let data = Arc::into_raw(ui.clone()).cast_mut().cast();
    if unsafe { ALooper_addFd(looper, fd, -1, 1, Some(callback), data) } != 1 {
        // Registration failed: the looper never owns this reference.
        drop(unsafe { Arc::from_raw(data.cast::<Ui>()) });
        return Err("Cannot register Android UI input channel".into());
    }
    // The callback reference and these global JNI refs live for the activity's
    // process lifetime, not per edit. Activity shutdown exits that process.
    UI.set(ui).ok();
    Ok(())
}

impl Ui {
    fn enqueue(&self, c: Command) {
        self.commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(c);
        let _ = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(&[1]);
    }

    fn try_show_keyboard(&self, env: &mut JNIEnv<'_>) -> jni::errors::Result<()> {
        let mut request = self
            .keyboard_request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(started) = *request else {
            return Ok(());
        };
        if !self.active.load(Ordering::Acquire) {
            *request = None;
            return Ok(());
        }
        let focused = env.call_method(&self.edit, "hasFocus", "()Z", &[])?.z()?;
        let window_focused = env
            .call_method(&self.edit, "hasWindowFocus", "()Z", &[])?
            .z()?;
        let served = focused
            && window_focused
            && env
                .call_method(
                    &self.input,
                    "isActive",
                    "(Landroid/view/View;)Z",
                    &[JValue::Object(self.edit.as_obj())],
                )?
                .z()?;
        if served
            && env
                .call_method(
                    &self.input,
                    "showSoftInput",
                    "(Landroid/view/View;I)Z",
                    &[JValue::Object(self.edit.as_obj()), JValue::Int(0)],
                )?
                .z()?
        {
            *request = None;
        } else if started.elapsed() > std::time::Duration::from_secs(2) {
            log::warn!(
                "Android keyboard was not served (view focus={focused}, window focus={window_focused}, served={served})"
            );
            *request = None;
        }
        Ok(())
    }

    fn apply(&self, env: &mut JNIEnv<'_>, c: Command) -> jni::errors::Result<()> {
        match c {
            Command::Show(text, mode) => {
                use horizon_client::system_keyboard::Mode;
                let secret = mode == Mode::Secret;
                // Text: 0x20001, text and multiline, so the Enter key types '\n' (the core's
                // Enter). A login and a password take the password variations instead (0x91
                // visible, 0x81 hidden, with 0x80000 no suggestions): only those keep every
                // keyboard (SwiftKey included, tried on a phone) from capitalising or
                // correcting what is typed. Their Enter key is a Done action that types
                // nothing; the form's own button sends it.
                let input_type = match mode {
                    Mode::Text => 0x20001,
                    Mode::Literal => 0x80091,
                    Mode::Secret => 0x80081,
                };
                env.call_method(
                    &self.edit,
                    "setInputType",
                    "(I)V",
                    &[JValue::Int(input_type)],
                )?;
                let mask = if secret {
                    env.call_static_method(
                        "android/text/method/PasswordTransformationMethod",
                        "getInstance",
                        "()Landroid/text/method/PasswordTransformationMethod;",
                        &[],
                    )?
                    .l()?
                } else {
                    JObject::null()
                };
                env.call_method(
                    &self.edit,
                    "setTransformationMethod",
                    "(Landroid/text/method/TransformationMethod;)V",
                    &[JValue::Object(&mask)],
                )?;
                let s = JObject::from(env.new_string(&text)?);
                env.call_method(
                    &self.edit,
                    "setText",
                    "(Ljava/lang/CharSequence;)V",
                    &[JValue::Object(&s)],
                )?;
                env.call_method(
                    &self.edit,
                    "setSelection",
                    "(I)V",
                    &[JValue::Int(text.encode_utf16().count() as i32)],
                )?;
                // Re-enable touch focus on every edit. NativeActivity owns focus
                // between editors, and some Android versions do not serve a view
                // until the next UI-loop turn after requestFocus.
                env.call_method(&self.native, "clearFocus", "()V", &[])?;
                env.call_method(
                    &self.edit,
                    "setFocusableInTouchMode",
                    "(Z)V",
                    &[JValue::Bool(1)],
                )?;
                env.call_method(&self.edit, "requestFocus", "()Z", &[])?;
                env.call_method(
                    &self.input,
                    "restartInput",
                    "(Landroid/view/View;)V",
                    &[JValue::Object(self.edit.as_obj())],
                )?;
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(std::time::Instant::now());
                self.try_show_keyboard(env)?;
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
                self.ready.store(true, Ordering::Release);
            }
            Command::SetText(text) => {
                let s = JObject::from(env.new_string(&text)?);
                env.call_method(
                    &self.edit,
                    "setText",
                    "(Ljava/lang/CharSequence;)V",
                    &[JValue::Object(&s)],
                )?;
                env.call_method(
                    &self.edit,
                    "setSelection",
                    "(I)V",
                    &[JValue::Int(text.encode_utf16().count() as i32)],
                )?;
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
            }
            Command::Orientation(orientation) => {
                // The native view's context is the activity.
                let activity = env
                    .call_method(
                        &self.native,
                        "getContext",
                        "()Landroid/content/Context;",
                        &[],
                    )?
                    .l()?;
                env.call_method(
                    &activity,
                    "setRequestedOrientation",
                    "(I)V",
                    &[JValue::Int(orientation)],
                )?;
            }
            Command::KeepScreenOn(on) => {
                env.call_method(
                    &self.native,
                    "setKeepScreenOn",
                    "(Z)V",
                    &[JValue::Bool(u8::from(on))],
                )?;
            }
            Command::Hide => {
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                let token = env
                    .call_method(&self.edit, "getWindowToken", "()Landroid/os/IBinder;", &[])?
                    .l()?;
                env.call_method(
                    &self.input,
                    "hideSoftInputFromWindow",
                    "(Landroid/os/IBinder;I)Z",
                    &[JValue::Object(&token), JValue::Int(0)],
                )?;
                env.call_method(&self.edit, "clearFocus", "()V", &[])?;
                env.call_method(&self.edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
                env.call_method(&self.native, "requestFocus", "()Z", &[])?;
            }
            Command::Poll(generation) => {
                self.polling.store(false, Ordering::Release);
                self.try_show_keyboard(env)?;
                // A set_text since this Poll was queued owns the text now; reading the box
                // would race the SetText behind this command and resurrect the line Enter took.
                if generation != self.generation.load(Ordering::Acquire) {
                    return Ok(());
                }
                let editable = env
                    .call_method(&self.edit, "getText", "()Landroid/text/Editable;", &[])?
                    .l()?;
                let text = env
                    .call_method(editable, "toString", "()Ljava/lang/String;", &[])?
                    .l()?;
                let text: String = env.get_string(&text.into())?.into();
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
            }
        }
        Ok(())
    }
}

unsafe extern "C" fn callback(_: i32, _: i32, data: *mut std::ffi::c_void) -> i32 {
    let ui = unsafe { &*data.cast::<Ui>() };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback_inner(ui))).unwrap_or_else(
        |_| {
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            log::error!("Android keyboard UI callback failed");
            0
        },
    )
}

fn callback_inner(ui: &Ui) -> i32 {
    let mut bytes = [0; 256];
    while ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .read(&mut bytes)
        .is_ok_and(|n| n > 0)
    {}
    let commands: Vec<_> = ui
        .commands
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect();
    match ui.vm.attach_current_thread() {
        Ok(mut env) => {
            for c in commands {
                if let Err(e) = env.with_local_frame(16, |env| ui.apply(env, c)) {
                    let _ = env.exception_clear();
                    log::warn!("IME command: {e}");
                    continue;
                }
            }
        }
        Err(e) => {
            log::error!("IME attach: {e}");
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            return 0;
        }
    }
    1
}

/// Show the system keyboard over the hidden box holding `text`, the caret at its end, typing as
/// `mode` says (a secret box is masked; a literal or secret one has no suggestions). False when
/// the keyboard cannot be shown.
pub fn show(text: &str, mode: horizon_client::system_keyboard::Mode) -> bool {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        winit::platform::android::set_native_text_input_active(true);
        // Do not diff an earlier editor buffer while Show is queued on the UI thread.
        ui.ready.store(false, Ordering::Release);
        ui.active.store(true, Ordering::Release);
        ui.enqueue(Command::Show(text.into(), mode));
        true
    } else {
        false
    }
}

/// Keep the screen on while `on` (the files' download), through the native view on the UI
/// thread. Not `AndroidApp::set_window_flags`: android-activity 0.6.1 takes its state for
/// writing there while the event loop holds it for reading around every callback, so a call from
/// the game's loop never returns. False when the UI thread can't be reached.
/// Turn the screen upright (`ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT`) or on its side
/// (`SENSOR_LANDSCAPE`), on the UI thread. False when it can't be reached.
pub fn set_upright(upright: bool) -> bool {
    match UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        Some(ui) => {
            ui.enqueue(Command::Orientation(if upright { 7 } else { 6 }));
            true
        }
        None => false,
    }
}

pub fn keep_screen_on(on: bool) -> bool {
    match UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        Some(ui) => {
            ui.enqueue(Command::KeepScreenOn(on));
            true
        }
        None => false,
    }
}

/// Hide the keyboard and give focus back to the native surface.
pub fn hide() {
    winit::platform::android::set_native_text_input_active(false);
    if let Some(ui) = UI.get() {
        ui.active.store(false, Ordering::Release);
        ui.enqueue(Command::Hide);
    }
}

/// The box's text now; `None` while the keyboard is not up.
pub fn text() -> Option<String> {
    let ui = UI.get()?;
    if !ui.active.load(Ordering::Acquire) || !ui.ready.load(Ordering::Acquire) {
        return None;
    }
    if !ui.polling.swap(true, Ordering::AcqRel) {
        ui.enqueue(Command::Poll(ui.generation.load(Ordering::Acquire)));
    }
    Some(
        ui.text
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
}

/// Replace the box's text and put the caret at its end (the core empties the box after Enter).
/// The cache is updated at once, so a `text()` before the UI thread applies this never reports
/// the old line again; the generation retires any Poll already on its way.
pub fn set_text(text: &str) {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        ui.generation.fetch_add(1, Ordering::AcqRel);
        *ui.text
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = text.to_owned();
        ui.enqueue(Command::SetText(text.into()));
    }
}
