//! The game files on the device: the prepared folder, assembled without a computer
//! (`docs/unification/mobile-boot.md` §2).
//!
//! Horizon's preparation runs Mojang's own code on Java for a few tables (registries, recipes,
//! world generation), which a phone cannot do. Those tables are published once, in the separate
//! `magnetite-tables` repository — no file of Mojang's in them — and everything else is the
//! player's own Minecraft from Mojang's servers: the client jar's `assets/` and `data/`, and the
//! asset index's objects. [`install`] assembles exactly the folder a computer's preparation
//! makes, so the game cannot tell the difference.
//!
//! The download belongs to the player: it starts after a sign-in (Microsoft or Ely.by, the
//! owner's rule, `docs/unification/plan.md`) and never ships with this repository.
//!
//! # The tables' `setup.json`
//!
//! The zip the build names holds the tables under `tables_root`, and beside them `setup.json`:
//!
//! ```json
//! {
//!   "minecraft": "26.3",
//!   "client": {
//!     "url": "https://piston-data.mojang.com/v1/objects/…/client.jar",
//!     "sha1": "<40 hex digits>",
//!     "size": 27123456
//!   },
//!   "asset_index": {
//!     "id": "26",
//!     "url": "https://piston-data.mojang.com/v1/objects/…/26.json",
//!     "sha1": "<40 hex digits>"
//!   },
//!   "jar_files": ["assets/", "data/", "pack.png"],
//!   "tables_root": "tables"
//! }
//! ```
//!
//! - `minecraft` is the version the tables were made for; another build's tables are refused
//!   rather than mixed into this build's folder.
//! - `client` and `asset_index` are the files Mojang's own launcher metadata names for that
//!   version, pinned here so no version manifest is fetched at run time.
//! - `jar_files`: a name ending in `/` is every member of the jar under that folder; any other
//!   name is one exact member.
//! - `tables_root` is the folder inside the zip that holds the ready non-Mojang files; `""`
//!   means the zip's root (`setup.json` itself aside). It defaults to `tables`.
//! - `client.size` is the jar's size for the progress bar; it is informational.
//!
//! # What the install does
//!
//! 1. `tables.zip`, checked against the build's sha256, unpacked into the staging folder;
//! 2. the client jar, checked against its sha1, with only the `jar_files` members extracted;
//! 3. the asset index, checked against its sha1, saved as `asset-index.json`;
//! 4. the index's objects, from `resources.download.minecraft.net`, eight at a time, each
//!    checked against its sha1 and written to the name the index gives it (`assets/…`);
//! 5. the staging folder must hold `horizon-resources.json` — the marker the game checks — and
//!    only then does it take the place of `resources` (a rename, or a copy across drives), the
//!    previous folder kept until the new one is in place and restored if it is not.
//!
//! Downloads are streamed to disk rather than held whole in memory, and resume by content hash:
//! a file already in place with the right hash is kept. That is what makes an interrupted
//! download continue on the next attempt (a Retry, or the next run). Every address is https and
//! named by an allowlist: Mojang's own endpoints, plus — for the tables alone — the host the
//! build's `MAGNETITE_TABLES_URL` names.

use sha1::Digest as _;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// The marker a finished preparation leaves, and the game refuses to start without.
const MARKER: &str = "horizon-resources.json";
/// The manifest's name inside the tables' zip.
const MANIFEST: &str = "setup.json";
/// Mojang's own endpoints, the only addresses the manifest may name (`docs/layers.md`).
const MOJANG_HOSTS: &[&str] = &[
    "piston-data.mojang.com",
    "piston-meta.mojang.com",
    "resources.download.minecraft.net",
];
/// Where the asset index's objects live (`OBJECTS`'s host, also one of `MOJANG_HOSTS`).
const OBJECTS: &str = "https://resources.download.minecraft.net";
/// Downloads at a time (the iOS fork's count: enough to fill a Wi-Fi line).
const LANES: usize = 8;
/// Attempts per file, with a pause that doubles between them.
const TRIES: usize = 4;
/// The largest file taken out of the tables' zip, and out of the client jar.
const MAX_TABLE_FILE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_JAR_FILE: u64 = 512 * 1024 * 1024;
/// A full disk's message, whose start says a retry cannot help.
const NO_SPACE: &str = "No space left on the device";

/// How far a download is, shared with the thread that draws it. `done` and `total` are byte
/// counts (0 total until the first size is known); `step` is the translation key of what is
/// downloading now — short text behind a short lock rather than an atomic.
pub struct InstallProgress {
    pub done: AtomicU64,
    pub total: AtomicU64,
    step: Mutex<String>,
}

impl InstallProgress {
    pub fn new() -> Self {
        Self {
            done: AtomicU64::new(0),
            total: AtomicU64::new(0),
            step: Mutex::new(String::new()),
        }
    }

    /// What the download is doing now, as a translation key (`horizon.boot.files.step.*`) that
    /// the screen draws over the bar.
    pub fn set_step(&self, key: &str) {
        *self.text() = key.to_owned();
    }

    /// `(done_bytes, total_bytes, step)` for one frame.
    pub fn snapshot(&self) -> (u64, u64, String) {
        (
            self.done.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
            self.text().clone(),
        )
    }

    /// A new attempt: the bar starts again from nothing.
    pub fn reset(&self) {
        self.done.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.text().clear();
    }

    fn text(&self) -> std::sync::MutexGuard<'_, String> {
        self.step.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn add_done(&self, bytes: u64) {
        self.done.fetch_add(bytes, Ordering::Relaxed);
    }

    fn sub_done(&self, bytes: u64) {
        // A compare-exchange loop, not fetch_update: that is deprecated on newer toolchains under
        // a name older ones (the workspace's rust-version) don't have.
        let mut done = self.done.load(Ordering::Relaxed);
        while let Err(now) = self.done.compare_exchange_weak(
            done,
            done.saturating_sub(bytes),
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            done = now;
        }
    }

    fn add_total(&self, bytes: u64) {
        self.total.fetch_add(bytes, Ordering::Relaxed);
    }
}

