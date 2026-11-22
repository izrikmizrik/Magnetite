// game-module-visibility: pub
//! What runs before the game on a phone (or a PC without a launcher): the sign-in, the game
//! files, then the hand-over to the game itself (`docs/unification/mobile-boot.md` §1.4, §3).
//! The process's one event loop opens here at once — iOS's watchdog ends an app that takes too
//! long to start — and a screen of its own (`horizon_client::boot_screen`) draws while the
//! sign-in and the download run on threads beside it. When the files are there and an account
//! (or an offline name) is ready, the session file is written fresh, the game is started through
//! the host's `start` closure, and its [`App`] is taken from the sink and run on this same loop.
//!
//! Platform-neutral: no `#[cfg(target_os)]`. The host (`magnetite-android`, `magnetite-ios`)
//! supplies the paths, the secret store, the link opener, the keep-awake switch, the installer
//! and the `start` closure; the event-loop factory is already set (`window::set_event_loop_factory`).
//!
//! The sign-in controller (`Controller`) is a pure state machine: `Controller::action` and
//! `Controller::apply` turn taps and thread results into transitions and `Job`s, and never
//! touch a network, a thread or a file themselves. `Boot` runs the jobs, keeps the secrets and
//! draws. That split is what the tests exercise without a window.
use super::*;
use horizon_account::{
    Account, DeviceCode, Endpoints, Kind, NEEDS_2FA, REFUSED, SecretStore, Stored, client_id,
    device_code, ely_sign_in, refresh_account, sign_in,
};
use horizon_client::boot_screen::{AccountKind, BootAction, BootScreen, BootState, ElyField};
use horizon_client::clipboard::SystemClipboard;
use horizon_client::game_files::{
    InstallProgress, InstallSpec, install as install_game_files, spec_from_build,
};
use horizon_client::osk::Output;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{SystemTime, UNIX_EPOCH};
use winit::event::TouchPhase;

/// The game files' installer, as the host hands it over: repeatable, so a failed download is
/// tried again from the Retry button instead of needing a restart. It runs on a thread of the
/// boot loop's own; the content hashes it keeps make the second call fetch only what is missing.
pub type Installer = Arc<dyn Fn(&InstallProgress) -> std::result::Result<(), String> + Send + Sync>;

/// The installer a build with `MAGNETITE_TABLES_URL` and `MAGNETITE_TABLES_SHA256` names, for
/// `BootConfig::install`: it assembles `resources` from the tables and the player's own
/// Minecraft, keeping its downloads in `work`. `None` leaves the boot screen's "this build
/// cannot download them", for a build that names no tables.
pub fn installer_from_build(resources: PathBuf, work: PathBuf) -> Option<Installer> {
    let spec: InstallSpec = spec_from_build(resources, work)?;
    Some(Arc::new(move |progress: &InstallProgress| {
        install_game_files(&spec, progress)
    }))
}

/// Everything the host hands the boot loop. `start` receives the game's ready command line and
/// starts the core (`entry::prepare_with_args` + `horizon_mod_host::main_with`); the core hands
/// its [`App`] back through the sink instead of opening a second loop. `install` is the game
/// files' installer (`horizon_client::game_files`, or `None` when this build names no tables):
/// without it, and without files, the screen explains and offline play is unavailable.
pub struct BootConfig {
    /// The player's folder: saves, `options.txt`, logs (`--game-dir`).
    pub game_dir: PathBuf,
    /// The prepared game files' folder (`--resources`).
    pub resources: PathBuf,
    /// The game files' download folder (staging and caches), kept between runs so an
    /// interrupted download continues.
    pub work: PathBuf,
    /// Where the fresh session file is written, beside the secrets (the core reads and deletes
    /// it; it must be under 120 s old and, on Unix, 0600).
    pub session_dir: PathBuf,
    /// The kept refresh secret (never an access token).
    pub secrets: Box<dyn SecretStore + Send>,
    /// Open the device-code page in the platform's browser; a no-op leaves the code and URL on
    /// the screen to type by hand.
    pub open_link: fn(&str),
    /// Keep the screen on while the files download.
    pub keep_awake: fn(bool),
    /// The game files' installer, run on a thread after any sign-in when the files are missing.
    pub install: Option<Installer>,
    /// Start the core with these arguments; it hands its app to the sink.
    pub start: Box<dyn FnOnce(Vec<String>)>,
    /// The system's language as a game language code (`ru_ru`), or `None`. A first start, with
    /// no language chosen yet, takes it: the boot screen and the game both speak it.
    pub language: Option<String>,
}

/// Run the boot loop until the game takes over (or the player leaves). One event loop per
/// process: the host sets the factory before calling this.
pub fn run(mut config: BootConfig) -> Result<()> {
    // The game's own log from the first screen on (`logs/latest.log` in the game folder, and the
    // host's platform sink): the game's start later keeps the same one.
    horizon_client::logging::init(&config.game_dir, "Render thread");
    log::info!("Horizon boot screen in {}", config.game_dir.display());
    // The fonts and the icon, before the first window: the screen draws in the design system's
    // look, not the engine's neutral one. `entry::start` gives it again later; that is harmless.
    crate::entry::horizon_look::give();
    // A host that brought no installer of its own still gets one when the build names its
    // tables: the same code path serves every platform.
    if config.install.is_none() {
        config.install = installer_from_build(config.resources.clone(), config.work.clone());
    }
    // The game folder is the working directory from here, as `--game-dir` makes it for the core:
    // the player's folder (`logging::player_dir`) is the one the game will use, so a renderer
    // start the boot window leaves unfinished is found by the game's options (`RENDER_MARKER`).
    std::env::set_current_dir(&config.game_dir)?;
    let mut boot = Boot::new(config);
    // The strings compiled in, in the player's language: there are no prepared ones yet, and the
    // game reads the player's own settings again after the hand-over.
    horizon_client::lang::set(horizon_client::lang::Language::load(
        &[],
        &boot.settings.language,
    ));
    let event_loop = horizon_client::window::new_event_loop()?;
    event_loop.run_app(&mut boot)?;
    boot.error.map_or(Ok(()), Err)
}

thread_local! {
    /// The game's app, handed back on this thread while `start` runs inside the loop.
    static HANDED: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// The sink the core hands its app to (`set_app_sink` takes a plain `fn`, so the app waits in a
/// thread-local the loop thread reads right after `start` returns).
fn hand_over(app: App) {
    HANDED.with(|handed| *handed.borrow_mut() = Some(app));
}

/// Whether an error is the network being unreachable rather than a service refusing: the
/// transport message `horizon-account` gives for every service. A refused token is forgotten;
/// no network keeps the account for the next start (the iOS fork's rule).
fn no_connection(error: &str) -> bool {
    error.contains("Check your internet connection")
}

/// Epoch seconds now, for the device code's remaining life.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn kind_of(kind: Kind) -> AccountKind {
    match kind {
        Kind::Microsoft => AccountKind::Microsoft,
        Kind::Ely => AccountKind::Ely,
    }
}

