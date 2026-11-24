//! `ClientMicrophoneClientHost`: the player's microphone for client mods
//! (`horizon_client::microphone`), and the hooks that bring them its samples
//! (`microphone_input`) and the system's answer (`microphone_permission_changed`).
use super::*;
use horizon_client::microphone::{self, Capture, Permission};
use mod_api::{MicrophoneError, MicrophonePermission};
use std::cell::RefCell;

/// The recording the mods share: one device stream, and the mods it is for.
#[derive(Default)]
struct Recording {
    capture: Option<Capture>,
    mods: Vec<String>,
    /// The permission the last tick saw, to tell mods when it changes.
    seen: Option<MicrophonePermission>,
    /// Ticks since the permission was last read: a phone's read goes through the system.
    since_read: u32,
    samples: Vec<f32>,
}

/// The permission is read every this many ticks (half a second).
const PERMISSION_EVERY: u32 = 10;

thread_local! {
    // The client's `Game` runs on one thread; a server thread's has no microphone.
    static RECORDING: RefCell<Recording> = RefCell::new(Recording::default());
}

fn to_api(permission: Permission) -> MicrophonePermission {
    match permission {
        Permission::Granted => MicrophonePermission::Granted,
        Permission::Denied => MicrophonePermission::Denied,
        Permission::Undetermined => MicrophonePermission::Undetermined,
        Permission::Unavailable => MicrophonePermission::Unavailable,
    }
}

impl Game<'_> {
    pub(in crate::game) fn microphone_permission(&self) -> MicrophonePermission {
        if !self.options.execution_role.is_client() {
            return MicrophonePermission::Unavailable;
        }
        to_api(microphone::permission())
    }

    pub(in crate::game) fn request_microphone(&mut self) -> MicrophonePermission {
        if !self.options.execution_role.is_client() {
            return MicrophonePermission::Unavailable;
        }
        microphone::request();
        to_api(microphone::permission())
    }

    pub(in crate::game) fn start_microphone(&mut self) -> Result<u32, MicrophoneError> {
        if !self.options.execution_role.is_client() {
            return Err(MicrophoneError::Unavailable);
        }
        let Some(owner) = mod_api::content::crash::current_native_mod() else {
            return Err(MicrophoneError::Unavailable);
        };
        let permission = to_api(microphone::permission());
        if permission != MicrophonePermission::Granted {
            return Err(MicrophoneError::NotAllowed(permission));
        }
        RECORDING.with_borrow_mut(|recording| {
            if recording.capture.is_none() {
                let capture = Capture::open().map_err(MicrophoneError::Device)?;
                log::info!(
                    "Magnetite: the microphone records at {} Hz for {owner}",
                    capture.rate()
                );
                recording.capture = Some(capture);
            }
            if !recording.mods.iter().any(|m| m == owner) {
                recording.mods.push(owner.to_owned());
            }
            Ok(recording.capture.as_ref().map_or(0, Capture::rate))
        })
    }

    pub(in crate::game) fn stop_microphone(&mut self) {
        let Some(owner) = mod_api::content::crash::current_native_mod() else {
            return;
        };
        RECORDING.with_borrow_mut(|recording| {
            recording.mods.retain(|m| m != owner);
            if recording.mods.is_empty() && recording.capture.take().is_some() {
                log::info!("Magnetite: the microphone stopped");
            }
        });
    }

    /// Once a client tick: the permission's change to every mod, then what was
    /// recorded to each mod recording.
    pub(in crate::game) fn tick_microphone(&mut self) {
        if !self.options.execution_role.is_client() {
            return;
        }
        let due = RECORDING.with_borrow_mut(|recording| {
            recording.since_read = (recording.since_read + 1) % PERMISSION_EVERY;
            recording.since_read == 0
        });
        if due {
            let permission = to_api(microphone::permission());
            let changed = RECORDING.with_borrow_mut(|recording| {
                let before = recording.seen.replace(permission);
                before.is_some_and(|before| before != permission)
            });
            if changed {
                self.native_microphone_permission_changed(permission);
            }
        }
        let taken = RECORDING.with_borrow_mut(|recording| {
            let capture = recording.capture.as_ref()?;
            let mut samples = std::mem::take(&mut recording.samples);
            capture.take(&mut samples);
            Some((samples, capture.rate(), recording.mods.clone()))
        });
        let Some((samples, rate, mods)) = taken else {
            return;
        };
        if !samples.is_empty() {
            for owner in &mods {
                self.native_microphone_input(owner, &samples, rate);
            }
        }
        // The buffer goes back for the next tick's samples.
        RECORDING.with_borrow_mut(|recording| recording.samples = samples);
    }

    /// What the microphone recorded since the last client tick, for mod
    /// `mod_id`, which started recording with `start_microphone`: `samples`
    /// are mono, from -1 to 1, at `sample_rate` samples per second.
    pub(in crate::game) fn native_microphone_input(
        &mut self,
        mod_id: &str,
        samples: &[f32],
        sample_rate: u32,
    ) {
        self.fire_microphone_input(mod_id, samples, sample_rate);
    }

    /// The system's permission to record changed to `permission`.
    pub(in crate::game) fn native_microphone_permission_changed(
        &mut self,
        permission: MicrophonePermission,
    ) {
        self.fire_microphone_permission_changed(permission);
    }
}
