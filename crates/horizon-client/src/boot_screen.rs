//! The screen shown before the game starts: the sign-in choice, Microsoft's
//! device code, the Ely.by form, the game files' download and their failures
//! (`docs/unification/mobile-boot.md` §1). A model, not a loop: whoever owns the
//! window calls [`BootScreen::layout`], [`BootScreen::draw`] and
//! [`BootScreen::click`], and runs the threads a [`BootAction`] asks for.
//!
//! It draws with `HudAtlas::boot` and the design system's bundled fonts, so it
//! shows before any of Minecraft's files are there, and the same code serves a
//! phone and a PC without a launcher. Every visible string is a `horizon.boot.*`
//! key of `assets/horizon/lang`.
//!
//! Stage 1 of the plan draws the screen and the Ely.by form's keyboard: the
//! sign-in threads, the download and the values the form holds are later
//! commits. The form's keys live in the band it keeps clear at the bottom
//! (`KEYBOARD`), and a tap there answers a [`BootAction::Typed`] — the screen
//! itself never edits a field.
use crate::lang;
use crate::osk::{Keyboard, Keys, Output};
use crate::ui_theme::{self, Face, Ui, color};
use std::cell::RefCell;

/// The content column's widest, in dp: wider screens get the same narrow page.
const COLUMN: f32 = 520.0;
/// The page's margin, in dp.
const MARGIN: f32 = 24.0;
/// Between a title and its text, and between the blocks, in dp.
const GAP: f32 = 12.0;
/// The share of the screen the on-screen keyboard takes on the Ely.by form, and
/// its cap in dp (`docs/unification/mobile-boot.md` §1.4, "Ввод текста"): the
/// form keeps out of this band, and the keys fill it.
const KEYBOARD: f32 = 0.42;
const KEYBOARD_MAX: f32 = 300.0;
/// Types and controls never shrink below this: a screen that small cannot be read.
const MIN_FIT: f32 = 0.15;
/// How far the type shrinks before a paragraph is dropped instead: smaller than
/// this and the page is not worth reading.
const FIT_FLOOR: f32 = 0.55;

/// Which provider signed an account in, as far as the screen cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountKind {
    Microsoft,
    Ely,
}

/// The Ely.by form's fields, in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElyField {
    Login,
    Password,
    Otp,
}

/// What a tap on the screen asks the boot loop for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootAction {
    /// Start Microsoft's device-code sign-in.
    SignInMicrosoft,
    /// Open the Ely.by form.
    SignInEly,
    /// Start without an account.
    PlayOffline,
    /// Open the device-code page (or the address shown) in the platform's browser.
    OpenLink,
    /// Put the code on the clipboard.
    CopyCode,
    /// Leave the code or the form.
    Cancel,
    /// Send the Ely.by form (the keyboard's Done key sends it too, without a
    /// second tap).
    Submit,
    /// The previous page.
    Back,
    /// Start the game with the account shown.
    Continue,
    /// Forget the account shown.
    SignOut,
    /// Start the failed step again (a download goes on where it stopped).
    Retry,
    /// Put the cursor in a field of the Ely.by form.
    FocusField(ElyField),
    /// A key of the form's on-screen keyboard was tapped: the value goes to
    /// the field that has focus. The screen never edits a field itself.
    Typed(Output),
}

/// What the screen shows. The boot loop owns the values; the screen only draws
/// them, and nothing here touches a network or a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootState {
    /// No account yet: sign in with Microsoft, with Ely.by, or play offline.
    Choose,
    /// Signing a kept account in again, without asking for its code or password.
    Resuming { name: String },
    /// Microsoft's device code, waiting for the player to enter it elsewhere.
    Code {
        /// The code, drawn large.
        code: String,
        /// The page it is entered on.
        url: String,
        /// What is left of its life.
        expires_in_secs: u64,
    },
    /// The Ely.by form: the login, the password (as stars) and the code of an
    /// account with two-factor sign-in.
    Ely {
        login: String,
        /// The password's length: it is never kept as text to draw.
        password_len: usize,
        otp: Option<String>,
        focus: ElyField,
        error: Option<String>,
    },
    /// An account is signed in. `note` is an extra line under the name: the
    /// boot loop's own words, already translated (a kept account played offline
    /// until the connection returns).
    Signed {
        name: String,
        kind: AccountKind,
        note: Option<String>,
    },
    /// The game files' download.
    Files {
        done_bytes: u64,
        total_bytes: u64,
        step: String,
    },
    /// A step did not work.
    Failed { message: String },
    /// Choosing the offline name: one field, typed on the same keys as the
    /// Ely.by form.
    OfflineName { name: String, error: Option<String> },
    /// Playing as an offline name.
    Offline { name: String },
}

/// A rectangle in dp, the design system's logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    /// Whether `p` (in dp) is inside, the right and bottom edges excluded.
    pub fn contains(&self, (px, py): (f32, f32)) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Where everything is: one layout for drawing and for taps, so they never
/// disagree.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// The screen, in dp.
    pub size: [f32; 2],
    /// Physical pixels per dp: what [`Layout::at`] divides a position by.
    pub scale: f32,
    /// How much the type and the controls were shrunk to fit (1 when nothing was).
    pub fit: f32,
    /// The page: the column between the margins.
    pub column: Rect,
    pub title: Rect,
    /// One box per paragraph, as tall as the lines it was estimated to take.
    pub paragraphs: Vec<Rect>,
    /// The device code or the offline name, drawn large.
    pub code: Option<Rect>,
    /// The form's complaint, under its text.
    pub error: Option<Rect>,
    /// The form's fields, in the order they are drawn.
    pub fields: Vec<(ElyField, Rect)>,
    /// The download's bar.
    pub progress: Option<Rect>,
    /// The line over the bar: what the download is doing.
    pub status: Option<Rect>,
    /// In the order they are drawn.
    pub buttons: Vec<(BootAction, Rect)>,
    /// The Ely.by form's keys: the band at the bottom, and the board fill it.
    pub keyboard: Option<(Rect, Keys)>,
    /// What the screen needs vertically; more than the screen only when even the
    /// smallest type does not fit.
    needed: f32,
}

impl Layout {
    /// The action a tap at `pos` asks for, `pos` in physical pixels as the window
    /// reports them: a button's, or a field's focus. None outside them (the
    /// form's keys are read by [`BootScreen::click`], which needs the keyboard's
    /// state to press one).
    pub fn at(&self, pos: (f32, f32)) -> Option<BootAction> {
        if self.scale <= 0.0 {
            return None;
        }
        let at = (pos.0 / self.scale, pos.1 / self.scale);
        if let Some((action, _)) = self.buttons.iter().find(|(_, rect)| rect.contains(at)) {
            return Some(action.clone());
        }
        self.fields
            .iter()
            .find(|(_, rect)| rect.contains(at))
            .map(|(field, _)| BootAction::FocusField(*field))
    }
}

