// game-module-cfg: not(feature = "agent-control")
// game-module-name: screenshot_sequence
//! `screenshot-sequence` left out of the build (the `agent-control` feature off): no control
//! command starts one, so none runs. The frame loop's hooks stay, seeing none.
//! game/runtime/screenshot_sequence.rs is the real one.
use super::Game;
use horizon_client::{Result, gfx::pack_uniforms::FrameTiming};
use serde_json::Value;

/// No sequence can run in this build.
pub(super) enum Sequence {}

impl Sequence {
    pub(super) fn timing(&self) -> FrameTiming {
        match *self {}
    }
}

impl Game<'_> {
    /// No sequence to end.
    pub(super) fn end_screenshot_sequence(&mut self, _outcome: Result<Value>) {}
    /// No sequence to draw a frame of.
    pub(super) fn render_sequence_frame(&mut self) -> Result<bool> {
        Ok(false)
    }
}
