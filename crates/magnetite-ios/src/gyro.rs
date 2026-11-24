//! The gyroscope (`Platform::gyro`, `docs/unification/mobile-boot.md` §1.4): CoreMotion's
//! `CMMotionManager`, read while the world is played and turned into the two angles the camera
//! takes for the screen held on its side.
//!
//! [`turn`] is the whole of the mapping, plain arithmetic, so the tests below run on the host as
//! well; what talks to CoreMotion or UIKit is `#[cfg(target_os = "ios")]`.

/// How long between two reads of the sensor, in seconds (100 Hz).
#[cfg(target_os = "ios")]
const UPDATE_INTERVAL: f64 = 0.01;

/// A longer gap than this is a pause — the game was paused, the app went to the background — and
/// the turn measured over it means nothing.
#[cfg(target_os = "ios")]
const LONGEST_STEP: f64 = 0.1;

/// The screen turned, held on its side: `[yaw, pitch]` in radians, yaw positive turning right,
/// pitch positive looking down.
///
/// `orientation` is the interface orientation's own number (`UIInterfaceOrientation`): 3 is
/// landscape right, 4 is landscape left, and anything else is read as upright. `rate` is the
/// device's rotation rate about its portrait axes — x right, y up, z out of the screen,
/// counter-clockwise positive — in radians per second.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
pub fn turn(orientation: isize, rate: [f64; 3]) -> [f64; 2] {
    let [x, y, _z] = rate;
    // The rates about the screen's right (sx) and up (sy) axes. The device's portrait axes lie on
    // the screen's only while it is upright; on its side they are swapped.
    let (sx, sy) = match orientation {
        // Landscape right and left: the device's own axes lie across the screen, swapped.
        3 => (-y, x),
        4 => (y, -x),
        // Portrait, and an orientation the system has not settled on yet: no swap.
        _ => (x, y),
    };
    // `yaw` is the rate about the screen's up axis, `pitch` the rate about its right axis, each
    // negated: `phone::Gyro::take` has yaw positive to the right and pitch positive looking down,
    // and a counter-clockwise rotation rate about an axis is the negated angle about it.
    // (`UIInterfaceOrientation 3/4 = landscapeRight/landscapeLeft` matches UIKit's header.)
    [-sy, -sx]
}

/// Whether this device has a gyroscope to read.
#[cfg(target_os = "ios")]
pub fn available() -> bool {
    use objc2::msg_send;
    let Some(manager) = manager() else {
        return false;
    };
    // SAFETY: CoreMotion's documented property, an Objective-C `BOOL`.
    let yes: objc2::runtime::Bool = unsafe { msg_send![&*manager, isDeviceMotionAvailable] };
    yes.as_bool()
}

/// Starts reading the sensor; starting twice changes nothing.
#[cfg(target_os = "ios")]
pub fn start() {
    use objc2::msg_send;
    let Some(manager) = manager() else {
        return;
    };
    // SAFETY: CoreMotion's documented property and method. The interval is an NSTimeInterval, a
    // double; no handler means the reads come from `deviceMotion` here rather than a queue.
    unsafe {
        let _: () = msg_send![&*manager, setDeviceMotionUpdateInterval: UPDATE_INTERVAL];
        let _: () = msg_send![&*manager, startDeviceMotionUpdates];
    }
    LAST.with(|last| last.set(Some(std::time::Instant::now())));
}

/// Stops reading it; stopping twice changes nothing.
#[cfg(target_os = "ios")]
pub fn stop() {
    use objc2::msg_send;
    let Some(manager) = manager() else {
        return;
    };
    // SAFETY: CoreMotion's documented method.
    unsafe {
        let _: () = msg_send![&*manager, stopDeviceMotionUpdates];
    }
    LAST.with(|last| last.set(None));
}

/// How far the phone turned since the last call: `[yaw, pitch]`, yaw positive turning right,
/// pitch positive looking down, the rate multiplied by the time since that call.
#[cfg(target_os = "ios")]
pub fn take() -> [f64; 2] {
    use objc2::msg_send;
    use std::time::Instant;
    let Some(manager) = manager() else {
        return [0.0, 0.0];
    };
    let now = Instant::now();
    let Some(previous) = LAST.with(|last| last.replace(Some(now))) else {
        // Nothing was started, so there is no step to measure over.
        return [0.0, 0.0];
    };
    let step = now.duration_since(previous).as_secs_f64();
    if step > LONGEST_STEP {
        return [0.0, 0.0];
    }
    // SAFETY: CoreMotion's documented property; `None` until the sensor's first update arrives.
    let motion: Option<objc2::rc::Retained<objc2::runtime::AnyObject>> =
        unsafe { objc2::msg_send_id![&*manager, deviceMotion] };
    let Some(motion) = motion else {
        return [0.0, 0.0];
    };
    // SAFETY: CoreMotion's documented property, the C struct `CMRotationRate` (`{?=ddd}`).
    let rate: RotationRate = unsafe { msg_send![&*motion, rotationRate] };
    let [yaw, pitch] = turn(orientation(), [rate.x, rate.y, rate.z]);
    [yaw * step, pitch * step]
}

/// The C struct `CMRotationRate`: three doubles. Apple's typedef names no struct, so the runtime
/// knows it as `{?=ddd}`.
#[cfg(target_os = "ios")]
#[repr(C)]
struct RotationRate {
    x: f64,
    y: f64,
    z: f64,
}

