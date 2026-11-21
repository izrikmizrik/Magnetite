//! The desktop's own handler left out of the build (the `desktop-opener` feature off, as in the
//! mobile build): opening a folder or a link needs a desktop's shell (xdg-open, `open`,
//! ShellExecute). The names the game calls stay and do nothing, so an "Open Folder" button
//! logs that it cannot open one instead of failing the screen. desktop.rs is the real one.
use std::io;

/// Opens `path` without waiting for its handler: there is none in this build.
pub fn open_path(_path: &std::path::Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this build cannot open a folder (the `desktop-opener` feature)",
    ))
}

/// `Blaze3D.openUri`: there is no handler for a link in this build either.
pub fn open_uri(_uri: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "this build cannot open a link (the `desktop-opener` feature)",
    ))
}