impl Default for InstallProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// What to install where, and which tables to take it from.
#[derive(Clone, Debug)]
pub struct InstallSpec {
    /// The prepared folder's final place (`--resources`).
    pub resources: PathBuf,
    /// The downloads' own folder: the staging folder inside it, the caches beside it. Kept
    /// between runs so an interrupted download continues; removed once an install succeeds.
    pub work: PathBuf,
    /// The tables' zip: https, and the address it names is the one fetched.
    pub tables_url: String,
    /// The tables' zip's sha256, hex (64 digits).
    pub tables_sha256: String,
}

/// The build's own tables, when it names them: `MAGNETITE_TABLES_URL` and
/// `MAGNETITE_TABLES_SHA256` at run time, else the same names compiled in (the release build
/// sets them for its platform). `None` means this build cannot download the game files, and
/// the boot screen says so instead of offering a download that cannot happen.
pub fn spec_from_build(resources: PathBuf, work: PathBuf) -> Option<InstallSpec> {
    let url = setting("MAGNETITE_TABLES_URL", option_env!("MAGNETITE_TABLES_URL"));
    let sha256 = setting(
        "MAGNETITE_TABLES_SHA256",
        option_env!("MAGNETITE_TABLES_SHA256"),
    );
    spec_from_values(url, sha256, resources, work)
}

/// [`spec_from_build`]'s check, over the two values themselves: a tables address must be https
/// and its sha256 64 hex digits, or the build has no usable installer.
fn spec_from_values(
    url: Option<String>,
    sha256: Option<String>,
    resources: PathBuf,
    work: PathBuf,
) -> Option<InstallSpec> {
    let tables_url = url?;
    let tables_sha256 = sha256?;
    if https_host(&tables_url).is_none() || !is_hex(&tables_sha256, 64) {
        return None;
    }
    Some(InstallSpec {
        resources,
        work,
        tables_url,
        tables_sha256,
    })
}

/// A build setting: the run-time variable, else the compiled-in value, trimmed; empty is none.
fn setting(name: &str, compiled: Option<&'static str>) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| compiled.map(str::to_owned))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// The game files into `spec.resources`, reporting to `progress`. Safe to call again after a
/// failure: finished files are kept and only the rest is fetched. Blocking, and meant for a
/// thread of its own.
pub fn install(spec: &InstallSpec, progress: &InstallProgress) -> Result<(), String> {
    let tables_host = https_host(&spec.tables_url)
        .ok_or_else(|| format!("the tables' address is not https: {}", spec.tables_url))?
        .to_owned();
    if !is_hex(&spec.tables_sha256, 64) {
        return Err("the tables' sha256 is not 64 hex digits".to_owned());
    }
    let http = Http::new(Some(tables_host));
    install_with(spec, progress, &http)
}

/// [`install`] over any transport: the tests hand it their own bytes.
fn install_with(
    spec: &InstallSpec,
    progress: &InstallProgress,
    fetch: &dyn Fetch,
) -> Result<(), String> {
    let work = &spec.work;
    fs::create_dir_all(work).map_err(|e| disk(work, e))?;
    let staging = work.join("staging");

    progress.set_step("horizon.boot.files.step.tables");
    let tables = work.join("tables.zip");
    download(
        fetch,
        &Wanted {
            url: &spec.tables_url,
            sha: &spec.tables_sha256,
            size: None,
        },
        &tables,
        progress,
    )?;
    let manifest = read_manifest(&tables)?;
    unpack_tables(&tables, &manifest, &staging)?;

    progress.set_step("horizon.boot.files.step.client");
    let jar = work.join("client.jar");
    download(
        fetch,
        &Wanted {
            url: &manifest.client.url,
            sha: &manifest.client.sha1,
            size: Some(manifest.client.size).filter(|size| *size > 0),
        },
        &jar,
        progress,
    )?;
    progress.set_step("horizon.boot.files.step.unpacking");
    extract_jar(&jar, &manifest.jar_files, &staging)?;

    progress.set_step("horizon.boot.files.step.index");
    let index = staging.join("asset-index.json");
    download(
        fetch,
        &Wanted {
            url: &manifest.asset_index.url,
            sha: &manifest.asset_index.sha1,
            size: None,
        },
        &index,
        progress,
    )?;
    let objects = read_objects(&index)?;

    progress.set_step("horizon.boot.files.step.assets");
    download_objects(fetch, &objects, &staging, progress)?;

    if !staging.join(MARKER).is_file() {
        return Err(format!("the tables have no {MARKER}"));
    }
    progress.set_step("horizon.boot.files.step.finishing");
    activate(&staging, &spec.resources)?;
    // The caches only help an unfinished install; this one is done.
    let _ = fs::remove_dir_all(work);
    Ok(())
}

/// A download's own facts: where, its hash, and the size the manifest declares for it (0 when
/// the caller has already counted that size in the bar).
struct Wanted<'a> {
    url: &'a str,
    sha: &'a str,
    size: Option<u64>,
}

