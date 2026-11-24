//! What a phone has beyond a window and a touch screen: a vibration motor and a gyroscope, which
//! the host gives once at start-up ([`set_platform`]), and the player's choices about them
//! ([`PhoneSettings`], `horizon-touch.txt` beside `options.txt`, so each launcher profile has its
//! own). A PC gives neither and has no such file.

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// The settings' file, in the player's folder.
pub const FILE: &str = "horizon-touch.txt";
/// The gyroscope's sensitivities the launcher offers, in percent.
pub const GYRO_SENSITIVITIES: [u32; 7] = [25, 50, 75, 100, 150, 200, 300];

/// The host's gyroscope.
#[derive(Clone, Copy)]
pub struct Gyro {
    /// Starts reading the sensor (the world is played). Starting twice changes nothing.
    pub start: fn(),
    /// Stops reading it. Stopping twice changes nothing.
    pub stop: fn(),
    /// How far the phone turned since the last call, in radians, for the screen held on its side:
    /// `[yaw, pitch]`, yaw positive turning right, pitch positive looking down.
    pub take: fn() -> [f64; 2],
}

/// What the host's phone has; `None` where the platform or the device lacks it.
#[derive(Clone, Copy, Default)]
pub struct Platform {
    /// One short tick of the vibration motor.
    pub haptic: Option<fn()>,
    pub gyro: Option<Gyro>,
}

static PLATFORM: OnceLock<Platform> = OnceLock::new();

/// Sets what the host has, once, before the boot loop.
pub fn set_platform(platform: Platform) {
    let _ = PLATFORM.set(platform);
}

/// What the host has.
pub fn platform() -> Platform {
    PLATFORM.get().copied().unwrap_or_default()
}

/// The player's choices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhoneSettings {
    /// A touch button's press ticks the vibration motor.
    pub vibration: bool,
    /// Turning the phone turns the view.
    pub gyro: bool,
    /// How far the view turns, in percent of the phone's own turn.
    pub gyro_sensitivity: u32,
    /// The game draws under the camera cutout too (Android).
    pub cutout: bool,
}

impl Default for PhoneSettings {
    fn default() -> Self {
        Self {
            vibration: true,
            gyro: false,
            gyro_sensitivity: 100,
            cutout: false,
        }
    }
}

impl PhoneSettings {
    /// The settings kept in `dir`, the defaults for what is missing or unreadable.
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(FILE)).map_or_else(|_| Self::default(), |t| Self::parse(&t))
    }

    fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            let flag = |old: bool| value.parse().unwrap_or(old);
            match key.trim() {
                "vibration" => out.vibration = flag(out.vibration),
                "gyro" => out.gyro = flag(out.gyro),
                "gyroSensitivity" => {
                    out.gyro_sensitivity = value
                        .parse()
                        .map_or(out.gyro_sensitivity, |v: u32| v.clamp(10, 500));
                }
                "cutout" => out.cutout = flag(out.cutout),
                _ => {}
            }
        }
        out
    }

    fn text(&self) -> String {
        format!(
            "vibration:{}\ngyro:{}\ngyroSensitivity:{}\ncutout:{}\n",
            self.vibration, self.gyro, self.gyro_sensitivity, self.cutout
        )
    }

    pub fn save(&self, dir: &Path) {
        if let Err(e) = fs::write(dir.join(FILE), self.text()) {
            log::warn!("Magnetite: {FILE} is not saved: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_read_back_and_a_bad_line_keeps_its_default() {
        let set = PhoneSettings {
            vibration: false,
            gyro: true,
            gyro_sensitivity: 150,
            cutout: false,
        };
        assert_eq!(PhoneSettings::parse(&set.text()), set);
        assert_eq!(
            PhoneSettings::parse("vibration:perhaps\ngyroSensitivity:9000\nnoise"),
            PhoneSettings {
                gyro_sensitivity: 500,
                ..PhoneSettings::default()
            }
        );
    }
}
