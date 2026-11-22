//! The phone launcher's profiles (docs/unification/launcher.md, "Профили"): each its own settings
//! and touch layout, the game folder's worlds, accounts and files shared. The main profile is the
//! game folder itself, as before there were profiles; another lives in `profiles/<id>/` and the
//! game plays it with `--player-dir` (`logging::set_player_dir`).
//!
//! `profiles/index.json` keeps the one that plays and the main profile's own name, if the player
//! gave it one; each other profile keeps its name in its folder's `profile.json`.
use serde_json::{Value, json};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The profiles' folder, in the game folder.
pub const DIR: &str = "profiles";
const INDEX: &str = "index.json";
const NAME_FILE: &str = "profile.json";
/// The settings a new profile starts from: the current profile's own, so it plays the same until
/// the player changes it. Logs, screenshots and the server list stay behind.
const SETTINGS: &[&str] = &[
    "options.txt",
    "horizon-graphics.txt",
    "horizon-extras.txt",
    "horizon-controller.txt",
    "touch-controls.json",
];
/// The longest name kept, in characters.
pub const NAME_MAX: usize = 32;

/// One profile: its folder's name (empty for the main profile) and the name the player gave it
/// (`None`: the main profile's own, in the player's language).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
}

impl Profile {
    pub fn is_main(&self) -> bool {
        self.id.is_empty()
    }
}

/// The game folder's profiles, the main one first, and the one that plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profiles {
    game_dir: PathBuf,
    pub list: Vec<Profile>,
    pub current: usize,
}

impl Profiles {
    /// The profiles in `game_dir`: the main one, then every folder of `profiles/` with a
    /// `profile.json`, by id.
    pub fn load(game_dir: &Path) -> Self {
        let root = game_dir.join(DIR);
        let index: Value = fs::read_to_string(root.join(INDEX))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Value::Null);
        let mut list = vec![Profile {
            id: String::new(),
            name: index.get("main").and_then(Value::as_str).map(str::to_owned),
        }];
        let mut others: Vec<Profile> = fs::read_dir(&root)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let id = e.file_name().to_str()?.to_owned();
                let text = fs::read_to_string(e.path().join(NAME_FILE)).ok()?;
                let name = serde_json::from_str::<Value>(&text)
                    .ok()?
                    .get("name")?
                    .as_str()?
                    .to_owned();
                Some(Profile {
                    id,
                    name: Some(name),
                })
            })
            .collect();
        others.sort_by_key(|p| (p.id.parse::<u64>().unwrap_or(u64::MAX), p.id.clone()));
        list.extend(others);
        let wanted = index.get("current").and_then(Value::as_str).unwrap_or("");
        let current = list.iter().position(|p| p.id == wanted).unwrap_or(0);
        Self {
            game_dir: game_dir.to_path_buf(),
            list,
            current,
        }
    }

    /// The profile that plays.
    pub fn current(&self) -> &Profile {
        &self.list[self.current]
    }

    /// The folder of the profile at `i`: the game folder for the main one.
    pub fn dir(&self, i: usize) -> PathBuf {
        match self.list.get(i) {
            Some(p) if !p.is_main() => self.game_dir.join(DIR).join(&p.id),
            _ => self.game_dir.clone(),
        }
    }

    /// The folder of the one that plays, and `None` for the main one (no `--player-dir`).
    pub fn player_dir(&self) -> Option<PathBuf> {
        (!self.current().is_main()).then(|| self.dir(self.current))
    }

    /// Makes the profile at `i` the one that plays.
    pub fn select(&mut self, i: usize) -> io::Result<()> {
        if i < self.list.len() {
            self.current = i;
            self.save_index()?;
        }
        Ok(())
    }

    /// A new profile named `name`, with the current one's settings, made the one that plays.
    pub fn create(&mut self, name: &str) -> io::Result<usize> {
        let id = (1..)
            .map(|n: u64| n.to_string())
            .find(|id| !self.game_dir.join(DIR).join(id).exists())
            .expect("a free id");
        let dir = self.game_dir.join(DIR).join(&id);
        fs::create_dir_all(&dir)?;
        let from = self.dir(self.current);
        for file in SETTINGS {
            let source = from.join(file);
            if source.is_file() {
                fs::copy(&source, dir.join(file))?;
            }
        }
        let name = clean(name);
        write_name(&dir, &name)?;
        self.list.push(Profile {
            id,
            name: Some(name),
        });
        let i = self.list.len() - 1;
        self.select(i)?;
        Ok(i)
    }

    /// Names the profile at `i` (the main one too).
    pub fn rename(&mut self, i: usize, name: &str) -> io::Result<()> {
        let name = clean(name);
        let Some(profile) = self.list.get_mut(i) else {
            return Ok(());
        };
        profile.name = Some(name.clone());
        if profile.is_main() {
            self.save_index()
        } else {
            write_name(&self.dir(i), &name)
        }
    }

    /// Deletes the profile at `i` and its folder (its settings, logs and screenshots); the main
    /// one stays. The main one plays if this one did.
    pub fn remove(&mut self, i: usize) -> io::Result<()> {
        if i == 0 || i >= self.list.len() {
            return Ok(());
        }
        fs::remove_dir_all(self.dir(i))?;
        self.list.remove(i);
        self.current = match self.current {
            c if c == i => 0,
            c if c > i => c - 1,
            c => c,
        };
        self.save_index()
    }

    fn save_index(&self) -> io::Result<()> {
        let root = self.game_dir.join(DIR);
        fs::create_dir_all(&root)?;
        let text = json!({
            "current": self.current().id,
            "main": self.list[0].name,
        })
        .to_string();
        fs::write(root.join(INDEX), text)
    }
}

