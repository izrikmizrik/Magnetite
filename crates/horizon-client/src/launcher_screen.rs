//! The phone launcher (docs/unification/launcher.md): the screen between signing in and the game,
//! portrait, in the design system's glass look (black, greys, white). Drawn on the boot screen's
//! window and atlas (`game/runtime/boot.rs`), with the boot screen's way of working: one layout for
//! drawing and for taps, kept from the last draw, and plain actions out that the boot loop acts on.
//!
//! What it shows comes in as a [`View`]; what a tap asks for goes out as an [`Action`]. It holds no
//! settings of its own: their values are the game's files', read and written by the boot loop.
use crate::boot_screen::{BootAction, Content, ElyField, Rect, wrap};
use crate::lang;
use crate::ui_theme::{Face, Ui, color};
use std::cell::{Cell, RefCell};

/// The column the launcher lays out for, in dp: a phone held upright, as its own apps are drawn.
const COLUMN_DP: f32 = 420.0;
/// The side margin and the gap between plates.
const MARGIN: f32 = 16.0;
const GAP: f32 = 8.0;
/// A row's and a button's height.
const ROW: f32 = 56.0;
/// The status bar and the notch at the top (the window draws under them on a phone).
const TOP: f32 = 36.0;
/// A wide window (a PC's): the sidebar's width, and the widest the page column grows.
const SIDEBAR: f32 = 232.0;
const CONTENT_MAX: f32 = 760.0;
/// The narrowest window, in dp, laid out wide.
const WIDE_MIN: f32 = 640.0;

/// Whether `ui` lays out wide: a window wider than tall and wide enough for the sidebar.
fn wide(ui: &Ui) -> bool {
    ui.w > ui.h && ui.w >= WIDE_MIN
}

/// The page column: the screen less its margins on a phone, right of the sidebar on a PC. `insets`
/// (left, top, right, bottom, in dp) are the system's own over the window: iOS's status bar,
/// Dynamic Island and home indicator, where the window is the whole screen; none on a PC.
fn area_for(ui: &Ui, insets: [f32; 4]) -> Rect {
    let [left, top, right, bottom] = insets;
    if wide(ui) {
        let x = left + SIDEBAR + 28.0;
        let y = top + 20.0;
        Rect::new(
            x,
            y,
            (ui.w - x - 28.0 - right).min(CONTENT_MAX),
            ui.h - y - 20.0 - bottom,
        )
    } else {
        let x = MARGIN.max(left);
        let y = TOP.max(top + 8.0);
        Rect::new(
            x,
            y,
            ui.w - x - MARGIN.max(right),
            ui.h - y - MARGIN.max(bottom),
        )
    }
}

/// The launcher's pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Settings,
    /// The kept accounts: pick the one that plays, remove one, add one.
    Accounts,
    /// The profiles: pick the one that plays, make a new one.
    Profiles,
    /// The current profile: its name, removing it.
    EditProfile,
    /// A setting's values, the chosen one checked.
    Pick(Setting),
    /// The installed mods, each turned on or off.
    Mods,
    /// Magnetite's mod platform: what it offers, installing one.
    BrowseMods,
    /// One installed mod: what it is, on or off, its settings, removing it.
    Mod,
    /// The app's version and the newest one released.
    Updates,
    /// What the game keeps on the disk, and clearing what it can make again.
    Data,
    /// The version, the licences and the project's pages.
    About,
}

/// A setting the settings page lists, by what it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Accounts,
    GraphicsApi,
    Resolution,
    FpsLimit,
    Memory,
    Sensitivity,
    Gyro,
    GyroSensitivity,
    Vibration,
    Cutout,
    Language,
    Updates,
    Data,
    About,
    Jit,
}

impl Setting {
    fn label_key(self) -> &'static str {
        match self {
            Self::Accounts => "horizon.launcher.settings.accounts",
            Self::GraphicsApi => "horizon.launcher.settings.graphics_api",
            Self::Resolution => "horizon.launcher.settings.resolution",
            Self::FpsLimit => "horizon.launcher.settings.fps",
            Self::Memory => "horizon.launcher.settings.memory",
            Self::Sensitivity => "horizon.launcher.settings.sensitivity",
            Self::Gyro => "horizon.launcher.settings.gyro",
            Self::GyroSensitivity => "horizon.launcher.settings.gyro_sensitivity",
            Self::Vibration => "horizon.launcher.settings.vibration",
            Self::Cutout => "horizon.launcher.settings.cutout",
            Self::Language => "horizon.launcher.settings.language",
            Self::Updates => "horizon.launcher.settings.updates",
            Self::Data => "horizon.launcher.settings.data",
            Self::About => "horizon.launcher.settings.about",
            Self::Jit => "horizon.launcher.settings.jit",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Accounts => "launcher/accounts",
            Self::GraphicsApi => "launcher/graphics",
            Self::Resolution => "launcher/resolution",
            Self::FpsLimit => "launcher/fps",
            Self::Memory => "launcher/profile",
            Self::Sensitivity | Self::Gyro | Self::GyroSensitivity | Self::Vibration => {
                "launcher/controls"
            }
            Self::Cutout => "launcher/resolution",
            Self::Language => "launcher/language",
            Self::Updates => "launcher/refresh",
            Self::Data => "launcher/delete",
            Self::About => "launcher/info",
            Self::Jit => "launcher/fps",
        }
    }
}

/// The settings page's sections, in order, with the settings each lists.
const SECTIONS: &[(&str, &[Setting])] = &[
    (
        "horizon.launcher.settings.section.accounts",
        &[Setting::Accounts],
    ),
    (
        "horizon.launcher.settings.section.graphics",
        &[
            Setting::GraphicsApi,
            Setting::Resolution,
            Setting::FpsLimit,
            Setting::Memory,
            Setting::Jit,
        ],
    ),
    (
        "horizon.launcher.settings.section.controls",
        &[
            Setting::Sensitivity,
            Setting::Gyro,
            Setting::GyroSensitivity,
            Setting::Vibration,
            Setting::Cutout,
        ],
    ),
    (
        "horizon.launcher.settings.section.app",
        &[
            Setting::Language,
            Setting::Updates,
            Setting::Data,
            Setting::About,
        ],
    ),
];

/// What a tap asks the boot loop for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// A place that does nothing.
    None,
    Play,
    Open(Page),
    Back,
    /// Sign another account in (the header's people button).
    AddAccount,
    /// Make the kept account at this index the one that plays.
    PickAccount(usize),
    /// Remove the kept account at this index (asked twice: the first tap asks to confirm).
    RemoveAccount(usize),
    /// A sign-in page's own control, for the boot loop's controller.
    Boot(BootAction),
    /// Leave the sign-in pages without signing in.
    FlowBack,
    Mods,
    Controls,
    ShareLog,
    OpenFolder,
    /// Make the profile at this index the one that plays.
    PickProfile(usize),
    /// A new profile, from the current one's settings.
    NewProfile,
    /// Type the current profile's name (the system keyboard).
    RenameProfile,
    /// Remove the current profile (asked twice).
    RemoveProfile,
    Setting(Setting),
    /// The value at this index on the pick page.
    Choose(usize),
    /// Turn the installed mod at this index on or off.
    ToggleMod(usize),
    /// Install (or update) the platform's mod at this index.
    InstallMod(usize),
    /// Pick a mod's file on the device and install it.
    ImportMod,
    /// The page of the installed mod at this index.
    OpenMod(usize),
    /// Turn the open mod on or off.
    ToggleOpenMod,
    /// Remove the open mod (asked twice).
    RemoveMod,
    /// The open mod's setting at this index one step on (`true`) or back.
    ModStep(usize, bool),
    /// Look for a newer release.
    CheckUpdate,
    /// Clear one of the kept things (asked twice).
    Clear(Store),
    /// Check the game's files and download what is missing.
    CheckFiles,
    /// A page in the system's browser.
    Link(String),
}

/// What the Data page clears: each is made again when it is needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Store {
    /// Old logs and crash reports.
    Logs,
    /// Prepared resources, compiled mods and the like.
    Cache,
    /// What is left of the game files' download.
    Downloads,
}

/// A row of the Updates, Data and About pages: its text, what stands on its right, and what a tap
/// does.
#[derive(Clone, Debug, PartialEq)]
pub struct InfoLine {
    pub icon: &'static str,
    pub label: String,
    pub value: String,
    pub action: Option<Action>,
}

