//! The gyroscope, through the NDK's own sensor API (`libandroid`, `<android/sensor.h>`): no Java
//! class, one thread with a looper of its own that reads the sensor at 100 Hz, and the turn it
//! measured since the last call. The host gives the core [`start`], [`stop`] and [`take`]
//! (`horizon_client::phone::Gyro`), so turning the phone turns the view.
//!
//! The sensor reports in the device's natural portrait axes (x right, y up, z out of the screen,
//! positive counter-clockwise); the screen's axes depend on how the display is rotated, and
//! [`turn`] maps one to the other. Nothing here panics: a missing sensor or a failed call leaves
//! the values at zero.

use crate::{early, with_activity};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// `ASENSOR_TYPE_GYROSCOPE`: the rate of rotation, in radians per second, around each axis.
const GYRO_TYPE: i32 = 4;
/// How often the sensor is asked for its rate, in microseconds (100 Hz).
const SAMPLE_MICROS: i32 = 10_000;
/// The longest wait for the queue per poll, in milliseconds: a stop is noticed within it.
const POLL_MILLIS: i32 = 100;
/// A step longer than this is a pause (the app was in the background), not a turn.
const MAX_STEP_SECONDS: f64 = 0.1;
/// `ALOOPER_PREPARE_ALLOW_NON_CALLBACKS`: events are read from the queue, not handed to a callback.
const ALOOPER_PREPARE_ALLOW_NON_CALLBACKS: i32 = 1;
/// `ALOOPER_POLL_ERROR`: the looper can no longer be polled.
const ALOOPER_POLL_ERROR: i32 = -4;
/// The queue's ident on the looper; any value of zero or more will do.
const QUEUE_IDENT: i32 = 1;

/// `ASensorEvent` from `<android/sensor.h>`, in its C layout: four 32-bit fields, the timestamp,
/// the union's first 16 floats (`data`, which starts with the axes), the flags and the reserved
/// words. A union cannot be spelled in Rust, and the gyroscope reads only `data[0..3]`, so the
/// union is the array the header puts first. The fields nothing here reads are the header's own,
/// kept so the size and the offsets are the C ones (a `const` assertion below checks the size).
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct ASensorEvent {
    version: i32,
    sensor: i32,
    /// `type` in the header; renamed for Rust's keyword.
    type_: i32,
    reserved0: i32,
    timestamp: i64,
    data: [f32; 16],
    flags: u32,
    reserved1: [i32; 3],
}

const _: () = assert!(std::mem::size_of::<ASensorEvent>() == 104);

impl Default for ASensorEvent {
    fn default() -> Self {
        Self {
            version: 0,
            sensor: 0,
            type_: 0,
            reserved0: 0,
            timestamp: 0,
            data: [0.0; 16],
            flags: 0,
            reserved1: [0; 3],
        }
    }
}

type LooperCallback = Option<unsafe extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32>;

