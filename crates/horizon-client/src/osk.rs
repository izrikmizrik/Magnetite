//! The on-screen keyboard a controller types with in focus mode (Horizon's
//! own; docs/controllers.md, "Text fields"): a QWERTY grid and a symbols
//! grid in Horizon's design language, moved over with the D-pad or stick.
//! A types the key, X deletes, Y is a space, Start is the Done key, B closes it.
//! A field of many lines (a book's page, a sign) gets a New Line key, so Done
//! never makes a line. Over chat, RT is Tab (fill in the suggestion, again for
//! the next) and the right stick moves through the suggestions.
//! It only produces [`Output`]s; the game feeds them to the focused field
//! the way typed keys reach it.
//!
//! The same grid takes a finger ([`Keyboard::tap`]) where the screen has no
//! pad, as the boot screen's sign-in form does: a tap presses the key under
//! it, and [`Keyboard::draw_touch`] draws the keys without the pad's hints.
use crate::gamepad::{Button, MenuJob};
use crate::hud_kit::hint::HintRow;
use crate::pad_glyph::{Glyph, Glyphs};
use crate::ui_theme::{self, Face, Ui, color};

/// A label of the keyboard: the language table's, or the English text.
fn tr(key: &str, english: &str) -> String {
    crate::lang::get_or(key, english)
}

/// What a key press does to the focused field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Output {
    Text(String),
    Backspace,
    /// The Enter key: a new line on a field of many lines.
    Enter,
    /// The typing is done: the field's own Done (a book's, a sign's), or Enter on a field of
    /// one line.
    Done,
    /// Tab: fill in the selected suggestion (vanilla's `CommandSuggestions`), again for the next.
    Complete,
    /// Up and Down: the suggestion before or after (or chat's history, as the arrow keys).
    Previous,
    Next,
    /// The keyboard closes; the field keeps its text.
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Char(char),
    Shift,
    Layer,
    Space,
    Backspace,
    NewLine,
    Done,
}

/// A row of keys and their widths in key units (every row is 10 wide).
type Row = &'static [(Key, f32)];

const fn c(ch: char) -> (Key, f32) {
    (Key::Char(ch), 1.0)
}
const BOTTOM: Row = &[
    (Key::Shift, 1.5),
    (Key::Layer, 1.5),
    (Key::Space, 4.0),
    (Key::Backspace, 1.5),
    (Key::Done, 1.5),
];
/// The bottom row on a field of many lines: New Line beside Done.
const BOTTOM_LINES: Row = &[
    (Key::Shift, 1.5),
    (Key::Layer, 1.5),
    (Key::Space, 2.5),
    (Key::NewLine, 1.5),
    (Key::Backspace, 1.5),
    (Key::Done, 1.5),
];
const LETTERS: [Row; 5] = [
    &[
        c('1'),
        c('2'),
        c('3'),
        c('4'),
        c('5'),
        c('6'),
        c('7'),
        c('8'),
        c('9'),
        c('0'),
    ],
    &[
        c('q'),
        c('w'),
        c('e'),
        c('r'),
        c('t'),
        c('y'),
        c('u'),
        c('i'),
        c('o'),
        c('p'),
    ],
    &[
        c('a'),
        c('s'),
        c('d'),
        c('f'),
        c('g'),
        c('h'),
        c('j'),
        c('k'),
        c('l'),
        c('\''),
    ],
    &[
        c('z'),
        c('x'),
        c('c'),
        c('v'),
        c('b'),
        c('n'),
        c('m'),
        c(','),
        c('.'),
        c('-'),
    ],
    BOTTOM,
];
const SYMBOLS: [Row; 5] = [
    &[
        c('!'),
        c('@'),
        c('#'),
        c('$'),
        c('%'),
        c('^'),
        c('&'),
        c('*'),
        c('('),
        c(')'),
    ],
    &[
        c('/'),
        c('\\'),
        c('_'),
        c('='),
        c('+'),
        c('['),
        c(']'),
        c('{'),
        c('}'),
        c('|'),
    ],
    &[
        c(';'),
        c(':'),
        c('"'),
        c('<'),
        c('>'),
        c('?'),
        c('~'),
        c('`'),
        c('§'),
        c('°'),
    ],
    &[
        c('é'),
        c('è'),
        c('à'),
        c('ç'),
        c('ù'),
        c('ü'),
        c('ö'),
        c('ä'),
        c('ß'),
        c('ñ'),
    ],
    BOTTOM,
];
const LETTERS_LINES: [Row; 5] = [LETTERS[0], LETTERS[1], LETTERS[2], LETTERS[3], BOTTOM_LINES];
const SYMBOLS_LINES: [Row; 5] = [SYMBOLS[0], SYMBOLS[1], SYMBOLS[2], SYMBOLS[3], BOTTOM_LINES];