/// The open mod's page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModPage {
    pub name: String,
    /// "1.2 · WebAssembly · Author".
    pub detail: String,
    pub description: String,
    pub enabled: bool,
    pub settings: Vec<ModSettingLine>,
    /// Said where the settings would be, when the mod has none to show.
    pub settings_note: String,
    pub homepage: Option<String>,
    pub confirm_remove: bool,
}

/// One of a mod's settings: its label, its value, and how it changes.
#[derive(Clone, Debug, PartialEq)]
pub struct ModSettingLine {
    pub label: String,
    pub value: String,
    pub kind: ModSettingKind,
}

/// How a mod's setting is drawn and changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModSettingKind {
    /// A switch.
    Toggle(bool),
    /// A minus and a plus around the value.
    Step,
    /// A minus and a plus around a patch of this colour (`0xRRGGBB`).
    Color(u32),
}

/// An installed mod's row: its name, its line under it ("1.2 · WebAssembly"), and whether it is
/// on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModLine {
    pub name: String,
    pub detail: String,
    pub enabled: bool,
}

/// A platform mod's row: its name, its summary, and its button ("Install", "Installed"...), which
/// taps only when it installs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrowseLine {
    pub name: String,
    pub summary: String,
    pub button: String,
    pub can_install: bool,
}

/// What the launcher shows: the boot loop fills it from the account, the profile and the game's
/// settings files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct View {
    /// The account's name and kind (Microsoft, Ely.by, offline), already in the player's language.
    pub name: String,
    pub account_kind: String,
    /// The profile's name and its line under it ("26.3 · 3 mods").
    pub profile: String,
    pub profile_detail: String,
    /// iOS: the signature row, already worded ("Re-sign in 5 d 3 h"), and whether that is under
    /// a day. `None` where nothing expires (Android, TrollStore).
    pub resign: Option<(String, bool)>,
    /// Each setting's value as shown; a setting left out is not listed (gyro only on Android,
    /// JIT only on iOS).
    pub values: Vec<(Setting, String)>,
    /// The settings drawn as a switch, and whether each is on.
    pub switches: Vec<(Setting, bool)>,
    /// Whether the touch layout is edited here (a phone; a PC has no touch controls to lay out).
    pub touch_layout: bool,
    /// The open mod's page, and the rows and text of the Updates, Data and About pages.
    pub mod_page: ModPage,
    pub info: Vec<InfoLine>,
    pub info_text: Vec<String>,
    /// Whether an account (or an offline name) plays: without one the header asks to sign in.
    pub signed_in: bool,
    /// The kept accounts (name, kind), the one that plays, and the one whose removal waits for a
    /// second tap.
    pub accounts: Vec<(String, String)>,
    pub current: Option<usize>,
    pub confirm_remove: Option<usize>,
    /// The profiles' names, the one that plays, and the edit page's state: the name as typed so
    /// far, whether it is being typed, whether the profile can be removed (not the main one) and
    /// whether its removal waits for a second tap.
    pub profiles: Vec<String>,
    pub profile_current: usize,
    pub profile_name: String,
    pub renaming: bool,
    pub profile_removable: bool,
    pub confirm_remove_profile: bool,
    /// The app's version, and the game files' line on a wide home ("Minecraft 26.3 · ready").
    pub version: String,
    pub files_line: String,
    /// The pick page's values as shown, and the chosen one.
    pub choices: Vec<String>,
    pub chosen: Option<usize>,
    /// The Mods page's rows, the platform's rows, and the platform page's note (loading, a
    /// failure, nothing there yet).
    pub mods: Vec<ModLine>,
    pub browse: Vec<BrowseLine>,
    pub browse_note: Option<String>,
}

/// A sign-in page (the boot screen's content), drawn in the launcher's look.
pub struct Flow {
    pub content: Content,
    pub focus: Option<ElyField>,
    /// A back arrow at the top (the method choice, when an account is kept to go back to).
    pub back: bool,
}

/// Where everything was drawn, in dp, with the action each place asks for.
#[derive(Clone, Debug, Default)]
struct Layout {
    density: f32,
    targets: Vec<(Action, Rect)>,
}

/// The launcher: its page, what it shows, and the layout the last draw made for taps.
pub struct LauncherScreen {
    page: Page,
    view: View,
    /// A short note over the foot ("Coming soon", "No log yet"), while the boot loop shows it.
    notice: Option<String>,
    /// A sign-in page over the pages, while the boot loop has one.
    flow: Option<Flow>,
    frame: RefCell<Layout>,
    /// How far the page is scrolled, in dp, and how far it can be (from the last draw).
    scroll: Cell<f32>,
    max_scroll: Cell<f32>,
    /// The page column the last draw laid out (`area_for`).
    area: Cell<Rect>,
    /// The system's insets over the window, in physical pixels (left, top, right, bottom).
    insets: Cell<[f32; 4]>,
}

impl LauncherScreen {
    pub fn new(view: View) -> Self {
        Self {
            page: Page::Home,
            view,
            notice: None,
            flow: None,
            frame: RefCell::new(Layout::default()),
            scroll: Cell::new(0.0),
            max_scroll: Cell::new(0.0),
            area: Cell::new(Rect::default()),
            insets: Cell::new([0.0; 4]),
        }
    }

    /// The system's insets over the window (left, top, right, bottom), in physical pixels: the
    /// safe area's edges where the window is the whole screen (iOS), zero elsewhere.
    pub fn set_insets(&self, insets: [f32; 4]) {
        self.insets.set(insets);
    }
    pub fn page(&self) -> Page {
        self.page
    }
    pub fn set_page(&mut self, page: Page) {
        if page != self.page {
            self.scroll.set(0.0);
        }
        self.page = page;
    }

    /// Scrolls the page by a finger's move of `dy` physical pixels (down: toward the top).
    pub fn scroll_by(&self, dy: f32) {
        let density = self.frame.borrow().density;
        if density <= 0.0 {
            return;
        }
        let to = (self.scroll.get() - dy / density).clamp(0.0, self.max_scroll.get());
        self.scroll.set(to);
    }

