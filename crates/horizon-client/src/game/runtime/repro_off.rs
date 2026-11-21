// game-module-cfg: not(feature = "agent-control")
// game-module-name: repro
//! Repro recordings left out of the build (the `agent-control` feature off): nothing records,
//! exports or replays a session. The hooks the tick, the world session and the F3 keys call
//! stay, doing nothing. game/runtime/repro.rs is the real one.
use super::Game;

#[derive(Default)]
pub(super) struct Repro;

impl Repro {
    /// No replay runs in this build.
    pub(super) fn replaying(&self) -> bool {
        false
    }
    /// No recording runs in this build.
    pub(super) fn recording_active(&self) -> bool {
        false
    }
}

impl Game<'_> {
    /// No recording to tick.
    pub(super) fn repro_tick(&mut self) {}
    /// No recording to stop at exit.
    pub(super) fn repro_on_exit(&mut self) {}
    /// No recording to stop when the world closes.
    pub(super) fn repro_world_closed(&mut self) {}
    /// The F3 recording shortcut: nothing to toggle.
    pub(super) fn toggle_recording_key(&mut self) {}
    /// The F3 export shortcut: nothing to export.
    pub(super) fn export_repro_key(&mut self) {}
}