/// The screen in one state. Mutating it runs no layout: the next
/// [`BootScreen::layout`] or [`BootScreen::draw`] does, and until then
/// [`BootScreen::click`] answers nothing.
pub struct BootScreen {
    state: BootState,
    /// The Ely.by form's keys: one line, no suggestions. The layout tells it
    /// where it sits; a tap on the keys moves its focus and shift state.
    typing: RefCell<Keyboard>,
    frame: RefCell<Option<Layout>>,
}

impl BootScreen {
    pub fn new(state: BootState) -> Self {
        Self {
            state,
            typing: RefCell::new(Keyboard::new(false, false)),
            frame: RefCell::new(None),
        }
    }
    pub fn state(&self) -> &BootState {
        &self.state
    }
    /// Show another state; the layout made for the old one is forgotten.
    pub fn set_state(&mut self, state: BootState) {
        self.state = state;
        *self.frame.borrow_mut() = None;
    }

    /// The screen's layout for a window of `size` physical pixels at `scale`
    /// pixels per dp, kept for [`BootScreen::click`]. It measures with an
    /// average character width, since it must not read the atlas;
    /// [`BootScreen::draw`] lays out again with the real font for its boxes.
    ///
    /// Everything that takes a tap stays on the screen at any size: the type and
    /// the controls shrink until the page fits, the last paragraphs go rather
    /// than the page shrinking to nothing.
    pub fn layout(&self, size: [f32; 2], scale: f32) -> Layout {
        let content = self.content();
        self.fitted(&content, size, scale, &estimate_lines)
    }

    /// The button, field or key at `pos` (physical pixels), from the last layout.
    pub fn click(&self, pos: (f32, f32)) -> Option<BootAction> {
        let layout = self.frame.borrow();
        let layout = layout.as_ref()?;
        if layout.scale <= 0.0 {
            return None;
        }
        let at = (pos.0 / layout.scale, pos.1 / layout.scale);
        if let Some(action) = layout.at(pos) {
            return Some(action);
        }
        // The form's keys take what the page does not: a key under the finger
        // is pressed where it is drawn, and its output goes to the field. Done
        // is the form's Submit, so a thumb needs no second button for it; the
        // grid has no Close key (that is the pad's B), so no other output is
        // given a page action here.
        let (_, keys) = layout.keyboard?;
        let (x, y) = at;
        if x < keys.x || x >= keys.x + keys.w || y < keys.y || y >= keys.y + keys.h {
            return None;
        }
        let out = self.typing.borrow_mut().tap([x, y], keys)?;
        Some(match out {
            Output::Done => BootAction::Submit,
            other => BootAction::Typed(other),
        })
    }

    pub fn draw(&self, ui: &mut Ui) {
        // Laid out again with the real font, so a paragraph's box is the lines it
        // takes: the same layout the next tap reads.
        let (layout, content) = {
            let view: &Ui = ui;
            let density = view.density;
            let content = self.content();
            let layout = self.fitted(
                &content,
                [view.w * density, view.h * density],
                density,
                &|text, size, width| wrap(view, Face::Regular, size, text, width).len(),
            );
            (layout, content)
        };
        let m = Metrics::new(layout.fit);
        ui.rounded(0.0, 0.0, ui.w, ui.h, 0.0, color::bg());
        ui.text_line(
            Face::SemiBold,
            m.title,
            m.title_line,
            layout.title.x,
            layout.title.y,
            color::text(),
            &content.title,
        );
        for (paragraph, rect) in content.lines.iter().zip(&layout.paragraphs) {
            let mut y = rect.y;
            for line in wrap(ui, Face::Regular, m.body, paragraph, rect.w) {
                ui.text_line(
                    Face::Regular,
                    m.body,
                    m.body_line,
                    rect.x,
                    y,
                    color::text(),
                    &line,
                );
                y += m.body_line;
            }
        }
        if let (Some(code), Some(rect)) = (&content.code, layout.code) {
            let code = clip(ui, Face::SemiBold, m.code, code, rect.w);
            ui.text_line(
                Face::SemiBold,
                m.code,
                m.code_line,
                rect.x,
                rect.y,
                color::accent(),
                &code,
            );
        }
        if let (Some(error), Some(rect)) = (&content.error, layout.error) {
            let room = (rect.h / m.small_line).floor().max(1.0) as usize;
            for (i, line) in wrap(ui, Face::Regular, m.small, error, rect.w)
                .into_iter()
                .take(room)
                .enumerate()
            {
                ui.text_line(
                    Face::Regular,
                    m.small,
                    m.small_line,
                    rect.x,
                    rect.y + i as f32 * m.small_line,
                    danger(),
                    &line,
                );
            }
        }
        if let (Some(status), Some(rect)) = (&content.status, layout.status) {
            let status = clip(ui, Face::Regular, m.small, status, rect.w);
            ui.text_line(
                Face::Regular,
                m.small,
                m.small_line,
                rect.x,
                rect.y,
                color::muted(),
                &status,
            );
        }
        if let (Some(fraction), Some(rect)) = (content.progress, layout.progress) {
            ui.rounded(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                rect.h / 2.0,
                color::raised(),
            );
            let filled = (rect.w * fraction.clamp(0.0, 1.0)).max(rect.h);
            ui.rounded(
                rect.x,
                rect.y,
                filled,
                rect.h,
                rect.h / 2.0,
                color::accent(),
            );
        }
        for ((_, caption, text), (field, rect)) in content.fields.iter().zip(&layout.fields) {
            ui.text_line(
                Face::Regular,
                m.caption,
                m.caption_line,
                rect.x,
                rect.y - m.caption_line,
                color::muted(),
                caption,
            );
            let focused = matches!(&self.state, BootState::Ely { focus, .. } if focus == field)
                || matches!(&self.state, BootState::OfflineName { .. });
            ui.rounded(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                ui_theme::radius(),
                color::raised(),
            );
            ui.border(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                ui_theme::radius(),
                if focused {
                    color::accent()
                } else {
                    color::border()
                },
            );
            let text = clip(ui, Face::Regular, m.body, text, rect.w - m.gap * 2.0);
            ui.text_line(
                Face::Regular,
                m.body,
                rect.h,
                rect.x + m.gap,
                rect.y,
                color::text(),
                &text,
            );
        }
        for ((label, _), (action, rect)) in content.buttons.iter().zip(&layout.buttons) {
            let hovered = rect.contains(ui.cursor);
            let (plate, ink) = if primary(action) {
                let alpha = if hovered { 0.85 } else { 1.0 };
                (
                    color::with_alpha(color::accent(), alpha),
                    color::on_accent(),
                )
            } else if hovered {
                (color::hover(), color::text())
            } else {
                (color::raised(), color::text())
            };
            ui.rounded(rect.x, rect.y, rect.w, rect.h, rect.h / 2.0, plate);
            let room = (rect.w - m.gap * 2.0).max(1.0);
            let size = ui.fit(Face::Medium, m.button_text, label, room);
            let width = ui.measure(Face::Medium, size, label);
            ui.text_line(
                Face::Medium,
                size,
                rect.h,
                rect.x + ((rect.w - width) / 2.0).round(),
                rect.y,
                ink,
                label,
            );
        }
        // The form's keys over the band the page left clear: drawn by the grid
        // the last tap read, so a key never moves under the finger that pressed it.
        if let Some((_, keys)) = layout.keyboard {
            self.typing.borrow().draw_touch(ui, keys);
        }
    }