/// `want` into `to`, streaming and checked. A file already there with the right hash is kept
/// and counts as done; anything else is fetched `TRIES` times, written beside `to` and renamed
/// into place only once its hash matches.
fn download(
    fetch: &dyn Fetch,
    want: &Wanted<'_>,
    to: &Path,
    progress: &InstallProgress,
) -> Result<(), String> {
    let hex_len = want.sha.len();
    if !(hex_len == 40 || hex_len == 64) || !is_hex(want.sha, hex_len) {
        return Err(format!("{}: its hash is not a sha1 or sha256", want.url));
    }
    if let Some(bytes) = file_hash(to, want.sha)? {
        progress.add_done(bytes);
        progress.add_total(want.size.unwrap_or(bytes));
        return Ok(());
    }
    let part = part_path(to);
    let mut last = format!("{}: nothing downloaded", want.url);
    // The size this file adds to the bar, once: a retry does not add it again.
    let mut in_total = false;
    for attempt in 0..TRIES {
        if attempt > 0 {
            pause(attempt as u32);
        }
        let counted = AtomicU64::new(0);
        let result = (|| -> Result<(), String> {
            let fetched = fetch.get(want.url)?;
            if let Some(size) = want.size.or(fetched.length)
                && !in_total
            {
                progress.add_total(size);
                in_total = true;
            }
            make_parent(to)?;
            let mut out = File::create(&part).map_err(|e| disk(&part, e))?;
            let mut digest = Hash::new(hex_len);
            let mut body = fetched.body;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let n = body
                    .read(&mut buffer)
                    .map_err(|e| read_error(want.url, e))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buffer[..n]).map_err(|e| disk(&part, e))?;
                digest.update(&buffer[..n]);
                counted.fetch_add(n as u64, Ordering::Relaxed);
                progress.add_done(n as u64);
            }
            out.sync_all().map_err(|e| disk(&part, e))?;
            drop(out);
            if !digest.finish().eq_ignore_ascii_case(want.sha) {
                return Err(format!(
                    "{}: the download does not match its hash",
                    want.url
                ));
            }
            fs::rename(&part, to).map_err(|e| at(to, e))?;
            Ok(())
        })();
        match result {
            Ok(()) => return Ok(()),
            Err(error) => {
                progress.sub_done(counted.load(Ordering::Relaxed));
                let _ = fs::remove_file(&part);
                if error.starts_with(NO_SPACE) {
                    return Err(error);
                }
                last = error;
            }
        }
    }
    Err(last)
}

/// The wait between two attempts: a doubling pause for a flaky network. A run under `cargo
/// test` does not wait, so the retry's count is what the tests check, not the wall clock.
fn pause(attempt: u32) {
    if !cfg!(test) {
        std::thread::sleep(Duration::from_secs(1 << attempt));
    }
}

/// The lanes: every object of the index, each written where the index's name says
/// (`assets/…`), resumed by hash like every other file.
fn download_objects(
    fetch: &dyn Fetch,
    objects: &[Object],
    staging: &Path,
    progress: &InstallProgress,
) -> Result<(), String> {
    // The bar counts the whole set before the first lane starts, so eight lanes tell one story.
    for object in objects {
        progress.add_total(object.size);
    }
    let next = AtomicUsize::new(0);
    let failed: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..LANES {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(object) = objects.get(i) else {
                        return;
                    };
                    if failed
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_some()
                    {
                        return;
                    }
                    let result = (|| -> Result<(), String> {
                        let path = inside(&staging.join("assets"), &object.name)?;
                        let url = format!("{OBJECTS}/{}/{}", &object.hash[..2], object.hash);
                        download(
                            fetch,
                            &Wanted {
                                url: &url,
                                sha: &object.hash,
                                // The whole set is already in the bar.
                                size: Some(0),
                            },
                            &path,
                            progress,
                        )
                    })();
                    if let Err(error) = result {
                        *failed.lock().unwrap_or_else(PoisonError::into_inner) = Some(error);
                        return;
                    }
                }
            });
        }
    });
    match failed.into_inner().unwrap_or_else(PoisonError::into_inner) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The tables' manifest, as `setup.json` states it.
#[derive(Debug)]
struct Manifest {
    client: Blob,
    asset_index: Index,
    /// Exact jar member names; one ending in `/` is a folder.
    jar_files: Vec<String>,
    /// The folder inside the zip (empty: its root).
    tables_root: String,
}

#[derive(Debug)]
struct Blob {
    url: String,
    sha1: String,
    /// The jar's size, for the bar; 0 when the manifest does not say.
    size: u64,
}

#[derive(Debug)]
struct Index {
    url: String,
    sha1: String,
}

