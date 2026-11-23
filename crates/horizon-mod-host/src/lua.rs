//! Lua mods: the selected content packages with a `[lua]` table become mod entries beside the
//! native and wasm ones, their scripts read from `data/<id>/scripts/`.
use horizon_mod_package::manifest::{Lua, Manifest};
use horizon_mod_package::selection::SelectedPackage;
use std::path::{Path, PathBuf};

/// A mod's scripts in its package, by their name under `data/<id>/scripts/`.
fn scripts(path: &Path, manifest: &Manifest) -> Result<horizon_mod_lua::Scripts, String> {
    let content = horizon_mod_package::Content::read(path, manifest)?;
    let dir = Lua::scripts_dir(&manifest.id);
    let mut reader = content.reader()?;
    let mut out = horizon_mod_lua::Scripts::new();
    for name in content.files.iter().filter(|name| name.ends_with(".lua")) {
        let Some(relative) = name.strip_prefix(&dir) else {
            continue;
        };
        let bytes = reader.read(name)?;
        let text = String::from_utf8(bytes).map_err(|e| format!("{name}: {e}"))?;
        out.insert(relative.to_owned(), text);
    }
    Ok(out)
}

/// The Lua mods among `packages`, each made and its entry script run, with their manifests. A
/// mod whose scripts don't load is left out with the error in the log: the game goes on.
pub(crate) fn load(
    packages: &[SelectedPackage],
    game_dir: &Path,
) -> Vec<(mod_api::ModEntry, Manifest)> {
    let settings = horizon_mod_lua::LuaSettings::read(game_dir);
    let mut out = Vec::new();
    for package in packages {
        let manifest = &package.package.manifest;
        let Some(lua) = &manifest.lua else {
            continue;
        };
        let path: PathBuf = package.path.clone();
        let read = {
            let (path, manifest) = (path.clone(), manifest.clone());
            Box::new(move || scripts(&path, &manifest)) as horizon_mod_lua::ScriptReader
        };
        let entry = scripts(&path, manifest).and_then(|found| {
            horizon_mod_lua::mod_entry(&manifest.id, &lua.entry, found, read, settings.clone())
        });
        match entry {
            Ok(entry) => {
                log::info!("Horizon: Lua mod {} loaded", manifest.id);
                out.push((entry, manifest.clone()));
            }
            Err(error) => log::error!("Horizon: Lua mod {} left out: {error}", manifest.id),
        }
    }
    out
}