    /// The room the on-screen keyboard will take, which the form keeps out of.
    fn keyboard_room(&self, h: f32) -> f32 {
        if matches!(
            self.state,
            BootState::Ely { .. } | BootState::OfflineName { .. }
        ) {
            (h * KEYBOARD).min(KEYBOARD_MAX)
        } else {
            0.0
        }
    }

    /// The layout shrunk until its page fits the room above the keyboard, and
    /// kept for [`BootScreen::click`]. Type shrinks first, then the least
    /// important paragraph is dropped, so a small screen loses a sentence before
    /// it makes the page unreadable; what takes a tap stays.
    fn fitted(
        &self,
        content: &Content,
        size: [f32; 2],
        scale: f32,
        lines: &dyn Fn(&str, f32, f32) -> usize,
    ) -> Layout {
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let size = [size[0].max(1.0), size[1].max(1.0)];
        // The page is measured in dp; the window is given in physical pixels.
        let dp = [size[0] / scale, size[1] / scale];
        let room = (dp[1] - self.keyboard_room(dp[1])).max(1.0);
        let mut drop = 0;
        let mut fit = 1.0;
        let mut layout = self.compose(content, size, scale, fit, drop, lines);
        for _ in 0..16 {
            if layout.needed <= room {
                break;
            }
            let next = (fit * room / layout.needed).max(MIN_FIT);
            if next < fit && next >= FIT_FLOOR {
                fit = next;
                layout = self.compose(content, size, scale, fit, drop, lines);
            } else if drop < content.lines.len() {
                // Smaller type would not be read: drop the last paragraph and
                // try the full size again. Every tap target stays.
                drop += 1;
                fit = 1.0;
                layout = self.compose(content, size, scale, fit, drop, lines);
            } else if next < fit {
                fit = next;
                layout = self.compose(content, size, scale, fit, drop, lines);
            } else {
                break;
            }
        }
        *self.frame.borrow_mut() = Some(layout.clone());
        layout
    }

    /// The layout at one fit factor, and the height it asks for.
    fn compose(
        &self,
        content: &Content,
        size: [f32; 2],
        scale: f32,
        fit: f32,
        drop: usize,
        lines: &dyn Fn(&str, f32, f32) -> usize,
    ) -> Layout {
        let m = Metrics::new(fit);
        // The page is dp: the window comes in physical pixels.
        let (w, h) = (size[0].max(1.0) / scale, size[1].max(1.0) / scale);
        let room = (h - self.keyboard_room(h)).max(1.0);
        let column_w = COLUMN.min(w - 2.0 * m.margin).max(1.0);
        let x = ((w - column_w) / 2.0).round().max(0.0);
        // The paragraphs that are still drawn: the last `drop` go on a screen
        // too small for them (the first, what the page is for, stays longest).
        let shown = content.lines.len().saturating_sub(drop);

        // The page's heights, before anything is placed.
        let counts: Vec<usize> = content
            .lines
            .iter()
            .take(shown)
            .map(|p| lines(p, m.body, column_w))
            .collect();
        let text_h: f32 = counts.iter().sum::<usize>() as f32 * m.body_line
            + counts.len().saturating_sub(1) as f32 * m.gap * 0.5;
        let code_h = if content.code.is_some() {
            m.gap * 0.5 + m.code_line
        } else {
            0.0
        };
        let error_lines = content
            .error
            .as_deref()
            .map_or(0, |e| lines(e, m.small, column_w));
        let error_h = if error_lines > 0 {
            m.gap * 0.4 + error_lines as f32 * m.small_line
        } else {
            0.0
        };
        let top_h = m.title_line + m.gap + text_h + code_h + error_h;

        let buttons_h = content.buttons.len() as f32 * m.control
            + content.buttons.len().saturating_sub(1) as f32 * m.gap;
        let status_h = if content.progress.is_some() {
            m.small_line + m.gap * 0.5 + m.bar
        } else {
            0.0
        };
        let fields_h = if content.fields.is_empty() {
            0.0
        } else {
            content.fields.len() as f32 * (m.control + m.caption_line)
                + (content.fields.len() - 1) as f32 * m.field_gap
        };
        let mut bottom_h = 0.0;
        let mut first = true;
        for block in [fields_h, status_h, buttons_h] {
            if block > 0.0 {
                if !first {
                    bottom_h += m.gap;
                }
                bottom_h += block;
                first = false;
            }
        }
        let needed = m.margin + top_h + m.gap + bottom_h + m.margin;

        // The text from the top: the title, then the paragraphs, the code, the error.
        let mut y = m.margin;
        let title = Rect::new(x, y, column_w, m.title_line);
        y += m.title_line + m.gap;
        let mut paragraphs = Vec::with_capacity(counts.len());
        for (i, count) in counts.iter().enumerate() {
            let height = *count as f32 * m.body_line;
            paragraphs.push(Rect::new(x, y, column_w, height));
            y += height;
            if i + 1 < counts.len() {
                y += m.gap * 0.5;
            }
        }
        let code = if content.code.is_some() {
            y += m.gap * 0.5;
            let rect = Rect::new(x, y, column_w, m.code_line);
            Some(rect)
        } else {
            None
        };
        let error = if error_lines > 0 {
            y += m.gap * 0.4;
            let height = error_lines as f32 * m.small_line;
            Some(Rect::new(x, y, column_w, height))
        } else {
            None
        };

        // The controls from the bottom up, where a thumb reaches: the buttons,
        // the download's line and bar, then the form's fields. They end at the
        // bottom margin when they fit above it, and take the whole height
        // otherwise, so nothing ever leaves the screen.
        let mut low = (room - m.margin).max(bottom_h.min(room));
        let mut buttons = vec![(BootAction::Cancel, Rect::default()); content.buttons.len()];
        for i in (0..content.buttons.len()).rev() {
            low -= m.control;
            let action = content.buttons[i].1.clone();
            buttons[i] = (action, Rect::new(x, low, column_w, m.control));
            low -= m.gap;
        }
        if !content.buttons.is_empty() {
            low += m.gap;
        }
        let mut progress = None;
        let mut status = None;
        if content.progress.is_some() {
            if !content.buttons.is_empty() {
                low -= m.gap;
            }
            low -= m.bar;
            progress = Some(Rect::new(x, low, column_w, m.bar));
            low -= m.gap * 0.5;
            low -= m.small_line;
            status = Some(Rect::new(x, low, column_w, m.small_line));
        }
        let mut fields = Vec::with_capacity(content.fields.len());
        if !content.fields.is_empty() {
            if !content.buttons.is_empty() || content.progress.is_some() {
                low -= m.gap;
            }
            // The block's height is `fields_h`: captions above each field, the
            // gap between them only.
            let mut top = low - fields_h;
            for (i, (field, _, _)) in content.fields.iter().enumerate() {
                let rect = Rect::new(x, top + m.caption_line, column_w, m.control);
                fields.push((*field, rect));
                top += m.caption_line + m.control;
                if i + 1 < content.fields.len() {
                    top += m.field_gap;
                }
            }
        }

        Layout {
            size: [w, h],
            scale,
            fit,
            column: Rect::new(x, m.margin, column_w, (room - 2.0 * m.margin).max(0.0)),
            title,
            paragraphs,
            code,
            error,
            fields,
            progress,
            status,
            buttons,
            keyboard: self.keys_band(w, h),
            needed,
        }
    }

