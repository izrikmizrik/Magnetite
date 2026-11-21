// game-module-cfg: not(feature = "agent-control")
// game-module-name: agent_settings
//! The AI Agent settings page left out of the build (the `agent-control` feature off): the
//! socket never opens, so the page has no state to tell of. game/runtime/agent_settings.rs
//! is the real one.
use super::Game;
use horizon_client::Result;

impl Game<'_> {
    /// No socket to open at startup in this build.
    pub(super) fn start_agent_control(&mut self) -> Result<()> {
        Ok(())
    }

    /// No socket to open or close after a settings change.
    pub(super) fn sync_agent_control(&mut self) {}

    /// The page's lines: none, the page is not reached in this build.
    pub(super) fn agent_status_lines(&self) -> Vec<String> {
        Vec::new()
    }

    /// No connection details to copy in this build.
    pub(super) fn agent_details(&self) -> String {
        String::new()
    }
}
