//! What the game keeps on the disk, for the launcher's Data page: how much, and clearing what it
//! makes again by itself. Worlds, settings, mods, packs and accounts are never touched.
use crate::launcher_screen::Store;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where the game keeps things.
#[derive(Clone, Debug)]
pub struct Places {
    pub game_dir: PathBuf,
    pub resources: PathBuf,
    /// The game files' download folder.
    pub work: PathBuf,
}

/// How many bytes each takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    pub logs: u64,
    pub cache: u64,
    pub downloads: u64,
    pub files: u64,
    pub worlds: u64,
    pub mods: u64,
}

impl Places {
    /// The files and folders [`Store`] holds, each of which clearing removes.
    fn of(&self, store: Store) -> Vec<PathBuf> {
        let inside = |dir: &Path| -> Vec<PathBuf> {
            fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .collect()
        };
        match store {
            Store::Logs => inside(&self.game_dir.join("logs"))
                .into_iter()
                // The one being written now.
                .filter(|p| p.file_name().is_none_or(|n| n != "latest.log"))
                .chain(inside(&self.game_dir.join("crash-reports")))
                .collect(),
            Store::Cache => [".horizon-cache", "cache", "mod-browser/downloads"]
                .iter()
                .flat_map(|dir| inside(&self.game_dir.join(dir)))
                .collect(),
            Store::Downloads => inside(&self.work),
        }
    }

    /// What everything takes now. It reads every folder: for a thread.
    pub fn sizes(&self) -> Sizes {
        let of = |store| self.of(store).iter().map(|p| size(p)).sum();
        Sizes {
            logs: of(Store::Logs),
            cache: of(Store::Cache),
            downloads: of(Store::Downloads),
            files: size(&self.resources),
            worlds: size(&self.game_dir.join("saves")),
            mods: size(&self.game_dir.join(crate::mod_list::DIR)),
        }
    }

    /// Clears `store`.
    pub fn clear(&self, store: Store) -> io::Result<()> {
        for path in self.of(store) {
            let kind = fs::symlink_metadata(&path)?;
            if kind.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

/// The bytes of a file or of everything in a folder; links are not followed.
fn size(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| size(&e.path()))
            .sum(),
        Ok(m) if m.is_file() => m.len(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearing_takes_what_comes_back_and_leaves_the_players_own() {
        let root = std::env::temp_dir().join(format!("magnetite-data-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let places = Places {
            game_dir: root.join("game"),
            resources: root.join("game/resources"),
            work: root.join("work"),
        };
        let put = |path: &str, bytes: usize| {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, vec![0; bytes]).unwrap();
        };
        put("game/logs/latest.log", 1);
        put("game/logs/2026-11-01-1.log.gz", 10);
        put("game/crash-reports/crash.txt", 5);
        put("game/.horizon-cache/wasm/a.cwasm", 100);
        put("game/cache/dx12-shaders.bin", 20);
        put("game/saves/World/level.dat", 40);
        put("game/horizon-mods/zoom.hmod", 7);
        put("game/options.txt", 3);
        put("game/resources/assets/a", 1000);
        put("work/staging/part", 60);
        assert_eq!(
            places.sizes(),
            Sizes {
                logs: 15,
                cache: 120,
                downloads: 60,
                files: 1000,
                worlds: 40,
                mods: 7,
            }
        );
        for store in [Store::Logs, Store::Cache, Store::Downloads] {
            places.clear(store).unwrap();
        }
        assert_eq!(
            places.sizes(),
            Sizes {
                files: 1000,
                worlds: 40,
                mods: 7,
                ..Sizes::default()
            }
        );
        for kept in [
            "game/logs/latest.log",
            "game/saves/World/level.dat",
            "game/options.txt",
            "game/horizon-mods/zoom.hmod",
        ] {
            assert!(root.join(kept).exists(), "{kept}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}
