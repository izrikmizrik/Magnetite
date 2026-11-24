//! The system's file picker: a mod, or a touch button's picture, comes from the Files app — or from
//! anywhere else the system offers — through `UIDocumentPickerViewController` (`BootConfig`'s
//! `pick_file` and `picked_file`, `docs/unification/mobile-boot.md` §1.4).
//!
//! The picker calls its delegate long after [`pick`] returned, so the picked file cannot travel
//! back as a return value: the delegate copies it into `Library/Caches/picked` and leaves it in
//! [`PICKED`]. The boot loop asks [`picked`] every tick and removes the copy once it took it.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Mutex;

use horizon_client::boot::PickKind;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ProtocolObject};
use objc2::{ClassType, DeclaredClass, declare_class, msg_send_id, mutability};
use objc2_foundation::{MainThreadMarker, NSArray, NSObject, NSObjectProtocol, NSURL};
use objc2_ui_kit::{
    UIApplication, UIDocumentPickerDelegate, UIDocumentPickerViewController, UIViewController,
};

// `UTType`, whose identifiers name the content types the picker is asked for.
#[link(name = "UniformTypeIdentifiers", kind = "framework")]
unsafe extern "C" {}

/// The file the player picked since the last call, copied where the app may read it, or why it
/// could not be; `None` while nothing was picked. The caller removes the copy.
pub fn picked() -> Option<Result<PathBuf, String>> {
    PICKED.lock().ok()?.take()
}

/// Shows the system's file picker for a file of `kind`; false when it could not be shown (no main
/// thread, no window yet, or a class the system does not have). What the player picks arrives
/// later, through [`picked`].
pub fn pick(kind: PickKind) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("Magnetite: a file picker can only be opened from the main thread");
        return false;
    };
    let Some(root) = root_controller(mtm) else {
        eprintln!("Magnetite: the key window has no root view controller to pick a file from");
        return false;
    };
    let Some(picker) = make_picker(mtm, kind) else {
        eprintln!("Magnetite: this system has no document picker for that file kind");
        return false;
    };
    // A fresh request: an answer to an earlier one is not this pick's.
    if let Ok(mut slot) = PICKED.lock() {
        *slot = None;
    }
    let delegate = PickerDelegate::new(mtm);
    {
        let as_protocol: &ProtocolObject<dyn UIDocumentPickerDelegate> =
            ProtocolObject::from_ref(&*delegate);
        // SAFETY: UIKit's documented setter, with a delegate of the protocol it takes. The picker
        // holds its delegate weakly, so DELEGATE below keeps this one alive while the picker is up.
        unsafe { picker.setDelegate(Some(as_protocol)) };
    }
    DELEGATE.with(|slot| *slot.borrow_mut() = Some(delegate));
    // SAFETY: UIKit's documented presentation from a live root view controller, on the main thread;
    // no completion handler means there is no block to outlive the call.
    unsafe { root.presentViewController_animated_completion(&picker, true, None) };
    true
}

// The delegate the picker messages, kept here because the picker's reference to it is weak.
thread_local! {
    static DELEGATE: RefCell<Option<Retained<PickerDelegate>>> = const { RefCell::new(None) };
}

/// The file the last pick returned, waiting for [`picked`].
static PICKED: Mutex<Option<Result<PathBuf, String>>> = Mutex::new(None);

/// The key window's root view controller, the one that presents the picker. `UIApplication`'s
/// `keyWindow`, deprecated for apps with several scenes, is the window this app has.
#[allow(deprecated)]
fn root_controller(mtm: MainThreadMarker) -> Option<Retained<UIViewController>> {
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented property; `None` before a window is on screen.
    let window = unsafe { app.keyWindow() }?;
    window.rootViewController()
}