    /// The Ely.by form's band at the bottom of the `w` by `h` dp screen, with the keys scaled to
    /// it; no band on a page a finger does not type into. The page keeps out of the band
    /// ([`BootScreen::keyboard_room`]), so the keys never cover a field or a button.
    fn keys_band(&self, w: f32, h: f32) -> Option<(Rect, Keys)> {
        let room = self.keyboard_room(h);
        if room <= 0.0 {
            return None;
        }
        let band = Rect::new(0.0, (h - room).max(0.0), w.max(1.0), room);
        let area = [band.x, band.y, band.w, band.h];
        Some((band, Keys::within(area)))
    }

    /// What the state says, in the current language.
    fn content(&self) -> Content {
        match &self.state {
            BootState::Choose => Content {
                title: lang::get("horizon.boot.title"),
                lines: vec![lang::get("horizon.boot.choose.about")],
                buttons: vec![
                    (
                        lang::get("horizon.boot.choose.microsoft"),
                        BootAction::SignInMicrosoft,
                    ),
                    (lang::get("horizon.boot.choose.ely"), BootAction::SignInEly),
                    (
                        lang::get("horizon.boot.choose.offline"),
                        BootAction::PlayOffline,
                    ),
                ],
                ..Content::default()
            },
            BootState::Resuming { name } => Content {
                title: lang::get("horizon.boot.resuming.title"),
                lines: vec![lang::tr("horizon.boot.resuming.line", &[name])],
                buttons: vec![(lang::get("horizon.boot.cancel"), BootAction::Cancel)],
                ..Content::default()
            },
            BootState::Code {
                code,
                url,
                expires_in_secs,
            } => Content {
                title: lang::get("horizon.boot.code.title"),
                lines: vec![
                    lang::get("horizon.boot.code.about"),
                    lang::tr("horizon.boot.code.open", &[url]),
                    lang::tr("horizon.boot.code.expires", &[&expires_in_secs]),
                ],
                code: Some(code.clone()),
                buttons: vec![
                    (
                        lang::get("horizon.boot.code.open_page"),
                        BootAction::OpenLink,
                    ),
                    (lang::get("horizon.boot.code.copy"), BootAction::CopyCode),
                    (lang::get("horizon.boot.cancel"), BootAction::Cancel),
                ],
                ..Content::default()
            },
            BootState::Ely {
                login,
                password_len,
                otp,
                error,
                ..
            } => {
                let mut fields = vec![
                    (
                        ElyField::Login,
                        lang::get("horizon.boot.ely.login"),
                        login.clone(),
                    ),
                    (
                        ElyField::Password,
                        lang::get("horizon.boot.ely.password"),
                        mask(*password_len),
                    ),
                ];
                if let Some(otp) = otp {
                    fields.push((
                        ElyField::Otp,
                        lang::get("horizon.boot.ely.otp"),
                        otp.clone(),
                    ));
                }
                Content {
                    title: lang::get("horizon.boot.ely.title"),
                    lines: vec![lang::get("horizon.boot.ely.about")],
                    error: error.clone(),
                    fields,
                    buttons: vec![
                        (lang::get("horizon.boot.ely.submit"), BootAction::Submit),
                        (lang::get("horizon.boot.back"), BootAction::Back),
                    ],
                    ..Content::default()
                }
            }
            BootState::Signed { name, kind, note } => {
                let mut lines = vec![lang::tr(
                    "horizon.boot.signed.line",
                    &[name, &kind_label(*kind)],
                )];
                lines.extend(note.clone());
                Content {
                    title: lang::get("horizon.boot.signed.title"),
                    lines,
                    buttons: vec![
                        (lang::get("horizon.boot.continue"), BootAction::Continue),
                        (lang::get("horizon.boot.sign_out"), BootAction::SignOut),
                    ],
                    ..Content::default()
                }
            }
            BootState::Files {
                done_bytes,
                total_bytes,
                step,
            } => Content {
                title: lang::get("horizon.boot.files.title"),
                lines: vec![lang::get("horizon.boot.files.about")],
                status: Some(if *total_bytes > 0 {
                    let done = megabytes(*done_bytes);
                    let total = megabytes(*total_bytes);
                    lang::tr("horizon.boot.files.stage", &[step, &done, &total])
                } else if step.is_empty() {
                    lang::get("horizon.boot.files.waiting")
                } else {
                    step.clone()
                }),
                progress: Some(if *total_bytes > 0 {
                    (*done_bytes as f32 / *total_bytes as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                }),
                ..Content::default()
            },
            BootState::Failed { message } => Content {
                title: lang::get("horizon.boot.failed.title"),
                lines: vec![message.clone(), lang::get("horizon.boot.failed.hint")],
                buttons: vec![
                    (lang::get("horizon.boot.retry"), BootAction::Retry),
                    (lang::get("horizon.boot.back"), BootAction::Back),
                ],
                ..Content::default()
            },
            BootState::OfflineName { name, error } => Content {
                title: lang::get("horizon.boot.offline.name.title"),
                lines: vec![lang::get("horizon.boot.offline.name.about")],
                error: error.clone(),
                fields: vec![(
                    ElyField::Login,
                    lang::get("horizon.boot.offline.name.field"),
                    name.clone(),
                )],
                buttons: vec![
                    (
                        lang::get("horizon.boot.offline.name.play"),
                        BootAction::Submit,
                    ),
                    (lang::get("horizon.boot.back"), BootAction::Back),
                ],
                ..Content::default()
            },
            BootState::Offline { name } => Content {
                title: lang::get("horizon.boot.offline.title"),
                lines: vec![lang::get("horizon.boot.offline.about")],
                code: Some(name.clone()),
                ..Content::default()
            },
        }
    }
}

/// One state's strings and controls, laid out and drawn from the same list.
#[derive(Default)]
struct Content {
    title: String,
    /// Paragraphs of body text.
    lines: Vec<String>,
    /// Drawn large: the device code, the offline name.
    code: Option<String>,
    /// The form's complaint.
    error: Option<String>,
    /// The download's line over its bar.
    status: Option<String>,
    /// The download's fraction, 0 to 1.
    progress: Option<f32>,
    /// The form's fields: which, its caption and its text.
    fields: Vec<(ElyField, String, String)>,
    buttons: Vec<(String, BootAction)>,
}

/// The design system's type and control sizes at one fit factor, in dp.
struct Metrics {
    margin: f32,
    gap: f32,
    field_gap: f32,
    control: f32,
    bar: f32,
    title: f32,
    title_line: f32,
    body: f32,
    body_line: f32,
    code: f32,
    code_line: f32,
    small: f32,
    small_line: f32,
    caption: f32,
    caption_line: f32,
    button_text: f32,
}

impl Metrics {
    fn new(fit: f32) -> Self {
        let sizes = ui_theme::sizes();
        let f = fit.clamp(MIN_FIT, 1.0);
        let pad = |v: f32| (v * f).round().max(1.0);
        Self {
            margin: pad(MARGIN),
            gap: pad(GAP),
            field_gap: pad(10.0),
            control: pad(sizes.control_large).max(8.0),
            bar: pad(8.0),
            title: sizes.title * f,
            title_line: (40.0 * f).max(6.0),
            body: sizes.body * f,
            body_line: (24.0 * f).max(4.0),
            code: sizes.display * f,
            code_line: (60.0 * f).max(6.0),
            small: sizes.caption * f,
            small_line: (18.0 * f).max(4.0),
            caption: sizes.caption * f,
            caption_line: (18.0 * f).max(4.0),
            button_text: sizes.ui * f,
        }
    }
}

/// Which button the page wants pressed: the one that goes on with what it is for.
fn primary(action: &BootAction) -> bool {
    matches!(
        action,
        BootAction::SignInMicrosoft
            | BootAction::OpenLink
            | BootAction::Submit
            | BootAction::Continue
            | BootAction::Retry
    )
}

/// The menu style has no danger colour for the overlays: the same red the iOS
/// screen used for a form's complaint.
fn danger() -> u32 {
    color::hex(0xE5484D, 255)
}

fn kind_label(kind: AccountKind) -> String {
    lang::get(match kind {
        AccountKind::Microsoft => "horizon.boot.account.microsoft",
        AccountKind::Ely => "horizon.boot.account.ely",
    })
}

/// A password as stars, one per character and at most a field's worth: the
/// password is never kept as text for the screen.
fn mask(len: usize) -> String {
    "*".repeat(len.min(32))
}

/// `bytes` in megabytes with one decimal, as the download's line shows them.
fn megabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_000_000.0)
}