// SAFETY: `CMRotationRate` is three C `double`s and the typedef is an anonymous struct, whose
// encoding is "?" — `@encode(CMRotationRate)` is "{?=ddd}".
#[cfg(target_os = "ios")]
unsafe impl objc2::encode::Encode for RotationRate {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "?",
        &[
            <f64 as objc2::encode::Encode>::ENCODING,
            <f64 as objc2::encode::Encode>::ENCODING,
            <f64 as objc2::encode::Encode>::ENCODING,
        ],
    );
}

/// The `CMMotionManager`, made on its first use and kept for the app's life.
#[cfg(target_os = "ios")]
fn manager() -> Option<objc2::rc::Retained<objc2::runtime::AnyObject>> {
    use objc2::msg_send_id;
    use objc2::rc::Allocated;
    use objc2::runtime::{AnyClass, AnyObject};
    MANAGER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            let class = AnyClass::get("CMMotionManager")?;
            // SAFETY: NSObject's documented allocation and initializer, on CoreMotion's class.
            let fresh: Option<objc2::rc::Retained<AnyObject>> = unsafe {
                let allocated: Allocated<AnyObject> = msg_send_id![class, alloc];
                msg_send_id![allocated, init]
            };
            *slot = fresh;
        }
        (*slot).clone()
    })
}

#[cfg(target_os = "ios")]
thread_local! {
    /// See [`manager`].
    static MANAGER: std::cell::RefCell<Option<objc2::rc::Retained<objc2::runtime::AnyObject>>> =
        const { std::cell::RefCell::new(None) };
    /// When [`take`] last read the sensor, for the step it multiplies by; `None` while stopped.
    static LAST: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

// `CMMotionManager`'s class and its methods live in CoreMotion.
#[cfg(target_os = "ios")]
#[link(name = "CoreMotion", kind = "framework")]
unsafe extern "C" {}

/// The interface orientation's number, from the key window's scene (UIKit, so the main thread);
/// upright when the system has not said. `keyWindow` is deprecated for apps with several scenes,
/// and this app has one.
#[cfg(target_os = "ios")]
#[allow(deprecated)]
fn orientation() -> isize {
    use objc2_foundation::MainThreadMarker;
    use objc2_ui_kit::UIApplication;
    let Some(mtm) = MainThreadMarker::new() else {
        return 1;
    };
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented properties, on the main thread; the values are UIKit's own.
    unsafe {
        let Some(window) = app.keyWindow() else {
            return 1;
        };
        let Some(scene) = window.windowScene() else {
            return 1;
        };
        scene.interfaceOrientation().0
    }
}

#[cfg(test)]
mod tests {
    use super::turn;

    // A rate of one radian per second about one device axis. The device's axes are its own, so
    // what each one means depends on how the screen is held: these tests take them as the
    // interface orientation says they lie.

    /// Upright, `sy = y`, so `yaw = -y`.
    #[test]
    fn upright_negates_the_rate_about_the_devices_up() {
        for orientation in [0, 1, 2] {
            assert_eq!(turn(orientation, [0.0, 1.0, 0.0]), [-1.0, 0.0], "{orientation}");
        }
    }

    /// Upright, `sx = x`, so `pitch = -x`.
    #[test]
    fn upright_negates_the_rate_about_the_devices_right() {
        for orientation in [0, 1, 2] {
            assert_eq!(turn(orientation, [-1.0, 0.0, 0.0]), [0.0, 1.0], "{orientation}");
            assert_eq!(turn(orientation, [1.0, 0.0, 0.0]), [0.0, -1.0], "{orientation}");
        }
    }

    /// Landscape right: `sy = x` and `sx = -y`, so the device's own axes cross the screen's.
    #[test]
    fn landscape_right_lays_the_device_axes_on_their_sides() {
        // Yaw: -sy = -x.
        assert_eq!(turn(3, [1.0, 0.0, 0.0]), [-1.0, 0.0]);
        // Pitch: -sx = y.
        assert_eq!(turn(3, [0.0, 1.0, 0.0]), [0.0, 1.0]);
    }

    /// Landscape left: `sy = -x`, `sx = y` — every answer is landscape right's, negated.
    #[test]
    fn landscape_left_is_the_mirror_of_landscape_right() {
        assert_eq!(turn(4, [1.0, 0.0, 0.0]), [1.0, 0.0]);
        assert_eq!(turn(4, [0.0, 1.0, 0.0]), [0.0, -1.0]);
        for device in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            let right = turn(3, device);
            let left = turn(4, device);
            assert_eq!(left, [-right[0], -right[1]], "{device:?}");
        }
    }

    /// The rate about the direction out of the screen (`z`) is not an axis the view turns on, so
    /// it never reaches the answer.
    #[test]
    fn the_rate_out_of_the_screen_is_dropped() {
        for orientation in [0, 1, 2, 3, 4] {
            assert_eq!(
                turn(orientation, [0.0, 0.0, 5.0]),
                [0.0, 0.0],
                "{orientation}"
            );
        }
    }

    /// The reading is the rate itself: twice the rate is twice the turn, and nothing is scaled.
    #[test]
    fn the_reading_is_linear_in_the_rate() {
        assert_eq!(turn(3, [2.5, 0.0, 0.0]), [-2.5, 0.0]);
        assert_eq!(turn(1, [0.0, -0.25, 0.0]), [0.25, 0.0]);
    }
}
