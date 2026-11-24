//! The microphone, reached by client mods.
#![allow(unused_variables)]
use crate::*;

/// The player's microphone for a client mod: the system's permission, and a recording whose
/// samples come to the mod through the `microphone_input` hook.
#[horizon_mod_api_macros::host_area]
pub trait ClientMicrophoneClientHost {
    /// Side: client.
    /// Whether the system lets the game record now. On a phone it is the app's permission
    /// (Android's `RECORD_AUDIO`, iOS's microphone access), `Undetermined` until the player was
    /// asked; on a PC it is `Granted`, and the system's own privacy settings may still refuse
    /// the device when recording starts.
    #[access(ClientSetup)]
    fn microphone_permission(&self) -> MicrophonePermission;
    /// Side: client.
    /// Shows the system's question for the microphone when the player was not asked yet, and
    /// gives the permission as it is now: the answer comes later, as the
    /// `microphone_permission_changed` hook. Asks nothing once the player answered: a denied
    /// permission is changed in the system's settings only.
    #[access(ClientSetup)]
    fn request_microphone(&mut self) -> MicrophonePermission;
    /// Side: client.
    /// Starts recording from the system's default input for this mod, and gives the samples per
    /// second it records at. From the next client tick on, `microphone_input` brings this mod
    /// what was recorded since the tick before: mono samples from -1 to 1. Several mods may
    /// record at once, each told separately. Starting again while recording changes nothing.
    /// Refused without the permission or a device, by name, as [`MicrophoneError`] lists.
    #[access(ClientSetup)]
    fn start_microphone(&mut self) -> Result<u32, MicrophoneError>;
    /// Side: client.
    /// Stops this mod's recording: `microphone_input` no longer comes to it. The device closes
    /// when no mod records. Nothing happens when this mod is not recording.
    #[access(ClientSetup)]
    fn stop_microphone(&mut self);
}