    /// Where a scrolled page's content starts.
    fn content_top(&self) -> f32 {
        self.area.get().y + 72.0 - self.scroll.get()
    }
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }
    pub fn view(&self) -> &View {
        &self.view
    }
    pub fn set_notice(&mut self, notice: Option<String>) {
        self.notice = notice;
    }
    pub fn set_flow(&mut self, flow: Option<Flow>) {
        self.flow = flow;
    }

    /// The physical pixels per dp for a window `width` by `height` physical pixels with the system's
    /// `scale`: a phone held upright gets about its own density (a 1080-pixel-wide phone, about
    /// 2.6), a PC's wide window the system's own scale; never less than one.
    pub fn density_for(width: f32, height: f32, scale: f32) -> f32 {
        let scale = scale.max(1.0);
        if width > height && width / scale >= WIDE_MIN {
            scale
        } else {
            (width / COLUMN_DP).max(1.0)
        }
    }

    /// The action at `pos`, in the window's physical pixels, from the last draw.
    pub fn click(&self, pos: (f32, f32)) -> Option<Action> {
        let frame = self.frame.borrow();
        if frame.density <= 0.0 {
            return None;
        }
        let at = (pos.0 / frame.density, pos.1 / frame.density);
        frame
            .targets
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .map(|(action, _)| action.clone())
    }

    /// Draws the page on `ui` (already at the launcher's density, `Ui::set_density`).
    pub fn draw(&self, ui: &mut Ui) {
        let mut targets = Vec::new();
        ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, color::bg());
        let wide = wide(ui);
        let insets = self.insets.get().map(|px| px / ui.density);
        self.area.set(area_for(ui, insets));
        // A wide window lists the top pages in its sidebar: they have no back arrow.
        let top = |back: Action| (!wide).then_some(back);
        // A page with a bar scrolls under it: its content first, then the bar over it, whose
        // targets come first so a row scrolled under the bar never takes its taps.
        let bar = match (&self.flow, self.page) {
            (Some(flow), _) => {
                draw_flow(ui, self.area.get(), flow, &mut targets);
                None
            }
            (None, Page::Home) if wide => {
                self.draw_home_wide(ui, &mut targets);
                None
            }
            (None, Page::Home) => {
                self.draw_home(ui, &mut targets);
                None
            }
            (None, Page::Settings) => {
                self.draw_settings(ui, &mut targets);
                Some(("horizon.launcher.settings", top(Action::Back)))
            }
            (None, Page::Accounts) => {
                self.draw_accounts(ui, &mut targets);
                Some(("horizon.launcher.accounts", top(Action::Back)))
            }
            (None, Page::Profiles) => {
                self.draw_profiles(ui, &mut targets);
                Some(("horizon.launcher.profiles", top(Action::Back)))
            }
            (None, Page::EditProfile) => {
                self.draw_edit_profile(ui, &mut targets);
                Some((
                    "horizon.launcher.profile.edit",
                    Some(Action::Open(Page::Profiles)),
                ))
            }
            (None, Page::Mods) => {
                self.draw_mods(ui, &mut targets);
                Some(("horizon.launcher.mods", top(Action::Back)))
            }
            (None, Page::BrowseMods) => {
                self.draw_browse(ui, &mut targets);
                Some(("horizon.launcher.browse", Some(Action::Open(Page::Mods))))
            }
            (None, Page::Mod) => {
                self.draw_mod(ui, &mut targets);
                Some(("horizon.launcher.mods.page", Some(Action::Open(Page::Mods))))
            }
            (None, page @ (Page::Updates | Page::Data | Page::About)) => {
                self.draw_info(ui, &mut targets);
                let title = match page {
                    Page::Updates => Setting::Updates,
                    Page::Data => Setting::Data,
                    _ => Setting::About,
                };
                Some((title.label_key(), Some(Action::Open(Page::Settings))))
            }
            (None, Page::Pick(setting)) => {
                self.draw_pick(ui, &mut targets);
                // The settings page, not home, is where its back arrow goes.
                Some((setting.label_key(), Some(Action::Open(Page::Settings))))
            }
        };
        let col = self.area.get();
        let bar_bottom = col.y + 57.0;
        if let Some((title, back)) = bar {
            let bottom = targets
                .iter()
                .map(|(_, r)| r.y + r.h)
                .fold(0.0f32, f32::max)
                + self.scroll.get();
            self.max_scroll
                .set((bottom + MARGIN.max(insets[3]) - ui.h).max(0.0));
            if self.scroll.get() > self.max_scroll.get() {
                self.scroll.set(self.max_scroll.get());
            }
            let content = std::mem::take(&mut targets);
            ui.rounded(
                col.x - MARGIN,
                0.0,
                col.w + 2.0 * MARGIN,
                bar_bottom,
                0.0,
                color::bg(),
            );
            top_bar(ui, col, &lang::get(title), back.is_some(), &mut targets);
            if let (Some(last), Some(back)) = (targets.last_mut(), back) {
                last.0 = back;
            }
            targets.extend(content.into_iter().filter(|(_, r)| r.y + r.h > bar_bottom));
        } else {
            self.max_scroll.set(0.0);
            self.scroll.set(0.0);
        }
        if wide {
            self.draw_sidebar(ui, &mut targets);
        }
        if let Some(note) = &self.notice {
            let lines = wrap(ui, Face::Regular, 13.0, note, col.w - 48.0);
            let text_w = lines
                .iter()
                .map(|line| ui.measure(Face::Regular, 13.0, line))
                .fold(0.0f32, f32::max);
            let (nw, nh) = (text_w + 32.0, lines.len() as f32 * 19.0 + 17.0);
            let (x, y) = (
                col.x + (col.w - nw) / 2.0,
                col.y + col.h - ROW - GAP - 64.0 - GAP - 8.0 - nh,
            );
            // Over the rows it covers, not through them.
            ui.rounded(x, y, nw, nh, 18.0, color::bg());
            ui.rounded(x, y, nw, nh, 18.0, color::hover());
            for (i, line) in lines.iter().enumerate() {
                let ty = y + 8.0 + i as f32 * 19.0;
                ui.text(Face::Regular, 13.0, x + 16.0, ty, color::text(), line);
            }
        }
        *self.frame.borrow_mut() = Layout {
            density: ui.density,
            targets,
        };
    }

    fn draw_home(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        // The header: the account and the two buttons on its right.
        let head = 64.0;
        let avatar = 40.0;
        ui.rounded(
            col.x,
            col.y + (head - avatar) / 2.0,
            avatar,
            avatar,
            8.0,
            color::hover(),
        );
        ui.icon(
            "launcher/user",
            col.x + 8.0,
            col.y + (head - avatar) / 2.0 + 8.0,
            24.0,
            color::text(),
        );
        let tx = col.x + avatar + 12.0;
        let (name, kind) = if self.view.signed_in {
            (self.view.name.clone(), self.view.account_kind.clone())
        } else {
            (
                lang::get("horizon.launcher.no_account"),
                lang::get("horizon.launcher.no_account.hint"),
            )
        };
        ui.text(Face::SemiBold, 17.0, tx, col.y + 10.0, color::text(), &name);
        ui.text(Face::Regular, 12.0, tx, col.y + 34.0, color::muted(), &kind);
        let button = 44.0;
        let settings = Rect::new(
            col.x + col.w - button,
            col.y + (head - button) / 2.0,
            button,
            button,
        );
        let accounts = Rect::new(settings.x - button - 4.0, settings.y, button, button);
        // The account's plate: the kept accounts, or signing in when there is none.
        let plate = Rect::new(col.x, col.y, accounts.x - col.x - 4.0, head);
        targets.push((
            if self.view.signed_in || !self.view.accounts.is_empty() {
                Action::Open(Page::Accounts)
            } else {
                Action::AddAccount
            },
            plate,
        ));
        for (rect, icon, action) in [
            (accounts, "launcher/accounts", Action::AddAccount),
            (settings, "launcher/settings", Action::Open(Page::Settings)),
        ] {
            ui.icon(icon, rect.x + 10.0, rect.y + 10.0, 24.0, color::text());
            targets.push((action, rect));
        }
        let mut y = col.y + head;
        ui.rounded(
            col.x - MARGIN,
            y,
            col.w + 2.0 * MARGIN,
            1.0,
            0.0,
            color::border(),
        );
        y += 12.0;
        // The rows.
        for (icon, key, action) in [
            (
                "launcher/mods",
                "horizon.launcher.mods",
                Action::Open(Page::Mods),
            ),
            (
                "launcher/controls",
                "horizon.launcher.controls",
                Action::Controls,
            ),
            (
                "launcher/share",
                "horizon.launcher.share_log",
                Action::ShareLog,
            ),
            (
                "launcher/folder",
                "horizon.launcher.open_folder",
                Action::OpenFolder,
            ),
        ] {
            if action == Action::Controls && !self.view.touch_layout {
                continue;
            }
            let rect = Rect::new(col.x, y, col.w, ROW);
            row(ui, rect, icon, &lang::get(key), None, true);
            targets.push((action, rect));
            y += ROW + GAP;
        }
        // iOS: the time left on the app's signature.
        if let Some((left, soon)) = &self.view.resign {
            let rect = Rect::new(col.x, y, col.w, ROW);
            row(ui, rect, "launcher/clock", left, None, false);
            if *soon {
                ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::text());
            }
        }
        // The foot: the profile and Play.
        let play = Rect::new(col.x, col.y + col.h - ROW, col.w, ROW);
        let card = Rect::new(col.x, play.y - GAP - 64.0, col.w, 64.0);
        self.profile_card(ui, card, targets);
        play_button(ui, play, targets);
    }

    /// The profile's card: its name and line; a tap picks another, its pencil edits it.
    fn profile_card(&self, ui: &mut Ui, card: Rect, targets: &mut Vec<(Action, Rect)>) {
        ui.border(card.x, card.y, card.w, card.h, 10.0, color::border());
        ui.rounded(
            card.x + 12.0,
            card.y + 14.0,
            36.0,
            36.0,
            8.0,
            color::hover(),
        );
        ui.icon(
            "launcher/profile",
            card.x + 20.0,
            card.y + 22.0,
            20.0,
            color::text(),
        );
        ui.text(
            Face::Medium,
            14.0,
            card.x + 60.0,
            card.y + 13.0,
            color::text(),
            &self.view.profile,
        );
        ui.text(
            Face::Regular,
            12.0,
            card.x + 60.0,
            card.y + 35.0,
            color::muted(),
            &self.view.profile_detail,
        );
        let pick = Rect::new(card.x, card.y, card.w - 56.0, card.h);
        let edit = Rect::new(card.x + card.w - 52.0, card.y + 10.0, 44.0, 44.0);
        ui.icon(
            "launcher/expand",
            pick.x + pick.w - 30.0,
            card.y + 22.0,
            20.0,
            color::muted(),
        );
        ui.icon(
            "launcher/edit",
            edit.x + 12.0,
            edit.y + 12.0,
            20.0,
            color::text(),
        );
        targets.push((Action::Open(Page::EditProfile), edit));
        targets.push((Action::Open(Page::Profiles), pick));
    }

    /// A wide window's home: the game's state, what is new, the profile and Play.
    fn draw_home_wide(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        ui.text(
            Face::SemiBold,
            20.0,
            col.x,
            col.y + 4.0,
            color::text(),
            &lang::get("horizon.launcher.home"),
        );
        ui.text(
            Face::Regular,
            13.0,
            col.x,
            col.y + 34.0,
            color::muted(),
            &self.view.files_line,
        );
        let foot = 64.0;
        let news = Rect::new(col.x, col.y + 66.0, col.w, col.h - 66.0 - 16.0 - foot);
        ui.border(news.x, news.y, news.w, news.h, 10.0, color::border());
        let title = lang::tr("horizon.launcher.news.title", &[&self.view.version]);
        ui.text(
            Face::Medium,
            15.0,
            news.x + 16.0,
            news.y + 14.0,
            color::text(),
            &title,
        );
        let mut y = news.y + 44.0;
        for item in lang::get("horizon.launcher.news").split('\n') {
            for line in wrap(ui, Face::Regular, 14.0, item, news.w - 32.0) {
                if y + 20.0 > news.y + news.h - 8.0 {
                    break;
                }
                ui.text(Face::Regular, 14.0, news.x + 16.0, y, color::muted(), &line);
                y += 22.0;
            }
        }
        let play_w = 220.0;
        let bottom = col.y + col.h - foot;
        let card = Rect::new(col.x, bottom, col.w - play_w - 12.0, foot);
        self.profile_card(ui, card, targets);
        play_button(
            ui,
            Rect::new(col.x + col.w - play_w, bottom, play_w, foot),
            targets,
        );
    }

    /// A wide window's sidebar: the account, the top pages, the game folder and the log.
    fn draw_sidebar(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        ui.rounded(0.0, 0.0, SIDEBAR, ui.h, 0.0, color::raised());
        ui.rounded(SIDEBAR, 0.0, 1.0, ui.h, 0.0, color::border());
        let (x, w) = (14.0, SIDEBAR - 28.0);
        let mut y = 20.0;
        // The account: its plate opens the kept accounts, the people button signs another in.
        let plate = Rect::new(x, y, w, 60.0);
        ui.border(plate.x, plate.y, plate.w, plate.h, 10.0, color::border());
        ui.rounded(
            plate.x + 10.0,
            plate.y + 12.0,
            36.0,
            36.0,
            8.0,
            color::hover(),
        );
        ui.icon(
            "launcher/user",
            plate.x + 18.0,
            plate.y + 20.0,
            20.0,
            color::text(),
        );
        let (name, kind) = if self.view.signed_in {
            (self.view.name.clone(), self.view.account_kind.clone())
        } else {
            (
                lang::get("horizon.launcher.no_account"),
                lang::get("horizon.launcher.no_account.hint"),
            )
        };
        let room = plate.w - 56.0 - 40.0;
        let name = first_line(ui, Face::Medium, 14.0, &name, room);
        ui.text(
            Face::Medium,
            14.0,
            plate.x + 56.0,
            plate.y + 11.0,
            color::text(),
            &name,
        );
        let kind = first_line(ui, Face::Regular, 11.0, &kind, room);
        ui.text(
            Face::Regular,
            11.0,
            plate.x + 56.0,
            plate.y + 33.0,
            color::muted(),
            &kind,
        );
        let add = Rect::new(plate.x + plate.w - 40.0, plate.y + 10.0, 36.0, 40.0);
        ui.icon(
            "launcher/accounts",
            add.x + 8.0,
            add.y + 10.0,
            20.0,
            color::muted(),
        );
        targets.push((Action::AddAccount, add));
        targets.push((
            if self.view.signed_in || !self.view.accounts.is_empty() {
                Action::Open(Page::Accounts)
            } else {
                Action::AddAccount
            },
            plate,
        ));
        y += 60.0 + 20.0;
        let current = match self.page {
            Page::Home => 0,
            Page::Mods | Page::BrowseMods | Page::Mod => 1,
            Page::Profiles | Page::EditProfile => 2,
            Page::Settings
            | Page::Pick(_)
            | Page::Accounts
            | Page::Updates
            | Page::Data
            | Page::About => 3,
        };
        for (i, (icon, key, page)) in [
            ("launcher/play", "horizon.launcher.home", Page::Home),
            ("launcher/mods", "horizon.launcher.mods", Page::Mods),
            (
                "launcher/profile",
                "horizon.launcher.profiles",
                Page::Profiles,
            ),
            (
                "launcher/settings",
                "horizon.launcher.settings",
                Page::Settings,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let rect = Rect::new(x, y, w, 40.0);
            let on = i == current && self.flow.is_none();
            if on {
                ui.rounded(rect.x, rect.y, rect.w, rect.h, 8.0, color::hover());
            }
            let ink = if on { color::text() } else { color::muted() };
            ui.icon(icon, rect.x + 12.0, rect.y + 10.0, 20.0, ink);
            let face = if on { Face::Medium } else { Face::Regular };
            ui.text(
                face,
                14.0,
                rect.x + 44.0,
                rect.y + 10.0,
                ink,
                &lang::get(key),
            );
            targets.push((Action::Open(page), rect));
            y += 40.0 + 4.0;
        }
        // The foot: the game folder, the log, the version.
        let mut y = ui.h - 20.0 - 18.0 - 2.0 * 36.0;
        for (icon, key, action) in [
            (
                "launcher/folder",
                "horizon.launcher.open_folder",
                Action::OpenFolder,
            ),
            (
                "launcher/share",
                "horizon.launcher.share_log",
                Action::ShareLog,
            ),
        ] {
            let rect = Rect::new(x, y, w, 36.0);
            ui.icon(icon, rect.x + 12.0, rect.y + 9.0, 18.0, color::muted());
            ui.text(
                Face::Regular,
                13.0,
                rect.x + 42.0,
                rect.y + 9.0,
                color::muted(),
                &lang::get(key),
            );
            targets.push((action, rect));
            y += 36.0;
        }
        ui.text(
            Face::Regular,
            11.0,
            x + 12.0,
            y + 6.0,
            color::muted(),
            &format!("Magnetite {}", self.view.version),
        );
    }

    fn draw_accounts(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        let row_h = 60.0;
        for (i, (name, kind)) in self.view.accounts.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, row_h);
            ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
            if self.view.current == Some(i) {
                ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::muted());
            }
            ui.icon(
                "launcher/user",
                rect.x + 16.0,
                rect.y + 18.0,
                24.0,
                color::text(),
            );
            ui.text(
                Face::Medium,
                15.0,
                rect.x + 54.0,
                rect.y + 10.0,
                color::text(),
                name,
            );
            ui.text(
                Face::Regular,
                12.0,
                rect.x + 54.0,
                rect.y + 32.0,
                color::muted(),
                kind,
            );
            let remove = Rect::new(rect.x + rect.w - 52.0, rect.y + 8.0, 44.0, 44.0);
            if self.view.confirm_remove == Some(i) {
                let ask = lang::get("horizon.launcher.accounts.remove_confirm");
                let aw = ui.measure(Face::Medium, 13.0, &ask);
                let ask_rect =
                    Rect::new(rect.x + rect.w - aw - 28.0, rect.y + 12.0, aw + 20.0, 36.0);
                ui.rounded(
                    ask_rect.x,
                    ask_rect.y,
                    ask_rect.w,
                    ask_rect.h,
                    18.0,
                    color::text(),
                );
                ui.text(
                    Face::Medium,
                    13.0,
                    ask_rect.x + 10.0,
                    ask_rect.y + 8.0,
                    color::bg(),
                    &ask,
                );
                targets.push((Action::RemoveAccount(i), ask_rect));
            } else {
                ui.icon(
                    "launcher/delete",
                    remove.x + 12.0,
                    remove.y + 12.0,
                    20.0,
                    color::muted(),
                );
                targets.push((Action::RemoveAccount(i), remove));
            }
            if self.view.current == Some(i) {
                ui.icon(
                    "launcher/check",
                    remove.x - 32.0,
                    remove.y + 12.0,
                    20.0,
                    color::text(),
                );
            }
            targets.push((Action::PickAccount(i), rect));
            y += row_h + GAP;
        }
        let add = Rect::new(col.x, y, col.w, ROW);
        row(
            ui,
            add,
            "launcher/add",
            &lang::get("horizon.launcher.accounts.add"),
            None,
            false,
        );
        targets.push((Action::AddAccount, add));
    }

    fn draw_profiles(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        for (i, name) in self.view.profiles.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, ROW);
            row(ui, rect, "launcher/profile", name, None, false);
            if self.view.profile_current == i {
                ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::muted());
                ui.icon(
                    "launcher/check",
                    rect.x + rect.w - 36.0,
                    rect.y + (rect.h - 20.0) / 2.0,
                    20.0,
                    color::text(),
                );
            }
            targets.push((Action::PickProfile(i), rect));
            y += ROW + GAP;
        }
        let add = Rect::new(col.x, y, col.w, ROW);
        row(
            ui,
            add,
            "launcher/add",
            &lang::get("horizon.launcher.profiles.new"),
            None,
            false,
        );
        targets.push((Action::NewProfile, add));
    }

    fn draw_mods(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        let find = Rect::new(col.x, y, col.w, ROW);
        row(
            ui,
            find,
            "launcher/download",
            &lang::get("horizon.launcher.mods.browse"),
            None,
            true,
        );
        targets.push((Action::Open(Page::BrowseMods), find));
        y += ROW + GAP;
        let import = Rect::new(col.x, y, col.w, ROW);
        row(
            ui,
            import,
            "launcher/add",
            &lang::get("horizon.launcher.mods.import"),
            None,
            true,
        );
        targets.push((Action::ImportMod, import));
        y += ROW + 12.0;
        let hint = if self.view.mods.is_empty() {
            "horizon.launcher.mods.empty"
        } else {
            "horizon.launcher.mods.hint"
        };
        for line in wrap(ui, Face::Regular, 13.0, &lang::get(hint), col.w) {
            ui.text(Face::Regular, 13.0, col.x + 2.0, y, color::muted(), &line);
            y += 19.0;
        }
        y += 10.0;
        let row_h = 60.0;
        for (i, m) in self.view.mods.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, row_h);
            ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
            let room = rect.w - 16.0 - 70.0;
            let name = first_line(ui, Face::Medium, 15.0, &m.name, room);
            ui.text(
                Face::Medium,
                15.0,
                rect.x + 16.0,
                rect.y + 10.0,
                color::text(),
                &name,
            );
            let detail = first_line(ui, Face::Regular, 12.0, &m.detail, room);
            ui.text(
                Face::Regular,
                12.0,
                rect.x + 16.0,
                rect.y + 32.0,
                color::muted(),
                &detail,
            );
            switch(ui, rect.x + rect.w - 62.0, rect.y + 16.0, m.enabled);
            // The switch's end of the row turns it; the rest opens the mod's page.
            let end = Rect::new(rect.x + rect.w - 78.0, rect.y, 78.0, rect.h);
            targets.push((Action::ToggleMod(i), end));
            targets.push((Action::OpenMod(i), rect));
            y += row_h + GAP;
        }
    }

    /// One installed mod: its name and what it says of itself, on or off, its settings, its page
    /// and removing it.
    fn draw_mod(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let page = &self.view.mod_page;
        let mut y = self.content_top();
        for line in wrap(ui, Face::SemiBold, 20.0, &page.name, col.w) {
            ui.text(Face::SemiBold, 20.0, col.x, y, color::text(), &line);
            y += 27.0;
        }
        for line in wrap(ui, Face::Regular, 13.0, &page.detail, col.w) {
            ui.text(Face::Regular, 13.0, col.x, y, color::muted(), &line);
            y += 19.0;
        }
        y += 6.0;
        for line in page.description.lines() {
            for line in wrap(ui, Face::Regular, 14.0, line, col.w) {
                ui.text(Face::Regular, 14.0, col.x, y, color::text(), &line);
                y += 20.0;
            }
        }
        y += 14.0;
        let on = Rect::new(col.x, y, col.w, ROW);
        row(
            ui,
            on,
            "launcher/mods",
            &lang::get("horizon.launcher.mods.enabled"),
            None,
            false,
        );
        switch(
            ui,
            on.x + on.w - 62.0,
            on.y + (ROW - 28.0) / 2.0,
            page.enabled,
        );
        targets.push((Action::ToggleOpenMod, on));
        y += ROW + 18.0;
        ui.text(
            Face::Regular,
            12.0,
            col.x + 4.0,
            y,
            color::muted(),
            &lang::get("horizon.launcher.mods.settings"),
        );
        y += 24.0;
        let row_h = 52.0;
        for (i, setting) in page.settings.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, row_h);
            ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
            // The two buttons and the value between them, as wide as the value needs.
            let control = (ui.measure(Face::Medium, 13.0, &setting.value) + 108.0)
                .max(160.0)
                .min(rect.w * 0.64);
            let room = match setting.kind {
                ModSettingKind::Toggle(_) => rect.w - 16.0 - 78.0,
                _ => rect.w - 16.0 - control - 8.0,
            };
            let label = first_line(ui, Face::Regular, 14.0, &setting.label, room);
            ui.text(
                Face::Regular,
                14.0,
                rect.x + 16.0,
                rect.y + 16.0,
                color::text(),
                &label,
            );
            if let ModSettingKind::Toggle(on) = setting.kind {
                switch(ui, rect.x + rect.w - 62.0, rect.y + 12.0, on);
                targets.push((Action::ModStep(i, true), rect));
            } else {
                let side = 40.0;
                let left = Rect::new(rect.x + rect.w - control, rect.y + 6.0, side, side);
                let right = Rect::new(rect.x + rect.w - side - 6.0, rect.y + 6.0, side, side);
                for (r, icon, forward) in [
                    (left, "launcher/back", false),
                    (right, "launcher/next", true),
                ] {
                    ui.rounded(r.x, r.y, r.w, r.h, 8.0, color::hover());
                    ui.icon(icon, r.x + 10.0, r.y + 10.0, 20.0, color::text());
                    targets.push((Action::ModStep(i, forward), r));
                }
                let (from, to) = (left.x + left.w + 4.0, right.x - 4.0);
                if let ModSettingKind::Color(rgb) = setting.kind {
                    let patch = Rect::new((from + to) / 2.0 - 18.0, rect.y + 14.0, 36.0, 24.0);
                    ui.rounded(
                        patch.x,
                        patch.y,
                        patch.w,
                        patch.h,
                        6.0,
                        crate::hud::rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255),
                    );
                    ui.border(patch.x, patch.y, patch.w, patch.h, 6.0, color::muted());
                } else {
                    let value = first_line(ui, Face::Medium, 13.0, &setting.value, to - from);
                    let vw = ui.measure(Face::Medium, 13.0, &value);
                    ui.text(
                        Face::Medium,
                        13.0,
                        (from + to - vw) / 2.0,
                        rect.y + 17.0,
                        color::text(),
                        &value,
                    );
                }
            }
            y += row_h + GAP;
        }
        if page.settings.is_empty() {
            for line in wrap(ui, Face::Regular, 13.0, &page.settings_note, col.w) {
                ui.text(Face::Regular, 13.0, col.x + 2.0, y, color::muted(), &line);
                y += 19.0;
            }
        }
        y += 12.0;
        if let Some(url) = &page.homepage {
            let rect = Rect::new(col.x, y, col.w, ROW);
            row(
                ui,
                rect,
                "launcher/link",
                &lang::get("horizon.launcher.mods.homepage"),
                None,
                true,
            );
            targets.push((Action::Link(url.clone()), rect));
            y += ROW + GAP;
        }
        let rect = Rect::new(col.x, y, col.w, ROW);
        let key = if page.confirm_remove {
            "horizon.launcher.profile.remove_confirm"
        } else {
            "horizon.launcher.mods.remove"
        };
        row(ui, rect, "launcher/delete", &lang::get(key), None, false);
        if page.confirm_remove {
            ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::text());
        }
        targets.push((Action::RemoveMod, rect));
    }

    /// The Updates, Data and About pages: some text, then rows.
    fn draw_info(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        for text in &self.view.info_text {
            for line in wrap(ui, Face::Regular, 14.0, text, col.w) {
                ui.text(Face::Regular, 14.0, col.x + 2.0, y, color::muted(), &line);
                y += 20.0;
            }
            y += 8.0;
        }
        if !self.view.info_text.is_empty() {
            y += 8.0;
        }
        for line in &self.view.info {
            let rect = Rect::new(col.x, y, col.w, ROW);
            let vw = ui.measure(Face::Regular, 13.0, &line.value);
            let room = rect.w - 52.0 - vw - 44.0;
            let label = first_line(ui, Face::Regular, 15.0, &line.label, room);
            let value = (!line.value.is_empty()).then_some(line.value.as_str());
            row(ui, rect, line.icon, &label, value, line.action.is_some());
            // A row that does nothing is a target too, so that the page scrolls to it.
            targets.push((line.action.clone().unwrap_or(Action::None), rect));
            y += ROW + GAP;
        }
    }

    fn draw_browse(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        if let Some(note) = &self.view.browse_note {
            for line in wrap(ui, Face::Regular, 14.0, note, col.w) {
                ui.text(Face::Regular, 14.0, col.x + 2.0, y, color::muted(), &line);
                y += 20.0;
            }
            y += 12.0;
        }
        let row_h = 64.0;
        for (i, m) in self.view.browse.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, row_h);
            ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
            let bw = ui.measure(Face::Medium, 13.0, &m.button) + 24.0;
            let room = rect.w - 16.0 - bw - 24.0;
            let name = first_line(ui, Face::Medium, 15.0, &m.name, room);
            ui.text(
                Face::Medium,
                15.0,
                rect.x + 16.0,
                rect.y + 11.0,
                color::text(),
                &name,
            );
            let summary = first_line(ui, Face::Regular, 12.0, &m.summary, room);
            ui.text(
                Face::Regular,
                12.0,
                rect.x + 16.0,
                rect.y + 35.0,
                color::muted(),
                &summary,
            );
            let button = Rect::new(rect.x + rect.w - bw - 12.0, rect.y + 16.0, bw, 32.0);
            let (fill, ink) = if m.can_install {
                (color::text(), color::bg())
            } else {
                (color::hover(), color::muted())
            };
            ui.rounded(button.x, button.y, button.w, button.h, 16.0, fill);
            ui.text(
                Face::Medium,
                13.0,
                button.x + 12.0,
                button.y + 7.0,
                ink,
                &m.button,
            );
            if m.can_install {
                targets.push((Action::InstallMod(i), rect));
            }
            y += row_h + GAP;
        }
    }

    fn draw_pick(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        let row_h = 48.0;
        let n = self.view.choices.len();
        if n > 0 {
            ui.rounded(col.x, y, col.w, row_h * n as f32, 10.0, color::raised());
        }
        for (i, label) in self.view.choices.iter().enumerate() {
            let rect = Rect::new(col.x, y, col.w, row_h);
            if i > 0 {
                ui.rounded(
                    rect.x + 12.0,
                    rect.y,
                    rect.w - 24.0,
                    1.0,
                    0.0,
                    color::border(),
                );
            }
            ui.text(
                Face::Regular,
                15.0,
                rect.x + 16.0,
                rect.y + 13.0,
                color::text(),
                label,
            );
            if self.view.chosen == Some(i) {
                ui.icon(
                    "launcher/check",
                    rect.x + rect.w - 36.0,
                    rect.y + 14.0,
                    20.0,
                    color::text(),
                );
            }
            targets.push((Action::Choose(i), rect));
            y += row_h;
        }
    }

    fn draw_edit_profile(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top();
        ui.text(
            Face::Regular,
            12.0,
            col.x + 2.0,
            y,
            color::muted(),
            &lang::get("horizon.launcher.profile.name"),
        );
        y += 20.0;
        let field = Rect::new(col.x, y, col.w, 48.0);
        ui.rounded(field.x, field.y, field.w, field.h, 10.0, color::raised());
        if self.view.renaming {
            ui.border(field.x, field.y, field.w, field.h, 10.0, color::text());
        }
        let shown = if self.view.renaming {
            format!("{}|", self.view.profile_name)
        } else {
            self.view.profile_name.clone()
        };
        ui.text(
            Face::Regular,
            15.0,
            field.x + 14.0,
            field.y + 13.0,
            color::text(),
            &shown,
        );
        if !self.view.renaming {
            ui.icon(
                "launcher/edit",
                field.x + field.w - 34.0,
                field.y + 14.0,
                20.0,
                color::muted(),
            );
        }
        targets.push((Action::RenameProfile, field));
        y += 48.0 + 16.0;
        for line in wrap(
            ui,
            Face::Regular,
            13.0,
            &lang::get("horizon.launcher.profile.hint"),
            col.w,
        ) {
            ui.text(Face::Regular, 13.0, col.x + 2.0, y, color::muted(), &line);
            y += 19.0;
        }
        y += 16.0;
        if self.view.profile_removable {
            let rect = Rect::new(col.x, y, col.w, ROW);
            if self.view.confirm_remove_profile {
                ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::text());
                let label = lang::get("horizon.launcher.profile.remove_confirm");
                let lw = ui.measure(Face::Medium, 15.0, &label);
                ui.text(
                    Face::Medium,
                    15.0,
                    rect.x + (rect.w - lw) / 2.0,
                    rect.y + 16.0,
                    color::bg(),
                    &label,
                );
            } else {
                row(
                    ui,
                    rect,
                    "launcher/delete",
                    &lang::get("horizon.launcher.profile.remove"),
                    None,
                    false,
                );
            }
            targets.push((Action::RemoveProfile, rect));
        }
    }

    fn draw_settings(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let col = self.area.get();
        let mut y = self.content_top() - 8.0;
        let row_h = 48.0;
        for (section, settings) in SECTIONS {
            let listed: Vec<_> = settings
                .iter()
                .filter_map(|s| {
                    self.view
                        .values
                        .iter()
                        .find(|(v, _)| v == s)
                        .map(|(_, value)| (*s, value.clone()))
                })
                .collect();
            if listed.is_empty() {
                continue;
            }
            ui.text(
                Face::Regular,
                12.0,
                col.x + 4.0,
                y + 8.0,
                color::muted(),
                &lang::get(section),
            );
            y += 30.0;
            let group = Rect::new(col.x, y, col.w, row_h * listed.len() as f32);
            ui.rounded(group.x, group.y, group.w, group.h, 10.0, color::raised());
            for (i, (setting, value)) in listed.into_iter().enumerate() {
                let rect = Rect::new(col.x, y, col.w, row_h);
                if i > 0 {
                    ui.rounded(
                        rect.x + 12.0,
                        rect.y,
                        rect.w - 24.0,
                        1.0,
                        0.0,
                        color::border(),
                    );
                }
                ui.icon(
                    setting.icon(),
                    rect.x + 14.0,
                    rect.y + 14.0,
                    20.0,
                    color::text(),
                );
                ui.text(
                    Face::Regular,
                    14.0,
                    rect.x + 46.0,
                    rect.y + 14.0,
                    color::text(),
                    &lang::get(setting.label_key()),
                );
                let on = self.view.switches.iter().find(|(s, _)| *s == setting);
                if let Some((_, on)) = on {
                    switch(ui, rect.x + rect.w - 60.0, rect.y + 10.0, *on);
                } else {
                    let vw = ui.measure(Face::Regular, 13.0, &value);
                    ui.text(
                        Face::Regular,
                        13.0,
                        rect.x + rect.w - 14.0 - vw,
                        rect.y + 15.0,
                        color::muted(),
                        &value,
                    );
                }
                targets.push((Action::Setting(setting), rect));
                y += row_h;
            }
            y += 12.0;
        }
    }
}