impl Manifest {
    fn parse(text: &str) -> Result<Self, String> {
        let json: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        let wanted = horizon_core::VANILLA_VERSION;
        let minecraft = text_of(&json, "minecraft")?;
        if minecraft != wanted {
            return Err(format!(
                "{MANIFEST}: the tables are for Minecraft {minecraft}, this build is {wanted}"
            ));
        }
        let client = &json["client"];
        let client = Blob {
            url: mojang_url(client, "client")?,
            sha1: sha1_of(client, "client")?,
            size: client["size"].as_u64().unwrap_or(0),
        };
        let index = &json["asset_index"];
        // `id` names the index Mojang publishes; the file itself comes from `url`.
        let _ = text_of(index, "id")?;
        let asset_index = Index {
            url: mojang_url(index, "asset_index")?,
            sha1: sha1_of(index, "asset_index")?,
        };
        let jar_files: Vec<String> = json["jar_files"]
            .as_array()
            .map(|names| {
                names
                    .iter()
                    .filter_map(|name| name.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        if jar_files.iter().all(String::is_empty) {
            return Err(format!("{MANIFEST}: no jar_files"));
        }
        let tables_root = json["tables_root"]
            .as_str()
            .unwrap_or("tables")
            .trim_matches('/')
            .to_owned();
        Ok(Self {
            client,
            asset_index,
            jar_files,
            tables_root,
        })
    }
}

fn text_of<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{MANIFEST}: no {key}"))
}

fn sha1_of(value: &serde_json::Value, what: &str) -> Result<String, String> {
    let sha1 = text_of(value, "sha1")?;
    if !is_hex(sha1, 40) {
        return Err(format!("{MANIFEST}: {what} has a bad sha1"));
    }
    Ok(sha1.to_owned())
}

/// A manifest address: https, on one of Mojang's own endpoints.
fn mojang_url(value: &serde_json::Value, what: &str) -> Result<String, String> {
    let url = text_of(value, "url")?;
    if https_host(url).is_none() || !MOJANG_HOSTS.iter().any(|host| host_matches(url, host)) {
        return Err(format!(
            "{MANIFEST}: {what}'s address is not Mojang's: {url}"
        ));
    }
    Ok(url.to_owned())
}

fn read_manifest(tables: &Path) -> Result<Manifest, String> {
    let file = File::open(tables).map_err(|e| at(tables, e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| at(tables, e))?;
    let mut text = String::new();
    zip.by_name(MANIFEST)
        .map_err(|e| format!("{}: no {MANIFEST}: {e}", tables.display()))?
        .read_to_string(&mut text)
        .map_err(|e| format!("{}: {MANIFEST}: {e}", tables.display()))?;
    Manifest::parse(&text)
}

/// The tables' own files, from `tables_root` in the zip into the staging folder.
fn unpack_tables(tables: &Path, manifest: &Manifest, staging: &Path) -> Result<(), String> {
    let file = File::open(tables).map_err(|e| at(tables, e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| at(tables, e))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| at(tables, e))?;
        let name = entry.name().map_err(|e| at(tables, e))?.into_owned();
        if entry.is_dir() {
            continue;
        }
        let Some(path) = staged_path(staging, &name, &manifest.tables_root)? else {
            continue;
        };
        if entry.is_symlink() {
            return Err(format!("the tables hold a link, not a file: {name}"));
        }
        if entry.size() > MAX_TABLE_FILE {
            return Err(format!("{name}: too large for the tables"));
        }
        let mut out = create(&path)?;
        io::copy(&mut entry, &mut out).map_err(|e| disk(&path, e))?;
    }
    Ok(())
}

/// Where a zip entry goes, or `None` when it is not the tables' (the manifest, another folder,
/// a directory). A name that would leave the folder is an error: a zip-slip.
fn staged_path(staging: &Path, name: &str, root: &str) -> Result<Option<PathBuf>, String> {
    let relative = if root.is_empty() {
        if name == MANIFEST {
            return Ok(None);
        }
        name
    } else {
        let prefix = format!("{root}/");
        match name.strip_prefix(&prefix) {
            Some(relative) => relative,
            None => return Ok(None),
        }
    };
    if relative.is_empty() || relative.ends_with('/') {
        return Ok(None);
    }
    Ok(Some(inside(staging, relative)?))
}

/// The jar's wanted members into the staging folder, as a computer's preparation copies them.
fn extract_jar(jar: &Path, wanted: &[String], staging: &Path) -> Result<(), String> {
    let file = File::open(jar).map_err(|e| at(jar, e))?;
    let mut zip = zip::ZipArchive::new(io::BufReader::new(file)).map_err(|e| at(jar, e))?;
    let mut names = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| at(jar, e))?;
        names.push(entry.name().map_err(|e| at(jar, e))?.into_owned());
    }
    for want in wanted {
        let members: Vec<&String> = if want.ends_with('/') {
            names
                .iter()
                .filter(|name| name.starts_with(want.as_str()))
                .collect()
        } else {
            names
                .iter()
                .filter(|name| name.as_str() == want.as_str())
                .collect()
        };
        if members.is_empty() {
            return Err(format!("the client jar has no {want}"));
        }
        for member in members {
            let mut entry = zip
                .by_name(member)
                .map_err(|e| format!("the client jar has no {member}: {e}"))?;
            // A jar lists its folders as members too (`data/`): nothing to write for one.
            if entry.is_dir() || member.ends_with('/') {
                continue;
            }
            if entry.is_symlink() {
                return Err(format!("the client jar holds a link: {member}"));
            }
            if entry.size() > MAX_JAR_FILE {
                return Err(format!("{member}: too large for the client jar"));
            }
            let path = inside(staging, member)?;
            let mut out = create(&path)?;
            io::copy(&mut entry, &mut out).map_err(|e| disk(&path, e))?;
        }
    }
    Ok(())
}

/// One object of the asset index: the name the prepared folder gives it, and its content hash.
struct Object {
    name: String,
    hash: String,
    size: u64,
}

/// The index's objects, in its own order.
fn read_objects(index: &Path) -> Result<Vec<Object>, String> {
    let bytes = fs::read(index).map_err(|e| at(index, e))?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("asset-index.json: {e}"))?;
    let Some(objects) = json.get("objects").and_then(|value| value.as_object()) else {
        return Err("asset-index.json has no objects".to_owned());
    };
    let mut out = Vec::with_capacity(objects.len());
    for (name, info) in objects {
        let hash = info
            .get("hash")
            .and_then(|value| value.as_str())
            .ok_or_else(|| format!("asset-index.json: {name} has no hash"))?;
        if !is_hex(hash, 40) {
            return Err(format!("asset-index.json: {name} has a bad hash"));
        }
        out.push(Object {
            name: name.clone(),
            hash: hash.to_owned(),
            size: info
                .get("size")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
        });
    }
    Ok(out)
}

/// The finished staging folder takes the place of `resources`: the old folder waits beside it
/// as `resources.previous` and is put back if the new one cannot take its place. A rename where
/// both are on one drive; Android keeps the downloads in the app's storage and the game folder
/// on shared storage, which a rename cannot cross, so a copy stands in for it.
fn activate(staging: &Path, resources: &Path) -> Result<(), String> {
    if let Some(parent) = resources.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| disk(parent, e))?;
    }
    let previous = sibling(resources, ".previous");
    if resources.exists() {
        if previous.exists() {
            fs::remove_dir_all(&previous).map_err(|e| at(&previous, e))?;
        }
        fs::rename(resources, &previous).map_err(|e| at(resources, e))?;
    }
    match move_dir(staging, resources) {
        Ok(()) => {
            if previous.exists() {
                let _ = fs::remove_dir_all(&previous);
            }
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_dir_all(resources);
            if previous.exists() {
                let _ = fs::rename(&previous, resources);
            }
            Err(at(resources, error))
        }
    }
}

/// `from` moved to `to`, copied and removed when the two are on different drives.
fn move_dir(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if crosses_drives(&error) => {
            if let Err(error) = copy_dir(from, to) {
                let _ = fs::remove_dir_all(to);
                return Err(error);
            }
            let _ = fs::remove_dir_all(from);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn crosses_drives(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::CrossesDevices
        || matches!(error.raw_os_error(), Some(18) | Some(17))
}

/// `from` copied into `to`, folders and all.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// `path` beside `suffix`'s own name (`resources` → `resources.previous`).
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// `to`'s own name with `.part`: the file a download fills before it may take its name.
fn part_path(to: &Path) -> PathBuf {
    sibling(to, ".part")
}

/// A relative name from a manifest or a zip, under `root`; refused if it could leave the folder
/// or name a drive — the zip-slip a hostile or damaged archive would use.
fn inside(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.contains('\\') || relative.starts_with('/') || relative.contains(':') {
        return Err(format!("a path outside the folder: {relative}"));
    }
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!("a path outside the folder: {relative}"));
        }
        path.push(part);
    }
    Ok(path)
}

