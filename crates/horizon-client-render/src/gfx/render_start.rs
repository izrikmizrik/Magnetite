//! A window's first renderer start and what a driver crash there leaves
//! (`graphics::RENDER_MARKER`). On a system that recovers from such a crash itself
//! (`graphics::handles_render_crash`), the marker is written beside `options.txt` before the device
//! search, names the backend once a device is open, and goes when the renderer presents its first
//! frame, or is dropped: a failed start that returns an error, a clean exit. Only a crash inside
//! the driver leaves it, which the next start's options take as a reason to change the graphics
//! API (`Settings::load`).
use std::path::PathBuf;

/// The marker of a renderer that has not presented its first frame yet; removed when dropped.
pub(crate) struct StartMarker(PathBuf);

impl StartMarker {
    /// Writes the marker in the player's folder for a window's first device on this system, or
    /// `None` where the game leaves driver crashes to the launcher.
    pub(crate) fn begin() -> Option<Self> {
        if !crate::graphics::handles_render_crash(std::env::consts::OS) {
            return None;
        }
        let path = crate::logging::player_dir().join(crate::graphics::RENDER_MARKER);
        match std::fs::write(&path, crate::graphics::render_marker_text("")) {
            Ok(()) => Some(Self(path)),
            Err(e) => {
                log::warn!("Horizon renderer: {} is not written: {e}", path.display());
                None
            }
        }
    }

    /// The device opened on `backend`: the marker names it, so a crash from here to the first
    /// frame is that backend's.
    pub(crate) fn opened_on(&self, backend: wgpu::Backend) {
        let name = match backend {
            wgpu::Backend::Vulkan => "vulkan",
            wgpu::Backend::Gl => "gl",
            wgpu::Backend::Metal => "metal",
            wgpu::Backend::Dx12 => "dx12",
            _ => "other",
        };
        let _ = std::fs::write(&self.0, crate::graphics::render_marker_text(name));
    }
}

impl Drop for StartMarker {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
