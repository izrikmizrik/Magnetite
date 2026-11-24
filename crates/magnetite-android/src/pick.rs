//! The system's file picker for a mod (`.hmod`) or a touch button's picture: the Storage Access
//! Framework's `ACTION_OPEN_DOCUMENT`, started in a small transparent activity of ours
//! (`packaging/android/java/dev/magnetite/PickFileActivity`). The system grants the app a document
//! only for the picker's own answer, so that activity copies what the player picked into
//! `<cacheDir>/picked`, where the app may read it later, and the boot loop takes that copy
//! (`BootConfig::picked_file`).
//!
//! These are two of the boot loop's plain hooks: [`pick`] asks for a file of a kind, [`picked`]
//! answers what has come back. JNI errors are logged and never panic.

use crate::with_activity;
use horizon_client::boot::PickKind;
use jni::objects::{JObject, JString, JValue};
use std::path::PathBuf;
use std::sync::OnceLock;

/// The MIME type the picker shows. A `.hmod` is a zip, but a zip filter would hide it behind
/// "archives"; `*/*` leaves the player free to pick any file, and the boot loop judges what came
/// back. A touch button's picture is an image.
fn mime(kind: PickKind) -> &'static str {
    match kind {
        PickKind::Mod => "*/*",
        PickKind::Picture => "image/*",
    }
}

/// The app's cache folder, read from Java once: [`picked`] runs ten times a second and must not
/// ask Java every time.
static CACHE_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// `<cacheDir>`, or `None` when Java could not be reached (nothing is retried).
fn cache_dir() -> Option<&'static PathBuf> {
    let cached = CACHE_DIR.get_or_init(|| {
        with_activity(|env, activity| {
            let folder = env
                .call_method(activity, "getCacheDir", "()Ljava/io/File;", &[])?
                .l()?;
            if folder.is_null() {
                return Ok(None);
            }
            let path = env
                .call_method(&folder, "getAbsolutePath", "()Ljava/lang/String;", &[])?
                .l()?;
            let path: String = env.get_string(&JString::from(path))?.into();
            Ok(Some(PathBuf::from(path)))
        })
        .flatten()
    });
    cached.as_ref()
}

/// Show the system's file picker for a file of `kind`; true when it started. What the player picks
/// comes later, through [`picked`].
pub fn pick(kind: PickKind) -> bool {
    with_activity(|env, activity| {
        let intent = env.new_object("android/content/Intent", "()V", &[])?;
        let package = env
            .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let class = JObject::from(env.new_string("dev.magnetite.PickFileActivity")?);
        env.call_method(
            &intent,
            "setClassName",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&package), JValue::Object(&class)],
        )?;
        // The picker's MIME filter comes from the activity: it starts ACTION_OPEN_DOCUMENT with
        // the kind this call asks for, in the intent's "type" extra.
        let key = JObject::from(env.new_string("type")?);
        let value = JObject::from(env.new_string(mime(kind))?);
        env.call_method(
            &intent,
            "putExtra",
            "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/Intent;",
            &[JValue::Object(&key), JValue::Object(&value)],
        )?;
        env.call_method(
            activity,
            "startActivity",
            "(Landroid/content/Intent;)V",
            &[JValue::Object(&intent)],
        )?;
        Ok(())
    })
    .is_some()
}

/// The file picked since the last call, copied under `<cacheDir>/picked`, or why the copy failed;
/// `None` while nothing has come back. The boot loop removes the copy once it took it.
pub fn picked() -> Option<Result<PathBuf, String>> {
    let folder = cache_dir()?.join("picked");
    // No folder yet means no pick has finished: the usual answer, ten times a second.
    for entry in std::fs::read_dir(&folder).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // The activity writes `<name>.part` and renames it once the copy is whole, so a `.part` is
        // a copy still running.
        if name.ends_with(".part") {
            continue;
        }
        let path = entry.path();
        // A folder called "picked" holds files; anything else is not a pick.
        if !path.is_file() {
            continue;
        }
        if name == "error.txt" {
            let text = std::fs::read(&path).map_or_else(
                |error| error.to_string(),
                |bytes| String::from_utf8_lossy(&bytes).into_owned(),
            );
            let _ = std::fs::remove_file(&path);
            return Some(Err(text));
        }
        return Some(Ok(path));
    }
    None
}
