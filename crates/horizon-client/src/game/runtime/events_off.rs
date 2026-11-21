// game-module-cfg: not(feature = "agent-control")
// game-module-name: events
//! The agent event log left out of the build (the `agent-control` feature off): nothing
//! watches for `events` or `wait-for`. The damage packet's note still lands, going nowhere.
//! game/runtime/events.rs is the real one.
#[derive(Default)]
pub(super) struct Events;

impl Events {
    /// `ClientboundDamageEventPacket` for this player: no log to keep it in.
    pub(super) fn note_hit(&mut self, _cause: String, _by: Option<String>) {}
}
