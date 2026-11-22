//! The system clipboard through Android's `ClipboardManager`, text only.
//!
//! Ported from the Android fork of Horizon (MIT licence), where it is `android/platform.rs`
//! (`clipboard_get` / `clipboard_set`); the sensitive-clip extra of the fork's login-code copy
//! stays behind. The activity comes from the `AndroidApp` `android_main` keeps, so these work
//! from any thread the core calls them on. JNI errors are logged and never panic.

use jni::{
    JNIEnv, JavaVM,
    objects::{JObject, JString, JValue},
};

fn with_context<R>(
    f: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<R>,
) -> Option<R> {
    let app = crate::ACTIVITY.get()?;
    // SAFETY: `AndroidApp` keeps the VM and the activity alive for this call; the activity
    // reference is used for this call only and not deleted with the local frame.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    let result = env.with_local_frame(64, |env| {
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        f(env, &activity)
    });
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            let _ = env.exception_clear();
            log::warn!("Android clipboard: {error}");
            None
        }
    }
}

/// The clipboard's text; `Some("")` when it holds none, as in the fork (the core reads that as
/// an empty clipboard). `None` only when the ClipboardManager could not be reached.
pub fn get() -> Option<String> {
    with_context(|env, activity| {
        let name = JObject::from(env.new_string("clipboard")?);
        let service = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let clip = env
            .call_method(
                &service,
                "getPrimaryClip",
                "()Landroid/content/ClipData;",
                &[],
            )?
            .l()?;
        if clip.is_null() || env.call_method(&clip, "getItemCount", "()I", &[])?.i()? == 0 {
            return Ok(String::new());
        }
        let item = env
            .call_method(
                &clip,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JValue::Int(0)],
            )?
            .l()?;
        // Only paste text. Do not resolve content URIs or read clipboard files.
        let text = env
            .call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[])?
            .l()?;
        if text.is_null() {
            return Ok(String::new());
        }
        let text = JString::from(
            env.call_method(text, "toString", "()Ljava/lang/String;", &[])?
                .l()?,
        );
        let value = env.get_string(&text)?.into();
        Ok(value)
    })
}

/// Put `text` on the clipboard as plain text.
pub fn set(text: &str) {
    let _ = with_context(|env, activity| {
        let name = JObject::from(env.new_string("clipboard")?);
        let service = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&name)],
            )?
            .l()?;
        let label = JObject::from(env.new_string("Magnetite")?);
        let value = JObject::from(env.new_string(text)?);
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&value)],
            )?
            .l()?;
        env.call_method(
            service,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )?;
        Ok(())
    });
}
