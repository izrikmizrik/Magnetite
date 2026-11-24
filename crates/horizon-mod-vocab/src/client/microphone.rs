//! The microphone for a client mod: whether the system lets the game record, and why a recording
//! did not start.

/// Whether the system lets the game record (Android's `RECORD_AUDIO`, iOS's microphone
/// permission; a PC answers `Granted`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum MicrophonePermission {
    /// Recording is allowed.
    Granted,
    /// The player said no, or a policy does: only the system's settings change it now.
    Denied,
    /// Not asked yet: `request_microphone` shows the system's question.
    Undetermined,
    /// This build or device has no microphone to give, or there is no client.
    Unavailable,
}

/// Why the microphone did not start recording.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
#[derive(horizon_mod_api_macros::Wire)]
pub enum MicrophoneError {
    /// The system does not let the game record: ask with `request_microphone`, or the player
    /// allows it in the system's settings.
    NotAllowed(MicrophonePermission),
    /// No input device opened: none is there, or the system refused it, with its reason.
    Device(String),
    /// There is no client to record for (a server's thread), or no game backend.
    Unavailable,
}

impl std::fmt::Display for MicrophoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed(permission) => write!(f, "recording is not allowed ({permission:?})"),
            Self::Device(reason) => write!(f, "the microphone did not open: {reason}"),
            Self::Unavailable => f.write_str("no client records here"),
        }
    }
}

impl std::error::Error for MicrophoneError {}

impl crate::NoBackend for MicrophoneError {
    fn no_backend() -> Self {
        Self::Unavailable
    }
}