/// A page's top: a back arrow (when `back`), its title and the rule under them; returns where the
/// page's content starts.
fn top_bar(
    ui: &mut Ui,
    col: Rect,
    title: &str,
    back: bool,
    targets: &mut Vec<(Action, Rect)>,
) -> f32 {
    let tx = if back {
        let rect = Rect::new(col.x - 4.0, col.y + 6.0, 44.0, 44.0);
        ui.icon(
            "launcher/back",
            rect.x + 12.0,
            rect.y + 12.0,
            20.0,
            color::text(),
        );
        targets.push((Action::Back, rect));
        col.x + 48.0
    } else {
        col.x + 4.0
    };
    ui.text(Face::SemiBold, 17.0, tx, col.y + 16.0, color::text(), title);
    ui.rounded(
        col.x - MARGIN,
        col.y + 56.0,
        col.w + 2.0 * MARGIN,
        1.0,
        0.0,
        color::border(),
    );
    col.y + 72.0
}

/// A sign-in page: the boot screen's content in the launcher's look, top to bottom, the buttons
/// after it (the first one white) so the system keyboard never covers them.
fn draw_flow(ui: &mut Ui, col: Rect, flow: &Flow, targets: &mut Vec<(Action, Rect)>) {
    let c = &flow.content;
    let mut y = top_bar(ui, col, &c.title, flow.back, targets);
    if flow.back {
        // The flow's back is its own: leaving the method choice.
        if let Some(last) = targets.last_mut() {
            last.0 = Action::FlowBack;
        }
    }
    for paragraph in &c.lines {
        for line in wrap(ui, Face::Regular, 15.0, paragraph, col.w) {
            ui.text(Face::Regular, 15.0, col.x, y, color::text(), &line);
            y += 22.0;
        }
        y += 10.0;
    }
    if let Some(code) = &c.code {
        ui.text(Face::SemiBold, 34.0, col.x, y, color::text(), code);
        y += 58.0;
    }
    if let Some(error) = &c.error {
        for line in wrap(ui, Face::Medium, 14.0, error, col.w) {
            ui.text(Face::Medium, 14.0, col.x, y, color::text(), &line);
            y += 20.0;
        }
        y += 10.0;
    }
    for (field, caption, value) in &c.fields {
        ui.text(Face::Regular, 12.0, col.x + 2.0, y, color::muted(), caption);
        y += 20.0;
        let rect = Rect::new(col.x, y, col.w, 48.0);
        ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
        if flow.focus == Some(*field) {
            ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::text());
        }
        ui.text(
            Face::Regular,
            15.0,
            rect.x + 14.0,
            rect.y + 13.0,
            color::text(),
            value,
        );
        targets.push((Action::Boot(BootAction::FocusField(*field)), rect));
        y += 48.0 + 12.0;
    }
    if let Some(status) = &c.status {
        ui.text(Face::Regular, 13.0, col.x, y, color::muted(), status);
        y += 24.0;
    }
    if let Some(progress) = c.progress {
        ui.rounded(col.x, y, col.w, 6.0, 3.0, color::hover());
        ui.rounded(
            col.x,
            y,
            col.w * progress.clamp(0.0, 1.0),
            6.0,
            3.0,
            color::text(),
        );
        y += 22.0;
    }
    y += 8.0;
    for (i, (label, action)) in c.buttons.iter().enumerate() {
        let rect = Rect::new(col.x, y, col.w, ROW);
        let (fill, ink) = if i == 0 {
            (color::text(), color::bg())
        } else {
            (color::raised(), color::text())
        };
        ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, fill);
        let lw = ui.measure(Face::Medium, 15.0, label);
        ui.text(
            Face::Medium,
            15.0,
            rect.x + (rect.w - lw) / 2.0,
            rect.y + 16.0,
            ink,
            label,
        );
        targets.push((Action::Boot(action.clone()), rect));
        y += ROW + GAP;
    }
}

