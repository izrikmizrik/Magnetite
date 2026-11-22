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
use std::cell::RefCell;

/// The column the launcher lays out for, in dp: a phone held upright, as its own apps are drawn.
const COLUMN_DP: f32 = 420.0;
/// The side margin and the gap between plates.
const MARGIN: f32 = 16.0;
const GAP: f32 = 8.0;
/// A row's and a button's height.
const ROW: f32 = 56.0;
/// The status bar and the notch at the top (the window draws under them on a phone).
const TOP: f32 = 36.0;

/// The launcher's pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Settings,
    /// The kept accounts: pick the one that plays, remove one, add one.
    Accounts,
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
            Self::Sensitivity | Self::Gyro | Self::Vibration => "launcher/controls",
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
    Profiles,
    EditProfile,
    Setting(Setting),
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
    /// iOS: how long until the app must be signed again, already worded ("5 d 3 h"), and whether
    /// that is under a day. `None` where nothing expires (Android, TrollStore).
    pub resign: Option<(String, bool)>,
    /// Each setting's value as shown; a setting left out is not listed (gyro only on Android,
    /// JIT only on iOS).
    pub values: Vec<(Setting, String)>,
    /// Whether an account (or an offline name) plays: without one the header asks to sign in.
    pub signed_in: bool,
    /// The kept accounts (name, kind), the one that plays, and the one whose removal waits for a
    /// second tap.
    pub accounts: Vec<(String, String)>,
    pub current: Option<usize>,
    pub confirm_remove: Option<usize>,
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
}

impl LauncherScreen {
    pub fn new(view: View) -> Self {
        Self {
            page: Page::Home,
            view,
            notice: None,
            flow: None,
            frame: RefCell::new(Layout::default()),
        }
    }
    pub fn page(&self) -> Page {
        self.page
    }
    pub fn set_page(&mut self, page: Page) {
        self.page = page;
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

    /// The physical pixels per dp for a screen `width` physical pixels across: a phone held upright
    /// gets about its own density (a 1080-pixel-wide phone, about 2.6), never less than one.
    pub fn density_for(width: f32) -> f32 {
        (width / COLUMN_DP).max(1.0)
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
        match (&self.flow, self.page) {
            (Some(flow), _) => draw_flow(ui, flow, &mut targets),
            (None, Page::Home) => self.draw_home(ui, &mut targets),
            (None, Page::Settings) => self.draw_settings(ui, &mut targets),
            (None, Page::Accounts) => self.draw_accounts(ui, &mut targets),
        }
        if let Some(note) = &self.notice {
            let nw = ui.measure(Face::Regular, 13.0, note) + 32.0;
            let (x, y) = (
                (ui.w - nw) / 2.0,
                ui.h - MARGIN - ROW - GAP - 64.0 - GAP - 44.0,
            );
            ui.rounded(x, y, nw, 36.0, 18.0, color::hover());
            ui.text(Face::Regular, 13.0, x + 16.0, y + 8.0, color::text(), note);
        }
        *self.frame.borrow_mut() = Layout {
            density: ui.density,
            targets,
        };
    }

    fn draw_home(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let (w, h) = (ui.w, ui.h);
        let col = Rect::new(MARGIN, TOP, w - 2.0 * MARGIN, h - TOP - MARGIN);
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
        ui.rounded(0.0, y, w, 1.0, 0.0, color::border());
        y += 12.0;
        // The rows.
        for (icon, key, action) in [
            ("launcher/mods", "horizon.launcher.mods", Action::Mods),
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
            let rect = Rect::new(col.x, y, col.w, ROW);
            row(ui, rect, icon, &lang::get(key), None, true);
            targets.push((action, rect));
            y += ROW + GAP;
        }
        // iOS: the time left on the app's signature.
        if let Some((left, soon)) = &self.view.resign {
            let rect = Rect::new(col.x, y, col.w, ROW);
            let label = lang::tr("horizon.launcher.resign", &[left]);
            row(ui, rect, "launcher/clock", &label, None, false);
            if *soon {
                ui.border(rect.x, rect.y, rect.w, rect.h, 10.0, color::text());
            }
        }
        // The foot: the profile and Play.
        let play = Rect::new(col.x, col.y + col.h - ROW, col.w, ROW);
        let card_h = 64.0;
        let card = Rect::new(col.x, play.y - GAP - card_h, col.w, card_h);
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
        targets.push((Action::EditProfile, edit));
        targets.push((Action::Profiles, pick));
        ui.rounded(play.x, play.y, play.w, play.h, 10.0, color::text());
        let label = lang::get("horizon.launcher.play");
        let lw = ui.measure(Face::SemiBold, 16.0, &label);
        ui.text(
            Face::SemiBold,
            16.0,
            play.x + (play.w - lw) / 2.0,
            play.y + (ROW - 23.0) / 2.0,
            color::bg(),
            &label,
        );
        targets.push((Action::Play, play));
    }

    fn draw_accounts(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let w = ui.w;
        let col = Rect::new(MARGIN, TOP, w - 2.0 * MARGIN, ui.h - TOP - MARGIN);
        let mut y = top_bar(
            ui,
            col,
            &lang::get("horizon.launcher.accounts"),
            true,
            targets,
        );
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

    fn draw_settings(&self, ui: &mut Ui, targets: &mut Vec<(Action, Rect)>) {
        let w = ui.w;
        let col = Rect::new(MARGIN, TOP, w - 2.0 * MARGIN, ui.h - TOP - MARGIN);
        let back = Rect::new(col.x - 4.0, col.y + 6.0, 44.0, 44.0);
        ui.icon(
            "launcher/back",
            back.x + 12.0,
            back.y + 12.0,
            20.0,
            color::text(),
        );
        targets.push((Action::Back, back));
        ui.text(
            Face::SemiBold,
            17.0,
            col.x + 48.0,
            col.y + 16.0,
            color::text(),
            &lang::get("horizon.launcher.settings"),
        );
        let mut y = col.y + 64.0;
        ui.rounded(0.0, y - 8.0, w, 1.0, 0.0, color::border());
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
                let vw = ui.measure(Face::Regular, 13.0, &value);
                ui.text(
                    Face::Regular,
                    13.0,
                    rect.x + rect.w - 14.0 - vw,
                    rect.y + 15.0,
                    color::muted(),
                    &value,
                );
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
    ui.rounded(0.0, col.y + 56.0, ui.w, 1.0, 0.0, color::border());
    col.y + 72.0
}

/// A sign-in page: the boot screen's content in the launcher's look, top to bottom, the buttons
/// after it (the first one white) so the system keyboard never covers them.
fn draw_flow(ui: &mut Ui, flow: &Flow, targets: &mut Vec<(Action, Rect)>) {
    let c = &flow.content;
    let col = Rect::new(MARGIN, TOP, ui.w - 2.0 * MARGIN, ui.h - TOP - MARGIN);
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
        assert!((LauncherScreen::density_for(1080.0) - 2.57).abs() < 0.01);
        assert_eq!(LauncherScreen::density_for(300.0), 1.0);
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
