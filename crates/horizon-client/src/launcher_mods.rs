//! The phone launcher's Mods page (docs/unification/launcher.md): the mods installed in the game
//! folder's `horizon-mods/`, read without running anything, as its rows show them. Turning one on
//! or off is the game's own list (`ModList`, `horizon-mods.txt`); installing one is
//! `mod_registry::install_any` from Magnetite's mod platform, or `mod_registry::import` from a file.
use std::path::Path;

/// One installed mod as the launcher lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModRow {
    pub id: String,
    pub name: String,
    pub version: String,
    /// What the mod is made of.
    pub code: Code,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub homepage: Option<String>,
}

/// What a mod the launcher installs is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    /// Content alone.
    Content,
    /// WebAssembly (`[wasm]`).
    Wasm,
    /// Lua scripts (`[lua]`).
    Lua,
}

/// The content, WebAssembly and Lua mods in `game_dir`'s `horizon-mods/`, by name. Native mods are
/// left out: a phone runs only those built into it, which the game's own Mods page lists.
pub fn installed(game_dir: &Path) -> Vec<ModRow> {
    let dir = game_dir.join(crate::mod_list::DIR);
    let mut rows: Vec<ModRow> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() || path.extension().is_some_and(|e| e == "hmod"))
        .filter_map(|path| match horizon_mod_package::Package::read(&path) {
            Ok(package) if !package.is_native() => {
                let m = package.manifest;
                Some(ModRow {
                    code: if m.wasm.is_some() {
                        Code::Wasm
                    } else if m.lua.is_some() {
                        Code::Lua
                    } else {
                        Code::Content
                    },
                    id: m.id,
                    name: m.name,
                    version: m.version,
                    description: m.description,
                    authors: m.authors,
                    license: m.license,
                    homepage: m.homepage,
                })
            }
            Ok(_) => None,
            Err(e) => {
                log::warn!(
                    "Horizon: {} is not a mod the launcher reads: {e}",
                    path.display()
                );
                None
            }
        })
        .collect();
    rows.sort_by_key(|row| row.name.to_lowercase());
    rows.dedup_by(|a, b| a.id == b.id);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_folder_without_mods_lists_none() {
        let dir = std::env::temp_dir().join(format!("magnetite-mods-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(crate::mod_list::DIR)).unwrap();
        std::fs::write(dir.join(crate::mod_list::DIR).join("notes.txt"), "x").unwrap();
        assert!(installed(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