/// How many lines `text` takes in `width` dp at `size`: the greedy wrap
/// [`wrap`] does, with an average advance instead of the atlas, since the layout
/// is made before any font is uploaded and must not read one.
fn estimate_lines(text: &str, size: f32, width: f32) -> usize {
    let advance = (size * 0.55).max(1.0);
    let mut lines = 1;
    let mut used = 0.0f32;
    for word in text.split(' ') {
        let word_w = word.chars().count() as f32 * advance;
        if used > 0.0 && used + advance + word_w > width {
            lines += 1;
            used = word_w;
        } else {
            used = if used > 0.0 {
                used + advance + word_w
            } else {
                word_w
            };
        }
        // A word with no space in it is cut where the column ends: count the
        // extra lines it takes, and start the next one at what is left.
        while used > width {
            lines += 1;
            used -= width;
        }
    }
    lines
}

/// `text` broken at spaces into lines no wider than `width` dp. A word with no
/// space in it (a link, a server's complaint) is cut where the column ends
/// rather than drawn over the page's margin.
fn wrap(ui: &Ui, face: Face, size: f32, text: &str, width: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split(' ') {
        let joins = lines
            .last()
            .is_some_and(|line| ui.measure(face, size, &format!("{line} {word}")) <= width);
        if !joins {
            // The word starts a line of its own, cut if it is wider than the column.
            push_cut(ui, face, size, word, width, &mut lines);
            continue;
        }
        if let Some(line) = lines.last_mut() {
            line.push(' ');
            line.push_str(word);
        }
    }
    lines
}

/// `word` appended to `lines`, cut into as many pieces as the column takes.
fn push_cut(ui: &Ui, face: Face, size: f32, word: &str, width: f32, lines: &mut Vec<String>) {
    if word.is_empty() {
        return;
    }
    let mut rest = word;
    while ui.measure(face, size, rest) > width {
        let mut at = None;
        for (i, ch) in rest.char_indices() {
            if i > 0 && ui.measure(face, size, &rest[..i + ch.len_utf8()]) > width {
                at = Some(i);
                break;
            }
        }
        let Some(i) = at else {
            // Not even the first character alone fits: keep the word whole.
            break;
        };
        lines.push(rest[..i].to_owned());
        rest = &rest[i..];
    }
    lines.push(rest.to_owned());
}