/// The panel's padding around the keys and the hints, the gap between them, and a hint line's
/// height in its text size, all in dp (a line box is 1.45 sizes; the pack's font can be a
/// little taller).
const PANEL_PAD: f32 = 12.0;
const HINT_GAP: f32 = 8.0;
const HINT_LINE: f32 = 1.6;
/// Between two hints on a line.
const HINT_SEP: &str = "   ";

/// The keys' board, in dp: one key unit wide, the gap beside it, a key's height, a row's width
/// (ten units and nine gaps) and the five rows' height. Drawing and tapping both read these,
/// so a key is never drawn where a finger cannot press it.
const UNIT: f32 = 50.0;
const GAP: f32 = 6.0;
const KEY_H: f32 = 38.0;
const BOARD_W: f32 = 10.0 * UNIT + 9.0 * GAP;
const BOARD_H: f32 = 5.0 * KEY_H + 4.0 * GAP;
/// The clear space a key keeps at either side of its label.
const LABEL_PAD: f32 = 4.0;
/// How far the pad keeps its panel from the screen's edge.
const EDGE: f32 = 16.0;

/// The box the keys fill, in dp: what [`Keyboard::draw_touch`] paints and
/// [`Keyboard::tap`] reads, so the picture and the finger never disagree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keys {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// How much the board is shrunk (1 at the pad's own size).
    scale: f32,
}

impl Keys {
    /// The board that fits `area` ([x, y, width, height] in dp), its panel's padding kept
    /// clear: shrunk as one where the room is smaller than the pad's board, never larger.
    pub fn within(area: [f32; 4]) -> Self {
        let [x, y, w, h] = area;
        let inner_w = (w - 2.0 * PANEL_PAD).max(1.0);
        let inner_h = (h - 2.0 * PANEL_PAD).max(1.0);
        let scale = (inner_w / BOARD_W).min(inner_h / BOARD_H).min(1.0);
        let (kw, kh) = (BOARD_W * scale, BOARD_H * scale);
        Self {
            x: x + ((w - kw) / 2.0).round(),
            y: y + ((h - kh) / 2.0).round(),
            w: kw,
            h: kh,
            scale,
        }
    }
    /// A key unit, the gap beside it, and a key's height at this box's scale.
    fn pitch(&self) -> (f32, f32, f32) {
        (UNIT * self.scale, GAP * self.scale, KEY_H * self.scale)
    }
}

