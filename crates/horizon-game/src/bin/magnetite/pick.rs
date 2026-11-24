//! The PC's file picker for the launcher: the system's own open dialog, on a thread of its own so
//! that the window keeps answering. What was picked is copied into the system's temporary folder,
//! since the boot loop removes the file it is given.

use horizon_client::boot::PickKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static PICKED: Mutex<Option<Result<PathBuf, String>>> = Mutex::new(None);
/// A dialog is up: another tap does not open a second one.
static OPEN: AtomicBool = AtomicBool::new(false);

/// Shows the dialog for a file of `kind`; true when it is up.
pub fn pick(kind: PickKind) -> bool {
    if OPEN.swap(true, Ordering::AcqRel) {
        return true;
    }
    let spawned = std::thread::Builder::new()
        .name("file-dialog".into())
        .spawn(move || {
            let answer = dialog(kind);
            if let Some(path) = answer.transpose() {
                let copied = path.and_then(|path| copy(&path));
                if let Ok(mut picked) = PICKED.lock() {
                    *picked = Some(copied);
                }
            }
            OPEN.store(false, Ordering::Release);
        });
    if spawned.is_err() {
        OPEN.store(false, Ordering::Release);
    }
    spawned.is_ok()
}

/// The file picked since the last call.
pub fn picked() -> Option<Result<PathBuf, String>> {
    PICKED.lock().ok()?.take()
}

/// A copy of `path`, under its own name.
fn copy(path: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("no file")?;
    let folder = std::env::temp_dir().join(format!("magnetite-picked-{}", std::process::id()));
    let copied = std::fs::create_dir_all(&folder)
        .and_then(|()| std::fs::copy(path, folder.join(name)))
        .map_err(|e| format!("{}: {e}", path.display()));
    copied.map(|_| folder.join(name))
}

/// The kind's extensions, without their dots.
fn extensions(kind: PickKind) -> &'static [&'static str] {
    match kind {
        PickKind::Mod => &["hmod"],
        PickKind::Picture => &["png", "jpg", "jpeg", "webp"],
    }
}

/// The file the player chose, or none when they closed the dialog.
#[cfg(windows)]
fn dialog(kind: PickKind) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        CommDlgExtendedError, GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY,
        OFN_NOCHANGEDIR, OPENFILENAMEW,
    };
    let patterns: Vec<String> = extensions(kind).iter().map(|e| format!("*.{e}")).collect();
    let patterns = patterns.join(";");
    // Pairs of a name and its patterns, each ended by a zero, the list by one more.
    let filter: Vec<u16> = format!("{patterns}\0{patterns}\0\0")
        .encode_utf16()
        .collect();
    let mut file = [0u16; 4096];
    // SAFETY: a plain C structure, for which zeroes are its empty values.
    let mut ask: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ask.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ask.lpstrFilter = filter.as_ptr();
    ask.lpstrFile = file.as_mut_ptr();
    ask.nMaxFile = file.len() as u32;
    // The game folder is the working directory, and stays it.
    ask.Flags = OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_HIDEREADONLY | OFN_NOCHANGEDIR;
    // SAFETY: every pointer above outlives the call, and the buffer is as long as it says.
    if unsafe { GetOpenFileNameW(&mut ask) } == 0 {
        // SAFETY: reads this thread's last dialog error; zero is the player closing it.
        return match unsafe { CommDlgExtendedError() } {
            0 => Ok(None),
            code => Err(format!("the file dialog failed ({code:#x})")),
        };
    }
    let end = file.iter().position(|c| *c == 0).unwrap_or(file.len());
    Ok(Some(std::ffi::OsString::from_wide(&file[..end]).into()))
}

/// The file the player chose, or none when they closed the dialog: `osascript`'s on macOS, the
/// desktop's `zenity` or `kdialog` elsewhere.
#[cfg(not(windows))]
fn dialog(kind: PickKind) -> Result<Option<PathBuf>, String> {
    let patterns: Vec<String> = extensions(kind).iter().map(|e| format!("*.{e}")).collect();
    let commands: Vec<Vec<String>> = if cfg!(target_os = "macos") {
        vec![vec![
            "osascript".into(),
            "-e".into(),
            "POSIX path of (choose file)".into(),
        ]]
    } else {
        let patterns = patterns.join(" ");
        vec![
            vec![
                "zenity".into(),
                "--file-selection".into(),
                format!("--file-filter={patterns}"),
            ],
            vec![
                "kdialog".into(),
                "--getopenfilename".into(),
                ".".into(),
                patterns,
            ],
        ]
    };
    for command in commands {
        // One that is not installed gives way to the next.
        let Ok(output) = std::process::Command::new(&command[0])
            .args(&command[1..])
            .output()
        else {
            continue;
        };
        let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Ok((output.status.success() && !path.is_empty()).then(|| path.into()));
    }
    Err("no file dialog: install zenity or kdialog, or drop the file on the window".into())
}
