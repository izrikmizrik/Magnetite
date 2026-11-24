//! The `microphone_permission_changed` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The system's permission to record changed to `permission`: the player answered the
    /// question `request_microphone` showed, or changed it in the system's settings while the
    /// game ran. Checked every half second; every client mod hears it.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "none (Magnetite's own: Android's onRequestPermissionsResult, iOS's requestRecordPermission answer)"
    )]
    fn microphone_permission_changed();
}