/// `items` on as few lines as fit `width` (as `measure` measures a line), in order. An item wider
/// than `width` breaks at its spaces onto lines of its own, its button's print kept with the
/// first word ("B Close"); a line still wider than `width` (one long word) is drawn smaller.
fn wrap_hints(
    items: impl IntoIterator<Item = String>,
    width: f32,
    measure: impl Fn(&str) -> f32,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    // The last line ends a hint that was broken: the next hint starts a line of its own.
    let mut sealed = false;
    for item in items {
        if measure(&item) > width {
            let mut line = String::new();
            let mut words = item.split(' ').filter(|w| !w.is_empty());
            // The button's print never stands alone on a line.
            let first = match (words.next(), words.next()) {
                (Some(glyph), Some(word)) => vec![format!("{glyph} {word}")],
                (Some(glyph), None) => vec![glyph.to_owned()],
                _ => Vec::new(),
            };
            for word in first.into_iter().chain(words.map(str::to_owned)) {
                let word = word.as_str();
                if !line.is_empty() && measure(&format!("{line} {word}")) > width {
                    lines.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            lines.push(line);
            sealed = true;
            continue;
        }
        match lines.last_mut() {
            Some(line) if !sealed && measure(&format!("{line}{HINT_SEP}{item}")) <= width => {
                line.push_str(HINT_SEP);
                line.push_str(&item);
            }
            _ => lines.push(item),
        }
        sealed = false;
    }
    lines
}

/// The right stick, up (+) or down (-), past this picks the suggestion before or after; it picks
/// again once back under [`STICK_REST`].
const STICK_PICK: f32 = 0.6;
const STICK_REST: f32 = 0.3;

/// The keyboard's state while open.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyboard {
    row: usize,
    col: usize,
    symbols: bool,
    /// The next letter is upper case (Shift), or every letter (double Shift).
    shift: bool,
    caps: bool,
    /// The field takes many lines: a New Line key, and Done finishes.
    lines: bool,
    /// The field has suggestions (chat): RT fills them in, the right stick picks one.
    suggests: bool,
    /// The right stick has picked and not come back to rest.
    stick_out: bool,
}
impl Default for Keyboard {
    fn default() -> Self {
        // Focus starts on `q`.
        Self {
            row: 1,
            col: 0,
            symbols: false,
            shift: false,
            caps: false,
            lines: false,
            suggests: false,
            stick_out: false,
        }
    }
}

impl Keyboard {
    /// A keyboard over a field of many `lines` (New Line beside Done), and one with
    /// suggestions (`suggests`: RT and the right stick reach them).
    pub fn new(lines: bool, suggests: bool) -> Self {
        Self {
            lines,
            suggests,
            ..Self::default()
        }
    }
    fn rows(&self) -> &'static [Row; 5] {
        match (self.symbols, self.lines) {
            (false, false) => &LETTERS,
            (true, false) => &SYMBOLS,
            (false, true) => &LETTERS_LINES,
            (true, true) => &SYMBOLS_LINES,
        }
    }
    /// The right stick's height (up is +): the suggestion before or after, once per push.
    pub fn stick(&mut self, y: f32) -> Option<Output> {
        if !self.suggests {
            return None;
        }
        if self.stick_out {
            self.stick_out = y.abs() >= STICK_REST;
            return None;
        }
        if y.abs() < STICK_PICK {
            return None;
        }
        self.stick_out = true;
        Some(if y > 0.0 {
            Output::Previous
        } else {
            Output::Next
        })
    }
    /// A key's left edge and width in key units.
    fn span(row: Row, col: usize) -> (f32, f32) {
        let left: f32 = row[..col].iter().map(|k| k.1).sum();
        (left, row[col].1)
    }
    /// A key's rectangle in dp, `[x, y, width, height]`: the grid's row and column on the board
    /// `at`, which [`Keyboard::tap`] reads as the drawing paints it.
    pub fn key_rect(&self, row: usize, col: usize, at: Keys) -> Option<[f32; 4]> {
        let line = *self.rows().get(row)?;
        if col >= line.len() {
            return None;
        }
        let (left, units) = Self::span(line, col);
        let (unit, gap, key_h) = at.pitch();
        let x = at.x + left * (unit + gap);
        let y = at.y + row as f32 * (key_h + gap);
        let w = units * unit + (units - 1.0) * gap;
        Some([x, y, w, key_h])
    }
    /// A tap at `pos` (dp, the screen `at` was laid out in): the key under the finger is
    /// pressed exactly as [`Keyboard::press`] presses the focused one, so a tap types, shifts,
    /// changes the grid, deletes, spaces or finishes. `None` between the keys and outside them.
    pub fn tap(&mut self, pos: [f32; 2], at: Keys) -> Option<Output> {
        let (row, col) = self.key_at(pos, at)?;
        self.row = row;
        self.col = col;
        self.press()
    }
    /// Which key of the grid is under `pos`, if any.
    fn key_at(&self, [px, py]: [f32; 2], at: Keys) -> Option<(usize, usize)> {
        for (row, line) in self.rows().iter().enumerate() {
            for col in 0..line.len() {
                let Some([x, y, w, h]) = self.key_rect(row, col, at) else {
                    continue;
                };
                if px >= x && px < x + w && py >= y && py < y + h {
                    return Some((row, col));
                }
            }
        }
        None
    }
    fn key(&self) -> Key {
        self.rows()[self.row][self.col].0
    }
    /// A D-pad step: across a row with wrap-around; up and down to the key
    /// under the focused key's centre (wrapping top and bottom).
    pub fn step(&mut self, button: Button) {
        let rows = self.rows();
        let row = rows[self.row];
        match button {
            Button::DPadLeft => self.col = (self.col + row.len() - 1) % row.len(),
            Button::DPadRight => self.col = (self.col + 1) % row.len(),
            Button::DPadUp | Button::DPadDown => {
                let (left, width) = Self::span(row, self.col);
                let centre = left + width / 2.0;
                self.row = if button == Button::DPadUp {
                    (self.row + rows.len() - 1) % rows.len()
                } else {
                    (self.row + 1) % rows.len()
                };
                let next = rows[self.row];
                self.col = (0..next.len())
                    .find(|&i| {
                        let (l, w) = Self::span(next, i);
                        centre < l + w
                    })
                    .unwrap_or(next.len() - 1);
            }
            _ => {}
        }
    }
    fn letter(&self, ch: char) -> String {
        if self.shift || self.caps {
            ch.to_uppercase().collect()
        } else {
            ch.to_string()
        }
    }
    /// A pressed: the focused key's output (Shift and the symbols switch only
    /// change the keyboard).
    pub fn press(&mut self) -> Option<Output> {
        match self.key() {
            Key::Char(ch) => {
                let text = self.letter(ch);
                self.shift = false;
                Some(Output::Text(text))
            }
            Key::Shift => {
                // Shift, then Shift again locks capitals; a third press lets go.
                (self.shift, self.caps) = match (self.shift, self.caps) {
                    (false, false) => (true, false),
                    (true, false) => (false, true),
                    _ => (false, false),
                };
                None
            }
            Key::Layer => {
                self.symbols = !self.symbols;
                self.col = self.col.min(self.rows()[self.row].len() - 1);
                None
            }
            Key::Space => Some(Output::Text(" ".into())),
            Key::Backspace => Some(Output::Backspace),
            Key::NewLine => Some(Output::Enter),
            Key::Done => Some(Output::Done),
        }
    }
    /// A pad button while the keyboard is open; `None` when it does nothing
    /// to the field (moving, Shift).
    /// Its jobs are menu jobs (`MenuJob`), the map its hint line's glyphs come from.
    pub fn button(&mut self, button: Button) -> Option<Output> {
        match MenuJob::of(button) {
            Some(MenuJob::Confirm) => self.press(),
            Some(MenuJob::Alternate) => Some(Output::Backspace),
            Some(MenuJob::Extra) => Some(Output::Text(" ".into())),
            Some(MenuJob::Start) => Some(Output::Done),
            Some(MenuJob::GroupNext) if self.suggests => Some(Output::Complete),
            Some(MenuJob::Back) => Some(Output::Close),
            Some(MenuJob::LeftStick | MenuJob::GroupPrev) => {
                self.shift = !self.shift;
                None
            }
            Some(MenuJob::PageNext | MenuJob::PagePrev) => {
                self.symbols = !self.symbols;
                self.col = self.col.min(self.rows()[self.row].len() - 1);
                None
            }
            _ if matches!(
                button,
                Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight
            ) =>
            {
                self.step(button);
                None
            }
            _ => None,
        }
    }

    /// The hint line under the keys ("A Type   X Delete ..."), wrapped onto as many lines as it
    /// needs to stay within `width`: the panel grows to hold them.
    fn hint_lines(&self, ui: &Ui, glyphs: Glyphs, width: f32) -> Vec<String> {
        let complete = self
            .suggests
            .then(|| (MenuJob::GroupNext, tr("horizon.osk.complete", "Complete")));
        let hints = [
            Some((MenuJob::Confirm, tr("horizon.osk.type", "Type"))),
            Some((MenuJob::Alternate, tr("horizon.osk.delete", "Delete"))),
            Some((MenuJob::Extra, tr("horizon.osk.space", "Space"))),
            Some((MenuJob::GroupPrev, tr("horizon.osk.shift", "Shift"))),
            complete,
            Some((MenuJob::PageNext, tr("horizon.osk.symbols", "Symbols"))),
            Some((MenuJob::Start, tr("gui.done", "Done"))),
            Some((MenuJob::Back, tr("horizon.osk.close", "Close"))),
        ];
        // This font draws no glyph art: each button by its print, "A Type".
        let items = hints
            .into_iter()
            .flatten()
            .map(|(job, what)| HintRow::new([Glyph::menu(job, glyphs)], what).text());
        wrap_hints(items, width, |line| {
            ui.measure(Face::Regular, ui_theme::small(), line)
        })
    }

    /// Draw the keyboard across the bottom of the screen, or the top when
    /// the field it types into is in the lower half.
    pub fn draw(&self, ui: &mut Ui, at_top: bool, glyphs: Glyphs) {
        let hints = self.hint_lines(ui, glyphs, BOARD_W);
        let line_h = HINT_LINE * ui_theme::small();
        let height = PANEL_PAD + BOARD_H + HINT_GAP + hints.len() as f32 * line_h + PANEL_PAD;
        let x0 = ((ui.w - BOARD_W) / 2.0).round();
        let y0 = if at_top { EDGE } else { ui.h - height - EDGE };
        let keys = Keys {
            x: x0,
            y: y0,
            w: BOARD_W,
            h: BOARD_H,
            scale: 1.0,
        };
        ui.rounded(
            x0 - PANEL_PAD,
            y0 - PANEL_PAD,
            BOARD_W + 2.0 * PANEL_PAD,
            height,
            ui_theme::radius_plate(),
            color::with_alpha(color::bg(), 0.94),
        );
        ui.border(
            x0 - PANEL_PAD,
            y0 - PANEL_PAD,
            BOARD_W + 2.0 * PANEL_PAD,
            height,
            ui_theme::radius_plate(),
            color::border(),
        );
        self.draw_keys(ui, keys, true);
        let top = y0 + BOARD_H + HINT_GAP;
        for (i, line) in hints.iter().enumerate() {
            // Only a line of one word wider than the panel is still too wide: smaller, to fit.
            let size = ui.fit(Face::Regular, ui_theme::small(), line, BOARD_W);
            ui.text_centered(
                Face::Regular,
                size,
                x0 + BOARD_W / 2.0,
                top + i as f32 * line_h,
                color::muted(),
                line,
            );
        }
    }

    /// Draw the keys alone across `at`, with no pad hints and no focused key: the keyboard of a
    /// screen a finger works, as the boot screen's sign-in form is. `at` is exactly what
    /// [`Keyboard::tap`] reads, so a key is never painted where a tap misses it.
    pub fn draw_touch(&self, ui: &mut Ui, at: Keys) {
        ui.rounded(
            at.x - PANEL_PAD,
            at.y - PANEL_PAD,
            at.w + 2.0 * PANEL_PAD,
            at.h + 2.0 * PANEL_PAD,
            ui_theme::radius_plate(),
            color::with_alpha(color::bg(), 0.94),
        );
        ui.border(
            at.x - PANEL_PAD,
            at.y - PANEL_PAD,
            at.w + 2.0 * PANEL_PAD,
            at.h + 2.0 * PANEL_PAD,
            ui_theme::radius_plate(),
            color::border(),
        );
        self.draw_keys(ui, at, false);
    }

    /// The keys of the current grid on the board `at`; `highlight` puts the pad's focus ring on
    /// the key A would press (a finger has no focus to show).
    fn draw_keys(&self, ui: &mut Ui, at: Keys, highlight: bool) {
        let (_, _, key_h) = at.pitch();
        for (r, row) in self.rows().iter().enumerate() {
            for (i, (key, _)) in row.iter().enumerate() {
                let Some([x, y, w, h]) = self.key_rect(r, i, at) else {
                    continue;
                };
                let focused = highlight && r == self.row && i == self.col;
                let lit = matches!(key, Key::Shift) && (self.shift || self.caps)
                    || matches!(key, Key::Layer) && self.symbols;
                let (plate, ink) = if focused {
                    (color::accent(), color::on_accent())
                } else if lit {
                    (color::hover(), color::text())
                } else {
                    (color::raised(), color::text())
                };
                // The pad's corner, shrunk with the board and never rounder than half a key.
                let radius = ui_theme::radius().min(KEY_H / 2.0) * at.scale;
                ui.rounded(x, y, w, h, radius, plate);
                let label = match key {
                    Key::Char(ch) => self.letter(*ch),
                    Key::Shift if self.caps => "CAPS".into(),
                    Key::Shift => tr("horizon.osk.shift", "Shift"),
                    Key::Layer if self.symbols => "abc".into(),
                    Key::Layer => "&123".into(),
                    Key::Space => tr("horizon.osk.space", "Space"),
                    Key::Backspace => tr("horizon.osk.delete", "Delete"),
                    Key::NewLine => tr("horizon.osk.new_line", "New Line"),
                    Key::Done => tr("gui.done", "Done"),
                };
                // Smaller when the label is wider than its key (a long translation).
                let room = w - 2.0 * LABEL_PAD * at.scale;
                let size = ui.fit(Face::Medium, ui_theme::body() * at.scale, &label, room);
                let lw = ui.measure(Face::Medium, size, &label);
                ui.text_line(
                    Face::Medium,
                    size,
                    key_h,
                    x + ((w - lw) / 2.0).round(),
                    y,
                    ink,
                    &label,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_ten_units_wide() {
        for rows in [&LETTERS, &SYMBOLS] {
            for row in rows {
                let units: f32 = row.iter().map(|k| k.1).sum();
                assert_eq!(units, 10.0);
            }
        }
    }

    #[test]
    fn types_shifts_and_moves() {
        let mut k = Keyboard::default();
        assert_eq!(k.press(), Some(Output::Text("q".into())));
        k.step(Button::DPadRight);
        k.button(Button::LeftTrigger);
        assert_eq!(k.press(), Some(Output::Text("W".into())));
        assert_eq!(k.press(), Some(Output::Text("w".into())), "one-shot shift");
        // Left wraps to the end of the row.
        k.col = 0;
        k.step(Button::DPadLeft);
        assert_eq!(k.press(), Some(Output::Text("p".into())));
        // Down from `p` to the row of `'`, then the bottom row: `Done` sits
        // under the right edge; up from the wide Space bar lands mid-row.
        k.step(Button::DPadDown);
        k.step(Button::DPadDown);
        k.step(Button::DPadDown);
        assert_eq!(k.press(), Some(Output::Done));
        k.col = 2;
        assert_eq!(k.press(), Some(Output::Text(" ".into())));
        k.step(Button::DPadUp);
        assert_eq!(k.press(), Some(Output::Text("n".into())));
        // Up from the top row wraps to the bottom row.
        k.row = 0;
        k.col = 0;
        k.step(Button::DPadUp);
        assert_eq!(k.key(), Key::Shift);
        // Shift twice locks capitals; the symbols grid has no case.
        k.press();
        k.press();
        k.row = 1;
        k.col = 0;
        assert_eq!(k.press(), Some(Output::Text("Q".into())));
        assert_eq!(k.press(), Some(Output::Text("Q".into())));
        k.button(Button::RightBumper);
        assert_eq!(k.press(), Some(Output::Text("/".into())));
        assert_eq!(k.button(Button::West), Some(Output::Backspace));
        assert_eq!(k.button(Button::East), Some(Output::Close));
        assert_eq!(k.button(Button::Start), Some(Output::Done));
        // No suggestions here: RT and the right stick do nothing.
        assert_eq!(k.button(Button::RightTrigger), None);
        assert_eq!(k.stick(1.0), None);
    }

    #[test]
    fn many_lines_have_a_new_line_key_and_done_still_finishes() {
        let mut k = Keyboard::new(true, false);
        // From `q` down to the bottom row's right end: Done; New Line two keys before it.
        for _ in 0..3 {
            k.step(Button::DPadDown);
        }
        k.col = k.rows()[4].len() - 1;
        assert_eq!(k.press(), Some(Output::Done));
        k.step(Button::DPadLeft);
        k.step(Button::DPadLeft);
        assert_eq!(k.press(), Some(Output::Enter));
        assert_eq!(k.button(Button::Start), Some(Output::Done));
        // The symbols grid keeps it.
        k.button(Button::RightBumper);
        assert_eq!(k.press(), Some(Output::Enter));
    }

    #[test]
    fn hints_wrap_to_the_panel_in_order() {
        // One unit per character: "A Type" is 6 wide, the separator 3.
        let items = || ["A Type", "X Delete", "Y Space", "B Close"].map(String::from);
        let measure = |s: &str| s.chars().count() as f32;
        assert_eq!(
            wrap_hints(items(), 100.0, measure),
            ["A Type   X Delete   Y Space   B Close"]
        );
        assert_eq!(
            wrap_hints(items(), 20.0, measure),
            ["A Type   X Delete", "Y Space   B Close"]
        );
        // Narrower than every hint: each on its own line, its print kept with its word, none lost.
        assert_eq!(
            wrap_hints(items(), 3.0, measure),
            ["A Type", "X Delete", "Y Space", "B Close"]
        );
    }

    #[test]
    fn a_hint_wider_than_the_panel_breaks_at_its_spaces() {
        let measure = |s: &str| s.chars().count() as f32;
        let items = ["A Type", "RT Fill in the next one", "B Close"].map(String::from);
        assert_eq!(
            wrap_hints(items, 10.0, measure),
            ["A Type", "RT Fill in", "the next", "one", "B Close"]
        );
        // The hint after a broken one starts its own line, though it would fit on the last.
        let items = ["RT Fill in the next one", "B Go"].map(String::from);
        assert_eq!(
            wrap_hints(items, 16.0, measure),
            ["RT Fill in the", "next one", "B Go"]
        );
        // One word wider than the panel stays with its button's print, on a line drawn smaller.
        let items = ["A Type", "X Unaussprechlich gross"].map(String::from);
        assert_eq!(
            wrap_hints(items, 10.0, measure),
            ["A Type", "X Unaussprechlich", "gross"]
        );
    }

    #[test]
    fn suggestions_take_rt_and_one_pick_per_push_of_the_right_stick() {
        let mut k = Keyboard::new(false, true);
        assert_eq!(k.button(Button::RightTrigger), Some(Output::Complete));
        assert_eq!(k.stick(0.3), None, "a lean isn't a push");
        assert_eq!(k.stick(0.9), Some(Output::Previous));
        assert_eq!(k.stick(1.0), None, "held, it picks once");
        assert_eq!(k.stick(0.4), None, "not yet back at rest");
        assert_eq!(k.stick(0.1), None);
        assert_eq!(k.stick(-0.9), Some(Output::Next));
    }

    /// A board at its own size, well inside a screen.
    fn board() -> Keys {
        Keys::within([40.0, 300.0, 640.0, 300.0])
    }

    /// The grid combinations a keyboard can show: single line or many, and with or
    /// without suggestions (the grid itself is the same either way).
    const GRIDS: [(bool, bool); 4] = [(false, false), (true, false), (false, true), (true, true)];

    #[test]
    fn the_board_fits_the_room_it_is_given() {
        // A room bigger than the pad's board keeps the pad's size, centred in it.
        let at = board();
        assert!((at.w - BOARD_W).abs() < 0.01, "{at:?}");
        assert!((at.h - BOARD_H).abs() < 0.01, "{at:?}");
        assert!(at.x >= 40.0 && at.x + at.w <= 680.0, "{at:?}");
        assert!(at.y >= 300.0 && at.y + at.h <= 600.0, "{at:?}");
        // A room smaller than it shrinks the whole board, never past the room.
        let small = Keys::within([0.0, 0.0, 300.0, 150.0]);
        assert!(small.w <= 300.0 && small.h <= 150.0, "{small:?}");
        assert!(small.scale < 1.0, "{small:?}");
        assert!(small.x >= 0.0 && small.y >= 0.0, "{small:?}");
        // The keys stay on the board, and no two of them share a point.
        let k = Keyboard::default();
        let mut rects = Vec::new();
        for (row, line) in k.rows().iter().enumerate() {
            for col in 0..line.len() {
                let [x, y, w, h] = k.key_rect(row, col, small).expect("a key");
                assert!(
                    x >= small.x - 0.01 && x + w <= small.x + small.w + 0.01,
                    "{x} {w}"
                );
                assert!(
                    y >= small.y - 0.01 && y + h <= small.y + small.h + 0.01,
                    "{y} {h}"
                );
                rects.push([x, y, w, h]);
            }
        }
        for (i, [ax, ay, aw, ah]) in rects.iter().enumerate() {
            for [bx, by, bw, bh] in &rects[i + 1..] {
                let apart = ax + aw <= bx + 0.01
                    || bx + bw <= ax + 0.01
                    || ay + ah <= by + 0.01
                    || by + bh <= ay + 0.01;
                assert!(apart, "keys {i} and {rects:?} overlap");
            }
        }
    }

    #[test]
    fn a_tap_on_every_key_types_what_choosing_it_and_pressing_does() {
        for (lines, suggests) in GRIDS {
            for symbols in [false, true] {
                // The grid's own keys, in the order they are drawn.
                let mut probe = Keyboard::new(lines, suggests);
                probe.symbols = symbols;
                let rows: Vec<Vec<usize>> = probe
                    .rows()
                    .iter()
                    .map(|r| (0..r.len()).collect())
                    .collect();
                for (row, cols) in rows.iter().enumerate() {
                    for &col in cols {
                        // What the pad does: focus the key, press it.
                        let mut pad = Keyboard::new(lines, suggests);
                        pad.symbols = symbols;
                        pad.row = row;
                        pad.col = col;
                        let by_pad = pad.press();
                        // What a finger does: tap its centre on the board.
                        let mut touch = Keyboard::new(lines, suggests);
                        touch.symbols = symbols;
                        let [x, y, w, h] = probe.key_rect(row, col, board()).expect("a key");
                        let tap = (x + w / 2.0, y + h / 2.0);
                        assert_eq!(
                            touch.tap([tap.0, tap.1], board()),
                            by_pad,
                            "row {row} col {col}"
                        );
                        // And the pad's state after the press is the state after the tap.
                        assert_eq!(touch, pad, "row {row} col {col}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_tap_between_the_keys_or_outside_them_does_nothing() {
        let at = board();
        let mut k = Keyboard::default();
        let before = k.clone();
        // Above and below the board, and beside it.
        assert_eq!(k.tap([at.x + 1.0, at.y - 20.0], at), None);
        assert_eq!(k.tap([at.x + 1.0, at.y + at.h + 5.0], at), None);
        assert_eq!(k.tap([at.x - 5.0, at.y + 10.0], at), None);
        assert_eq!(k.tap([at.x + at.w + 5.0, at.y + 10.0], at), None);
        // The gap between two keys: `q` and `w` of the second row.
        let [x, _, w, _] = k.key_rect(1, 0, at).expect("q");
        let gap_x = x + w + GAP / 2.0;
        let [_, y, _, h] = k.key_rect(1, 0, at).expect("q");
        assert_eq!(k.tap([gap_x, y + h / 2.0], at), None);
        // A tap that hits nothing changes nothing.
        assert_eq!(k, before);
    }

    #[test]
    fn taps_shift_switch_and_finish_like_the_pad() {
        let at = board();
        let mut k = Keyboard::new(false, false);
        let centre = |k: &Keyboard, row: usize, col: usize| {
            let [x, y, w, h] = k.key_rect(row, col, at).expect("a key");
            [x + w / 2.0, y + h / 2.0]
        };
        // Shift, then a letter: upper case, and the shift lets go.
        assert_eq!(k.tap(centre(&k, 4, 0), at), None);
        assert_eq!(k.tap(centre(&k, 1, 0), at), Some(Output::Text("Q".into())));
        assert_eq!(k.tap(centre(&k, 1, 0), at), Some(Output::Text("q".into())));
        // The layer key shows the symbols, where q's own key is `/`.
        assert_eq!(k.tap(centre(&k, 4, 1), at), None);
        assert_eq!(k.tap(centre(&k, 1, 0), at), Some(Output::Text("/".into())));
        // Backspace and Done are keys like any other (the last row is Shift,
        // symbols, Space, Delete, Done).
        assert_eq!(k.tap(centre(&k, 4, 3), at), Some(Output::Backspace));
        assert_eq!(k.tap(centre(&k, 4, 4), at), Some(Output::Done));
        // On a field of many lines the row is Shift, symbols, Space, New Line, Delete,
        // Done, and Done still finishes.
        let mut lines = Keyboard::new(true, false);
        assert_eq!(lines.tap(centre(&lines, 4, 3), at), Some(Output::Enter));
        assert_eq!(lines.tap(centre(&lines, 4, 5), at), Some(Output::Done));
    }
}
