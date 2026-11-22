//! Magnetite iOS: the app's executable. iOS starts it in `/` inside a read-only bundle and gives
//! it no arguments, so it hands the shared core the app's own folders and runs the boot loop on
//! this, the main thread (`docs/unification/mobile-boot.md` §1.4). The loop opens at once and
//! draws while the sign-in and the game files run on threads beside it, which is what keeps a
//! long first start inside iOS's launch time.
//!
//! Stage 1 of the port (docs/unification/ios-entry.md): the screen runs at once — signing in, the
//! game files, then the game itself, handed the session file and the prepared resources. Touch
//! controls and the in-game launcher come in later stages.

#[cfg(target_os = "ios")]
fn main() {
    use std::path::PathBuf;
    // Every kept core log line also to stderr, which Xcode's and devicectl's consoles show; the
    // core's own log file stays in Documents/logs.
    horizon_client::logging::set_platform_sink(|level, target, line| {
        eprintln!("[{level}] {target}: {line}");
    });
    // UIKit's window is the screen; its inner area is the safe area, and touches come in points.
    horizon_client::window::set_outer_is_screen(true);
    // The system clipboard: the sign-in code and the game's copy and paste.
    horizon_client::clipboard::set_platform_clipboard(clipboard_get, clipboard_set);
    let Some(container) = std::env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("Magnetite: iOS gave the app no container folder");
        return;
    };
    // Documents is the game folder: the Files app shows it (UIFileSharingEnabled). The account and
    // the session file sit in Application Support, which the Files app does not show; the
    // download's staging is a cache, which the system may purge.
    let game_dir = container.join("Documents");
    let resources = game_dir.join("resources");
    let support = container
        .join("Library")
        .join("Application Support")
        .join("horizon");
    let work = container
        .join("Library")
        .join("Caches")
        .join("horizon-setup");
    for dir in [game_dir.as_path(), support.as_path(), work.as_path()] {
        if let Err(error) = std::fs::create_dir_all(dir) {
            eprintln!("Magnetite: cannot make {}: {error}", dir.display());
            return;
        }
    }
    let config = horizon_client::boot::BootConfig {
        game_dir,
        resources: resources.clone(),
        work: work.clone(),
        session_dir: support.clone(),
        secrets: Box::new(horizon_account::FileStore::new(
            support.join("account.json"),
        )),
        open_link,
        keep_awake,
        install: horizon_client::boot::installer_from_build(resources, work),
        // Not horizon_game::main: iOS's main thread can't take the guest stack (the core calls
        // guests from its own guest-sized threads), and there is no compiler child or argv.
        start: Box::new(|args: Vec<String>| {
            let startup = horizon_client::entry::prepare_with_args(args);
            horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
        }),
        language: system_language(),
        launcher: true,
        set_orientation,
        share_log,
        open_folder,
        signature_expires: signature_expires(),
    };
    if let Err(error) = horizon_client::boot::run(config) {
        eprintln!("Magnetite: the boot loop stopped: {error}");
    }
}

/// The game folder in the Files app: `shareddocuments://` opens a folder of an app that shares its
/// documents (`UIFileSharingEnabled`), as the Files app's own links do.
#[cfg(target_os = "ios")]
fn open_folder(path: &std::path::Path) -> bool {
    let path = path.to_string_lossy().replace(' ', "%20");
    open_link(&format!("shareddocuments://{path}"));
    true
}

/// Opens `url` in Safari (`UIApplication.openURL:options:completionHandler:`). Called on the boot
/// loop's thread, which is the main thread here, as UIKit requires.
#[cfg(target_os = "ios")]
fn open_link(url: &str) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{MainThreadMarker, NSDictionary, NSString, NSURL};
    use objc2_ui_kit::{UIApplication, UIApplicationOpenExternalURLOptionsKey};
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("Magnetite: a link can only be opened from the main thread");
        return;
    };
    let text: Retained<NSString> = NSString::from_str(url);
    // SAFETY: Foundation's URL parser on a string this call owns; `None` for a string that is not
    // a URL, which ends the call before the URL is used.
    let Some(link) = (unsafe { NSURL::URLWithString(&text) }) else {
        eprintln!("Magnetite: not a URL: {url}");
        return;
    };
    let options: Retained<NSDictionary<UIApplicationOpenExternalURLOptionsKey, AnyObject>> =
        NSDictionary::new();
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented method with the argument types it takes, on the main thread; no
    // completion handler means there is no block to outlive the call.
    unsafe {
        app.openURL_options_completionHandler(&link, &options, None);
    }
}