fn make_parent(path: &Path) -> Result<(), String> {
    match path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        Some(dir) => fs::create_dir_all(dir).map_err(|e| disk(dir, e)),
        None => Ok(()),
    }
}

fn create(path: &Path) -> Result<File, String> {
    make_parent(path)?;
    File::create(path).map_err(|e| disk(path, e))
}

/// `path`'s size when it holds exactly `expected`, else `None` (missing, short, or another
/// file): the resume rule every download follows.
fn file_hash(path: &Path, expected: &str) -> Result<Option<u64>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(at(path, error)),
    };
    let mut digest = Hash::new(expected.len());
    let mut body = io::BufReader::new(file);
    let mut bytes = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = body.read(&mut buffer).map_err(|e| at(path, e))?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
        bytes += n as u64;
    }
    Ok(digest
        .finish()
        .eq_ignore_ascii_case(expected)
        .then_some(bytes))
}

/// A sha1 or a sha256, chosen by the digest's hex length, fed a file or a download at a time.
enum Hash {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
}

impl Hash {
    fn new(hex_len: usize) -> Self {
        if hex_len == 64 {
            Self::Sha256(sha2::Sha256::new())
        } else {
            Self::Sha1(sha1::Sha1::new())
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(digest) => sha1::Digest::update(digest, bytes),
            Self::Sha256(digest) => sha2::Digest::update(digest, bytes),
        }
    }

    fn finish(self) -> String {
        match self {
            Self::Sha1(digest) => hex(&sha1::Digest::finalize(digest)),
            Self::Sha256(digest) => hex(&sha2::Digest::finalize(digest)),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The host of an https address on the default port; anything else is refused, so a plain-http,
/// a credentialed or an odd-port address never reaches the network.
fn https_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') || authority.contains(':') {
        return None;
    }
    Some(authority)
}

/// Whether `url` may be fetched: https, on one of Mojang's endpoints, or — when a tables host
/// is named — on that host. The tables come from a build setting, so their host is the build's
/// own; the manifest's addresses are checked against Mojang's only.
fn url_allowed(url: &str, tables: Option<&str>) -> bool {
    if https_host(url).is_none() {
        return false;
    }
    MOJANG_HOSTS.iter().any(|host| host_matches(url, host))
        || tables.is_some_and(|tables| host_matches(url, tables))
        || tables.is_some_and(|tables| tables.eq_ignore_ascii_case("github.com"))
            && GITHUB_ASSET_HOSTS
                .iter()
                .any(|host| host_matches(url, host))
}

/// Where github.com sends a release asset's download (a redirect): allowed only when the tables
/// are a GitHub release.
const GITHUB_ASSET_HOSTS: [&str; 2] = [
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

/// Redirects one download may take: GitHub's release assets take one.
const MAX_REDIRECTS: usize = 5;

/// Whether `url`'s host is exactly `host` (not a name that merely ends with it).
fn host_matches(url: &str, host: &str) -> bool {
    https_host(url).is_some_and(|named| named.eq_ignore_ascii_case(host))
}

/// Anything that answers a URL with bytes, so the tests need no network. It travels to the
/// download lanes on a shared reference (`&dyn Fetch`), which `Sync` makes safe.
pub trait Fetch: Sync {
    fn get(&self, url: &str) -> Result<Fetched, String>;
}

/// A response's body, and its length when the server names one.
pub struct Fetched {
    pub length: Option<u64>,
    pub body: Box<dyn Read + Send + Sync>,
}

/// The real transport: ureq over rustls. Redirects are followed by hand, each new address held to
/// the same allowlist (GitHub serves a release asset through one), never silently elsewhere.
struct Http {
    agent: ureq::Agent,
    /// The tables' host, the one address outside Mojang's own that this install may fetch.
    tables: Option<String>,
}

impl Http {
    fn new(tables: Option<String>) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(60))
                .user_agent(&format!(
                    "Magnetite/{} (game files setup)",
                    horizon_core::release::VERSION
                ))
                .build(),
            tables,
        }
    }
}

impl Fetch for Http {
    fn get(&self, url: &str) -> Result<Fetched, String> {
        let mut at = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            if !url_allowed(&at, self.tables.as_deref()) {
                return Err(format!("{url}: {at} is not an allowed address"));
            }
            match self.agent.get(&at).call() {
                Ok(response) if (300..400).contains(&response.status()) => {
                    let Some(next) = response.header("location") else {
                        return Err(format!("{url}: a redirect without an address"));
                    };
                    at = next.to_owned();
                }
                Ok(response) => {
                    let length = response
                        .header("content-length")
                        .and_then(|value| value.trim().parse::<u64>().ok());
                    return Ok(Fetched {
                        length,
                        body: response.into_reader(),
                    });
                }
                Err(ureq::Error::Status(code, _)) => return Err(format!("{url}: HTTP {code}")),
                Err(ureq::Error::Transport(error)) => return Err(no_connection(url, error)),
            }
        }
        Err(format!("{url}: too many redirects"))
    }
}

/// A request that never got an answer: the short words the boot screen shows.
fn no_connection(url: &str, error: impl std::fmt::Display) -> String {
    format!("{url}: no connection ({error})")
}

/// A read that failed mid-body: the network went away, or the disk did.
fn read_error(url: &str, error: io::Error) -> String {
    if is_network(&error) {
        no_connection(url, error)
    } else {
        format!("{url}: {error}")
    }
}

fn is_network(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::UnexpectedEof
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::TimedOut
            | io::ErrorKind::NotConnected
            | io::ErrorKind::HostUnreachable
            | io::ErrorKind::NetworkUnreachable
            | io::ErrorKind::NetworkDown
    )
}

/// `path` and the failure: a full disk says so in words the screen can show.
fn disk(path: &Path, error: io::Error) -> String {
    if is_no_space(&error) {
        format!("{NO_SPACE}: {}", path.display())
    } else {
        at(path, error)
    }
}

