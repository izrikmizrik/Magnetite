// game-module-cfg: not(feature = "agent-control")
// game-module-name: input_trace
//! The input trace left out of the build (the `agent-control` feature off): no `input-log`
//! and no repro recording reads it, so input events are not kept. The origins and kinds the
//! window's and the pad's events carry stay, going nowhere. game/runtime/input_trace.rs is
//! the real one.
use super::Game;
use winit::event::WindowEvent;

/// Who caused an input event: a person at this computer, or the game itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InputOrigin {
    /// The window's own events: someone at this computer's keyboard, mouse or pad.
    User,
    /// Neither: input the game made up itself (pad menus, UI tasks).
    Game,
}

/// What was pressed, clicked or typed, as the window's and the pad's events carry it.
#[derive(Clone, Debug)]
pub(super) enum TraceKind {
    /// Relative mouse motion, which turns the view.
    #[cfg_attr(not(feature = "agent-control"), allow(dead_code))]
    Motion { x: f64, y: f64 },
    #[cfg_attr(not(feature = "agent-control"), allow(dead_code))]
    Pad { button: String, action: String },
}

impl Game<'_> {
    /// No trace to record an input event in.
    pub(super) fn note_input(&mut self, _origin: InputOrigin, _kind: TraceKind) {}

    /// No trace to keep the window's input events in.
    pub(super) fn trace_window_event(&mut self, _origin: InputOrigin, _event: &WindowEvent) {}
}