/// A service's own words, under a translated label: the error text is the service's data, the
/// label is the screen's (`horizon.boot.*`).
fn detail(error: &str) -> String {
    horizon_client::lang::tr("horizon.boot.service_said", &[&error])
}

/// The game's command line: the folders, then the session file for a signed-in account or the
/// name for an offline one.
fn game_args(
    game_dir: &Path,
    resources: &Path,
    session: Option<&Path>,
    name: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "--game-dir".to_owned(),
        game_dir.to_string_lossy().into_owned(),
        "--resources".to_owned(),
        resources.to_string_lossy().into_owned(),
    ];
    if let Some(path) = session {
        args.push("--session-file".to_owned());
        args.push(path.to_string_lossy().into_owned());
    }
    if let Some(name) = name {
        args.push("--name".to_owned());
        args.push(name.to_owned());
    }
    args
}

/// Write the session file the core reads at startup: private (0600 on Unix), synced, fresh. The
/// core refuses one older than 120 s and deletes it after reading, so this is written immediately
/// before the game starts, not at sign-in (the download can take minutes).
fn write_session(path: &Path, json: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    std::io::Write::write_all(&mut file, json.as_bytes())?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // The mode above only applies to a file this call creates; an old one keeps its bits.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// The game language closest to the system's (`ru_ua`): the same code if the game has it, else
/// the language in its own country (`ru_ru`, `de_de`); `None` leaves the default.
fn game_language(system: &str) -> Option<String> {
    let system = system.trim().to_lowercase().replace('-', "_");
    let language = system.split('_').next().filter(|l| !l.is_empty())?;
    let fallback = horizon_client::lang::Language::load(&[], horizon_client::lang::DEFAULT);
    let has = |code: &str| {
        code == horizon_client::lang::DEFAULT
            || horizon_client::lang::Language::load(&[], code).map() != fallback.map()
    };
    [system.clone(), format!("{language}_{language}")]
        .into_iter()
        .find(|code| has(code))
}

/// The Microsoft code the player was asked to type, kept in the session folder while the game
/// waits for the approval: Android unloads a backgrounded app, and the next start waits for the
/// same approval instead of asking for a new code (`DeviceCode::to_json`).
const PENDING_CODE: &str = "pending-sign-in.json";

/// A thread to run, or the game to start. The controller asks; [`Boot`] does.
enum Job {
    /// Sign a kept account in again.
    Resume(Stored),
    /// Microsoft's device code, then the wait for the player.
    Microsoft,
    /// The wait for the player on a code the last start asked for and kept (`PENDING_CODE`): the
    /// app was unloaded while the player was at the page.
    MicrosoftKept(DeviceCode),
    /// Ely.by authenticate.
    Ely {
        login: String,
        password: String,
        otp: String,
    },
    /// The game files' installer.
    Install,
    /// Hand over to the game (on the loop thread, not a worker).
    Start,
}

/// What a worker sends back, with the sign-in attempt it belongs to.
enum Event {
    Resume(std::result::Result<Account, String>),
    Code(std::result::Result<DeviceCode, String>),
    SignIn(std::result::Result<Account, String>),
    Ely(std::result::Result<Account, String>),
    Install(std::result::Result<(), String>),
}

/// The field the platform's keyboard is up for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardFor {
    Ely(ElyField),
    Offline,
}

/// The Ely.by form's values: the controller keeps them, never the screen. The password is used
/// for one request and never stored or drawn as text.
#[derive(Clone)]
struct ElyForm {
    login: String,
    password: String,
    /// `Some` once Ely.by asked for a two-factor code.
    otp: Option<String>,
    focus: ElyField,
    error: Option<String>,
}

impl Default for ElyForm {
    fn default() -> Self {
        Self {
            login: String::new(),
            password: String::new(),
            otp: None,
            focus: ElyField::Login,
            error: None,
        }
    }
}

/// Where the account side stands.
enum Phase {
    Choose,
    Resuming {
        stored: Stored,
    },
    /// Asking Microsoft for a code (brief; the screen stays on the choice).
    Asking,
    Code {
        code: DeviceCode,
    },
    Ely(ElyForm),
    Signed {
        account: Account,
        restored: bool,
    },
    /// No network at resume: the kept identity stands, the token comes when the connection does.
    SavedOffline {
        stored: Stored,
    },
    OfflineName {
        name: String,
        error: Option<String>,
    },
    Offline {
        name: String,
    },
}

/// What the Retry button does after a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Retry {
    /// Ask Microsoft for a new code.
    Microsoft,
    /// Run the installer again, from what the last attempt kept.
    Install,
    /// Back to the sign-in choice.
    Choose,
}

#[derive(Debug, PartialEq, Eq)]
struct Failure {
    message: String,
    retry: Retry,
}

/// The sign-in and files state machine. Pure: it transitions and names jobs; it never spawns a
/// thread, opens a file or touches a network. [`Boot`] drains its side effects (`take_save`,
/// `take_forget`) and runs its jobs.
struct Controller {
    /// Counts the sign-ins started: a result whose attempt is no longer the latest changes
    /// nothing. Shared with the workers so a cancelled one stops polling.
    attempt: Arc<AtomicU64>,
    phase: Phase,
    files_ready: bool,
    /// Whether this build has an installer at all (`false`: the screen says the files cannot
    /// be downloaded here). The installer itself is repeatable, so a failed attempt is retried
    /// in place rather than needing a restart.
    has_installer: bool,
    installing: bool,
    failure: Option<Failure>,
    client_id: Option<String>,
    pending_save: Option<String>,
    pending_forget: bool,
    /// Bumped at every change, for the screen to draw again.
    version: u64,
}

impl Controller {
    fn new(files_ready: bool, has_installer: bool, client_id: Option<String>) -> Self {
        Self {
            attempt: Arc::new(AtomicU64::new(0)),
            phase: Phase::Choose,
            files_ready,
            has_installer,
            installing: false,
            failure: None,
            client_id,
            pending_save: None,
            pending_forget: false,
            version: 0,
        }
    }

    fn current_attempt(&self) -> u64 {
        self.attempt.load(Ordering::SeqCst)
    }

    /// A new sign-in: the ones before it stop and their results are ignored.
    fn begin_attempt(&mut self) -> u64 {
        let attempt = self.attempt.fetch_add(1, Ordering::SeqCst) + 1;
        self.version += 1;
        attempt
    }

    fn take_save(&mut self) -> Option<String> {
        self.pending_save.take()
    }

    fn take_forget(&mut self) -> bool {
        std::mem::take(&mut self.pending_forget)
    }

    fn fail(&mut self, message: String, retry: Retry) {
        self.failure = Some(Failure { message, retry });
    }

    /// A signed-in account: keep its refresh secret, and download the files if they are missing
    /// (the owner's rule: files come after any sign-in, Microsoft or Ely.by).
    fn signed_in(&mut self, account: Account, restored: bool) -> Vec<Job> {
        self.pending_save = Some(account.stored_json());
        self.phase = Phase::Signed { account, restored };
        self.failure = None;
        self.start_install()
    }