fn is_no_space(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::StorageFull
        || matches!(error.raw_os_error(), Some(28) | Some(112))
}

fn at(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Cursor;

    /// A transport over local bytes, which remembers what it was asked for.
    #[derive(Default)]
    struct MapFetch {
        files: HashMap<String, Vec<u8>>,
        calls: Mutex<Vec<String>>,
    }

    impl MapFetch {
        fn with(files: &[(&str, Vec<u8>)]) -> Self {
            Self {
                files: files
                    .iter()
                    .map(|(url, bytes)| ((*url).to_owned(), bytes.clone()))
                    .collect(),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl Fetch for MapFetch {
        fn get(&self, url: &str) -> Result<Fetched, String> {
            self.calls.lock().unwrap().push(url.to_owned());
            let bytes = self
                .files
                .get(url)
                .ok_or_else(|| format!("{url}: not here"))?;
            Ok(Fetched {
                length: Some(bytes.len() as u64),
                body: Box::new(Cursor::new(bytes.clone())),
            })
        }
    }

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut out);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, bytes) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(bytes).unwrap();
            }
            writer.finish().unwrap();
        }
        out.into_inner()
    }

    fn sha1_of(bytes: &[u8]) -> String {
        hex(&sha1::Sha1::digest(bytes))
    }

    fn sha256_of(bytes: &[u8]) -> String {
        hex(&sha2::Sha256::digest(bytes))
    }

    /// The whole install's inputs: the tables (with their manifest), a client jar, and an asset
    /// index naming one object.
    struct Fixture {
        _root: tempfile::TempDir,
        fetch: MapFetch,
        spec: InstallSpec,
        object: Vec<u8>,
        jar_member: Vec<u8>,
        table_file: Vec<u8>,
    }

    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let object = b"OggS a sound".to_vec();
            let jar_member = b"Minecraft's own data".to_vec();
            let table_file = b"{\"a table\": true}".to_vec();
            let object_hash = sha1_of(&object);
            let index = format!(
                "{{\"objects\": {{\"minecraft/sounds/a.ogg\": \
                 {{\"hash\": \"{object_hash}\", \"size\": {}}}}}}}",
                object.len()
            );
            let jar = zip_of(&[
                ("data/minecraft/example.json", &jar_member[..]),
                ("assets/.mcassetsroot", b""),
            ]);
            let client = format!("https://{}/client.jar", MOJANG_HOSTS[0]);
            let asset_index = format!("https://{}/26.json", MOJANG_HOSTS[0]);
            let manifest = format!(
                "{{\"minecraft\": \"{}\", \"client\": {{\"url\": \"{client}\", \"sha1\": \"{}\", \
                 \"size\": {}}}, \"asset_index\": {{\"id\": \"26\", \"url\": \"{asset_index}\", \
                 \"sha1\": \"{}\"}}, \"jar_files\": [\"data/\", \"assets/\"], \
                 \"tables_root\": \"tables\"}}",
                horizon_core::VANILLA_VERSION,
                sha1_of(&jar),
                jar.len(),
                sha1_of(index.as_bytes()),
            );
            let tables = zip_of(&[
                ("setup.json", manifest.as_bytes()),
                ("tables/horizon-resources.json", b"{}"),
                ("tables/data/horizon/table.json", &table_file[..]),
                ("tables-not-ours/x", b"no"),
            ]);
            let tables_url = "https://tables.invalid/tables.zip".to_owned();
            let object_url = format!("{OBJECTS}/{}/{}", &object_hash[..2], object_hash);
            let fetch = MapFetch::with(&[
                (tables_url.as_str(), tables.clone()),
                (client.as_str(), jar),
                (asset_index.as_str(), index.into_bytes()),
                (object_url.as_str(), object.clone()),
            ]);
            let spec = InstallSpec {
                resources: root.path().join("resources"),
                work: root.path().join("work"),
                tables_url,
                tables_sha256: sha256_of(&tables),
            };
            Self {
                _root: root,
                fetch,
                spec,
                object,
                jar_member,
                table_file,
            }
        }

        fn staging(&self) -> PathBuf {
            self.spec.work.join("staging")
        }
    }

    #[test]
    fn the_setup_assembles_the_folder_and_takes_its_place() {
        let fixture = Fixture::new();
        // A previous install leaves its own files behind: only the finished one may survive.
        fs::create_dir_all(&fixture.spec.resources).unwrap();
        fs::write(fixture.spec.resources.join("old.json"), b"old").unwrap();
        let progress = InstallProgress::new();
        install_with(&fixture.spec, &progress, &fixture.fetch).unwrap();
        let resources = &fixture.spec.resources;
        assert!(resources.join(MARKER).exists(), "the game's marker");
        assert_eq!(
            fs::read(resources.join("data/horizon/table.json")).unwrap(),
            fixture.table_file,
            "the tables' own files"
        );
        assert_eq!(
            fs::read(resources.join("data/minecraft/example.json")).unwrap(),
            fixture.jar_member,
            "the jar's members"
        );
        assert_eq!(
            fs::read(resources.join("assets/minecraft/sounds/a.ogg")).unwrap(),
            fixture.object,
            "the index's objects, at the name the index gives"
        );
        assert!(resources.join("asset-index.json").exists());
        assert!(!resources.join("setup.json").exists(), "not the manifest");
        assert!(
            !resources.join("tables-not-ours/x").exists(),
            "not another folder"
        );
        assert!(
            !resources.join("old.json").exists(),
            "the old folder is gone"
        );
        assert!(
            !fixture.spec.work.exists(),
            "the caches leave with the install"
        );
        assert!(!fixture.staging().exists());
        let (done, total, _) = progress.snapshot();
        assert_eq!(
            done, total,
            "every byte counted is a byte that was expected"
        );
        assert!(
            !resources.with_file_name("resources.previous").exists(),
            "nothing waits behind the new folder"
        );
    }

    #[test]
    fn a_failure_leaves_the_old_folder_and_the_download_alone() {
        let mut fixture = Fixture::new();
        fs::create_dir_all(&fixture.spec.resources).unwrap();
        fs::write(fixture.spec.resources.join(MARKER), b"{}").unwrap();
        // The jar's address answers with nothing: the install stops after the tables.
        let client = format!("https://{}/client.jar", MOJANG_HOSTS[0]);
        fixture.fetch.files.remove(&client);
        let progress = InstallProgress::new();
        let error = install_with(&fixture.spec, &progress, &fixture.fetch).unwrap_err();
        assert!(error.contains("not here"), "{error}");
        assert!(
            fixture.spec.resources.join(MARKER).exists(),
            "the folder the player had still works"
        );
        assert!(
            fixture.staging().join(MARKER).exists(),
            "what was unpacked waits for the next attempt"
        );
        // The tables are kept: the next attempt does not fetch them again.
        assert!(fixture.spec.work.join("tables.zip").exists());
        fixture.fetch.files.insert(client.clone(), Vec::new());
        let calls_before = fixture.fetch.calls().len();
        let _ = install_with(&fixture.spec, &progress, &fixture.fetch);
        let calls = fixture.fetch.calls();
        assert!(
            calls[calls_before..].iter().all(|url| url == &client),
            "only the jar is asked for again: {:?}",
            &calls[calls_before..]
        );
    }

    #[test]
    fn a_file_with_the_right_hash_is_not_fetched_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        let bytes = b"already here".to_vec();
        fs::write(&path, &bytes).unwrap();
        let fetch = MapFetch::with(&[("https://piston-data.mojang.com/a", bytes.clone())]);
        let progress = InstallProgress::new();
        let want = Wanted {
            url: "https://piston-data.mojang.com/a",
            sha: &sha1_of(&bytes),
            size: Some(bytes.len() as u64),
        };
        download(&fetch, &want, &path, &progress).unwrap();
        assert!(fetch.calls().is_empty(), "kept, not downloaded");
        assert_eq!(
            progress.snapshot(),
            (bytes.len() as u64, bytes.len() as u64, String::new())
        );
        // A file that does not match is fetched.
        fs::write(&path, b"corrupt").unwrap();
        download(&fetch, &want, &path, &progress).unwrap();
        assert_eq!(fetch.calls().len(), 1);
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn a_download_that_does_not_match_its_hash_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jar.bin");
        let fetch = MapFetch::with(&[(
            "https://piston-data.mojang.com/jar",
            b"other bytes".to_vec(),
        )]);
        let progress = InstallProgress::new();
        let want = Wanted {
            url: "https://piston-data.mojang.com/jar",
            sha: &sha1_of(b"the jar"),
            size: None,
        };
        let error = download(&fetch, &want, &path, &progress).unwrap_err();
        assert!(error.contains("does not match its hash"), "{error}");
        assert!(!path.exists());
        assert!(!part_path(&path).exists(), "the partial file is gone too");
        assert_eq!(fetch.calls().len(), TRIES, "each try asked again");
        assert_eq!(
            progress.done.load(Ordering::Relaxed),
            0,
            "nothing stays counted"
        );
    }

    #[test]
    fn a_path_from_a_zip_cannot_leave_the_folder() {
        let staging = Path::new("/staging");
        assert_eq!(
            staged_path(staging, "tables/data/a.json", "tables").unwrap(),
            Some(staging.join("data/a.json"))
        );
        assert_eq!(staged_path(staging, "setup.json", "tables").unwrap(), None);
        assert_eq!(staged_path(staging, "other/a", "tables").unwrap(), None);
        for name in [
            "tables/../evil.json",
            "tables/data/../../evil.json",
            "tables//etc/passwd",
            "tables/data\\evil.json",
            "tables/C:/evil.json",
        ] {
            assert!(
                staged_path(staging, name, "tables").is_err(),
                "{name} must be refused"
            );
        }
        // With no root, everything is the tables' but the manifest.
        assert_eq!(
            staged_path(staging, "data/a.json", "").unwrap(),
            Some(staging.join("data/a.json"))
        );
        assert!(inside(staging, "a/../../b").is_err());
        assert!(inside(staging, "/b").is_err());
    }

    #[test]
    fn only_https_on_an_allowed_host_may_be_fetched() {
        assert!(url_allowed("https://piston-data.mojang.com/a", None));
        assert!(url_allowed(
            "https://resources.download.minecraft.net/ab/abcd",
            None
        ));
        assert!(
            !url_allowed("http://piston-data.mojang.com/a", None),
            "https only"
        );
        assert!(
            !url_allowed("https://mojang.com.evil.invalid/a", None),
            "another host"
        );
        assert!(
            !url_allowed("https://piston-data.mojang.com:8443/a", None),
            "the default port only"
        );
        assert!(
            !url_allowed("https://user@piston-data.mojang.com/a", None),
            "no credentials in the address"
        );
        // A GitHub release's asset redirect is allowed only when the tables are on github.com.
        assert!(url_allowed(
            "https://release-assets.githubusercontent.com/a",
            Some("github.com")
        ));
        assert!(!url_allowed(
            "https://objects.githubusercontent.com/a",
            Some("tables.invalid")
        ));
        assert!(!url_allowed(
            "https://objects.githubusercontent.com/a",
            None
        ));
        // The tables' own host is allowed when the build names it, and nothing else is.
        assert!(url_allowed(
            "https://tables.invalid/t.zip",
            Some("tables.invalid")
        ));
        assert!(!url_allowed(
            "https://elsewhere.invalid/t.zip",
            Some("tables.invalid")
        ));
    }

    #[test]
    fn the_builds_tables_must_be_https_and_pinned() {
        let resources = PathBuf::from("/r");
        let work = PathBuf::from("/w");
        let good = |url: &str, sha: &str| {
            spec_from_values(
                Some(url.to_owned()),
                Some(sha.to_owned()),
                resources.clone(),
                work.clone(),
            )
        };
        assert!(good("https://tables.invalid/t.zip", &"a".repeat(64)).is_some());
        assert!(good("http://tables.invalid/t.zip", &"a".repeat(64)).is_none());
        assert!(good("https://tables.invalid/t.zip", "not a hash").is_none());
        assert!(good("https://tables.invalid/t.zip", &"a".repeat(63)).is_none());
        assert!(
            spec_from_values(None, Some("a".repeat(64)), resources.clone(), work.clone()).is_none()
        );
        assert!(
            spec_from_values(
                Some("https://tables.invalid/t.zip".into()),
                None,
                resources,
                work
            )
            .is_none()
        );
    }

    #[test]
    fn the_manifest_is_read_and_its_version_checked() {
        let good = format!(
            "{{\"minecraft\": \"{}\", \"client\": {{\"url\": \"https://{}/client.jar\", \
             \"sha1\": \"{}\", \"size\": 12}}, \"asset_index\": {{\"id\": \"26\", \
             \"url\": \"https://{}/26.json\", \"sha1\": \"{}\"}}, \
             \"jar_files\": [\"assets/\", \"data/\", \"pack.png\"], \"tables_root\": \"tables\"}}",
            horizon_core::VANILLA_VERSION,
            MOJANG_HOSTS[0],
            "a".repeat(40),
            MOJANG_HOSTS[1],
            "b".repeat(40),
        );
        let manifest = Manifest::parse(&good).unwrap();
        assert_eq!(manifest.client.size, 12);
        assert_eq!(manifest.jar_files.len(), 3);
        assert_eq!(manifest.tables_root, "tables");
        // The tables of another version are refused, not mixed into this build's folder.
        let other = good.replace(
            &format!("\"minecraft\": \"{}\"", horizon_core::VANILLA_VERSION),
            "\"minecraft\": \"26.2\"",
        );
        assert!(Manifest::parse(&other).unwrap_err().contains("26.2"));
        // A Mojang address that is not Mojang's is refused, and so is plain http.
        assert!(Manifest::parse(&good.replace(MOJANG_HOSTS[0], "downloads.invalid")).is_err());
        assert!(Manifest::parse(&good.replace("https://", "http://")).is_err());
        // No jar files is an error: the folder would lack Minecraft's assets.
        let no_jar_files = good.replace(
            "\"jar_files\": [\"assets/\", \"data/\", \"pack.png\"]",
            "\"jar_files\": []",
        );
        assert!(Manifest::parse(&no_jar_files).is_err());
        // A bad sha1 is an error.
        assert!(Manifest::parse(&good.replace(&"a".repeat(40), "abc")).is_err());
        // The tables' root defaults to `tables`, and may be the zip's own root.
        let no_root = good.replace(", \"tables_root\": \"tables\"", "");
        assert_eq!(Manifest::parse(&no_root).unwrap().tables_root, "tables");
        let at_root = good.replace("\"tables_root\": \"tables\"", "\"tables_root\": \"\"");
        assert_eq!(Manifest::parse(&at_root).unwrap().tables_root, "");
    }

    #[test]
    fn the_old_folder_comes_back_when_the_new_one_cannot_take_its_place() {
        let dir = tempfile::tempdir().unwrap();
        let resources = dir.path().join("resources");
        fs::create_dir_all(resources.join("keep")).unwrap();
        fs::write(resources.join("keep/a"), b"old").unwrap();
        // A staging folder that is not there cannot be moved: the previous folder returns.
        let missing = dir.path().join("nothing");
        assert!(activate(&missing, &resources).is_err());
        assert_eq!(fs::read(resources.join("keep/a")).unwrap(), b"old");
        assert_eq!(
            fs::read(dir.path().join("resources.previous/keep/a"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound,
            "the old folder is back in place, not beside it"
        );
        // A staging folder that is there takes the place, and the old one is removed.
        let staging = dir.path().join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join(MARKER), b"{}").unwrap();
        activate(&staging, &resources).unwrap();
        assert!(resources.join(MARKER).exists());
        assert!(!resources.join("keep/a").exists());
        assert!(!dir.path().join("resources.previous").exists());
    }

    #[test]
    fn a_folder_copies_whole() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from");
        let to = dir.path().join("to");
        fs::create_dir_all(from.join("a/b")).unwrap();
        fs::write(from.join("a/b/c.txt"), b"hi").unwrap();
        copy_dir(&from, &to).unwrap();
        assert_eq!(fs::read(to.join("a/b/c.txt")).unwrap(), b"hi");
        assert!(from.join("a/b/c.txt").exists(), "a copy leaves the source");
    }

    #[test]
    fn the_tables_without_the_marker_are_not_activated() {
        let dir = tempfile::tempdir().unwrap();
        let table = b"{}".to_vec();
        let jar = zip_of(&[("data/a.json", &table[..])]);
        let index_json = b"{\"objects\": {}}".to_vec();
        let manifest = format!(
            "{{\"minecraft\": \"{}\", \"client\": {{\"url\": \"https://{}/client.jar\", \
             \"sha1\": \"{}\"}}, \"asset_index\": {{\"id\": \"26\", \
             \"url\": \"https://{}/26.json\", \"sha1\": \"{}\"}}, \"jar_files\": [\"data/\"]}}",
            horizon_core::VANILLA_VERSION,
            MOJANG_HOSTS[0],
            sha1_of(&jar),
            MOJANG_HOSTS[1],
            sha1_of(&index_json),
        );
        let client = format!("https://{}/client.jar", MOJANG_HOSTS[0]);
        let index = format!("https://{}/26.json", MOJANG_HOSTS[1]);
        let tables = zip_of(&[
            ("setup.json", manifest.as_bytes()),
            ("tables/data/only.json", &table[..]),
        ]);
        let tables_url = "https://tables.invalid/t.zip".to_owned();
        let fetch = MapFetch::with(&[
            (tables_url.as_str(), tables.clone()),
            (client.as_str(), jar),
            (index.as_str(), index_json),
        ]);
        let spec = InstallSpec {
            resources: dir.path().join("resources"),
            work: dir.path().join("work"),
            tables_url,
            tables_sha256: sha256_of(&tables),
        };
        let error = install_with(&spec, &InstallProgress::new(), &fetch).unwrap_err();
        assert!(error.contains(MARKER), "{error}");
        assert!(!spec.resources.exists(), "nothing takes the place");
    }
}
