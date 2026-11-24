//! One short tick of the vibration motor, for a touch button's press. The `Vibrator` service is
//! looked up once and kept; [`available`] says whether this device has a motor at all, [`tick`]
//! buzzes it. Both are cheap, are called from the game's loop, and never panic: a JNI failure is
//! logged and its exception cleared.

use crate::{early, with_activity};
use jni::objects::{GlobalRef, JValue};
use std::sync::OnceLock;

/// `android.os.VibrationEffect.EFFECT_TICK`, the system's shortest click (API 29).
const EFFECT_TICK: i32 = 2;
/// The fallback's length, in milliseconds, and its strength, out of 255.
const TICK_MILLIS: i64 = 12;
const TICK_AMPLITUDE: i32 = 80;

/// The kept `Vibrator`, or `None` where this device has none (or Java could not be reached).
static VIBRATOR: OnceLock<Option<GlobalRef>> = OnceLock::new();

fn vibrator() -> Option<&'static GlobalRef> {
    let kept = VIBRATOR.get_or_init(|| {
        with_activity(|env, activity| {
            let name = env.new_string("vibrator")?;
            let service = env
                .call_method(
                    activity,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[JValue::Object(&jni::objects::JObject::from(name))],
                )?
                .l()?;
            if service.is_null() {
                return Ok(None);
            }
            if !env.call_method(&service, "hasVibrator", "()Z", &[])?.z()? {
                return Ok(None);
            }
            Ok(Some(env.new_global_ref(&service)?))
        })
        .flatten()
    });
    kept.as_ref()
}

/// Whether this device has a vibration motor.
pub fn available() -> bool {
    vibrator().is_some()
}

/// One short tick. Does nothing where there is no motor.
pub fn tick() {
    let Some(vibrator) = vibrator() else {
        return;
    };
    let done = with_activity(|env, _activity| {
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
            .and_then(|v| v.i())
            .unwrap_or(0);
        // API 29 has the system's own click; before it, a very short one-shot is the same feel.
        let effect = if sdk >= 29 {
            env.call_static_method(
                "android/os/VibrationEffect",
                "createPredefined",
                "(I)Landroid/os/VibrationEffect;",
                &[JValue::Int(EFFECT_TICK)],
            )?
            .l()?
        } else {
            env.call_static_method(
                "android/os/VibrationEffect",
                "createOneShot",
                "(JI)Landroid/os/VibrationEffect;",
                &[JValue::Long(TICK_MILLIS), JValue::Int(TICK_AMPLITUDE)],
            )?
            .l()?
        };
        env.call_method(
            vibrator.as_obj(),
            "vibrate",
            "(Landroid/os/VibrationEffect;)V",
            &[JValue::Object(&effect)],
        )?;
        Ok(())
    });
    if done.is_none() {
        early(log::Level::Warn, "Android would not vibrate");
    }
}
