//! The platform's own on-screen keyboard, for a host whose window gets no usable text from it
//! (Android's NativeActivity: a soft keyboard's composed and non-Latin text never reaches the
//! window as key events). The host keeps a hidden text box the keyboard types into
//! (`magnetite-android`'s `ime.rs`, after the Horizon Android fork's `native_ime.rs`); the game
//! shows it while one of its own text fields has focus and turns what changed in the box into the
//! edits a keyboard would make ([`edits`]): text, backspaces and Enter.
use std::sync::{Mutex, OnceLock};

/// What a field takes, which sets how the keyboard types into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Prose (chat, a sign, a book): the keyboard's suggestions and capitals help.
    Text,
    /// Typed exactly as keyed (a login, an e-mail, a name, a code): no suggestions, no capitals
    /// the keyboard adds by itself.
    Literal,
    /// A password: as `Literal`, and hidden.
    Secret,
}

/// The host's keyboard: plain functions, callable from the game's thread.
#[derive(Clone, Copy)]
pub struct PlatformKeyboard {
    /// Show the keyboard over a box holding `text` with the caret at its end, typing as `Mode`
    /// says. False when it can't be shown.
    pub show: fn(&str, Mode) -> bool,
    /// Hide the keyboard.
    pub hide: fn(),
    /// The box's text now; `None` while the keyboard is not up.
    pub text: fn() -> Option<String>,
    /// Replace the box's text (after Enter took a line, the field is empty again).
    pub set_text: fn(&str),
}

static KEYBOARD: OnceLock<PlatformKeyboard> = OnceLock::new();

/// The box's text when the game last read it: `Some` while the game shows the keyboard.
static SHOWN: Mutex<Option<String>> = Mutex::new(None);

/// Give the game the platform's keyboard, before the game starts. The first call wins.
pub fn set_platform_keyboard(keyboard: PlatformKeyboard) {
    let _ = KEYBOARD.set(keyboard);
}

/// Whether the host gave a keyboard: the game then uses it instead of its own (`osk`).
pub fn platform_keyboard() -> bool {
    KEYBOARD.get().is_some()
}

/// What the keyboard did to the focused field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    Text(String),
    Backspace,
    Enter,
}

/// Show the keyboard for a field holding `text` (its own text, so a backspace has something to
/// take), typing as `mode` says. False with no platform keyboard or when it failed.
pub fn show(text: &str, mode: Mode) -> bool {
    let Some(keyboard) = KEYBOARD.get() else {
        return false;
    };
    let shown = (keyboard.show)(text, mode);
    *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) = shown.then(|| text.to_owned());
    shown
}

/// Hide the keyboard, if the game showed it.
pub fn hide() {
    let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
    if shown.take().is_some()
        && let Some(keyboard) = KEYBOARD.get()
    {
        (keyboard.hide)();
    }
}

/// Whether the game is showing the keyboard.
pub fn shown() -> bool {
    SHOWN.lock().unwrap_or_else(|e| e.into_inner()).is_some()
}

/// What the keyboard typed since the last call. A line ended with Enter leaves the box empty, as
/// the field it went to is; `multiline` keeps line breaks as text (a book's page).
pub fn poll(multiline: bool) -> Vec<Edit> {
    let Some(keyboard) = KEYBOARD.get() else {
        return Vec::new();
    };
    let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
    let Some(old) = shown.as_mut() else {
        return Vec::new();
    };
    let Some(new) = (keyboard.text)() else {
        return Vec::new();
    };
    let out = edits(old, &new, multiline);
    if !multiline && out.last() == Some(&Edit::Enter) {
        (keyboard.set_text)("");
        old.clear();
    } else {
        *old = new;
    }
    out
}

/// The edits that turn `old` into `new`: backspaces back to where they part, then the rest as
/// text. A single-line field takes a final line break as Enter.
pub fn edits(old: &str, new: &str, multiline: bool) -> Vec<Edit> {
    if old == new {
        return Vec::new();
    }
    let kept = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let mut out: Vec<_> = (kept..old.chars().count())
        .map(|_| Edit::Backspace)
        .collect();
    let mut tail: String = new.chars().skip(kept).collect();
    let enter = !multiline && tail.ends_with('\n');
    if enter {
        tail.pop();
    }
    if !multiline {
        tail.retain(|c| c != '\n' && c != '\r');
    }
    if !tail.is_empty() {
        out.push(Edit::Text(tail));
    }
    if enter {
        out.push(Edit::Enter);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Edit, edits};

    #[test]
    fn typing_deleting_and_replacing_become_a_keyboards_edits() {
        assert_eq!(edits("при", "привет", false), [Edit::Text("вет".into())]);
        assert_eq!(edits("привет", "при", false), vec![Edit::Backspace; 3]);
        // A suggestion that replaces the word: back to where it parts, then the new end.
        assert_eq!(
            edits("превет", "привёт🙂", false),
            [
                Edit::Backspace,
                Edit::Backspace,
                Edit::Backspace,
                Edit::Backspace,
                Edit::Text("ивёт🙂".into())
            ]
        );
        assert!(edits("same", "same", false).is_empty());
    }

    #[test]
    fn a_line_break_is_enter_in_a_field_and_text_on_a_page() {
        assert_eq!(
            edits("hi", "hi!\n", false),
            [Edit::Text("!".into()), Edit::Enter]
        );
        assert_eq!(edits("hi", "hi\n", true), [Edit::Text("\n".into())]);
    }
}