#[link(name = "android")]
unsafe extern "C" {
    fn ASensorManager_getInstanceForPackage(
        package: *const std::ffi::c_char,
    ) -> *mut std::ffi::c_void;
    fn ASensorManager_getDefaultSensor(
        manager: *mut std::ffi::c_void,
        kind: i32,
    ) -> *mut std::ffi::c_void;
    fn ASensorManager_createEventQueue(
        manager: *mut std::ffi::c_void,
        looper: *mut std::ffi::c_void,
        ident: i32,
        callback: LooperCallback,
        data: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn ASensorManager_destroyEventQueue(
        manager: *mut std::ffi::c_void,
        queue: *mut std::ffi::c_void,
    ) -> i32;
    fn ASensorEventQueue_enableSensor(
        queue: *mut std::ffi::c_void,
        sensor: *mut std::ffi::c_void,
    ) -> i32;
    fn ASensorEventQueue_disableSensor(
        queue: *mut std::ffi::c_void,
        sensor: *mut std::ffi::c_void,
    ) -> i32;
    fn ASensorEventQueue_setEventRate(
        queue: *mut std::ffi::c_void,
        sensor: *mut std::ffi::c_void,
        micros: i32,
    ) -> i32;
    fn ASensorEventQueue_getEvents(
        queue: *mut std::ffi::c_void,
        events: *mut ASensorEvent,
        count: usize,
    ) -> isize;
    fn ALooper_prepare(options: i32) -> *mut std::ffi::c_void;
    fn ALooper_pollOnce(
        timeout_millis: i32,
        out_fd: *mut i32,
        out_events: *mut i32,
        out_data: *mut *mut std::ffi::c_void,
    ) -> i32;
}

/// The turn measured since the last [`take`]: `[yaw, pitch]`, in radians.
static TURNED: Mutex<[f64; 2]> = Mutex::new([0.0, 0.0]);

/// The process's sensor manager, or `None` when it cannot be made.
static MANAGER: OnceLock<Option<usize>> = OnceLock::new();

/// The reading thread, while it runs.
static STATE: Mutex<State> = Mutex::new(State {
    running: None,
    dying: None,
});

struct Run {
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}

struct State {
    running: Option<Run>,
    /// A thread told to stop that may not have left its looper yet; the next start waits for it.
    dying: Option<std::thread::JoinHandle<()>>,
}

/// The app's package name as a C string: `ASensorManager_getInstanceForPackage` takes it.
fn package_name() -> Option<std::ffi::CString> {
    let name = with_activity(|env, activity| {
        let name = env
            .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?
            .l()?;
        let name: String = env.get_string(&jni::objects::JString::from(name))?.into();
        Ok(name)
    })?;
    std::ffi::CString::new(name).ok()
}

/// The sensor manager, made once; `None` when Java or the sensor service cannot be reached.
fn manager() -> Option<*mut std::ffi::c_void> {
    let manager: Option<usize> = *MANAGER.get_or_init(|| {
        let package = package_name()?;
        // SAFETY: the package name outlives the call; the manager belongs to the process and is
        // never freed.
        let manager = unsafe { ASensorManager_getInstanceForPackage(package.as_ptr()) };
        (!manager.is_null()).then_some(manager as usize)
    });
    Some(manager? as *mut std::ffi::c_void)
}

/// Whether this device has a gyroscope.
pub fn available() -> bool {
    manager().is_some_and(|manager| {
        // SAFETY: the manager is the process's own, made above.
        unsafe { !ASensorManager_getDefaultSensor(manager, GYRO_TYPE).is_null() }
    })
}

/// The display's rotation (`Surface.ROTATION_0`, `_90`, `_180`, `_270`), read once when the
/// reading starts; the sensor's axes are the device's, and the screen's turn with it.
fn display_rotation() -> Option<i32> {
    with_activity(|env, activity| {
        let window = env
            .call_method(
                activity,
                "getWindowManager",
                "()Landroid/view/WindowManager;",
                &[],
            )?
            .l()?;
        let display = env
            .call_method(
                &window,
                "getDefaultDisplay",
                "()Landroid/view/Display;",
                &[],
            )?
            .l()?;
        env.call_method(&display, "getRotation", "()I", &[])?.i()
    })
}

/// Starts reading the sensor; nothing when it already runs or this device has none.
pub fn start() {
    let Some(manager) = manager() else {
        return;
    };
    if STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .running
        .is_some()
    {
        return;
    }
    // SAFETY: the manager is the process's own.
    let sensor = unsafe { ASensorManager_getDefaultSensor(manager, GYRO_TYPE) };
    if sensor.is_null() {
        return;
    }
    // Java is asked before the lock is taken: the sensor's answer is not worth holding it for.
    let rotation = display_rotation().unwrap_or(0);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let address = manager as usize;
    let sensor = sensor as usize;
    let mut state = STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.running.is_some() {
        return;
    }
    // A quick stop then start: the thread told to stop leaves its looper within `POLL_MILLIS`.
    if let Some(dying) = state.dying.take() {
        let _ = dying.join();
    }
    let spawned = std::thread::Builder::new()
        .name("magnetite-gyro".into())
        .spawn(move || read(address, sensor, rotation, flag));
    match spawned {
        Ok(thread) => state.running = Some(Run { stop, thread }),
        Err(error) => early(
            log::Level::Warn,
            &format!("Android gave no thread for the gyroscope: {error}"),
        ),
    }
}

/// Stops reading the sensor; nothing when it is not running.
pub fn stop() {
    let mut state = STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(run) = state.running.take() {
        run.stop.store(true, Ordering::Release);
        state.dying = Some(run.thread);
    }
}

/// How far the phone turned since the last call, in radians, for the screen held on its side:
/// `[yaw, pitch]`, yaw positive turning right, pitch positive looking down.
pub fn take() -> [f64; 2] {
    let mut turned = TURNED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::mem::take(&mut *turned)
}

/// The screen's angular rate from the device's: the display's rotation says how the screen's axes
/// (sx right, sy up) sit among the device's (x right, y up). Yaw is positive turning right and
/// pitch positive looking down, the two directions the camera follows.
fn turn(rotation: i32, rate: [f64; 3]) -> [f64; 2] {
    let [x, y, _] = rate;
    let (sx, sy) = match rotation {
        1 => (-y, x),
        2 => (-x, -y),
        3 => (y, -x),
        _ => (x, y),
    };
    [-sy, -sx]
}

/// The reading thread: its own looper and event queue, until `stop` says to end.
fn read(manager: usize, sensor: usize, rotation: i32, stop: Arc<AtomicBool>) {
    let manager = manager as *mut std::ffi::c_void;
    let sensor = sensor as *mut std::ffi::c_void;
    // SAFETY: `manager` and `sensor` belong to the process and outlive this thread; the looper
    // and the queue are this thread's own and are destroyed before it returns.
    unsafe {
        // A looper belongs to the thread that prepares it, so both are made here.
        let looper = ALooper_prepare(ALOOPER_PREPARE_ALLOW_NON_CALLBACKS);
        if looper.is_null() {
            early(log::Level::Warn, "Android gave the gyroscope no looper");
            return;
        }
        let queue = ASensorManager_createEventQueue(
            manager,
            looper,
            QUEUE_IDENT,
            None,
            std::ptr::null_mut(),
        );
        if queue.is_null() {
            early(
                log::Level::Warn,
                "Android gave the gyroscope no event queue",
            );
            return;
        }
        if ASensorEventQueue_enableSensor(queue, sensor) < 0 {
            early(log::Level::Warn, "Android would not enable the gyroscope");
            ASensorManager_destroyEventQueue(manager, queue);
            return;
        }
        ASensorEventQueue_setEventRate(queue, sensor, SAMPLE_MICROS);
        let mut last: Option<i64> = None;
        let mut events = [ASensorEvent::default(); 8];
        while !stop.load(Ordering::Acquire) {
            let mut out_events = 0;
            let mut out_data: *mut std::ffi::c_void = std::ptr::null_mut();
            let polled = ALooper_pollOnce(
                POLL_MILLIS,
                std::ptr::null_mut(),
                &mut out_events,
                &mut out_data,
            );
            if polled == ALOOPER_POLL_ERROR {
                break;
            }
            loop {
                let got = ASensorEventQueue_getEvents(queue, events.as_mut_ptr(), events.len());
                if got <= 0 {
                    break;
                }
                for event in &events[..got as usize] {
                    if event.type_ != GYRO_TYPE {
                        continue;
                    }
                    // The sensor's rate is per second; the time since the event before it turns
                    // it into an angle. The first event has nothing to measure from.
                    if let Some(previous) = last {
                        let step = (event.timestamp - previous) as f64 / 1e9;
                        if step > 0.0 && step <= MAX_STEP_SECONDS {
                            let rate = [
                                f64::from(event.data[0]),
                                f64::from(event.data[1]),
                                f64::from(event.data[2]),
                            ];
                            let [yaw, pitch] = turn(rotation, rate);
                            let mut turned = TURNED
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                            turned[0] += yaw * step;
                            turned[1] += pitch * step;
                        }
                    }
                    last = Some(event.timestamp);
                }
            }
        }
        ASensorEventQueue_disableSensor(queue, sensor);
        ASensorManager_destroyEventQueue(manager, queue);
    }
}

#[cfg(test)]
mod tests {
    use super::turn;

    #[test]
    fn the_screen_axes_follow_the_display_rotation() {
        // 1 rad/s right and 10 rad/s up, in the device's own axes.
        let rate = [1.0, 10.0, 0.0];
        // Portrait: the screen's axes are the device's, so yaw is -y and pitch is -x.
        assert_eq!(turn(0, rate), [-10.0, -1.0]);
        // A quarter turn right: the screen's right is the device's -y, its up the device's x.
        assert_eq!(turn(1, rate), [-1.0, 10.0]);
        assert_eq!(turn(2, rate), [10.0, 1.0]);
        assert_eq!(turn(3, rate), [1.0, -10.0]);
    }
}