/// The white Play button.
fn play_button(ui: &mut Ui, play: Rect, targets: &mut Vec<(Action, Rect)>) {
    ui.rounded(play.x, play.y, play.w, play.h, 10.0, color::text());
    let label = lang::get("horizon.launcher.play");
    let lw = ui.measure(Face::SemiBold, 16.0, &label);
    ui.text(
        Face::SemiBold,
        16.0,
        play.x + (play.w - lw) / 2.0,
        play.y + (play.h - 23.0) / 2.0,
        color::bg(),
        &label,
    );
    targets.push((Action::Play, play));
}

/// The first line of `text` that fits `width`, with an ellipsis when there was more.
fn first_line(ui: &Ui, face: Face, size: f32, text: &str, width: f32) -> String {
    let lines = wrap(ui, face, size, text, width);
    match lines.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first, ..] => format!("{first}…"),
    }
}

/// An on/off switch at (`x`, `y`), 46 by 28 dp: white with the knob right when on.
fn switch(ui: &mut Ui, x: f32, y: f32, on: bool) {
    let (track, knob, at) = if on {
        (color::text(), color::bg(), x + 21.0)
    } else {
        (color::hover(), color::muted(), x + 3.0)
    };
    ui.rounded(x, y, 46.0, 28.0, 14.0, track);
    ui.rounded(at, y + 3.0, 22.0, 22.0, 11.0, knob);
}

