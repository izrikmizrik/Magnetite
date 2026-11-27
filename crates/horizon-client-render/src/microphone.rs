//! The microphone for mods: the system's permission to record, and a capture stream of the default
//! input device, mono `f32` samples at the device's own rate.
//!
//! The permission is the platform's: a phone's host sets [`Platform`] at start-up (Android's
//! `RECORD_AUDIO`, iOS's `AVAudioSession`), and a PC, which asks nothing of the game, answers
//! granted. The capture itself is `cpal` on every platform, as the game's sound output; without the
//! `audio` feature there is no microphone.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, OnceLock},
};

/// Whether the system lets the game record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Recording is allowed.
    Granted,
    /// The player said no, or a policy does: only the system's settings change it now.
    Denied,
    /// Not asked yet: [`request`] shows the system's question.
    Undetermined,
    /// This build or device has no microphone to give.
    Unavailable,
}

/// What a phone's host supplies: its permission state, and the system's question.
#[derive(Clone, Copy)]
pub struct Platform {
    /// The permission as the system has it now.
    pub permission: fn() -> Permission,
    /// Shows the system's question, when it is still to be asked; the answer comes later, read
    /// through `permission`.
    pub request: fn(),
    /// Readies the platform's audio for recording before a capture opens (iOS: the audio session's
    /// play-and-record category).
    pub prepare: fn(),
}

static PLATFORM: OnceLock<Platform> = OnceLock::new();

/// Sets the platform's permission functions, once, before the game starts.
pub fn set_platform(platform: Platform) {
    let _ = PLATFORM.set(platform);
}

/// The permission to record now.
pub fn permission() -> Permission {
    if !cfg!(feature = "audio") {
        return Permission::Unavailable;
    }
    match PLATFORM.get() {
        Some(platform) => (platform.permission)(),
        // A PC: the system asks the player itself when a stream opens, if it asks at all.
        None => Permission::Granted,
    }
}

/// Asks the system for the permission, when it is still to be asked.
pub fn request() {
    if permission() == Permission::Undetermined
        && let Some(platform) = PLATFORM.get()
    {
        (platform.request)();
    }
}

/// At most this many seconds of samples wait for [`Capture::take`]: older ones are dropped, so a
/// mod that stops reading costs no more memory.
#[cfg(feature = "audio")]
const KEPT_SECONDS: usize = 2;

/// A recording from the default input device. Dropping it stops the recording.
pub struct Capture {
    #[cfg(feature = "audio")]
    _stream: cpal::Stream,
    samples: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
}

impl Capture {
    /// Opens the default input device. Fails, with why, without the permission or a device.
    pub fn open() -> Result<Self, String> {
        match permission() {
            Permission::Granted => {}
            Permission::Unavailable => return Err("this build has no microphone".into()),
            other => return Err(format!("recording is not allowed ({other:?})")),
        }
        if let Some(platform) = PLATFORM.get() {
            (platform.prepare)();
        }
        Self::open_device()
    }

    #[cfg(feature = "audio")]
    fn open_device() -> Result<Self, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let device = cpal::default_host()
            .default_input_device()
            .ok_or("no input device")?;
        let config = device.default_input_config().map_err(|e| e.to_string())?;
        let rate = config.sample_rate().0;
        let samples = Arc::new(Mutex::new(VecDeque::with_capacity(
            rate as usize * KEPT_SECONDS,
        )));
        let stream_config: cpal::StreamConfig = config.clone().into();
        let channels = usize::from(config.channels()).max(1);
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                build::<f32>(&device, &stream_config, channels, rate, &samples)
            }
            cpal::SampleFormat::I16 => {
                build::<i16>(&device, &stream_config, channels, rate, &samples)
            }
            cpal::SampleFormat::U16 => {
                build::<u16>(&device, &stream_config, channels, rate, &samples)
            }
            cpal::SampleFormat::I32 => {
                build::<i32>(&device, &stream_config, channels, rate, &samples)
            }
            other => return Err(format!("the input's sample format {other} is not read")),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            samples,
            rate,
        })
    }

    #[cfg(not(feature = "audio"))]
    fn open_device() -> Result<Self, String> {
        Err("this build has no microphone".into())
    }

    /// The samples per second of [`Capture::take`]'s samples.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Moves the samples recorded since the last call into `out` (cleared first): mono, -1 to 1.
    pub fn take(&self, out: &mut Vec<f32>) {
        out.clear();
        if let Ok(mut samples) = self.samples.lock() {
            out.extend(samples.drain(..));
        }
    }
}

/// The input stream: each callback's frames, their channels averaged, onto `samples`.
#[cfg(feature = "audio")]
fn build<T: cpal::SizedSample>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    rate: u32,
    samples: &Arc<Mutex<VecDeque<f32>>>,
) -> Result<cpal::Stream, String>
where
    f32: cpal::FromSample<T>,
{
    use cpal::traits::DeviceTrait;
    let samples = samples.clone();
    let kept = rate as usize * KEPT_SECONDS;
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let Ok(mut samples) = samples.try_lock() else {
                    return;
                };
                samples.extend(data.chunks(channels).map(|frame| {
                    frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / frame.len() as f32
                }));
                if samples.len() > kept {
                    let over = samples.len() - kept;
                    samples.drain(..over);
                }
            },
            |e| log::warn!("Magnetite: microphone stream error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pc_without_a_platform_is_granted_with_audio() {
        let expected = if cfg!(feature = "audio") {
            Permission::Granted
        } else {
            Permission::Unavailable
        };
        assert_eq!(permission(), expected);
    }
}