    /// The files step: ready already, or the installer runs, or this build has none.
    fn start_install(&mut self) -> Vec<Job> {
        if self.files_ready {
            return Vec::new();
        }
        if self.has_installer {
            self.installing = true;
            vec![Job::Install]
        } else {
            self.fail(
                horizon_client::lang::get("horizon.boot.files.no_installer"),
                Retry::Choose,
            );
            Vec::new()
        }
    }

    /// A restored sign-in or a confirmed offline name, with the files here: start without a tap.
    fn ready_to_start(&self) -> bool {
        self.files_ready
            && !self.installing
            && self.failure.is_none()
            && matches!(
                self.phase,
                Phase::Signed { restored: true, .. } | Phase::Offline { .. }
            )
    }

    /// The screen's state for this moment: a failure over everything, the download over the
    /// account, then the account phase.
    fn screen_state(&self, progress: &InstallProgress) -> BootState {
        if let Some(failure) = &self.failure {
            return BootState::Failed {
                message: failure.message.clone(),
                resumes: matches!(failure.retry, Retry::Install),
            };
        }
        if self.installing {
            let (done_bytes, total_bytes, step) = progress.snapshot();
            return BootState::Files {
                done_bytes,
                total_bytes,
                step,
            };
        }
        match &self.phase {
            Phase::Choose | Phase::Asking => BootState::Choose,
            Phase::Resuming { stored } => BootState::Resuming {
                name: stored.name.clone(),
            },
            Phase::Code { code } => BootState::Code {
                code: code.user_code.clone(),
                url: code.verification_uri.clone(),
                expires_in_secs: code.expires_at.saturating_sub(now_secs()),
            },
            Phase::Ely(form) => BootState::Ely {
                login: form.login.clone(),
                password_len: form.password.chars().count(),
                otp: form.otp.clone(),
                focus: form.focus,
                error: form.error.clone(),
            },
            Phase::Signed { account, .. } => BootState::Signed {
                name: account.name.clone(),
                kind: kind_of(account.kind),
                note: None,
            },
            Phase::SavedOffline { stored } => BootState::Signed {
                name: stored.name.clone(),
                kind: kind_of(stored.kind),
                note: Some(horizon_client::lang::get(
                    "horizon.boot.signed.offline_note",
                )),
            },
            Phase::OfflineName { name, error } => BootState::OfflineName {
                name: name.clone(),
                error: error.clone(),
            },
            Phase::Offline { name } => BootState::Offline { name: name.clone() },
        }
    }

    /// A tap's action: the transitions and the jobs it asks for. `OpenLink` and `CopyCode` are
    /// the host's own (they need its callbacks) and do nothing here.
    fn action(&mut self, action: BootAction) -> Vec<Job> {
        let jobs = match action {
            BootAction::SignInMicrosoft => {
                if self.client_id.is_none() {
                    self.phase = Phase::Choose;
                    self.fail(
                        horizon_client::lang::get("horizon.boot.microsoft.not_configured"),
                        Retry::Choose,
                    );
                    Vec::new()
                } else {
                    self.begin_attempt();
                    self.phase = Phase::Asking;
                    self.failure = None;
                    vec![Job::Microsoft]
                }
            }
            BootAction::SignInEly => {
                self.begin_attempt();
                self.phase = Phase::Ely(ElyForm::default());
                self.failure = None;
                Vec::new()
            }
            BootAction::PlayOffline => {
                if self.files_ready {
                    self.phase = Phase::OfflineName {
                        name: String::new(),
                        error: None,
                    };
                    self.failure = None;
                } else {
                    self.phase = Phase::Choose;
                    self.fail(
                        horizon_client::lang::get("horizon.boot.offline.need_files"),
                        Retry::Choose,
                    );
                }
                Vec::new()
            }
            BootAction::Cancel => {
                self.begin_attempt();
                self.phase = Phase::Choose;
                self.failure = None;
                Vec::new()
            }
            BootAction::Back => {
                if self.failure.is_some() {
                    self.failure = None;
                } else {
                    self.begin_attempt();
                    self.phase = Phase::Choose;
                }
                Vec::new()
            }
            BootAction::Submit => match &mut self.phase {
                Phase::Ely(form) => {
                    form.error = None;
                    let job = Job::Ely {
                        login: form.login.clone(),
                        password: form.password.clone(),
                        otp: form.otp.clone().unwrap_or_default(),
                    };
                    self.begin_attempt();
                    vec![job]
                }
                Phase::OfflineName { name, error } => {
                    if valid_offline_name(name) {
                        let chosen = name.clone();
                        self.phase = Phase::Offline { name: chosen };
                    } else {
                        *error = Some(horizon_client::lang::get(
                            "horizon.boot.offline.name.invalid",
                        ));
                    }
                    Vec::new()
                }
                _ => Vec::new(),
            },
            BootAction::Continue => {
                if matches!(
                    self.phase,
                    Phase::Signed { .. } | Phase::SavedOffline { .. }
                ) {
                    if self.files_ready {
                        vec![Job::Start]
                    } else {
                        self.start_install()
                    }
                } else {
                    Vec::new()
                }
            }
            BootAction::SignOut => {
                self.begin_attempt();
                self.pending_forget = true;
                self.phase = Phase::Choose;
                self.failure = None;
                Vec::new()
            }
            BootAction::Retry => {
                let retry = self.failure.take().map_or(Retry::Choose, |f| f.retry);
                match retry {
                    Retry::Microsoft => {
                        self.begin_attempt();
                        self.phase = Phase::Asking;
                        vec![Job::Microsoft]
                    }
                    Retry::Install => {
                        if self.files_ready || !self.has_installer {
                            Vec::new()
                        } else {
                            self.installing = true;
                            vec![Job::Install]
                        }
                    }
                    Retry::Choose => {
                        self.phase = Phase::Choose;
                        Vec::new()
                    }
                }
            }
            BootAction::FocusField(field) => {
                if let Phase::Ely(form) = &mut self.phase {
                    form.focus = field;
                }
                Vec::new()
            }
            BootAction::Typed(output) => {
                self.type_into_field(&output);
                Vec::new()
            }
            BootAction::OpenLink | BootAction::CopyCode => Vec::new(),
        };
        self.version += 1;
        jobs
    }

    /// A key of the on-screen keyboard into the field that has focus.
    fn type_into_field(&mut self, output: &Output) {
        let text = match &mut self.phase {
            Phase::Ely(form) => match form.focus {
                ElyField::Login => Some(&mut form.login),
                ElyField::Password => Some(&mut form.password),
                ElyField::Otp => form.otp.as_mut(),
            },
            Phase::OfflineName { name, .. } => Some(name),
            _ => None,
        };
        let Some(text) = text else {
            return;
        };
        match output {
            Output::Text(typed) => text.push_str(typed),
            Output::Backspace => {
                text.pop();
            }
            // One-line fields: Done is the form's Submit (the screen maps it), the rest is no-op.
            _ => {}
        }
    }

