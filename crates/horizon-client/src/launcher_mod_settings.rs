//! The settings mods register (`Registrar::register_setting`), for the launcher's mod pages.
//!
//! A mod says what its settings are only by running, and the launcher runs none. So the game writes
//! what its mods registered, with the labels in the player's language, into [`FILE`] beside the
//! mods' data each time it loads them; the launcher reads that and keeps the values in the mods'
//! own data folders, one file a key, as the game's own pages do (`game/native_hooks/mod_settings.rs`).
use mod_api::{SettingKind, SettingSpec, SettingValue};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The mods' data folder, in `horizon-mods/`.
pub const DATA_DIR: &str = ".horizon-mod-data";
/// What the mods registered, in the mods' data folder. No mod id starts with a dot.
pub const FILE: &str = ".settings.json";

/// One setting as the launcher shows it: the mod's own declaration, its label and its options in
/// the player's language.
#[derive(Clone, Debug, PartialEq)]
pub struct ModSetting {
    pub spec: SettingSpec,
    pub label: String,
    pub options: Vec<String>,
}

impl ModSetting {
    /// `spec` with its texts looked up by `text`, a translation key's.
    pub fn new(spec: &SettingSpec, text: impl Fn(&str) -> String) -> Self {
        let options = match &spec.kind {
            SettingKind::Choice { options, .. } => options.iter().map(|o| text(o)).collect(),
            _ => Vec::new(),
        };
        Self {
            label: text(&spec.label),
            options,
            spec: spec.clone(),
        }
    }

    fn json(&self) -> Value {
        let kind = match &self.spec.kind {
            SettingKind::Toggle { default } => json!({"toggle": default}),
            SettingKind::Choice { options, default } => {
                json!({"choice": options, "default": default, "shown": self.options})
            }
            SettingKind::Range {
                min,
                max,
                step,
                default,
            } => json!({"range": [min, max, step], "default": default}),
            SettingKind::Color { default } => json!({"color": default}),
            _ => Value::Null,
        };
        json!({"key": self.spec.key, "label": self.spec.label, "shown": self.label, "kind": kind})
    }

    fn parse(value: &Value) -> Option<Self> {
        let text = |v: &Value| v.as_str().map(str::to_owned);
        let list = |v: &Value| -> Option<Vec<String>> { v.as_array()?.iter().map(text).collect() };
        let (key, label) = (text(&value["key"])?, text(&value["label"])?);
        let kind = &value["kind"];
        let mut shown = Vec::new();
        let spec = if let Some(default) = kind["toggle"].as_bool() {
            SettingSpec::toggle(key, label, default)
        } else if let Some(options) = list(&kind["choice"]) {
            shown = list(&kind["shown"])?;
            SettingSpec::choice(key, label, options, kind["default"].as_u64()? as usize)
        } else if let Some([min, max, step]) = kind["range"].as_array().and_then(|r| {
            let n: Vec<f64> = r.iter().filter_map(Value::as_f64).collect();
            <[f64; 3]>::try_from(n).ok()
        }) {
            SettingSpec::range(key, label, min, max, step, kind["default"].as_f64()?)
        } else {
            let default = u32::try_from(kind["color"].as_u64()?).ok()?;
            SettingSpec::color(key, label, default)
        };
        // A key that names another file than its own is refused, as at registration.
        spec.check().ok()?;
        Some(Self {
            spec,
            label: text(&value["shown"])?,
            options: shown,
        })
    }
}

fn data_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(crate::mod_list::DIR).join(DATA_DIR)
}

/// A mod's id as a folder's name: anything that could lead elsewhere is no id.
fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
}