/// A plate row: an icon, a label, a value or a chevron on the right.
fn row(ui: &mut Ui, rect: Rect, icon: &str, label: &str, value: Option<&str>, chevron: bool) {
    ui.rounded(rect.x, rect.y, rect.w, rect.h, 10.0, color::raised());
    ui.icon(
        icon,
        rect.x + 16.0,
        rect.y + (rect.h - 22.0) / 2.0,
        22.0,
        color::text(),
    );
    ui.text(
        Face::Regular,
        15.0,
        rect.x + 52.0,
        rect.y + (rect.h - 22.0) / 2.0,
        color::text(),
        label,
    );
    if let Some(value) = value {
        let vw = ui.measure(Face::Regular, 13.0, value);
        ui.text(
            Face::Regular,
            13.0,
            rect.x + rect.w - 16.0 - vw,
            rect.y + (rect.h - 19.0) / 2.0,
            color::muted(),
            value,
        );
    } else if chevron {
        ui.icon(
            "launcher/next",
            rect.x + rect.w - 34.0,
            rect.y + (rect.h - 18.0) / 2.0,
            18.0,
            color::muted(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_phone_upright_gets_about_its_own_density() {
        assert!((LauncherScreen::density_for(1080.0, 2400.0, 2.6) - 2.57).abs() < 0.01);
        assert_eq!(LauncherScreen::density_for(300.0, 600.0, 1.0), 1.0);
        // A PC's window keeps the system's scale.
        assert_eq!(LauncherScreen::density_for(1920.0, 1080.0, 1.5), 1.5);
    }

    /// The frame `draw` makes at `size` pixels, painted as the HUD's shader paints it: RGBA rows.
    fn painted(size: [u32; 2], density: f32, draw: impl Fn(&mut Ui)) -> Vec<u8> {
        use crate::hud::{HudAtlas, HudBatch};
        let atlas = HudAtlas::boot();
        let mut pixels = atlas.pixels().into_owned();
        let mut batch = HudBatch::scratch(size);
        let mut ui = Ui::new(&atlas, &mut batch, (0.0, 0.0));
        ui.set_density(density);
        draw(&mut ui);
        ui.finish();
        for patch in &batch.patches {
            for row in 0..patch.height {
                let from = (row * patch.width * 4) as usize;
                let to = (((patch.y + row) * atlas.width + patch.x) * 4) as usize;
                let n = (patch.width * 4) as usize;
                pixels[to..to + n].copy_from_slice(&patch.rgba[from..from + n]);
            }
        }
        let [w, h] = size;
        let mut out = [0u8, 0, 0, 255].repeat((w * h) as usize);
        // Every quad is two triangles over an upright rectangle.
        for quad in batch.vertices.chunks(6) {
            let edge = |f: fn(f32, f32) -> f32, of: fn(&crate::hud::HudVertex) -> f32, start| {
                quad.iter().map(of).fold(start, f)
            };
            let (inf, ninf) = (f32::INFINITY, f32::NEG_INFINITY);
            let s = batch.scale;
            let (x0, x1) = (
                edge(f32::min, |v| v.pos[0], inf) * s,
                edge(f32::max, |v| v.pos[0], ninf) * s,
            );
            let (y0, y1) = (
                edge(f32::min, |v| v.pos[1], inf) * s,
                edge(f32::max, |v| v.pos[1], ninf) * s,
            );
            let at = |x: bool, y: bool| {
                quad.iter()
                    .find(|v| {
                        (v.pos[0] * s > (x0 + x1) / 2.0) == x
                            && (v.pos[1] * s > (y0 + y1) / 2.0) == y
                    })
                    .map(|v| v.uv)
            };
            let (Some(a), Some(b)) = (at(false, false), at(true, true)) else {
                continue;
            };
            let tint = quad[0].color.to_le_bytes();
            for py in y0.round().max(0.0) as u32..(y1.round().max(0.0) as u32).min(h) {
                for px in x0.round().max(0.0) as u32..(x1.round().max(0.0) as u32).min(w) {
                    let (fx, fy) = (
                        (px as f32 + 0.5 - x0) / (x1 - x0),
                        (py as f32 + 0.5 - y0) / (y1 - y0),
                    );
                    let u = (a[0] + (b[0] - a[0]) * fx) * atlas.width as f32;
                    let v = (a[1] + (b[1] - a[1]) * fy) * atlas.height as f32;
                    let i = ((v as u32).min(atlas.height - 1) * atlas.width
                        + (u as u32).min(atlas.width - 1)) as usize
                        * 4;
                    let alpha = u32::from(pixels[i + 3]) * u32::from(tint[3]) / 255;
                    let o = ((py * w + px) * 4) as usize;
                    for c in 0..3 {
                        let ink = u32::from(pixels[i + c]) * u32::from(tint[c]) / 255;
                        out[o + c] =
                            ((ink * alpha + u32::from(out[o + c]) * (255 - alpha)) / 255) as u8;
                    }
                }
            }
        }
        out
    }

    /// Pictures of the pages, to look at: `MAGNETITE_LAUNCHER_SHOTS=<folder> cargo test -p
    /// horizon-client-lib --lib -- --ignored launcher_pictures`.
    #[test]
    #[ignore = "writes pictures of the pages to look at"]
    fn launcher_pictures() {
        let Some(folder) = std::env::var_os("MAGNETITE_LAUNCHER_SHOTS") else {
            return;
        };
        let folder = std::path::PathBuf::from(folder);
        crate::lang::set(crate::lang::Language::load(&[], "ru_ru"));
        let setting = |s: Setting, v: &str| (s, v.to_owned());
        let info = |icon, label: &str, value: &str, action| InfoLine {
            icon,
            label: lang::get(label),
            value: value.to_owned(),
            action,
        };
        let link = || Some(Action::Link(String::new()));
        let view = View {
            name: "Steve".into(),
            account_kind: "Microsoft".into(),
            signed_in: true,
            profile: "Основной".into(),
            profile_detail: "Minecraft 26.3".into(),
            touch_layout: true,
            version: "0.1.2".into(),
            values: vec![
                setting(Setting::Accounts, "1"),
                setting(Setting::GraphicsApi, "Vulkan"),
                setting(Setting::FpsLimit, "Без ограничения"),
                setting(Setting::Memory, "Без ограничения"),
                setting(Setting::Jit, "Выключен"),
                setting(Setting::Sensitivity, "100 %"),
                setting(Setting::Gyro, ""),
                setting(Setting::GyroSensitivity, "100 %"),
                setting(Setting::Vibration, ""),
                setting(Setting::Cutout, ""),
                setting(Setting::Language, "Русский (Россия)"),
                setting(Setting::Updates, "Вышла версия 0.1.3"),
                setting(Setting::Data, "642 МБ"),
                setting(Setting::About, ""),
            ],
            switches: vec![
                (Setting::Gyro, true),
                (Setting::Vibration, true),
                (Setting::Cutout, false),
            ],
            mods: vec![
                ModLine {
                    name: "Farmer's Delight".into(),
                    detail: "1.4.0 · WebAssembly".into(),
                    enabled: true,
                },
                ModLine {
                    name: "Очень длинное название мода, которое не помещается в строку".into(),
                    detail: "0.1 · Lua".into(),
                    enabled: false,
                },
            ],
            mod_page: ModPage {
                name: "Farmer's Delight".into(),
                detail: "1.4.0 · WebAssembly · vectorwing · MIT".into(),
                description:
                    "Расширяет фермерство и кулинарию: новые культуры, кухонная утварь и блюда."
                        .into(),
                enabled: true,
                settings: vec![
                    ModSettingLine {
                        label: "Показывать подсказки в инвентаре и на рабочих столах".into(),
                        value: String::new(),
                        kind: ModSettingKind::Toggle(true),
                    },
                    ModSettingLine {
                        label: "Скорость готовки".into(),
                        value: "1.25".into(),
                        kind: ModSettingKind::Step,
                    },
                    ModSettingLine {
                        label: "Режим".into(),
                        value: "Как в ванильной игре".into(),
                        kind: ModSettingKind::Step,
                    },
                    ModSettingLine {
                        label: "Цвет пара".into(),
                        value: "#3AB3DA".into(),
                        kind: ModSettingKind::Color(0x3AB3DA),
                    },
                ],
                settings_note: String::new(),
                homepage: Some("https://example.com".into()),
                confirm_remove: false,
            },
            ..View::default()
        };
        let data = vec![
            info(
                "launcher/language",
                "horizon.launcher.data.worlds",
                "84 МБ",
                None,
            ),
            info(
                "launcher/mods",
                "horizon.launcher.data.mods",
                "3.2 МБ",
                None,
            ),
            info(
                "launcher/profile",
                "horizon.launcher.data.files",
                "530 МБ",
                None,
            ),
            info(
                "launcher/delete",
                "horizon.launcher.data.cache",
                "12 МБ",
                Some(Action::Clear(Store::Cache)),
            ),
            info(
                "launcher/delete",
                "horizon.launcher.data.downloads",
                "0.0 МБ",
                Some(Action::Clear(Store::Downloads)),
            ),
            info(
                "launcher/refresh",
                "horizon.launcher.data.check",
                "",
                Some(Action::CheckFiles),
            ),
        ];
        let about = vec![
            info(
                "launcher/info",
                "horizon.launcher.updates.current",
                "0.1.2",
                None,
            ),
            info(
                "launcher/github",
                "horizon.launcher.about.project",
                "",
                link(),
            ),
            info("launcher/mods", "horizon.launcher.about.mods", "", link()),
        ];
        let text = |keys: &[&str]| keys.iter().map(|k| lang::get(k)).collect::<Vec<_>>();
        for (name, page, info, info_text) in [
            ("home", Page::Home, Vec::new(), Vec::new()),
            ("settings", Page::Settings, Vec::new(), Vec::new()),
            ("mods", Page::Mods, Vec::new(), Vec::new()),
            ("mod", Page::Mod, Vec::new(), Vec::new()),
            (
                "data",
                Page::Data,
                data,
                text(&["horizon.launcher.data.hint"]),
            ),
            (
                "about",
                Page::About,
                about,
                text(&[
                    "horizon.launcher.about.note",
                    "horizon.launcher.about.thanks",
                    "horizon.launcher.about.mojang",
                ]),
            ),
        ] {
            let mut screen = LauncherScreen::new(View {
                info,
                info_text,
                ..view.clone()
            });
            screen.set_page(page);
            if page == Page::Mods {
                screen.set_notice(Some(lang::get("horizon.launcher.import.native")));
            }
            // An iPhone: 393 by 852 points, the Dynamic Island and the home indicator; then a PC.
            screen.set_insets([0.0, 118.0, 0.0, 68.0]);
            for (kind, size, scale) in
                [("phone", [786u32, 1704u32], 2.0), ("pc", [1300, 875], 1.25)]
            {
                if kind == "pc" {
                    screen.set_insets([0.0; 4]);
                }
                let density = LauncherScreen::density_for(size[0] as f32, size[1] as f32, scale);
                crate::ui_theme::reset_atlas();
                let rgba = painted(size, density, |ui| screen.draw(ui));
                let file =
                    std::fs::File::create(folder.join(format!("{name}-{kind}.png"))).unwrap();
                let mut encoder = png::Encoder::new(file, size[0], size[1]);
                encoder.set_color(png::ColorType::Rgba);
                encoder
                    .write_header()
                    .and_then(|mut w| w.write_image_data(&rgba))
                    .unwrap();
            }
        }
    }

    #[test]
    fn taps_read_the_last_draws_targets() {
        let screen = LauncherScreen::new(View::default());
        *screen.frame.borrow_mut() = Layout {
            density: 2.0,
            targets: vec![(Action::Play, Rect::new(10.0, 100.0, 50.0, 20.0))],
        };
        assert_eq!(screen.click((40.0, 220.0)), Some(Action::Play));
        assert_eq!(screen.click((10.0, 10.0)), None);
    }
}
