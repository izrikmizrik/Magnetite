//! One short tick of the vibration motor when a touch button is pressed (`Platform::haptic`,
//! `docs/unification/mobile-boot.md` §1.4). UIKit's `UIImpactFeedbackGenerator` is best made once
//! and kept, so this keeps one per thread — the main thread, where UIKit lets it be used.

use std::cell::RefCell;

use objc2::rc::Retained;
use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::{UIImpactFeedbackGenerator, UIImpactFeedbackStyle};

/// One short tick. Called from a touch button's press; anywhere but the main thread it does
/// nothing, since UIKit would not allow the generator there.
pub fn tick() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    GENERATOR.with(|slot| {
        let mut slot = slot.borrow_mut();
        let generator = slot.get_or_insert_with(|| make(mtm));
        // SAFETY: UIKit's documented method, on the main thread with a generator this object keeps.
        unsafe { generator.impactOccurred() };
    });
}

// The generator, made on its first tick.
thread_local! {
    static GENERATOR: RefCell<Option<Retained<UIImpactFeedbackGenerator>>> =
        const { RefCell::new(None) };
}

/// A generator of the light impact style (`UIImpactFeedbackStyle` 0). `initWithStyle:` is marked
/// deprecated in the headers, but it is still how a style is chosen.
#[allow(deprecated)]
fn make(mtm: MainThreadMarker) -> Retained<UIImpactFeedbackGenerator> {
    // SAFETY: UIKit's documented initializer on an instance allocated on the main thread; the
    // style is one of the enum's own values.
    unsafe {
        UIImpactFeedbackGenerator::initWithStyle(
            mtm.alloc::<UIImpactFeedbackGenerator>(),
            UIImpactFeedbackStyle::Light,
        )
    }
}
