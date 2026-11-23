//! Magnetite for PC: the launcher window first (sign-in, profiles, mods, settings), then the game
//! in the same process, as the phones do (`horizon_client::boot`). The game's own files come as on
//! a phone: downloaded once the player signs in, into Magnetite's folder in the platform's data
//! folder, so nothing else needs installing. `horizon-client` stays the plain game executable that
//! a launcher starts with arguments.

fn main() {
    horizon_game::prepare_main_stack();
    // The game compiles WebAssembly mods in a child process that is this executable again.
    horizon_mod_host::exit_if_compile_child();
    let root = horizon_client::logging::data_dir().join("magnetite");
    // The game folder holds what the player sees (saves, options, mods, logs, profiles); the
    // account and the session file sit beside it, the download's staging in a folder of its own.
    let game_dir = root.join("game");
    let resources = game_dir.join("resources");
    let support = root.join("account");
    let work = root.join("setup");
    for dir in [&game_dir, &support, &work] {
        if let Err(error) = std::fs::create_dir_all(dir) {
            eprintln!("Magnetite: cannot make {}: {error}", dir.display());
            std::process::exit(1);
        }
    }
    let config = horizon_client::boot::BootConfig {
        game_dir,
        resources: resources.clone(),
        work: work.clone(),
        session_dir: support.clone(),
        secrets: Box::new(horizon_account::FileStore::new(
            support.join("account.json"),
        )),
        open_link,
        keep_awake: |_| {},
        install: horizon_client::boot::installer_from_build(resources, work),
        start: Box::new(|args: Vec<String>| {
            let startup = horizon_client::entry::prepare_with_args(args);
            horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
        }),
        language: sys_locale::get_locale(),
        launcher: true,
        set_orientation: horizon_client::boot::no_orientation,
        share_log,
        open_folder,
        signature_expires: None,
    };
    if let Err(error) = horizon_client::boot::run(config) {
        eprintln!("Magnetite: the boot loop stopped: {error}");
        std::process::exit(1);
    }
}

/// A link in the system's browser.
fn open_link(url: &str) {
    if let Err(error) = horizon_client::desktop::open_uri(url) {
        log::warn!("Magnetite: the link did not open: {error}");
    }
}

/// The log, on a PC: the file itself, in the system's handler for text, to read or attach.
fn share_log(path: &std::path::Path, _text: &str) {
    if let Err(error) = horizon_client::desktop::open_path(path) {
        log::warn!("Magnetite: the log did not open: {error}");
    }
}

/// The game folder in the system's file manager.
fn open_folder(path: &std::path::Path) -> bool {
    horizon_client::desktop::open_path(path)
        .inspect_err(|error| log::warn!("Magnetite: the game folder did not open: {error}"))
        .is_ok()
}