    /// A worker's result. A sign-in result whose attempt is stale changes nothing; the installer
    /// is not tied to a sign-in attempt (it outlives a sign-out).
    fn apply(&mut self, attempt: u64, event: Event) -> Vec<Job> {
        let installing = matches!(event, Event::Install(_));
        if !installing && attempt != self.current_attempt() {
            return Vec::new();
        }
        let jobs = match event {
            Event::Resume(Ok(account)) => self.signed_in(account, true),
            Event::Resume(Err(error)) => {
                let stored = match &self.phase {
                    Phase::Resuming { stored } => Some(stored.clone()),
                    _ => None,
                };
                match stored {
                    Some(stored) if no_connection(&error) => {
                        self.phase = Phase::SavedOffline { stored };
                    }
                    _ => {
                        // A refused token is gone for good; no network keeps it for next time.
                        self.pending_forget = true;
                        self.phase = Phase::Choose;
                    }
                }
                Vec::new()
            }
            Event::Code(Ok(code)) => {
                self.phase = Phase::Code { code };
                Vec::new()
            }
            Event::Code(Err(error)) => {
                self.phase = Phase::Choose;
                self.fail(detail(&error), Retry::Microsoft);
                Vec::new()
            }
            Event::SignIn(Ok(account)) => self.signed_in(account, false),
            Event::SignIn(Err(error)) => {
                self.phase = Phase::Choose;
                self.fail(detail(&error), Retry::Microsoft);
                Vec::new()
            }
            Event::Ely(Ok(account)) => self.signed_in(account, false),
            Event::Ely(Err(error)) => {
                if let Phase::Ely(form) = &mut self.phase {
                    if error == NEEDS_2FA {
                        form.otp = Some(String::new());
                        form.focus = ElyField::Otp;
                        form.error = Some(horizon_client::lang::get("horizon.boot.ely.needs_2fa"));
                    } else if error == REFUSED {
                        form.error = Some(horizon_client::lang::get("horizon.boot.ely.refused"));
                    } else {
                        form.error = Some(detail(&error));
                    }
                }
                Vec::new()
            }
            Event::Install(Ok(())) => {
                self.installing = false;
                self.files_ready = true;
                self.failure = None;
                Vec::new()
            }
            Event::Install(Err(error)) => {
                self.installing = false;
                // The installer keeps what it fetched: Retry fetches only the rest.
                self.fail(
                    horizon_client::lang::tr("horizon.boot.files.fetch_failed", &[&error]),
                    Retry::Install,
                );
                Vec::new()
            }
        };
        self.version += 1;
        jobs
    }
}

