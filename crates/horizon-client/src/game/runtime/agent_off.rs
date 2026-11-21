// game-module-cfg: not(feature = "agent-control")
// game-module-name: agent
//! The agent left out of the build (the `agent-control` feature off): no task runs and no
//! glide turns the view. The control-fixture barrier the server tick reads stays, never held.
//! game/runtime/agent/mod.rs is the real one.
use super::Game;

#[derive(Default)]
pub(super) struct Agent {
    /// Control-fixture barrier: hold simulation and packet handling, not transport threads.
    pub hold_server: bool,
}

impl Game<'_> {
    /// No agent action to cancel in this build.
    pub(super) fn agent_cancel(&mut self, _why: &str) {}
    /// A block state as text, as the real agent module gives it (the F3 keys read it too).
    pub(super) fn state_string(&self, state: horizon_world::registry::BlockStateId) -> String {
        self.block_state_string(state).unwrap_or_else(|| "?".into())
    }
}