/// A name as kept: trimmed, at most [`NAME_MAX`] characters.
fn clean(name: &str) -> String {
    name.trim().chars().take(NAME_MAX).collect()
}

fn write_name(dir: &Path, name: &str) -> io::Result<()> {
    fs::write(dir.join(NAME_FILE), json!({ "name": name }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_dir(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("magnetite-profiles-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_game_folder_without_profiles_plays_the_main_one_in_itself() {
        let game = game_dir("main");
        let profiles = Profiles::load(&game);
        assert_eq!(profiles.list.len(), 1);
        assert!(profiles.current().is_main());
        assert_eq!(profiles.dir(0), game);
        assert_eq!(profiles.player_dir(), None);
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn a_new_profile_starts_from_the_current_settings_and_plays() {
        let game = game_dir("create");
        fs::write(game.join("options.txt"), "lang:ru_ru\n").unwrap();
        fs::write(game.join("servers.dat"), "x").unwrap();
        let mut profiles = Profiles::load(&game);
        let i = profiles.create("  PvP  ").unwrap();
        let dir = profiles.dir(i);
        assert_eq!(dir, game.join("profiles").join("1"));
        assert_eq!(
            fs::read_to_string(dir.join("options.txt")).unwrap(),
            "lang:ru_ru\n"
        );
        assert!(!dir.join("servers.dat").exists());
        let back = Profiles::load(&game);
        assert_eq!(back.current().name.as_deref(), Some("PvP"));
        assert_eq!(back.player_dir(), Some(dir));
        let _ = fs::remove_dir_all(&game);
    }

    #[test]
    fn renaming_and_removing_are_kept() {
        let game = game_dir("edit");
        let mut profiles = Profiles::load(&game);
        profiles.create("A").unwrap();
        profiles.create("B").unwrap();
        profiles.rename(0, "Mine").unwrap();
        profiles.rename(1, "A2").unwrap();
        profiles.remove(2).unwrap();
        assert!(!game.join("profiles").join("2").exists());
        let back = Profiles::load(&game);
        assert_eq!(back.list.len(), 2);
        assert_eq!(back.list[0].name.as_deref(), Some("Mine"));
        assert_eq!(back.list[1].name.as_deref(), Some("A2"));
        assert!(back.current().is_main());
        let _ = fs::remove_dir_all(&game);
    }
}
