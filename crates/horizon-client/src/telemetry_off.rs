//! The telemetry store left out of the build (the `telemetry-store` feature off, as in the
//! mobile build): nothing is kept for a launcher to send, and there is no consent to read, so a
//! crash report stays the local file `crash::install` writes. The names the rest of the game
//! calls stay: the start is not timed, a play session records nothing, and the two reads (the
//! version, the adapter's name) answer as they did. telemetry.rs is the real one.
use std::path::Path;

use crate::spans::Span;

/// When the game started: nothing is timed without the store.
pub fn mark_start() {}

/// The adapter's name, once the renderer has one.
pub fn gpu_name() -> Option<String> {
    crate::gfx::adapter().map(|gpu| gpu.name)
}

/// The release (`v0.2.3`), or the client's crate version for a build from source.
pub fn version() -> String {
    crate::crash::release_tag().unwrap_or_else(|| format!("dev {}", env!("CARGO_PKG_VERSION")))
}

/// A crash report is not kept for sending: `crash::install` already saved it beside the game.
pub fn keep_crash(_home: &Path, _report: &crate::crash::Report) {}

/// One play session's performance: without the store nothing is recorded and nothing is kept.
#[derive(Default)]
pub struct Session;

impl Session {
    /// The title screen drew a frame.
    pub fn title_shown(&mut self) {}

    /// A world starts loading.
    pub fn world_opening(&mut self) {}

    /// An entry queued during startup was cancelled before opening a world.
    pub fn cancel_world_opening(&mut self) {}

    /// A frame drawn while playing, with the spans the render thread recorded in it.
    pub fn frame(&mut self, _ms: f32, _spans: &[Span]) {}
}
