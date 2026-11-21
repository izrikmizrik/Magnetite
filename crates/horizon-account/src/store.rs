//! Where the refresh secret is kept between runs. Only the refresh secret, the kind, the name
//! and the uuid are ever stored — never the access token (it lives in memory, like the game's
//! session file). The OS keychain and Android's secure store are other backends of
//! [`SecretStore`], added by the platforms that have them; this crate ships the portable one: a
//! private file, 0600 on Unix, replaced atomically.

use std::io;
use std::path::PathBuf;

/// Where the refresh secret is kept. The stored value is the JSON of
/// [`crate::Account::stored_json`].
pub trait SecretStore {
    fn load(&self) -> io::Result<Option<String>>;
    fn save(&self, json: &str) -> io::Result<()>;
    fn clear(&self) -> io::Result<()>;
}

/// A private file holding the one stored account: `json` written whole through a temporary file
/// of this write's own beside it, so a crash leaves the old file or the new one. On Unix the
/// file is readable and writable by its owner only (0600).
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn shown(&self, e: io::Error) -> io::Error {
        io::Error::new(e.kind(), format!("{}: {e}", self.path.display()))
    }

    /// Replaces the file with `json`: a unique temporary file beside it, synced, then renamed.
    fn write(&self, json: &str) -> io::Result<()> {
        static WRITES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = WRITES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = self
            .path
            .with_extension(format!("json.{}.{n}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let written = options.open(&tmp).and_then(|mut file| {
            use std::io::Write;
            file.write_all(json.as_bytes())?;
            file.sync_all()
        });
        if let Err(e) = written.and_then(|()| std::fs::rename(&tmp, &self.path)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(self.shown(e));
        }
        Ok(())
    }
}

impl SecretStore for FileStore {
    fn load(&self) -> io::Result<Option<String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(self.shown(e)),
        }
    }

    fn save(&self, json: &str) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| self.shown(e))?;
        }
        self.write(json)
    }

    fn clear(&self) -> io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(self.shown(e)),
            Ok(()) => Ok(()),
        }
    }
}