/// `text` cut to what fits `room` dp, with an ellipsis when it was cut: a device
/// code or a name never draws over the page's margin.
fn clip(ui: &Ui, face: Face, size: f32, text: &str, room: f32) -> String {
    if ui.measure(face, size, text) <= room {
        return text.to_owned();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let mut candidate = out.clone();
        candidate.push(ch);
        candidate.push('…');
        if ui.measure(face, size, &candidate) > room {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{HudAtlas, HudBatch};

    /// One screen in every state, with two Ely.by forms: with and without 2FA.
    fn states() -> Vec<BootState> {
        vec![
            BootState::Choose,
            BootState::Resuming {
                name: "Steve".into(),
            },
            BootState::Code {
                code: "ABCD1234".into(),
                url: "microsoft.com/link".into(),
                expires_in_secs: 900,
            },
            BootState::Ely {
                login: "me@example.org".into(),
                password_len: 12,
                otp: Some("123456".into()),
                focus: ElyField::Password,
                error: None,
            },
            BootState::Ely {
                login: String::new(),
                password_len: 0,
                otp: None,
                focus: ElyField::Login,
                error: Some("Wrong password".into()),
            },
            BootState::Signed {
                name: "Steve".into(),
                kind: AccountKind::Microsoft,
                note: None,
            },
            BootState::Signed {
                name: "Steve".into(),
                kind: AccountKind::Ely,
                note: Some("Offline for now.".into()),
            },
            BootState::Files {
                done_bytes: 130_000_000,
                total_bytes: 520_000_000,
                step: "client.jar".into(),
            },
            BootState::Failed {
                message: "No connection".into(),
            },
            BootState::OfflineName {
                name: "Steve".into(),
                error: None,
            },
            BootState::OfflineName {
                name: String::new(),
                error: Some("A name is 1-16 letters, digits or _.".into()),
            },
            BootState::Offline {
                name: "Steve".into(),
            },
        ]
    }

    /// The buttons a state offers, in the order it draws them.
    fn expected(state: &BootState) -> Vec<BootAction> {
        use BootAction::*;
        match state {
            BootState::Choose => vec![SignInMicrosoft, SignInEly, PlayOffline],
            BootState::Resuming { .. } => vec![Cancel],
            BootState::Code { .. } => vec![OpenLink, CopyCode, Cancel],
            BootState::Ely { .. } => vec![Submit, Back],
            BootState::Signed { .. } => vec![Continue, SignOut],
            BootState::Failed { .. } => vec![Retry, Back],
            BootState::OfflineName { .. } => vec![Submit, Back],
            BootState::Files { .. } | BootState::Offline { .. } => Vec::new(),
        }
    }

    /// A window of `size` physical pixels at `scale`: what a phone and a PC give.
    const WINDOWS: [([f32; 2], f32); 5] = [
        ([854.0, 480.0], 3.0),
        ([2400.0, 1080.0], 3.0),
        ([1040.0, 700.0], 1.0),
        ([640.0, 360.0], 1.0),
        ([320.0, 240.0], 1.0),
    ];

    #[test]
    fn every_button_answers_its_action_at_its_centre() {
        for state in states() {
            let screen = BootScreen::new(state.clone());
            for (size, scale) in WINDOWS {
                let layout = screen.layout(size, scale);
                let actions: Vec<BootAction> =
                    layout.buttons.iter().map(|(a, _)| a.clone()).collect();
                assert_eq!(actions, expected(&state), "{state:?} at {size:?}");
                for (action, rect) in &layout.buttons {
                    let centre = (
                        (rect.x + rect.w / 2.0) * scale,
                        (rect.y + rect.h / 2.0) * scale,
                    );
                    assert_eq!(
                        screen.click(centre),
                        Some(action.clone()),
                        "{state:?} at {size:?}"
                    );
                }
                // The window's corners are nobody's button.
                assert_eq!(screen.click((0.0, 0.0)), None, "{state:?} at {size:?}");
                let corner = (size[0] - 1.0, size[1] - 1.0);
                assert_eq!(screen.click(corner), None, "{state:?} at {size:?}");
            }
        }
    }

    #[test]
    fn nothing_leaves_the_screen() {
        for state in states() {
            let screen = BootScreen::new(state.clone());
            for (size, scale) in WINDOWS {
                let layout = screen.layout(size, scale);
                let (w, h) = (size[0] / scale, size[1] / scale);
                assert!(layout.fit > 0.0 && layout.fit <= 1.0, "{state:?}");
                let mut rects = vec![layout.title, layout.column];
                rects.extend(layout.paragraphs.iter().copied());
                rects.extend(layout.code);
                rects.extend(layout.error);
                rects.extend(layout.progress);
                rects.extend(layout.status);
                rects.extend(layout.fields.iter().map(|(_, rect)| *rect));
                rects.extend(layout.buttons.iter().map(|(_, rect)| *rect));
                if let Some((band, keys)) = layout.keyboard {
                    rects.push(band);
                    rects.push(Rect::new(keys.x, keys.y, keys.w, keys.h));
                }
                for rect in rects {
                    assert!(
                        rect.x >= -0.5 && rect.y >= -0.5,
                        "{state:?} {size:?}: {rect:?}"
                    );
                    assert!(
                        rect.x + rect.w <= w + 0.5,
                        "{state:?} {size:?}: {rect:?} past {w}"
                    );
                    assert!(
                        rect.y + rect.h <= h + 0.5,
                        "{state:?} {size:?}: {rect:?} past {h}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_form_s_fields_take_the_focus_by_tap() {
        let screen = BootScreen::new(BootState::Ely {
            login: "me@example.org".into(),
            password_len: 7,
            otp: Some("123456".into()),
            focus: ElyField::Login,
            error: None,
        });
        let layout = screen.layout([2400.0, 1080.0], 3.0);
        let fields: Vec<ElyField> = layout.fields.iter().map(|(f, _)| *f).collect();
        assert_eq!(fields, [ElyField::Login, ElyField::Password, ElyField::Otp]);
        for (field, rect) in &layout.fields {
            let centre = ((rect.x + rect.w / 2.0) * 3.0, (rect.y + rect.h / 2.0) * 3.0);
            assert_eq!(screen.click(centre), Some(BootAction::FocusField(*field)));
        }
        // An account without two-factor sign-in has no code field.
        let mut no_otp = screen;
        no_otp.set_state(BootState::Ely {
            login: String::new(),
            password_len: 0,
            otp: None,
            focus: ElyField::Login,
            error: None,
        });
        let layout = no_otp.layout([2400.0, 1080.0], 3.0);
        assert_eq!(layout.fields.len(), 2);
    }

    /// Every key of the form's keyboard belongs to the keyboard: a typing one answers a
    /// `Typed`, Shift and the symbols key change the keys, and Done sends the form.
    #[test]
    fn the_form_s_keyboard_types_into_the_focused_field() {
        let screen = BootScreen::new(BootState::Ely {
            login: "me@example.org".into(),
            password_len: 7,
            otp: None,
            focus: ElyField::Login,
            error: None,
        });
        let keyboard = Keyboard::new(false, false);
        let layout = screen.layout([2400.0, 1080.0], 3.0);
        let (band, keys) = layout.keyboard.expect("the form has keys");
        assert!(band.y >= 0.0 && band.y + band.h <= 360.0, "{band:?}");
        // The key the grid opens on types its letter.
        let [x, y, w, h] = keyboard.key_rect(1, 0, keys).expect("q");
        let centre = ((x + w / 2.0) * 3.0, (y + h / 2.0) * 3.0);
        assert_eq!(
            screen.click(centre),
            Some(BootAction::Typed(Output::Text("q".into())))
        );
        // Every key of the grid belongs to the keyboard: a typing key answers a Typed,
        // Shift and the layer key only change the keys, Done is the form's Submit.
        // None of them ever answers a field or a button's action.
        let mut seen = 0;
        let mut done = false;
        for row in 0..5 {
            for col in 0..10 {
                let Some([x, y, w, h]) = keyboard.key_rect(row, col, keys) else {
                    continue;
                };
                let centre = ((x + w / 2.0) * 3.0, (y + h / 2.0) * 3.0);
                let before = screen.typing.borrow().clone();
                match screen.click(centre) {
                    Some(BootAction::Typed(_)) => {}
                    Some(BootAction::Submit) => done = true,
                    None => assert_ne!(
                        *screen.typing.borrow(),
                        before,
                        "row {row} col {col}: the tap neither typed nor changed the keys"
                    ),
                    other => panic!("row {row} col {col} reached the page: {other:?}"),
                }
                seen += 1;
            }
        }
        // 45 of the 50 cells are keys: the grid has five rows of ten, its last
        // row five keys wide (Shift, symbols, Space, Delete, Done).
        assert_eq!(seen, 45, "every key of the one-line grid");
        assert!(done, "the Done key submits the form");
        // A tap beside the keys asks nothing of the page.
        let beside = ((keys.x + keys.w + 40.0) * 3.0, (keys.y + 10.0) * 3.0);
        assert_eq!(screen.click(beside), None);
        // The keys sit under a field, never over it.
        for (_, rect) in &layout.fields {
            assert!(rect.y + rect.h <= band.y + 0.5, "{rect:?} over the keys");
        }
    }

    #[test]
    fn taps_on_keys_type_and_the_buttons_and_fields_still_answer() {
        let screen = BootScreen::new(BootState::Ely {
            login: String::new(),
            password_len: 0,
            otp: None,
            focus: ElyField::Login,
            error: None,
        });
        let keyboard = Keyboard::new(false, false);
        for (size, scale) in [([854.0, 480.0], 3.0), ([2400.0, 1080.0], 3.0)] {
            // The keys as the form opens them, letters and no shift, for each window: the
            // taps below change the grid.
            *screen.typing.borrow_mut() = Keyboard::new(false, false);
            let layout = screen.layout(size, scale);
            let (band, keys) = layout.keyboard.expect("the form has keys");
            let (w, h) = (size[0] / scale, size[1] / scale);
            assert!(keys.y >= 0.0 && keys.y + keys.h <= h, "{keys:?} past {h}");
            assert!(keys.x >= 0.0 && keys.x + keys.w <= w, "{keys:?} past {w}");
            // Nothing on the page is in the band the keys take.
            let mut rects: Vec<Rect> = layout.fields.iter().map(|(_, r)| *r).collect();
            rects.extend(layout.buttons.iter().map(|(_, r)| *r));
            for rect in rects {
                assert!(
                    rect.y + rect.h <= band.y + 0.5,
                    "{size:?}: {rect:?} over the keys at {band:?}"
                );
            }
            // A letter, tapped where it is drawn, types that letter.
            let [x, y, w, h] = keyboard.key_rect(1, 0, keys).expect("q");
            let centre = ((x + w / 2.0) * scale, (y + h / 2.0) * scale);
            assert_eq!(
                screen.click(centre),
                Some(BootAction::Typed(Output::Text("q".into()))),
                "{size:?}"
            );
            // The fields and the buttons answer their own actions.
            for (field, rect) in &layout.fields {
                let centre = (
                    (rect.x + rect.w / 2.0) * scale,
                    (rect.y + rect.h / 2.0) * scale,
                );
                assert_eq!(screen.click(centre), Some(BootAction::FocusField(*field)));
            }
            for (action, rect) in &layout.buttons {
                let centre = (
                    (rect.x + rect.w / 2.0) * scale,
                    (rect.y + rect.h / 2.0) * scale,
                );
                assert_eq!(screen.click(centre), Some(action.clone()));
            }
        }
    }

    #[test]
    fn the_done_key_sends_the_form_rather_than_typing() {
        let screen = BootScreen::new(BootState::Ely {
            login: "me".into(),
            password_len: 1,
            otp: None,
            focus: ElyField::Password,
            error: None,
        });
        let keyboard = Keyboard::new(false, false);
        let layout = screen.layout([1040.0, 700.0], 1.0);
        let (_, keys) = layout.keyboard.expect("the form has keys");
        // The last key of the last row is Done.
        let last = 4;
        let [x, y, w, h] = keyboard.key_rect(4, last, keys).expect("Done");
        let centre = (x + w / 2.0, y + h / 2.0);
        assert_eq!(screen.click(centre), Some(BootAction::Submit));
        // A letter is still typed, not submitted.
        let [x, y, w, h] = keyboard.key_rect(1, 0, keys).expect("q");
        assert_eq!(
            screen.click((x + w / 2.0, y + h / 2.0)),
            Some(BootAction::Typed(Output::Text("q".into())))
        );
    }

    #[test]
    fn the_form_s_keys_leave_the_fields_and_buttons_their_own_taps() {
        let screen = BootScreen::new(BootState::Ely {
            login: "me".into(),
            password_len: 1,
            otp: Some("123456".into()),
            focus: ElyField::Login,
            error: Some("Wrong password".into()),
        });
        let layout = screen.layout([1040.0, 700.0], 1.0);
        let (band, _) = layout.keyboard.expect("the form has keys");
        // Every field and button is above the band, so its own tap still answers
        // what it always did.
        for (field, rect) in &layout.fields {
            assert!(rect.y + rect.h <= band.y, "{rect:?} reaches into the keys");
            let centre = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
            assert_eq!(screen.click(centre), Some(BootAction::FocusField(*field)));
        }
        for (action, rect) in &layout.buttons {
            assert!(rect.y + rect.h <= band.y, "{rect:?} reaches into the keys");
            let centre = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
            assert_eq!(screen.click(centre), Some(action.clone()));
        }
    }

    #[test]
    fn only_the_forms_have_keys() {
        for state in states() {
            let screen = BootScreen::new(state.clone());
            let layout = screen.layout([2400.0, 1080.0], 3.0);
            let form = matches!(state, BootState::Ely { .. } | BootState::OfflineName { .. });
            assert_eq!(layout.keyboard.is_some(), form, "{state:?}");
        }
    }

    #[test]
    fn the_password_is_drawn_as_stars() {
        let screen = BootScreen::new(BootState::Ely {
            login: "me".into(),
            password_len: 12,
            otp: None,
            focus: ElyField::Login,
            error: None,
        });
        let (content, missed) =
            lang::scoped(lang::Language::load(&[], "en_us"), || screen.content());
        assert_eq!(missed, Vec::<String>::new());
        let field = |wanted: ElyField| {
            content
                .fields
                .iter()
                .find(|(field, _, _)| *field == wanted)
                .map(|(_, caption, text)| (caption.clone(), text.clone()))
                .expect("the field is there")
        };
        assert_eq!(
            field(ElyField::Login),
            ("Login".to_owned(), "me".to_owned())
        );
        assert_eq!(field(ElyField::Password).1, "************");
        assert_eq!(content.fields.len(), 2);
    }

    #[test]
    fn the_download_says_how_much_is_done() {
        let screen = BootScreen::new(BootState::Files {
            done_bytes: 130_000_000,
            total_bytes: 520_000_000,
            step: "client.jar".into(),
        });
        let (content, missed) =
            lang::scoped(lang::Language::load(&[], "en_us"), || screen.content());
        assert_eq!(missed, Vec::<String>::new());
        assert_eq!(content.progress, Some(0.25));
        assert_eq!(
            content.status.as_deref(),
            Some("client.jar: 130.0 of 520.0 MB")
        );
        let screen = BootScreen::new(BootState::Files {
            done_bytes: 0,
            total_bytes: 0,
            step: String::new(),
        });
        let (content, _) = lang::scoped(lang::Language::load(&[], "en_us"), || screen.content());
        assert_eq!(content.status.as_deref(), Some("Starting the download..."));
    }

    #[test]
    fn every_state_says_its_strings_in_both_languages() {
        for code in ["en_us", "ru_ru"] {
            let (said, missed) = lang::scoped(lang::Language::load(&[], code), || {
                let mut said = Vec::new();
                for state in states() {
                    let screen = BootScreen::new(state);
                    let content = screen.content();
                    said.push(content.title);
                    said.extend(content.lines);
                    said.extend(content.error);
                    said.extend(content.status);
                    said.extend(content.code);
                    said.extend(
                        content
                            .fields
                            .into_iter()
                            .flat_map(|(_, caption, text)| [caption, text]),
                    );
                    said.extend(content.buttons.into_iter().map(|(label, _)| label));
                }
                said
            });
            assert_eq!(missed, Vec::<String>::new(), "{code}");
            for text in &said {
                assert!(
                    !text.starts_with("horizon.boot."),
                    "{code}: {text} is a key, not a translation"
                );
            }
        }
        // Russian is really Russian, not the English fallback: a key left out of
        // ru_ru.json would come back in English and this would catch it.
        let ru = lang::Language::load(&[], "ru_ru");
        assert_eq!(
            ru.get("horizon.boot.choose.microsoft"),
            Some("Войти через Microsoft")
        );
        assert_eq!(
            ru.get("horizon.boot.files.title"),
            Some("Скачиваем файлы игры")
        );
    }

    #[test]
    fn every_state_draws_on_the_boot_atlas() {
        // No prepared files at all: the atlas the screen draws with before a pack.
        let atlas = HudAtlas::boot();
        for state in states() {
            let screen = BootScreen::new(state);
            let mut batch = HudBatch::scratch([960, 720]);
            let mut ui = Ui::new(&atlas, &mut batch, (10.0, 10.0));
            screen.draw(&mut ui);
            ui.finish();
            assert!(!batch.vertices.is_empty());
        }
    }

    /// The keys band draws on the boot atlas too, at the pad's size and shrunk, with the
    /// symbols a tap turns on: nothing in it reads a game file.
    #[test]
    fn the_keys_draw_on_the_boot_atlas() {
        use crate::gamepad::Button;
        let atlas = HudAtlas::boot();
        let screen = BootScreen::new(BootState::Ely {
            login: "me@example.org".into(),
            password_len: 7,
            otp: None,
            focus: ElyField::Login,
            error: None,
        });
        for (size, scale) in [([854.0, 480.0], 3.0), ([2400.0, 1080.0], 3.0)] {
            let (_, keys) = screen.layout(size, scale).keyboard.expect("keys");
            // The symbols grid, Shift lit, draws the same way.
            screen.typing.borrow_mut().button(Button::RightBumper);
            screen.typing.borrow_mut().button(Button::LeftTrigger);
            let mut batch = HudBatch::scratch([size[0] as u32, size[1] as u32]);
            let mut ui = Ui::new(&atlas, &mut batch, (10.0, 10.0));
            screen.typing.borrow().draw_touch(&mut ui, keys);
            ui.finish();
            assert!(!batch.vertices.is_empty(), "{size:?}");
            screen.typing.borrow_mut().button(Button::RightBumper);
        }
    }

    #[test]
    fn a_small_screen_loses_a_sentence_before_it_loses_its_buttons() {
        // A desktop window shows the whole page at its full size.
        let state = BootState::Code {
            code: "ABCD1234".into(),
            url: "microsoft.com/link".into(),
            expires_in_secs: 900,
        };
        let screen = BootScreen::new(state.clone());
        let roomy = screen.layout([1600.0, 1000.0], 1.0);
        assert_eq!(roomy.paragraphs.len(), 3, "all the text");
        assert_eq!(roomy.fit, 1.0, "nothing shrunk");
        // A phone held sideways (2400x1080 at scale 3 is 800x360 dp) gives way:
        // the type shrinks, and if that were not enough paragraphs would go.
        // Every button stays on the screen, and the type never gets unreadable.
        let tight = screen.layout([2400.0, 1080.0], 3.0);
        assert!(
            tight.fit < 1.0 || tight.paragraphs.len() < 3,
            "{tight:?} did not give way"
        );
        assert!(tight.fit >= FIT_FLOOR, "{tight:?}");
        assert_eq!(tight.buttons.len(), 3, "every button still there");
        for (_, rect) in &tight.buttons {
            assert!(rect.y >= 0.0 && rect.y + rect.h <= 360.0, "{rect:?}");
        }
    }

    #[test]
    fn a_long_word_is_cut_inside_the_column() {
        let atlas = HudAtlas::boot();
        let mut batch = HudBatch::scratch([960, 720]);
        let ui = Ui::new(&atlas, &mut batch, (0.0, 0.0));
        let room = 200.0;
        let lines = wrap(
            &ui,
            Face::Regular,
            15.0,
            "See https://example.org/very/long/address/that/never/ends for the page",
            room,
        );
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(
                ui.measure(Face::Regular, 15.0, line) <= room,
                "{line:?} is wider than {room}"
            );
        }
        assert!(lines.concat().contains("https://"));
    }

    #[test]
    fn a_tap_is_read_from_the_layout_drawn_last() {
        let mut screen = BootScreen::new(BootState::Choose);
        // Nothing laid out yet: a tap answers nothing.
        assert_eq!(screen.click((100.0, 100.0)), None);
        let layout = screen.layout([1040.0, 700.0], 1.0);
        let (_, rect) = layout
            .buttons
            .iter()
            .find(|(action, _)| action == &BootAction::PlayOffline)
            .expect("the offline button");
        let centre = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
        assert_eq!(screen.click(centre), Some(BootAction::PlayOffline));
        // Another state forgets the old buttons: the same tap now hits nothing.
        screen.set_state(BootState::Files {
            done_bytes: 0,
            total_bytes: 0,
            step: String::new(),
        });
        assert_eq!(screen.click(centre), None);
    }
}