/// What the launcher knows of every mod's settings, by mod id.
pub fn read(game_dir: &Path) -> BTreeMap<String, Vec<ModSetting>> {
    let Some(Value::Object(mods)) = fs::read_to_string(data_dir(game_dir).join(FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
    else {
        return BTreeMap::new();
    };
    mods.into_iter()
        .filter(|(id, _)| is_id(id))
        .map(|(id, rows)| {
            let rows = rows.as_array().map_or(&[][..], Vec::as_slice);
            (id, rows.iter().filter_map(ModSetting::parse).collect())
        })
        .collect()
}

/// Keeps what the mods of this game registered. A mod that is off this time keeps what it
/// registered the last time it ran.
pub fn write(game_dir: &Path, now: BTreeMap<String, Vec<ModSetting>>) {
    let before = read(game_dir);
    let mut all = before.clone();
    all.extend(now);
    if all == before {
        return;
    }
    let text = Value::Object(
        all.into_iter()
            .map(|(id, rows)| (id, rows.iter().map(ModSetting::json).collect()))
            .collect(),
    )
    .to_string();
    let dir = data_dir(game_dir);
    let part = dir.join(format!("{FILE}.part"));
    let wrote = fs::create_dir_all(&dir)
        .and_then(|()| fs::write(&part, text))
        .and_then(|()| fs::rename(&part, dir.join(FILE)));
    if let Err(e) = wrote {
        log::warn!("Magnetite: the mods' settings are not listed for the launcher: {e}");
    }
}

/// The value the player keeps for `setting` of mod `id`, else its default.
pub fn value(game_dir: &Path, id: &str, setting: &ModSetting) -> SettingValue {
    let kept = is_id(id)
        .then(|| fs::read_to_string(data_dir(game_dir).join(id).join(&setting.spec.key)).ok())
        .flatten();
    setting.spec.read(kept.as_deref())
}

/// Keeps `value` for `setting` of mod `id`, as the game does; false when it is none of this
/// setting's values or could not be written.
pub fn store(game_dir: &Path, id: &str, setting: &ModSetting, value: SettingValue) -> bool {
    let Some(text) = setting.spec.store(value).filter(|_| is_id(id)) else {
        return false;
    };
    let dir = data_dir(game_dir).join(id);
    fs::create_dir_all(&dir)
        .and_then(|()| fs::write(dir.join(&setting.spec.key), text))
        .inspect_err(|e| log::warn!("Magnetite: {id}'s setting is not saved: {e}"))
        .is_ok()
}

/// The value one step from `value`: a toggle flips, a choice and a colour move on around, a range
/// moves a step and stops at its ends. `forward` is the plus side.
pub fn step(setting: &ModSetting, value: SettingValue, forward: bool) -> SettingValue {
    let turn = |at: usize, n: usize| {
        if forward {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }
    };
    match (&setting.spec.kind, value) {
        (SettingKind::Toggle { .. }, SettingValue::Toggle(on)) => SettingValue::Toggle(!on),
        (SettingKind::Choice { options, .. }, SettingValue::Choice(at)) if !options.is_empty() => {
            SettingValue::Choice(turn(at, options.len()))
        }
        (SettingKind::Range { min, max, step, .. }, SettingValue::Range(v)) => {
            // On the grid of steps from `min`, so that adding never drifts.
            let n = ((v - min) / step).round() + if forward { 1.0 } else { -1.0 };
            SettingValue::Range((min + n * step).clamp(*min, *max))
        }
        (SettingKind::Color { .. }, SettingValue::Color(rgb)) => {
            let colors = crate::mod_list::DYE_COLORS;
            let at = colors.iter().position(|c| *c == rgb);
            SettingValue::Color(colors[at.map_or(0, |at| turn(at, colors.len()))])
        }
        (_, value) => value,
    }
}

/// `value` as its row shows it; empty for a toggle, which shows a switch.
pub fn shown(setting: &ModSetting, value: SettingValue) -> String {
    match value {
        SettingValue::Choice(at) => setting.options.get(at).cloned().unwrap_or_default(),
        SettingValue::Range(v) => {
            let text = format!("{v:.3}");
            text.trim_end_matches('0').trim_end_matches('.').to_owned()
        }
        SettingValue::Color(rgb) => format!("#{rgb:06X}"),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "magnetite-mod-settings-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn settings() -> Vec<ModSetting> {
        [
            SettingSpec::toggle("on", "zoom.on", true),
            SettingSpec::choice("mode", "zoom.mode", ["zoom.hold", "zoom.press"], 1),
            SettingSpec::range("speed", "zoom.speed", 0.5, 2.0, 0.1, 1.0),
            SettingSpec::color("tint", "zoom.tint", 0xFF0000),
        ]
        .iter()
        .map(|spec| ModSetting::new(spec, |key| format!("<{key}>")))
        .collect()
    }

    #[test]
    fn what_the_game_writes_the_launcher_reads() {
        let home = scratch("round");
        write(&home, BTreeMap::from([("zoom".to_owned(), settings())]));
        assert_eq!(read(&home)["zoom"], settings());
        // A mod that is off now keeps its page.
        write(&home, BTreeMap::from([("other".to_owned(), Vec::new())]));
        assert_eq!(read(&home)["zoom"], settings());
        // A file edited to lead elsewhere names no mod and no setting.
        let bad = json!({"../zoom": [], "zoom": [{"key": "../on", "label": "l", "shown": "l", "kind": {"toggle": true}}]});
        fs::write(data_dir(&home).join(FILE), bad.to_string()).unwrap();
        assert_eq!(
            read(&home),
            BTreeMap::from([("zoom".to_owned(), Vec::new())])
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn a_value_steps_and_keeps_as_the_game_keeps_it() {
        let home = scratch("values");
        let all = settings();
        let next = |i: usize, forward| {
            let at = value(&home, "zoom", &all[i]);
            let to = step(&all[i], at, forward);
            assert!(store(&home, "zoom", &all[i], to));
            shown(&all[i], value(&home, "zoom", &all[i]))
        };
        next(0, true);
        assert_eq!(
            value(&home, "zoom", &all[0]),
            SettingValue::Toggle(false),
            "a toggle flips"
        );
        assert_eq!(
            next(1, true),
            "<zoom.hold>",
            "past the last option, the first"
        );
        assert_eq!(next(2, true), "1.1");
        for _ in 0..20 {
            next(2, true);
        }
        assert_eq!(next(2, true), "2", "a range stops at its end");
        assert_eq!(next(2, false), "1.9");
        assert!(next(3, true).starts_with('#'));
        assert!(!store(
            &home,
            "../zoom",
            &all[0],
            SettingValue::Toggle(true)
        ));
        fs::remove_dir_all(home).unwrap();
    }
}