/// An offline player name: 1-16 letters, digits or `_`, as the core's session parser accepts.
fn valid_offline_name(name: &str) -> bool {
    let len = name.chars().count();
    (1..=16).contains(&len) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

struct Boot {
    ctrl: Controller,
    screen: BootScreen,
    tx: Sender<(u64, Event)>,
    rx: Receiver<(u64, Event)>,
    progress: Arc<InstallProgress>,
    install: Option<Installer>,
    start: Option<Box<dyn FnOnce(Vec<String>)>>,
    secrets: Box<dyn SecretStore + Send>,
    open_link: fn(&str),
    keep_awake: fn(bool),
    game_dir: PathBuf,
    resources: PathBuf,
    session_dir: PathBuf,
    ep: Endpoints,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui: UiResources,
    settings: Settings,
    /// The pointer in physical pixels.
    pointer: (f64, f64),
    clipboard: SystemClipboard,
    awake: bool,
    /// The field the platform's keyboard was shown for (`sync_system_keyboard`); `None` when it
    /// is down, or must be shown again (a tap on a field).
    keyboard_for: Option<KeyboardFor>,
    drawn: u64,
    /// The game's app once it is handed over: every event goes to it from then on.
    app: Option<App>,
    error: Option<Box<dyn std::error::Error>>,
}

impl Boot {
    fn new(config: BootConfig) -> Self {
        // The core's own check: the marker and the top-level files' health.
        let files_ready = loading_screen::missing_resources(&config.resources, None).is_none();
        let ctrl = Controller::new(files_ready, config.install.is_some(), client_id());
        let (tx, rx) = channel();
        // The player's own options, for the graphics API: reading them also switches it after a
        // driver crash at the last start's renderer (`Settings::load`), before the boot window
        // opens a device. The game reads them again after the hand-over.
        let options = config.game_dir.join("options.txt");
        let mut settings = Settings::load(&options);
        // A first start (no options yet) speaks the system's language, and the game keeps it; a
        // kept code the game has no file for (`ru_ua`) becomes the closest one it has.
        let wanted = if options.exists() {
            Some(settings.language.clone())
        } else {
            config.language.clone()
        };
        if let Some(language) = wanted.as_deref().and_then(game_language)
            && (language != settings.language || !options.exists())
        {
            settings.language = language;
            settings.save(&options);
        }
        let mut boot = Self {
            ctrl,
            screen: BootScreen::new(BootState::Choose),
            tx,
            rx,
            progress: Arc::new(InstallProgress::new()),
            install: config.install,
            start: Some(config.start),
            secrets: config.secrets,
            open_link: config.open_link,
            keep_awake: config.keep_awake,
            game_dir: config.game_dir,
            resources: config.resources,
            session_dir: config.session_dir,
            ep: Endpoints::live(),
            window: None,
            renderer: None,
            gui: UiResources {
                atlas: HudAtlas::boot(),
                panorama: Arc::new(horizon_client::gfx::panorama::Faces::colour([0; 4])),
                splashes: Arc::from(""),
            },
            settings,
            pointer: (0.0, 0.0),
            clipboard: SystemClipboard::default(),
            awake: false,
            keyboard_for: None,
            drawn: 0,
            app: None,
            error: None,
        };
        boot.resume_saved();
        boot.resume_pending_code();
        boot
    }

    /// Sign the kept account in again on a thread: success restores it, a refused token forgets
    /// it, no network keeps it to play offline.
    fn resume_saved(&mut self) {
        let json = match self.secrets.load() {
            Ok(json) => json,
            Err(e) => {
                log::warn!("Horizon: the account store could not be read: {e}");
                return;
            }
        };
        let Some(json) = json else {
            return;
        };
        match Account::from_stored_json(&json) {
            Ok(stored) => {
                self.ctrl.begin_attempt();
                self.ctrl.phase = Phase::Resuming {
                    stored: stored.clone(),
                };
                self.run_job(Job::Resume(stored));
            }
            Err(e) => {
                log::warn!("Horizon: the kept account is unreadable: {e}");
                let _ = self.secrets.clear();
            }
        }
    }

    /// With no kept account, a Microsoft code the last start was waiting on and kept
    /// (`PENDING_CODE`): the wait goes on, the code still on the screen.
    fn resume_pending_code(&mut self) {
        let path = self.session_dir.join(PENDING_CODE);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let code = DeviceCode::from_json(&text);
        let Some(code) = code.filter(|_| {
            matches!(self.ctrl.phase, Phase::Choose) && self.ctrl.client_id.is_some()
        }) else {
            let _ = std::fs::remove_file(&path);
            return;
        };
        log::info!("Horizon: waiting again for the Microsoft sign-in the last start began");
        self.ctrl.begin_attempt();
        self.ctrl.phase = Phase::Code { code: code.clone() };
        self.run_job(Job::MicrosoftKept(code));
    }

    fn try_spawn(&self, name: &str, job: impl FnOnce() + Send + 'static) -> bool {
        match std::thread::Builder::new().name(name.to_owned()).spawn(job) {
            Ok(_) => true,
            Err(e) => {
                log::warn!("Horizon: no {name} thread: {e}");
                false
            }
        }
    }

    /// Run a job the controller asked for. The workers send their result back with the current
    /// attempt; a stale one is ignored when it arrives.
    fn run_job(&mut self, job: Job) {
        let attempt = self.ctrl.current_attempt();
        let tx = self.tx.clone();
        match job {
            Job::Start => {} // handled by `start_game`, which needs the event loop
            Job::Resume(stored) => {
                let ep = self.ep.clone();
                if !self.try_spawn("Sign-in resume", move || {
                    let result = refresh_account(&ep, &stored);
                    let _ = tx.send((attempt, Event::Resume(result)));
                }) {
                    let _ = self
                        .tx
                        .send((attempt, Event::Resume(Err("no resume thread".into()))));
                }
            }
            job @ (Job::Microsoft | Job::MicrosoftKept(_)) => {
                let kept = match job {
                    Job::MicrosoftKept(code) => Some(code),
                    _ => None,
                };
                let ep = self.ep.clone();
                let id = self.ctrl.client_id.clone().unwrap_or_default();
                let current = self.ctrl.attempt.clone();
                let pending = self.session_dir.join(PENDING_CODE);
                if !self.try_spawn("Microsoft sign-in", move || {
                    let code = match kept.map_or_else(|| device_code(&ep, &id), Ok) {
                        Ok(code) => code,
                        Err(e) => {
                            let _ = tx.send((attempt, Event::SignIn(Err(e))));
                            return;
                        }
                    };
                    // Kept while the player is at the page; gone once the wait ends either way.
                    if let Err(e) = write_session(&pending, &code.to_json()) {
                        log::warn!("Horizon: the sign-in code is not kept: {e}");
                    }
                    let _ = tx.send((attempt, Event::Code(Ok(code.clone()))));
                    let cancelled = || current.load(Ordering::SeqCst) != attempt;
                    let result = sign_in(&ep, &id, &code, &cancelled);
                    // A newer attempt's code stays: only this one's goes.
                    let theirs = std::fs::read_to_string(&pending)
                        .ok()
                        .and_then(|text| DeviceCode::from_json(&text))
                        .is_some_and(|kept| kept.user_code != code.user_code);
                    if !theirs {
                        let _ = std::fs::remove_file(&pending);
                    }
                    let _ = tx.send((attempt, Event::SignIn(result)));
                }) {
                    let _ = self
                        .tx
                        .send((attempt, Event::SignIn(Err("no sign-in thread".into()))));
                }
            }
            Job::Ely {
                login,
                password,
                otp,
            } => {
                let ep = self.ep.clone();
                if !self.try_spawn("Ely.by sign-in", move || {
                    let result = ely_sign_in(&ep, &login, &password, &otp);
                    let _ = tx.send((attempt, Event::Ely(result)));
                }) {
                    let _ = self
                        .tx
                        .send((attempt, Event::Ely(Err("no sign-in thread".into()))));
                }
            }
            Job::Install => {
                let Some(install) = self.install.clone() else {
                    return;
                };
                let progress = self.progress.clone();
                // A new attempt starts its bar from nothing; the kept files resume by hash.
                progress.reset();
                if !self.try_spawn("Game files", move || {
                    let result = install(&progress);
                    let _ = tx.send((attempt, Event::Install(result)));
                }) {
                    let _ = self
                        .tx
                        .send((attempt, Event::Install(Err("no install thread".into()))));
                }
            }
        }
    }

    /// Keep or forget the account after a transition that asked for it.
    fn drain_side_effects(&mut self) {
        if let Some(json) = self.ctrl.take_save()
            && let Err(e) = self.secrets.save(&json)
        {
            log::warn!("Horizon: the account could not be kept: {e}");
        }
        if self.ctrl.take_forget()
            && let Err(e) = self.secrets.clear()
        {
            log::warn!("Horizon: the account could not be forgotten: {e}");
        }
    }

    fn sync_screen(&mut self) {
        let state = self.ctrl.screen_state(&self.progress);
        if self.screen.state() != &state {
            self.screen.set_state(state);
        }
    }

    fn size(&self) -> [u32; 2] {
        self.renderer
            .as_ref()
            .map_or([1280, 720], |r| [r.config.width, r.config.height])
    }

    /// `f` over a Ui of the window's size, the cursor where the pointer is.
    fn with_ui<R>(
        &self,
        batch: &mut HudBatch,
        f: impl FnOnce(&mut horizon_client::ui_theme::Ui) -> R,
    ) -> R {
        let cursor = (
            self.pointer.0 as f32 / batch.scale,
            self.pointer.1 as f32 / batch.scale,
        );
        let mut ui = horizon_client::ui_theme::Ui::new(&self.gui.atlas, batch, cursor);
        let result = f(&mut ui);
        ui.finish();
        result
    }

    fn draw(&mut self) {
        let mut batch = HudBatch::new(self.size(), self.settings.gui_scale_inputs());
        self.with_ui(&mut batch, |ui| self.screen.draw(ui));
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        match renderer.render(&loading_frame(&batch)) {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => log::warn!("Horizon: boot frame failed: {e}"),
        }
        // The frame was not shown: its rasterized glyphs never reached the atlas, so draw again.
        horizon_client::ui_theme::reset_atlas();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn click(&mut self, event_loop: &ActiveEventLoop) {
        // The layout the last draw made: a tap reads the same boxes the finger saw.
        let Some(action) = self
            .screen
            .click((self.pointer.0 as f32, self.pointer.1 as f32))
        else {
            return;
        };
        match action {
            BootAction::OpenLink => {
                if let Phase::Code { code } = &self.ctrl.phase {
                    (self.open_link)(&code.verification_uri);
                }
            }
            BootAction::CopyCode => {
                if let Phase::Code { code } = &self.ctrl.phase {
                    self.clipboard.set_text(&code.user_code);
                }
            }
            other => {
                // A tap on a field brings the platform's keyboard back up, even for the field it
                // was last shown for (the player may have closed it).
                if matches!(other, BootAction::FocusField(_)) {
                    self.keyboard_for = None;
                }
                self.act(event_loop, other);
            }
        }
    }

    /// A screen action through the controller, and the jobs it names.
    fn act(&mut self, event_loop: &ActiveEventLoop, action: BootAction) {
        let jobs = self.ctrl.action(action);
        self.drain_side_effects();
        for job in jobs {
            if matches!(job, Job::Start) {
                self.start_game(event_loop);
            } else {
                self.run_job(job);
            }
        }
    }

    /// The platform's own keyboard (`system_keyboard`), where the host gave one: up over the
    /// field that has focus, with that field's text, and what it typed into the field. Enter
    /// sends the form.
    fn sync_system_keyboard(&mut self, event_loop: &ActiveEventLoop) {
        if !horizon_client::system_keyboard::platform_keyboard() || self.app.is_some() {
            return;
        }
        use horizon_client::system_keyboard::Mode;
        let wanted = match &self.ctrl.phase {
            Phase::Ely(form) => Some(match form.focus {
                ElyField::Login => (KeyboardFor::Ely(form.focus), form.login.clone(), Mode::Literal),
                ElyField::Password => (
                    KeyboardFor::Ely(form.focus),
                    form.password.clone(),
                    Mode::Secret,
                ),
                ElyField::Otp => (
                    KeyboardFor::Ely(form.focus),
                    form.otp.clone().unwrap_or_default(),
                    Mode::Literal,
                ),
            }),
            Phase::OfflineName { name, .. } => {
                Some((KeyboardFor::Offline, name.clone(), Mode::Literal))
            }
            _ => None,
        };
        match wanted {
            Some((field, text, mode)) => {
                if self.keyboard_for != Some(field) {
                    horizon_client::system_keyboard::show(&text, mode);
                    self.keyboard_for = Some(field);
                }
            }
            None => {
                if self.keyboard_for.take().is_some() {
                    horizon_client::system_keyboard::hide();
                }
                return;
            }
        }
        for edit in horizon_client::system_keyboard::poll(false) {
            use horizon_client::system_keyboard::Edit;
            let action = match edit {
                Edit::Text(text) => BootAction::Typed(Output::Text(text)),
                Edit::Backspace => BootAction::Typed(Output::Backspace),
                Edit::Enter => BootAction::Submit,
            };
            self.act(event_loop, action);
        }
    }

    /// Start the game: write the fresh session file, close this screen's window and renderer (the
    /// game makes its own in `resumed`), hand the sink over, run the host's `start`, then take the
    /// app and run it on this loop.
    fn start_game(&mut self, event_loop: &ActiveEventLoop) {
        let args = match &self.ctrl.phase {
            Phase::Signed { account, .. } => {
                let path = self.session_dir.join("session.json");
                match write_session(&path, &account.session_json()) {
                    Ok(()) => {
                        game_args(&self.game_dir, &self.resources, Some(path.as_path()), None)
                    }
                    Err(e) => {
                        self.error = Some(format!("session file {}: {e}", path.display()).into());
                        event_loop.exit();
                        return;
                    }
                }
            }
            Phase::SavedOffline { stored } => game_args(
                &self.game_dir,
                &self.resources,
                None,
                Some(stored.name.as_str()),
            ),
            Phase::Offline { name } => {
                game_args(&self.game_dir, &self.resources, None, Some(name.as_str()))
            }
            _ => return,
        };
        // The game opens its own window: this one goes first, the renderer before the window it
        // holds, so the surface and the device are freed before the game builds its own.
        self.renderer = None;
        self.window = None;
        self.awake = false;
        (self.keep_awake)(false);
        set_app_sink(hand_over);
        let Some(start) = self.start.take() else {
            return;
        };
        start(args);
        let app = HANDED.with(|handed| handed.borrow_mut().take());
        match app {
            Some(mut app) => {
                // The loop's own `resumed` came before there was an app to hear it.
                app.resumed(event_loop);
                self.app = Some(app);
                event_loop.set_control_flow(ControlFlow::Poll);
            }
            None => {
                self.error = Some("the game did not start (are its resources in place?)".into());
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for Boot {
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            return app.suspended(event_loop);
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.suspend_surface();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            return app.resumed(event_loop);
        }
        // Back from the background the platform's keyboard is down: shown again for the field.
        self.keyboard_for = None;
        if let Some(window) = &self.window {
            // Back from the background: the window stayed, its surface is made again, and the
            // screen drawn on it at once (a sign-in may have finished meanwhile).
            window.request_redraw();
            if let Some(renderer) = self.renderer.as_mut().filter(|r| r.suspended) {
                if let Err(error) = renderer.recreate_surface() {
                    log::error!("The window's surface could not be made again: {error}");
                } else {
                    let size = window.inner_size();
                    renderer.resize(size.width, size.height);
                }
            }
            return;
        }
        let attributes =
            horizon_client::window::with_icon(Window::default_attributes().with_title("Horizon"))
                .with_inner_size(winit::dpi::LogicalSize::new(1040.0, 700.0));
        let made = event_loop
            .create_window(attributes)
            .map_err(Into::into)
            .and_then(|window| {
                let window = Arc::new(window);
                let renderer = loading_renderer(
                    window.clone(),
                    false,
                    Some(&self.gui.atlas),
                    Some(&self.gui.panorama),
                    None,
                    self.settings.horizon.api,
                    false,
                )?;
                Ok((window, renderer))
            });
        match made {
            Ok((window, renderer)) => {
                window.request_redraw();
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(app) = &mut self.app {
            return app.window_event(event_loop, id, event);
        }
        let Some(window) = self.window.clone().filter(|w| w.id() == id) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x, position.y);
                window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.click(event_loop),
            WindowEvent::Touch(touch) => {
                self.pointer = (touch.location.x, touch.location.y);
                window.request_redraw();
                if touch.phase == TouchPhase::Ended {
                    self.click(event_loop);
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape) =>
            {
                event_loop.exit()
            }
            _ => {}
        }
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        if let Some(app) = &mut self.app {
            app.device_event(event_loop, id, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            return app.about_to_wait(event_loop);
        }
        if horizon_threads::stop_requests() > 0 {
            event_loop.exit();
            return;
        }
        self.sync_system_keyboard(event_loop);
        while let Ok((attempt, event)) = self.rx.try_recv() {
            let jobs = self.ctrl.apply(attempt, event);
            self.drain_side_effects();
            for job in jobs {
                if matches!(job, Job::Start) {
                    self.start_game(event_loop);
                } else {
                    self.run_job(job);
                }
            }
        }
        // Keep the screen on while the files download.
        let want_awake = self.ctrl.installing;
        if want_awake != self.awake {
            self.awake = want_awake;
            (self.keep_awake)(want_awake);
        }
        // A restored sign-in or a confirmed offline name, with the files here: start at once.
        if self.ctrl.ready_to_start() {
            self.start_game(event_loop);
            if self.app.is_some() {
                return;
            }
        }
        self.sync_screen();
        if self.ctrl.version != self.drawn || self.ctrl.installing {
            self.drawn = self.ctrl.version;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(100),
        ));
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.exiting(event_loop);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemStore(Mutex<Option<String>>);

    impl SecretStore for MemStore {
        fn load(&self) -> io::Result<Option<String>> {
            Ok(self.0.lock().unwrap_or_else(|e| e.into_inner()).clone())
        }
        fn save(&self, json: &str) -> io::Result<()> {
            *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(json.to_owned());
            Ok(())
        }
        fn clear(&self) -> io::Result<()> {
            *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
            Ok(())
        }
    }

    fn ctrl(files_ready: bool, has_installer: bool) -> Controller {
        Controller::new(files_ready, has_installer, Some("test-client".into()))
    }

    fn account() -> Account {
        Account::new(
            Kind::Microsoft,
            "Steve",
            "00000000000000000000000000000001",
            "access",
            "refresh",
            u64::MAX,
        )
    }

    fn stored() -> Stored {
        Stored {
            kind: Kind::Ely,
            name: "Steve".into(),
            uuid: "00000000000000000000000000000001".into(),
            refresh_secret: "refresh".into(),
            client_token: "client".into(),
        }
    }

    /// The current attempt, for applying a result that belongs to it.
    fn attempt(ctrl: &mut Controller) -> u64 {
        ctrl.begin_attempt()
    }

    #[test]
    fn microsoft_asks_for_a_code_or_explains_it_cannot() {
        let mut c = ctrl(true, false);
        let jobs = c.action(BootAction::SignInMicrosoft);
        assert!(matches!(c.phase, Phase::Asking));
        assert!(matches!(jobs.as_slice(), [Job::Microsoft]));
        // No Azure app configured: the screen explains, no thread starts.
        let mut none = Controller::new(true, false, None);
        let jobs = none.action(BootAction::SignInMicrosoft);
        assert!(jobs.is_empty());
        assert!(matches!(none.phase, Phase::Choose));
        assert_eq!(none.failure.as_ref().unwrap().retry, Retry::Choose);
    }

    #[test]
    fn a_sign_in_keeps_the_account_and_downloads_missing_files() {
        let mut c = ctrl(false, true);
        let a = attempt(&mut c);
        let jobs = c.apply(a, Event::SignIn(Ok(account())));
        assert!(matches!(
            c.phase,
            Phase::Signed {
                restored: false,
                ..
            }
        ));
        assert!(c.pending_save.is_some(), "the refresh secret is kept");
        assert!(c.installing);
        assert!(matches!(jobs.as_slice(), [Job::Install]));
        // With the files already there, no download starts.
        let mut ready = ctrl(true, true);
        let a = attempt(&mut ready);
        let jobs = ready.apply(a, Event::SignIn(Ok(account())));
        assert!(jobs.is_empty());
        assert!(!ready.installing);
    }

    #[test]
    fn a_stale_attempt_changes_nothing() {
        let mut c = ctrl(true, false);
        let old = attempt(&mut c);
        let _new = attempt(&mut c);
        let jobs = c.apply(old, Event::SignIn(Ok(account())));
        assert!(jobs.is_empty());
        assert!(
            matches!(c.phase, Phase::Choose),
            "the stale sign-in was ignored"
        );
        assert!(c.pending_save.is_none());
    }

    #[test]
    fn resume_restores_forgets_or_keeps_offline() {
        // Success: a restored sign-in.
        let mut c = ctrl(true, false);
        let a = attempt(&mut c);
        c.phase = Phase::Resuming { stored: stored() };
        c.apply(a, Event::Resume(Ok(account())));
        assert!(matches!(c.phase, Phase::Signed { restored: true, .. }));
        assert!(
            c.ready_to_start(),
            "restored with the files here: start at once"
        );
        // A refused token is forgotten.
        let mut refused = ctrl(true, false);
        let a = attempt(&mut refused);
        refused.phase = Phase::Resuming { stored: stored() };
        refused.apply(a, Event::Resume(Err("Ely.by sign-in was refused.".into())));
        assert!(matches!(refused.phase, Phase::Choose));
        assert!(refused.pending_forget);
        // No network keeps the account, played offline until it returns.
        let mut offline = ctrl(true, false);
        let a = attempt(&mut offline);
        offline.phase = Phase::Resuming { stored: stored() };
        offline.apply(
            a,
            Event::Resume(Err(
                "Could not reach Ely.by. Check your internet connection.".into(),
            )),
        );
        assert!(matches!(offline.phase, Phase::SavedOffline { .. }));
        assert!(!offline.pending_forget);
        let state = offline.screen_state(&InstallProgress::new());
        assert!(matches!(state, BootState::Signed { note: Some(_), .. }));
    }

    #[test]
    fn typing_edits_the_focused_field() {
        let mut c = ctrl(true, false);
        c.action(BootAction::SignInEly);
        c.action(BootAction::Typed(Output::Text("me".into())));
        c.action(BootAction::FocusField(ElyField::Password));
        c.action(BootAction::Typed(Output::Text("pw".into())));
        c.action(BootAction::Typed(Output::Backspace));
        let Phase::Ely(form) = &c.phase else {
            panic!("the form is open");
        };
        assert_eq!(form.login, "me");
        assert_eq!(form.password, "p");
        assert_eq!(form.focus, ElyField::Password);
    }

    #[test]
    fn the_system_language_becomes_the_closest_game_language() {
        assert_eq!(game_language("ru-UA").as_deref(), Some("ru_ru"));
        assert_eq!(game_language("uk_UA").as_deref(), Some("uk_ua"));
        assert_eq!(game_language("fr_BE").as_deref(), Some("fr_fr"));
        assert_eq!(game_language("en_US").as_deref(), Some("en_us"));
        assert_eq!(game_language("xx_YY"), None);
    }

    #[test]
    fn ely_two_factor_opens_the_code_field_and_refusal_stays_on_the_form() {
        let mut c = ctrl(true, false);
        c.action(BootAction::SignInEly);
        let a = attempt(&mut c);
        c.apply(a, Event::Ely(Err(NEEDS_2FA.into())));
        let Phase::Ely(form) = &c.phase else {
            panic!("the form stays");
        };
        assert_eq!(form.otp.as_deref(), Some(""));
        assert_eq!(form.focus, ElyField::Otp);
        assert!(form.error.is_some());
        // A refusal keeps the form's values for another try.
        let a = attempt(&mut c);
        c.apply(a, Event::Ely(Err(REFUSED.into())));
        assert!(matches!(c.phase, Phase::Ely(_)));
        // Success signs in.
        let a = attempt(&mut c);
        c.apply(a, Event::Ely(Ok(account())));
        assert!(matches!(c.phase, Phase::Signed { .. }));
    }

    #[test]
    fn the_offline_name_is_validated_then_plays() {
        let mut c = ctrl(true, false);
        c.action(BootAction::PlayOffline);
        assert!(matches!(c.phase, Phase::OfflineName { .. }));
        c.action(BootAction::Typed(Output::Text("Bad Name!".into())));
        c.action(BootAction::Submit);
        assert!(matches!(c.phase, Phase::OfflineName { error: Some(_), .. }));
        // Clear it and type a valid name.
        if let Phase::OfflineName { name, .. } = &mut c.phase {
            name.clear();
        }
        c.action(BootAction::Typed(Output::Text("Steve_2".into())));
        c.action(BootAction::Submit);
        assert!(matches!(c.phase, Phase::Offline { .. }));
        assert!(c.ready_to_start());
        // Without the files, offline is unavailable and the screen explains.
        let mut no_files = ctrl(false, false);
        no_files.action(BootAction::PlayOffline);
        assert!(matches!(no_files.phase, Phase::Choose));
        assert_eq!(no_files.failure.as_ref().unwrap().retry, Retry::Choose);
    }

    #[test]
    fn install_failure_retries_in_place_then_the_files_are_ready() {
        let mut c = ctrl(false, true);
        let a = attempt(&mut c);
        c.apply(a, Event::SignIn(Ok(account())));
        assert!(c.installing);
        let a = c.current_attempt();
        c.apply(a, Event::Install(Err("No space left on the device".into())));
        assert!(!c.installing);
        assert_eq!(c.failure.as_ref().unwrap().retry, Retry::Install);
        // Retry runs the installer again in this run: nothing about a restart.
        let jobs = c.action(BootAction::Retry);
        assert!(matches!(jobs.as_slice(), [Job::Install]));
        assert!(c.installing);
        assert!(c.failure.is_none());
        // Then the same installer succeeds and the files are ready.
        c.apply(c.current_attempt(), Event::Install(Ok(())));
        assert!(c.files_ready);
        assert!(!c.installing);
        // A build with no installer cannot retry a download at all.
        let mut none = ctrl(false, false);
        let a = attempt(&mut none);
        none.apply(a, Event::SignIn(Ok(account())));
        assert_eq!(none.failure.as_ref().unwrap().retry, Retry::Choose);
        let jobs = none.action(BootAction::Retry);
        assert!(jobs.is_empty());
        assert!(!none.installing);
    }

    #[test]
    fn continue_starts_when_ready_and_downloads_when_missing() {
        let mut ready = ctrl(true, false);
        let a = attempt(&mut ready);
        ready.apply(a, Event::SignIn(Ok(account())));
        let jobs = ready.action(BootAction::Continue);
        assert!(matches!(jobs.as_slice(), [Job::Start]));
        let mut missing = ctrl(false, true);
        let a = attempt(&mut missing);
        missing.apply(a, Event::SignIn(Ok(account())));
        missing.apply(
            missing.current_attempt(),
            Event::Install(Err("dropped".into())),
        );
        // Continue with the files still missing: the download starts again, in place.
        let jobs = missing.action(BootAction::Continue);
        assert!(matches!(jobs.as_slice(), [Job::Install]));
        assert!(missing.installing);
    }

    #[test]
    fn sign_out_forgets_the_account() {
        let mut c = ctrl(true, false);
        let a = attempt(&mut c);
        c.apply(a, Event::SignIn(Ok(account())));
        c.action(BootAction::SignOut);
        assert!(matches!(c.phase, Phase::Choose));
        assert!(c.take_forget());
    }

    #[test]
    fn the_command_line_carries_the_folders_and_the_session_or_name() {
        let dir = Path::new("/game");
        let res = Path::new("/game/resources");
        let session = Path::new("/game/session.json");
        assert_eq!(
            game_args(dir, res, Some(session), None),
            vec![
                "--game-dir",
                "/game",
                "--resources",
                "/game/resources",
                "--session-file",
                "/game/session.json",
            ]
        );
        assert_eq!(
            game_args(dir, res, None, Some("Steve")),
            vec![
                "--game-dir",
                "/game",
                "--resources",
                "/game/resources",
                "--name",
                "Steve"
            ]
        );
    }

    #[test]
    fn the_session_file_is_private_and_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("session.json");
        let json = account().session_json();
        write_session(&path, &json).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
        let meta = std::fs::metadata(&path).unwrap();
        let age = meta.modified().unwrap().elapsed().unwrap();
        assert!(
            age < std::time::Duration::from_secs(120),
            "fresh for the core"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(meta.permissions().mode() & 0o777, 0o600);
            // An existing file with loose bits is tightened too.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            write_session(&path, &json).unwrap();
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn only_a_transport_error_counts_as_no_connection() {
        assert!(no_connection(
            "Could not reach Microsoft. Check your internet connection."
        ));
        assert!(!no_connection(
            "Your Microsoft sign-in expired. Sign in again."
        ));
        assert!(!no_connection(REFUSED));
    }

    #[test]
    fn the_new_states_say_their_strings_in_both_languages() {
        let keys = [
            "horizon.boot.resuming.title",
            "horizon.boot.resuming.line",
            "horizon.boot.signed.offline_note",
            "horizon.boot.offline.name.title",
            "horizon.boot.offline.name.about",
            "horizon.boot.offline.name.field",
            "horizon.boot.offline.name.play",
            "horizon.boot.offline.name.invalid",
            "horizon.boot.offline.need_files",
            "horizon.boot.ely.needs_2fa",
            "horizon.boot.ely.refused",
            "horizon.boot.service_said",
            "horizon.boot.files.no_installer",
            "horizon.boot.files.fetch_failed",
            "horizon.boot.microsoft.not_configured",
        ];
        for code in ["en_us", "ru_ru"] {
            let (said, missed) = horizon_client::lang::scoped(
                horizon_client::lang::Language::load(&[], code),
                || {
                    keys.iter()
                        .map(|k| horizon_client::lang::get(k))
                        .collect::<Vec<_>>()
                },
            );
            assert_eq!(missed, Vec::<String>::new(), "{code}");
            for text in &said {
                assert!(
                    !text.starts_with("horizon.boot."),
                    "{code}: {text} is a key"
                );
            }
        }
    }

    #[test]
    fn the_screen_state_follows_the_phase_and_the_download() {
        let mut c = ctrl(false, true);
        assert_eq!(c.screen_state(&InstallProgress::new()), BootState::Choose);
        let a = attempt(&mut c);
        c.apply(a, Event::SignIn(Ok(account())));
        // The download covers the account while it runs.
        let progress = InstallProgress::new();
        progress.done.store(10, Ordering::Relaxed);
        progress.total.store(100, Ordering::Relaxed);
        progress.set_step("horizon.boot.files.step.client");
        assert!(matches!(
            c.screen_state(&progress),
            BootState::Files {
                done_bytes: 10,
                total_bytes: 100,
                ..
            }
        ));
        c.apply(c.current_attempt(), Event::Install(Ok(())));
        assert!(matches!(
            c.screen_state(&progress),
            BootState::Signed { note: None, .. }
        ));
    }
}