/// The system's first preferred language as a game language code: `ru-RU` becomes `ru_ru`.
#[cfg(target_os = "ios")]
fn system_language() -> Option<String> {
    use objc2_foundation::NSLocale;
    // SAFETY: Foundation's documented class property.
    let languages = unsafe { NSLocale::preferredLanguages() };
    // SAFETY: an NSArray of NSString, as the property returns.
    let first = unsafe { languages.firstObject() }?.to_string();
    let code = first.replace('-', "_").to_lowercase();
    // "zh_hans_cn" style: script codes are not in the game's names; keep language and region.
    let mut parts = code.split('_').filter(|p| p.len() == 2);
    let language = parts.next()?;
    // A language with no region has no game code to match (`ru` alone): the default stays.
    let region = parts.next()?;
    Some(format!("{language}_{region}"))
}

/// The launcher upright, the game on its side: the window's own orientations.
#[cfg(target_os = "ios")]
fn set_orientation(window: &winit::window::Window, upright: bool) {
    use winit::platform::ios::{ValidOrientations, WindowExtIOS};
    window.set_valid_orientations(if upright {
        ValidOrientations::Portrait
    } else {
        ValidOrientations::Landscape
    });
}

/// The general pasteboard's text (`UIPasteboard.generalPasteboard.string`).
#[cfg(target_os = "ios")]
fn clipboard_get() -> Option<String> {
    use objc2_ui_kit::UIPasteboard;
    // SAFETY: UIKit's documented class method and property; the pasteboard is thread-safe.
    let text = unsafe { UIPasteboard::generalPasteboard().string() }?;
    Some(text.to_string())
}

/// Puts `text` on the general pasteboard.
#[cfg(target_os = "ios")]
fn clipboard_set(text: &str) {
    use objc2_foundation::NSString;
    use objc2_ui_kit::UIPasteboard;
    let text = NSString::from_str(text);
    // SAFETY: UIKit's documented class method and setter, with a string this call owns.
    unsafe { UIPasteboard::generalPasteboard().setString(Some(&text)) };
}

/// Keeps the screen on while the files download (`UIApplication.idleTimerDisabled`).
#[cfg(target_os = "ios")]
fn keep_awake(on: bool) {
    use objc2_foundation::MainThreadMarker;
    use objc2_ui_kit::UIApplication;
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented property, on the main thread.
    unsafe {
        app.setIdleTimerDisabled(on);
    }
}

/// When this build's signature expires, as Unix seconds: the launcher warns a few days before a
/// sideloaded copy stops opening. The provisioning profile is `embedded.mobileprovision` beside the
/// executable, which is the bundle's root on iOS. Its CMS signature wraps a plain XML plist; a copy
/// with no profile (TrollStore, the simulator) or one that cannot be read has no date.
#[cfg(target_os = "ios")]
fn signature_expires() -> Option<u64> {
    let executable = std::env::current_exe().ok()?;
    let profile = std::fs::read(executable.parent()?.join("embedded.mobileprovision")).ok()?;
    expiration_of(&profile)
}

/// The `ExpirationDate` in a provisioning profile, as Unix seconds. The profile's bytes are the
/// signed plist with CMS data around it, so the plist's own text is found rather than decoded.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn expiration_of(profile: &[u8]) -> Option<u64> {
    const KEY: &[u8] = b"<key>ExpirationDate</key>";
    let key = find(profile, KEY)? + KEY.len();
    let open = find(&profile[key..], b"<date>")? + key + b"<date>".len();
    let close = find(&profile[open..], b"</date>")? + open;
    let date = std::str::from_utf8(&profile[open..close]).ok()?;
    parse_utc_date(date)
}