/// The picker for `kind`, made for a copy of the file (`asCopy` YES: the system puts a copy in the
/// app's inbox, which is readable without a security scope of its own).
fn make_picker(
    mtm: MainThreadMarker,
    kind: PickKind,
) -> Option<Retained<UIDocumentPickerViewController>> {
    let class = AnyClass::get("UTType")?;
    let identifier = objc2_foundation::NSString::from_str(match kind {
        // Everything a document picker can hand over; the mod package (`.hmod`) is a plain file.
        PickKind::Mod => "public.item",
        PickKind::Picture => "public.image",
    });
    // SAFETY: UniformTypeIdentifiers' documented class method, linked above, with a string this
    // call owns; `None` for an identifier the system does not know.
    let content_type: Option<Retained<AnyObject>> =
        unsafe { msg_send_id![class, typeWithIdentifier: &*identifier] };
    let types: Retained<NSArray<AnyObject>> = NSArray::from_vec(vec![content_type?]);
    // SAFETY: UIKit's documented initializer for a list of content types, on the main thread; the
    // array is one this call owns, and `true` is BOOL's YES. Called by selector because UIKit's
    // binding of it sits behind a dependency this crate does not need elsewhere (the identifiers
    // above are strings, not `UTType`s).
    unsafe {
        msg_send_id![
            mtm.alloc::<UIDocumentPickerViewController>(),
            initForOpeningContentTypes: &*types,
            asCopy: true
        ]
    }
}

// The picker's delegate: copies what the player picked and leaves it for the boot loop.
declare_class!(
    /// See [`pick`]: UIKit calls `documentPicker:didPickDocumentsAtURLs:` on this object, on the
    /// main thread, once the player has chosen a file.
    struct PickerDelegate;

    // SAFETY: NSObject has no subclassing requirements, and UIKit calls a document picker's
    // delegate on the main thread only, which is what `MainThreadOnly` says.
    unsafe impl ClassType for PickerDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "MagnetitePickerDelegate";
    }

    impl DeclaredClass for PickerDelegate {}

    unsafe impl NSObjectProtocol for PickerDelegate {}

    // SAFETY: NSObject's initializer on an instance of a class with no ivars to set.
    unsafe impl PickerDelegate {
        #[method_id(init)]
        unsafe fn init(this: objc2::rc::Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(());
            // SAFETY: NSObject's own initializer, sent on this instance.
            unsafe { msg_send_id![super(this), init] }
        }
    }

    // SAFETY: the protocol's method as UIKit declares it: the picker, and the URLs it picked.
    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[method(documentPicker:didPickDocumentsAtURLs:)]
        unsafe fn did_pick_documents(
            &self,
            _controller: &UIDocumentPickerViewController,
            urls: &NSArray<NSURL>,
        ) {
            let picked = match urls.first() {
                Some(url) => copy_into_cache(url),
                None => Err("the picker returned no file".to_owned()),
            };
            if let Err(reason) = &picked {
                eprintln!("Magnetite: the picked file did not arrive: {reason}");
            }
            if let Ok(mut slot) = PICKED.lock() {
                *slot = Some(picked);
            }
        }

        /// The player dismissed the picker: an answer, so the boot loop stops waiting for one.
        #[method(documentPickerWasCancelled:)]
        unsafe fn was_cancelled(&self, _controller: &UIDocumentPickerViewController) {
            if let Ok(mut slot) = PICKED.lock() {
                *slot = Some(Err("the picker was dismissed".to_owned()));
            }
        }
    }
);

impl PickerDelegate {
    /// A delegate with no state of its own; UIKit keeps what it does not, so this module does.
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>();
        // SAFETY: the `init` above, on an instance allocated on the main thread by UIKit's rules.
        unsafe { msg_send_id![this, init] }
    }
}

/// Copies the file `url` names into `Library/Caches/picked`, emptying that folder first, and
/// returns the copy's path.
fn copy_into_cache(url: &NSURL) -> Result<PathBuf, String> {
    // SAFETY: NSURL's documented `path`, a string the URL holds; `None` for a URL that is not a
    // file URL.
    let path = unsafe { url.path() }.ok_or_else(|| "the picked URL is not a file".to_owned())?;
    let source = PathBuf::from(path.to_string());
    let name = source
        .file_name()
        .ok_or_else(|| format!("the picked file has no name: {}", source.display()))?
        .to_owned();
    let dir = cache_dir().ok_or_else(|| "iOS gave the app no container folder".to_owned())?;
    // A folder for this pick alone: nothing of an earlier pick stays beside it.
    if let Err(error) = std::fs::remove_dir_all(&dir)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!("cannot empty {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("cannot make {}: {error}", dir.display()))?;
    let target = dir.join(name);
    std::fs::copy(&source, &target).map_err(|error| {
        format!(
            "cannot copy {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(target)
}

/// `<HOME>/Library/Caches/picked`, where the picker's copy waits for the boot loop.
fn cache_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join("picked"),
    )
}