/// A plist date, `2026-12-01T10:20:30Z`, as Unix seconds. Seconds may be 60 for a leap second.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn parse_utc_date(date: &str) -> Option<u64> {
    let bytes = date.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let fields = |start: usize, end: usize| -> Option<i64> {
        let digits = &bytes[start..end];
        if !digits.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(digits).ok()?.parse().ok()
    };
    let (year, month, day) = (fields(0, 4)?, fields(5, 7)?, fields(8, 10)?);
    let (hour, minute, second) = (fields(11, 13)?, fields(14, 16)?, fields(17, 19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    u64::try_from(days * 86_400 + hour * 3_600 + minute * 60 + second).ok()
}

/// Days since 1970-01-01 for a civil date, by Howard Hinnant's `days_from_civil`: 400-year eras of
/// whole weeks keep the leap rules exact with plain integer division.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // March-first years put the leap day at the end, where the era maths counts it.
    let year = if month <= 2 { year - 1 } else { year };
    // The bias keeps a year before 0 in the era below 1 rather than rounding toward zero.
    let era = (if year >= 0 { year } else { year - 399 }) / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The offset of `needle` in `haystack`.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// The system share sheet (`UIActivityViewController`) with the log's text, presented from the key
/// window's root view controller. Called on the boot loop's thread, which is the main thread here.
/// `UIApplication.keyWindow`, deprecated for apps with several scenes, is the window this app has.
#[cfg(target_os = "ios")]
#[allow(deprecated)]
fn share_log(_path: &std::path::Path, text: &str) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{MainThreadMarker, NSArray, NSString};
    use objc2_ui_kit::{UIActivityViewController, UIApplication};
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("Magnetite: a share sheet can only be opened from the main thread");
        return;
    };
    let app = UIApplication::sharedApplication(mtm);
    // SAFETY: UIKit's documented property; `None` before a window is on screen.
    let Some(window) = (unsafe { app.keyWindow() }) else {
        eprintln!("Magnetite: no key window to present the share sheet from");
        return;
    };
    let Some(root) = window.rootViewController() else {
        eprintln!("Magnetite: the key window has no root view controller");
        return;
    };
    // One item, the log's text, as the widest object type the sheet accepts.
    let text: Retained<NSString> = NSString::from_str(text);
    let item: Retained<AnyObject> = Retained::into_super(Retained::into_super(text));
    let items: Retained<NSArray> = NSArray::from_vec(vec![item]);
    // SAFETY: UIKit's documented initializer with the array's documented element type, on the main
    // thread; no application activities means the system's own list, and no completion handler
    // means there is no block to outlive the call. The sheet keeps its own copy of the items.
    let sheet = unsafe {
        UIActivityViewController::initWithActivityItems_applicationActivities(
            mtm.alloc::<UIActivityViewController>(),
            &items,
            None,
        )
    };
    // SAFETY: UIKit's documented properties. On iPad the sheet is a popover and needs an anchor:
    // the root view, which the root controller holds for as long as the sheet is up.
    unsafe {
        if let Some(popover) = sheet.popoverPresentationController() {
            popover.setSourceView(root.view().as_deref());
        }
        root.presentViewController_animated_completion(&sheet, true, None);
    }
}

/// The executable is iOS's only; elsewhere it says so.
#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("magnetite-ios runs on iOS only: build it with --target aarch64-apple-ios");
}

#[cfg(test)]
mod tests {
    use super::expiration_of;

    /// The plist inside a profile, with the keys a real one carries.
    const PLIST: &str = concat!(
        "<plist version=\"1.0\"><dict>",
        "<key>AppIDName</key><string>Magnetite</string>",
        "<key>ExpirationDate</key>",
        "<date>2026-12-01T10:20:30Z</date>",
        "<key>Name</key><string>Magnetite development</string>",
        "</dict></plist>",
    );

    /// A profile as Apple ships it: the CMS signature's bytes around the plist.
    fn profile(plist: &str) -> Vec<u8> {
        [
            &[
                0x30, 0x82, 0x0b, 0xd1, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d,
            ][..],
            plist.as_bytes(),
            &[0x31, 0x82, 0x09, 0xf7, 0x30, 0x82, 0x09, 0xf3][..],
        ]
        .concat()
    }

    #[test]
    fn reads_the_expiration_between_the_cms_bytes() {
        assert_eq!(expiration_of(&profile(PLIST)), Some(1_796_120_430));
    }

    #[test]
    fn a_profile_without_the_key_has_no_expiration() {
        let keyless = PLIST.replace("ExpirationDate", "CreationDate");
        assert_eq!(expiration_of(&profile(&keyless)), None);
    }

    #[test]
    fn a_malformed_date_has_no_expiration() {
        for date in [
            "2026-13-01T10:20:30Z", // no thirteenth month
            "2026-12-01 10:20:30Z", // a space where the plist writes `T`
            "2026-12-01T10:20Z",    // no seconds
            "not a date",
        ] {
            let plist = PLIST.replace("2026-12-01T10:20:30Z", date);
            assert_eq!(expiration_of(&profile(&plist)), None, "{date}");
        }
    }

    #[test]
    fn a_date_with_no_closing_tag_has_no_expiration() {
        let cut = PLIST.find("</date>").unwrap();
        assert_eq!(expiration_of(&profile(&PLIST[..cut])), None);
    }
}
