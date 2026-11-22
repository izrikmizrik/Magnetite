//! Touch controls: fingers on the glass become the inputs the game already takes.
//!
//! Ported from the iOS fork of Horizon (MIT licence), where it is `game/touch.rs`, and
//! adapted to main: one code for Android, iOS and a touchscreen PC, no `cfg(target_os)`
//! in the logic. The platform pieces still to come (stage 2.3) are hooks: the picture
//! picker (`set_picture_picker`), the safe area (`safe_insets`) and the iOS view-scale
//! touch correction (`touch_pixels`).
//!
//! - **In a screen** (title, menus, inventories) a finger is the mouse. A tap is a left click
//!   where it lands. A quick vertical swipe scrolls. A sideways drag, or a drag after holding
//!   still for a moment, is a left button drag (sliders, scroll bars). A second finger tapped
//!   while the first rests is a right click at the first finger.
//! - **In the world** the left of the screen is a floating move stick, the buttons are the
//!   controller's (pad.rs merges them as one more pad), and a finger anywhere else turns the view
//!   like a grabbed mouse. A finger on a button turns the view too, so mining follows the thumb.
//!
//! The logic here is plain data in and out (`Controls::touch` returns what the game should do),
//! so it is tested without a window.

use super::event_loop::EventLoopCtl;
use super::{Game, Screen, input_trace};
use horizon_client::gamepad::{Button, PadState};
use horizon_client::hud::{HudBatch, rgba};
use std::time::{Duration, Instant};
use winit::event::{
    DeviceId, ElementState, MouseButton, MouseScrollDelta, Touch, TouchPhase, WindowEvent,
};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// A press held this long without moving becomes a drag instead of a scroll.
const HOLD: Duration = Duration::from_millis(220);
/// Finger travel per degree-ish of turn, against a mouse's counts.
const LOOK_SCALE: f64 = 0.55;
/// The move stick past this many radii holds sprint.
const SPRINT_AT: f32 = 1.6;

/// What a button on the glass does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Key {
    Pad(Button),
    /// 0 is the left trigger (use), 1 the right (attack).
    Trigger(usize),
    /// Keyboard keys (the `key.keyboard.<name>` names), held in order while the finger stays:
    /// F1, F3, or a debug combination such as F3 then G.
    Keys(&'static [&'static str]),
}

/// One action a touch button can have.
pub(super) struct Action {
    /// Its name in `touch-controls.json`.
    pub id: &'static str,
    /// On the button: a lang key (`horizon.touch.*`).
    pub label: &'static str,
    /// In Edit HUD's Add list: a lang key.
    pub name: &'static str,
    pub key: Key,
}

const fn action(id: &'static str, label: &'static str, name: &'static str, key: Key) -> Action {
    Action {
        id,
        label,
        name,
        key,
    }
}

/// Every action, in Edit HUD's order. The controller's buttons go through the virtual pad (their
/// bindings: docs/controllers.md); the others press keyboard keys.
pub(super) const ACTIONS: &[Action] = &[
    action("jump", "horizon.touch.jump", "horizon.touch.jump.name", Key::Pad(Button::South)),
    action("sneak", "horizon.touch.sneak", "horizon.touch.sneak.name", Key::Pad(Button::RightStick)),
    action("use", "horizon.touch.use", "horizon.touch.use.name", Key::Trigger(0)),
    action("hit", "horizon.touch.hit", "horizon.touch.hit.name", Key::Trigger(1)),
    action("inventory", "horizon.touch.inventory", "horizon.touch.inventory.name", Key::Pad(Button::North)),
    action("chat", "horizon.touch.chat", "horizon.touch.chat.name", Key::Pad(Button::DPadRight)),
    action("drop", "horizon.touch.drop", "horizon.touch.drop.name", Key::Pad(Button::East)),
    action("camera", "horizon.touch.camera", "horizon.touch.camera.name", Key::Pad(Button::DPadUp)),
    action("hotbar_previous", "horizon.touch.hotbar_previous", "horizon.touch.hotbar_previous.name", Key::Pad(Button::LeftBumper)),
    action("hotbar_next", "horizon.touch.hotbar_next", "horizon.touch.hotbar_next.name", Key::Pad(Button::RightBumper)),
    action("sprint", "horizon.touch.sprint", "horizon.touch.sprint.name", Key::Pad(Button::LeftStick)),
    action("swap_hands", "horizon.touch.swap_hands", "horizon.touch.swap_hands.name", Key::Pad(Button::West)),
    action("pick_block", "horizon.touch.pick_block", "horizon.touch.pick_block.name", Key::Pad(Button::DPadLeft)),
    action("advancements", "horizon.touch.advancements", "horizon.touch.advancements.name", Key::Pad(Button::DPadDown)),
    action("player_list", "horizon.touch.player_list", "horizon.touch.player_list.name", Key::Pad(Button::Select)),
    action("command", "horizon.touch.command", "horizon.touch.command.name", Key::Keys(&["slash"])),
    action("hotbar_1", "horizon.touch.hotbar_1", "horizon.touch.hotbar_1.name", Key::Keys(&["1"])),
    action("hotbar_2", "horizon.touch.hotbar_2", "horizon.touch.hotbar_2.name", Key::Keys(&["2"])),
    action("hotbar_3", "horizon.touch.hotbar_3", "horizon.touch.hotbar_3.name", Key::Keys(&["3"])),
    action("hotbar_4", "horizon.touch.hotbar_4", "horizon.touch.hotbar_4.name", Key::Keys(&["4"])),
    action("hotbar_5", "horizon.touch.hotbar_5", "horizon.touch.hotbar_5.name", Key::Keys(&["5"])),
    action("hotbar_6", "horizon.touch.hotbar_6", "horizon.touch.hotbar_6.name", Key::Keys(&["6"])),
    action("hotbar_7", "horizon.touch.hotbar_7", "horizon.touch.hotbar_7.name", Key::Keys(&["7"])),
    action("hotbar_8", "horizon.touch.hotbar_8", "horizon.touch.hotbar_8.name", Key::Keys(&["8"])),
    action("hotbar_9", "horizon.touch.hotbar_9", "horizon.touch.hotbar_9.name", Key::Keys(&["9"])),
    action("f1", "horizon.touch.f1", "horizon.touch.f1.name", Key::Keys(&["f1"])),
    action("f2", "horizon.touch.f2", "horizon.touch.f2.name", Key::Keys(&["f2"])),
    action("f3", "horizon.touch.f3", "horizon.touch.f3.name", Key::Keys(&["f3"])),
    action("f3_b", "horizon.touch.f3_b", "horizon.touch.f3_b.name", Key::Keys(&["f3", "b"])),
    action("f3_g", "horizon.touch.f3_g", "horizon.touch.f3_g.name", Key::Keys(&["f3", "g"])),
    action("f3_h", "horizon.touch.f3_h", "horizon.touch.f3_h.name", Key::Keys(&["f3", "h"])),
    action("f3_n", "horizon.touch.f3_n", "horizon.touch.f3_n.name", Key::Keys(&["f3", "n"])),
    action("f3_t", "horizon.touch.f3_t", "horizon.touch.f3_t.name", Key::Keys(&["f3", "t"])),
    action("f3_f4", "horizon.touch.f3_f4", "horizon.touch.f3_f4.name", Key::Keys(&["f3", "f4"])),
    // Keyboard keys for native mods' keys (Options > Mods > a mod's keys): a button holds its
    // key while the finger stays.
    action("key_b", "horizon.touch.key_b", "horizon.touch.key_b.name", Key::Keys(&["b"])),
    action("key_c", "horizon.touch.key_c", "horizon.touch.key_c.name", Key::Keys(&["c"])),
    action("key_e", "horizon.touch.key_e", "horizon.touch.key_e.name", Key::Keys(&["e"])),
    action("key_g", "horizon.touch.key_g", "horizon.touch.key_g.name", Key::Keys(&["g"])),
    action("key_h", "horizon.touch.key_h", "horizon.touch.key_h.name", Key::Keys(&["h"])),
    action("key_i", "horizon.touch.key_i", "horizon.touch.key_i.name", Key::Keys(&["i"])),
    action("key_j", "horizon.touch.key_j", "horizon.touch.key_j.name", Key::Keys(&["j"])),
    action("key_k", "horizon.touch.key_k", "horizon.touch.key_k.name", Key::Keys(&["k"])),
    action("key_m", "horizon.touch.key_m", "horizon.touch.key_m.name", Key::Keys(&["m"])),
    action("key_n", "horizon.touch.key_n", "horizon.touch.key_n.name", Key::Keys(&["n"])),
    action("key_o", "horizon.touch.key_o", "horizon.touch.key_o.name", Key::Keys(&["o"])),
    action("key_p", "horizon.touch.key_p", "horizon.touch.key_p.name", Key::Keys(&["p"])),
    action("key_r", "horizon.touch.key_r", "horizon.touch.key_r.name", Key::Keys(&["r"])),
    action("key_u", "horizon.touch.key_u", "horizon.touch.key_u.name", Key::Keys(&["u"])),
    action("key_v", "horizon.touch.key_v", "horizon.touch.key_v.name", Key::Keys(&["v"])),
    action("key_x", "horizon.touch.key_x", "horizon.touch.key_x.name", Key::Keys(&["x"])),
    action("key_y", "horizon.touch.key_y", "horizon.touch.key_y.name", Key::Keys(&["y"])),
    action("key_z", "horizon.touch.key_z", "horizon.touch.key_z.name", Key::Keys(&["z"])),
    action("key_tab", "horizon.touch.key_tab", "horizon.touch.key_tab.name", Key::Keys(&["tab"])),
];

/// The action named `id`.
pub(super) fn find_action(id: &str) -> Option<&'static Action> {
    ACTIONS.iter().find(|a| a.id == id)
}

/// A button the player placed (Edit HUD): its action, its centre as shares of the screen's width
/// and height, and its side as a share of the short side.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Placed {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

impl Placed {
    /// Its rectangle in window pixels.
    pub(super) fn rect(&self, [w, h]: [f32; 2]) -> [f32; 4] {
        let s = self.size * w.min(h);
        [self.x * w - s / 2.0, self.y * h - s / 2.0, s, s]
    }
}

/// One Edit HUD layout: the buttons, the move stick's area, the Esc button and how opaque they
/// are. Positions are shares of the screen, so a layout fits any screen of the same shape.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HudLayout {
    pub buttons: Vec<Placed>,
    /// The move stick's area: x, y, width and height.
    pub stick: [f32; 4],
    /// The Esc button's centre.
    pub esc: [f32; 2],
    /// The Esc button's size, against its default.
    pub esc_scale: f32,
    /// 0.2 to 1.
    pub opacity: f32,
}

impl HudLayout {
    pub(super) fn stick_rect(&self, [w, h]: [f32; 2]) -> [f32; 4] {
        let [x, y, sw, sh] = self.stick;
        [x * w, y * h, sw * w, sh * h]
    }
    pub(super) fn esc_rect(&self, [w, h]: [f32; 2]) -> [f32; 4] {
        let u = w.min(h) * 0.15 * self.esc_scale;
        let [cx, cy] = self.esc;
        [cx * w - u * 0.4, cy * h - u * 0.25, u * 0.8, u * 0.5]
    }
}

/// How many layouts Edit HUD keeps.
pub(super) const LAYOUTS: usize = 3;

/// `touch-controls.json`: the layouts the player saved (none: the defaults) and the one in use.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct HudFile {
    pub active: usize,
    pub layouts: Vec<Option<HudLayout>>,
}

impl HudFile {
    fn current(&self) -> Option<&HudLayout> {
        self.layouts.get(self.active)?.as_ref()
    }
}

/// The file the layouts are kept in, in the game folder.
const LAYOUT_FILE: &str = "touch-controls.json";

fn parse_buttons(value: &serde_json::Value) -> Vec<Placed> {
    value
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|b| {
                    let id = b.get("id")?.as_str()?;
                    find_action(id)?;
                    let n = |k: &str| b.get(k).and_then(serde_json::Value::as_f64).map(|v| v as f32);
                    Some(Placed {
                        id: id.to_owned(),
                        x: n("x")?.clamp(0.0, 1.0),
                        y: n("y")?.clamp(0.0, 1.0),
                        size: n("size")?.clamp(0.04, 0.4),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn numbers<const N: usize>(value: &serde_json::Value) -> Option<[f32; N]> {
    let list = value.as_array()?;
    let mut out = [0.0; N];
    for (slot, v) in out.iter_mut().zip(list) {
        *slot = (v.as_f64()? as f32).clamp(0.0, 1.0);
    }
    (list.len() == N).then_some(out)
}

fn parse_layout(value: &serde_json::Value, defaults: &HudLayout) -> Option<HudLayout> {
    if value.is_null() {
        return None;
    }
    Some(HudLayout {
        buttons: parse_buttons(&value["buttons"]),
        stick: numbers(&value["stick"]).unwrap_or(defaults.stick),
        esc: numbers(&value["esc"]).unwrap_or(defaults.esc),
        esc_scale: value["esc_scale"]
            .as_f64()
            .map_or(defaults.esc_scale, |v| (v as f32).clamp(0.5, 2.5)),
        // Edit HUD no longer sets it (its Fainter and Bolder took the screen): full, whatever an
        // earlier build saved.
        opacity: defaults.opacity,
    })
}

/// The file, read against the default layout (for what it leaves out). The first Edit HUD kept
/// one layout of buttons only: it is the first layout.
fn load_layouts(defaults: &HudLayout) -> HudFile {
    let Some(value) = std::fs::read_to_string(LAYOUT_FILE)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    else {
        return HudFile::default();
    };
    if let Some(list) = value["layouts"].as_array() {
        return HudFile {
            active: value["active"].as_u64().unwrap_or(0) as usize,
            layouts: list.iter().take(LAYOUTS).map(|l| parse_layout(l, defaults)).collect(),
        };
    }
    HudFile {
        active: 0,
        layouts: vec![parse_layout(&value, defaults)],
    }
}

fn save_layouts(file: &HudFile) {
    let layout = |l: &Option<HudLayout>| match l {
        None => serde_json::Value::Null,
        Some(l) => serde_json::json!({
            "buttons": l.buttons.iter().map(|b| serde_json::json!({
                "id": b.id, "x": b.x, "y": b.y, "size": b.size,
            })).collect::<Vec<_>>(),
            "stick": l.stick,
            "esc": l.esc,
            "esc_scale": l.esc_scale,
            "opacity": l.opacity,
        }),
    };
    let text = serde_json::json!({
        "active": file.active,
        "layouts": file.layouts.iter().map(layout).collect::<Vec<_>>(),
    })
    .to_string();
    if let Err(e) = std::fs::write(LAYOUT_FILE, text) {
        log::warn!("Horizon: the touch layout could not be saved: {e}");
    }
}

/// A button over the chat screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ChatKey {
    /// Completes the command or name being typed.
    Tab,
    /// The clipboard's text into the line.
    Paste,
    /// Sends the line.
    Enter,
}

/// A button: its rectangle in window pixels, what it does and what it shows (a lang key).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pad {
    pub rect: [f32; 4],
    pub key: Key,
    pub label: &'static str,
    /// The action's name (`ACTIONS`).
    pub id: &'static str,
}

/// Where everything sits for a window of `size` pixels whose GUI is drawn at `gui_scale`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Layout {
    pub keys: Vec<Pad>,
    /// The move stick's area: a finger landing here that hits no button starts the stick.
    pub stick_zone: [f32; 4],
    pub stick_radius: f32,
    /// The slop a finger may move and still be a tap.
    pub slop: f32,
    /// Finger travel per scrolled line in a screen.
    pub line: f32,
    /// The Esc button, top centre: shown in the world and in every screen.
    pub escape: [f32; 4],
    /// Edit HUD's plate, top left: shown over the title while touch plays.
    pub edit: [f32; 4],
    /// How opaque the buttons are.
    pub opacity: f32,
    /// Over the chat screen, top right: the key, its label's lang key, its rectangle.
    pub chat_keys: Vec<(ChatKey, &'static str, [f32; 4])>,
}

impl Layout {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn new(size: [f32; 2], gui_scale: f32) -> Self {
        Self::new_in(size, gui_scale, [0.0; 4])
    }

    /// The default layout clear of the safe area's `insets` (left, top, right, bottom pixels).
    pub(super) fn new_in(size: [f32; 2], gui_scale: f32, insets: [f32; 4]) -> Self {
        let [w, h] = size;
        let [inset_left, inset_top, inset_right, inset_bottom] = insets;
        let short = w.min(h);
        // A thumb-sized button: about 9 mm on a phone held sideways.
        let u = short * 0.15;
        let gap = u * 0.2;
        // Clear of the rounded corners, the camera housing and the home indicator.
        let side = (w * 0.055).max(u * 0.5).max(inset_right + u * 0.12);
        let left_side = (w * 0.055).max(u * 0.5).max(inset_left + u * 0.12);
        let bottom = u * 0.35 + inset_bottom;
        let top = (u * 0.18).max(inset_top + u * 0.08);
        let col = |i: f32| w - side - u * (i + 1.0) - gap * i;
        let row = |i: f32| h - bottom - u * (i + 1.0) - gap * i;
        let small = u * 0.72;
        let top_col = |i: f32| w - side - small * (i + 1.0) - gap * i;
        // Over the ends of the hotbar (182 GUI pixels wide, centred), clear of the bars above it
        // (39 GUI pixels from the bottom): beside it they met the buttons on a small phone.
        let hotbar_half = 91.0 * gui_scale;
        // The GUI is centred in the safe area.
        let gui_centre = inset_left + (w - inset_left - inset_right) / 2.0;
        let hot_y = h - inset_bottom - 39.0 * gui_scale - gap - small;
        let key = |x: f32, y: f32, s: f32, key: Key, label: &'static str| Pad {
            rect: [x, y, s, s],
            key,
            label,
            id: ACTIONS
                .iter()
                .find(|a| a.key == key)
                .map_or("", |a| a.id),
        };
        let keys = vec![
            key(col(0.0), row(0.0), u, Key::Pad(Button::South), "horizon.touch.jump"),
            key(col(1.0), row(0.0), u, Key::Pad(Button::RightStick), "horizon.touch.sneak"),
            key(col(0.0), row(1.0), u, Key::Trigger(0), "horizon.touch.use"),
            key(col(1.0), row(1.0), u, Key::Trigger(1), "horizon.touch.hit"),
            key(top_col(0.0), top, small, Key::Pad(Button::North), "horizon.touch.inventory"),
            key(top_col(1.0), top, small, Key::Pad(Button::DPadRight), "horizon.touch.chat"),
            key(top_col(2.0), top, small, Key::Pad(Button::East), "horizon.touch.drop"),
            key(top_col(3.0), top, small, Key::Pad(Button::DPadUp), "horizon.touch.camera"),
            key(
                gui_centre - hotbar_half,
                hot_y,
                small,
                Key::Pad(Button::LeftBumper),
                "horizon.touch.hotbar_previous",
            ),
            key(
                gui_centre + hotbar_half - small,
                hot_y,
                small,
                Key::Pad(Button::RightBumper),
                "horizon.touch.hotbar_next",
            ),
        ];
        let esc_y = u * 0.12 + inset_top;
        let (cw, ch) = (u * 0.95, u * 0.5);
        let chat_keys = [
            (ChatKey::Tab, "horizon.touch.tab"),
            (ChatKey::Paste, "horizon.touch.paste"),
            (ChatKey::Enter, "horizon.touch.enter"),
        ]
        .iter()
        .enumerate()
        .map(|(i, &(k, label))| {
            let x = w - side - (3 - i) as f32 * (cw + gap) + gap;
            (k, label, [x, esc_y, cw, ch])
        })
        .collect();
        Self {
            keys,
            stick_zone: [inset_left, h * 0.28, w * 0.40 - inset_left, h * 0.72 - inset_bottom],
            stick_radius: u * 0.8,
            slop: u * 0.14,
            line: u * 0.35,
            escape: [w / 2.0 - u * 0.4, esc_y, u * 0.8, u * 0.5],
            edit: [left_side, esc_y, cw, ch],
            opacity: 1.0,
            chat_keys,
        }
    }
    /// The layout with the player's own (Edit HUD) in place of the defaults.
    pub(super) fn with(
        size: [f32; 2],
        gui_scale: f32,
        insets: [f32; 4],
        custom: Option<&HudLayout>,
    ) -> Self {
        let mut layout = Self::new_in(size, gui_scale, insets);
        if let Some(custom) = custom {
            layout.stick_zone = custom.stick_rect(size);
            layout.escape = custom.esc_rect(size);
            layout.opacity = custom.opacity;
            layout.keys = custom
                .buttons
                .iter()
                .filter_map(|p| {
                    let a = find_action(&p.id)?;
                    Some(Pad {
                        rect: p.rect(size),
                        key: a.key,
                        label: a.label,
                        id: a.id,
                    })
                })
                .collect();
        }
        layout
    }

    /// The whole layout as Edit HUD keeps it.
    pub(super) fn hud(&self, size: [f32; 2]) -> HudLayout {
        let [w, h] = size;
        let [x, y, sw, sh] = self.stick_zone;
        let [ex, ey, ew, eh] = self.escape;
        HudLayout {
            buttons: self.placed(size),
            stick: [x / w, y / h, sw / w, sh / h],
            esc: [(ex + ew / 2.0) / w, (ey + eh / 2.0) / h],
            esc_scale: ew / (w.min(h) * 0.15 * 0.8),
            opacity: self.opacity,
        }
    }

    /// The buttons as Edit HUD keeps them.
    pub(super) fn placed(&self, [w, h]: [f32; 2]) -> Vec<Placed> {
        let short = w.min(h);
        self.keys
            .iter()
            .map(|k| Placed {
                id: k.id.to_owned(),
                x: (k.rect[0] + k.rect[2] / 2.0) / w,
                y: (k.rect[1] + k.rect[3] / 2.0) / h,
                size: k.rect[2] / short,
            })
            .collect()
    }

    fn key_at(&self, at: [f32; 2]) -> Option<usize> {
        self.keys.iter().position(|k| inside(k.rect, at))
    }
}

fn inside([x, y, w, h]: [f32; 4], [px, py]: [f32; 2]) -> bool {
    px >= x && px < x + w && py >= y && py < y + h
}

/// What the game should do about a touch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Out {
    /// The pointer is here now (window pixels).
    Cursor([f64; 2]),
    /// A mouse button went down or up.
    Mouse(MouseButton, bool),
    /// Scroll by this many lines (positive moves the content down).
    Wheel(f32),
    /// Turn the view by this much grabbed-mouse motion.
    Look(f64, f64),
    /// The Esc button: what Escape does where the player is (`pad_back`).
    Escape,
    /// Keyboard keys go down (in order) or up (in reverse).
    Keys(&'static [&'static str], bool),
}

impl Out {
    /// The event a mouse would have made; `None` for a turn of the view, which is no event.
    pub(super) fn window_event(self) -> Option<WindowEvent> {
        let device_id = DeviceId::dummy();
        Some(match self {
            Out::Cursor([x, y]) => WindowEvent::CursorMoved {
                device_id,
                position: winit::dpi::PhysicalPosition::new(x, y),
            },
            Out::Mouse(button, down) => WindowEvent::MouseInput {
                device_id,
                state: if down {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
                button,
            },
            Out::Wheel(lines) => WindowEvent::MouseWheel {
                device_id,
                delta: MouseScrollDelta::LineDelta(0.0, lines),
                phase: TouchPhase::Moved,
            },
            Out::Look(..) | Out::Escape | Out::Keys(..) => return None,
        })
    }
}

/// A touch in the renderer's pixels (`surface`). Where the window's outer frame is the screen
/// (`window::outer_is_screen`: iOS, whose touches come in the view's scale), a touch is scaled
/// from the outer frame to the drawn one; elsewhere it is in the window's pixels already.
pub(super) fn touch_pixels(window: Option<&Window>, surface: [u32; 2], touch: &Touch) -> [f64; 2] {
    let at = [touch.location.x, touch.location.y];
    let Some(window) = window.filter(|_| horizon_client::window::outer_is_screen()) else {
        return at;
    };
    let outer = window.outer_size();
    let ratio = |s: u32, o: u32| {
        if o == 0 || s == 0 {
            1.0
        } else {
            f64::from(s) / f64::from(o)
        }
    };
    [
        at[0] * ratio(surface[0], outer.width),
        at[1] * ratio(surface[1], outer.height),
    ]
}

/// The safe area's insets in the drawn frame's pixels (left, top, right, bottom): the part of the
/// outer frame outside the inner area, where the outer frame is the screen
/// (`window::outer_is_screen`); zero elsewhere.
pub(super) fn safe_insets_of(window: Option<&Window>, surface: [u32; 2]) -> [f32; 4] {
    let Some(window) = window.filter(|_| horizon_client::window::outer_is_screen()) else {
        return [0.0; 4];
    };
    let (outer, inner) = (window.outer_size(), window.inner_size());
    let Ok(origin) = window.inner_position() else {
        return [0.0; 4];
    };
    if outer.width == 0 || outer.height == 0 {
        return [0.0; 4];
    }
    let fx = surface[0] as f32 / outer.width as f32;
    let fy = surface[1] as f32 / outer.height as f32;
    let (ox, oy) = (origin.x.max(0) as f32, origin.y.max(0) as f32);
    [
        (ox * fx).round(),
        (oy * fy).round(),
        ((outer.width as f32 - ox - inner.width as f32) * fx)
            .max(0.0)
            .round(),
        ((outer.height as f32 - oy - inner.height as f32) * fy)
            .max(0.0)
            .round(),
    ]
}

/// The layout for `size` at `gui_scale`, clear of the safe area's `insets`, with the player's
/// own (Edit HUD) in place of the defaults.
pub(super) fn screen_layout(
    size: [u32; 2],
    gui_scale: f32,
    insets: [f32; 4],
    custom: Option<&HudLayout>,
) -> Layout {
    Layout::with([size[0] as f32, size[1] as f32], gui_scale, insets, custom)
}

/// A fling slows by this factor every second (iOS's normal deceleration, 0.998 a millisecond).
const FLING_DECAY: f64 = 0.135;
/// A fling stops below this many lines a second, and starts only above `FLING_START`.
const FLING_STOP: f64 = 1.0;
const FLING_START: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    /// A screen's pointer, before it is known to be a tap, a scroll or a drag.
    Pending,
    Scroll,
    Drag,
    /// A right button drag: a stack held still on a slot, then slid (one item a slot).
    RightDrag,
    /// A second finger in a screen: a right click when it lifts.
    Second,
    Stick,
    Look,
    Key(usize),
    /// On the Esc button: nothing more until it lifts.
    Escape,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: u64,
    role: Role,
    start: [f64; 2],
    last: [f64; 2],
    began: Instant,
    /// It landed on an inventory slot while a stack was carried: it slides to split the stack
    /// over the slots it crosses, and never scrolls.
    on_slot: bool,
    /// A scroll's speed in lines a second (smoothed), and when it last moved.
    speed: f64,
    moved_at: Instant,
}

/// The fingers down and what they hold.
#[derive(Default)]
pub(super) struct Controls {
    fingers: Vec<Finger>,
    /// The fingers were last read in the world (true) or in a screen.
    world: bool,
    /// What the fingers hold now.
    held: PadState,
    /// Buttons that went down since the last poll: a tap shorter than a frame still counts.
    pulse: PadState,
    /// The player's own layouts (Edit HUD), once `touch-controls.json` was read.
    hud: HudFile,
    loaded: bool,
    /// Edit HUD, while it is open.
    editor: Option<super::touch_editor::Editor>,
    /// Set by the game before a finger lands: it lands on an inventory slot with a stack carried.
    pub(super) slot_under_next: bool,
    /// A list still gliding after the finger let go: lines a second, and the last frame.
    fling: Option<(f64, Instant)>,
    /// A controller, a keyboard or a mouse drove the game last: the touch HUD steps aside until
    /// a finger touches the glass again.
    pub(super) hardware: bool,
}

impl Controls {
    /// Reads `touch-controls.json` once, against the default layout for `size` at `gui_scale`.
    fn load(&mut self, size: [u32; 2], gui_scale: f32) {
        if !self.loaded {
            self.loaded = true;
            let defaults =
                screen_layout(size, gui_scale, [0.0; 4], None).hud([size[0] as f32, size[1] as f32]);
            self.hud = load_layouts(&defaults);
        }
    }

    /// The virtual pad for this poll.
    pub(super) fn poll(&mut self) -> PadState {
        let state = self.held.merge(&std::mem::take(&mut self.pulse));
        // `merge` adds sticks: the pulse carries none.
        PadState {
            left: self.held.left,
            right: [0.0, 0.0],
            ..state
        }
    }

    /// The move stick's base and knob, in window pixels, while a finger holds it.
    pub(super) fn stick(&self) -> Option<([f64; 2], [f64; 2])> {
        self.fingers
            .iter()
            .find(|f| f.role == Role::Stick)
            .map(|f| (f.start, f.last))
    }

    /// Whether a finger is on the Esc button.
    pub(super) fn escape_held(&self) -> bool {
        self.fingers.iter().any(|f| f.role == Role::Escape)
    }

    /// Whether the button at `index` of the layout is held.
    pub(super) fn key_held(&self, index: usize) -> bool {
        self.fingers.iter().any(|f| f.role == Role::Key(index))
    }

    /// Lets go of everything: the mode changed under the fingers (a screen opened or closed).
    fn release(&mut self, layout: &Layout, out: &mut Vec<Out>) {
        for f in self.fingers.drain(..) {
            match f.role {
                Role::Drag => out.push(Out::Mouse(MouseButton::Left, false)),
                Role::RightDrag => out.push(Out::Mouse(MouseButton::Right, false)),
                Role::Key(i) => {
                    if let Some(Key::Keys(keys)) = layout.keys.get(i).map(|k| k.key) {
                        out.push(Out::Keys(keys, false));
                    }
                }
                _ => {}
            }
        }
        self.held = PadState::default();
        self.fling = None;
    }

    /// Each frame: a list let go mid-swipe keeps gliding and slows down, as lists do on a phone.
    pub(super) fn fling_frame(&mut self, now: Instant) -> Vec<Out> {
        let Some((speed, last)) = self.fling else {
            return Vec::new();
        };
        let dt = now.saturating_duration_since(last).as_secs_f64().min(0.1);
        let speed = speed * FLING_DECAY.powf(dt);
        if speed.abs() < FLING_STOP || self.world {
            self.fling = None;
            return Vec::new();
        }
        self.fling = Some((speed, now));
        vec![Out::Wheel((speed * dt) as f32)]
    }

    /// One touch event. `world` is whether the game holds the pointer (the player is in the
    /// world with no screen open).
    pub(super) fn touch(
        &mut self,
        id: u64,
        phase: TouchPhase,
        at: [f64; 2],
        now: Instant,
        world: bool,
        layout: &Layout,
    ) -> Vec<Out> {
        let mut out = Vec::new();
        if world != self.world {
            self.release(layout, &mut out);
            self.world = world;
        }
        let known = self.fingers.iter().position(|f| f.id == id);
        let slot_under = std::mem::take(&mut self.slot_under_next);
        match (phase, known) {
            (TouchPhase::Started, _) => {
                // A finger on the glass stops a gliding list, as on a phone, and brings the
                // touch HUD back after a controller, keyboard or mouse.
                self.fling = None;
                self.hardware = false;
                // A finger the system never ended.
                if let Some(i) = known {
                    self.lift(i, at, false, now, layout, &mut out);
                }
                let role = if inside(layout.escape, [at[0] as f32, at[1] as f32]) {
                    out.push(Out::Escape);
                    Role::Escape
                } else if world {
                    self.world_role(at, layout)
                } else if self.fingers.is_empty() {
                    out.push(Out::Cursor(at));
                    Role::Pending
                } else {
                    Role::Second
                };
                self.fingers.push(Finger {
                    id,
                    role,
                    start: at,
                    last: at,
                    began: now,
                    on_slot: role == Role::Pending && slot_under,
                    speed: 0.0,
                    moved_at: now,
                });
                if let Role::Key(i) = role {
                    match layout.keys[i].key {
                        Key::Keys(keys) => out.push(Out::Keys(keys, true)),
                        key => self.press(key),
                    }
                }
            }
            (TouchPhase::Moved, Some(i)) => self.moved(i, at, now, layout, &mut out),
            (TouchPhase::Ended, Some(i)) => self.lift(i, at, true, now, layout, &mut out),
            (TouchPhase::Cancelled, Some(i)) => self.lift(i, at, false, now, layout, &mut out),
            _ => {}
        }
        self.hold(layout);
        out
    }

    fn world_role(&self, at: [f64; 2], layout: &Layout) -> Role {
        let p = [at[0] as f32, at[1] as f32];
        if let Some(i) = layout.key_at(p) {
            return Role::Key(i);
        }
        let stick_free = !self.fingers.iter().any(|f| f.role == Role::Stick);
        if stick_free && inside(layout.stick_zone, p) {
            return Role::Stick;
        }
        Role::Look
    }

    fn press(&mut self, key: Key) {
        match key {
            Key::Pad(button) => self.pulse.set(button, true),
            Key::Trigger(i) => self.pulse.triggers[i] = 1.0,
            Key::Keys(_) => {}
        }
    }

    fn moved(&mut self, i: usize, at: [f64; 2], now: Instant, layout: &Layout, out: &mut Vec<Out>) {
        let f = self.fingers[i];
        let (dx, dy) = (at[0] - f.last[0], at[1] - f.last[1]);
        self.fingers[i].last = at;
        match f.role {
            Role::Pending => {
                let (tx, ty) = (at[0] - f.start[0], at[1] - f.start[1]);
                if tx.hypot(ty) <= f64::from(layout.slop) {
                    return;
                }
                let held = now.duration_since(f.began) >= HOLD;
                if f.on_slot {
                    // A carried stack slid over slots: vanilla's drag that spreads it, evenly
                    // with the left button, or one item a slot (the right) after holding still.
                    let button = if held {
                        self.fingers[i].role = Role::RightDrag;
                        MouseButton::Right
                    } else {
                        self.fingers[i].role = Role::Drag;
                        MouseButton::Left
                    };
                    out.push(Out::Cursor(f.start));
                    out.push(Out::Mouse(button, true));
                    out.push(Out::Cursor(at));
                } else if tx.abs() > ty.abs() || held {
                    self.fingers[i].role = Role::Drag;
                    out.push(Out::Cursor(f.start));
                    out.push(Out::Mouse(MouseButton::Left, true));
                    out.push(Out::Cursor(at));
                } else {
                    self.fingers[i].role = Role::Scroll;
                    out.push(Out::Wheel(ty as f32 / layout.line));
                }
            }
            Role::Scroll => {
                let lines = dy / f64::from(layout.line);
                // The speed for a fling: this move's, blended with the ones before.
                let dt = now.saturating_duration_since(f.moved_at).as_secs_f64();
                if dt > 0.0 {
                    let speed = lines / dt.max(0.008);
                    self.fingers[i].speed = f.speed * 0.4 + speed * 0.6;
                }
                self.fingers[i].moved_at = now;
                out.push(Out::Wheel(lines as f32));
            }
            Role::Drag | Role::RightDrag => out.push(Out::Cursor(at)),
            Role::Look | Role::Key(_) => {
                if dx != 0.0 || dy != 0.0 {
                    out.push(Out::Look(dx * LOOK_SCALE, dy * LOOK_SCALE));
                }
            }
            Role::Stick | Role::Second | Role::Escape => {}
        }
    }

    fn lift(
        &mut self,
        i: usize,
        at: [f64; 2],
        ended: bool,
        now: Instant,
        layout: &Layout,
        out: &mut Vec<Out>,
    ) {
        let f = self.fingers.remove(i);
        match f.role {
            Role::Pending if ended => {
                out.push(Out::Cursor(at));
                out.push(Out::Mouse(MouseButton::Left, true));
                out.push(Out::Mouse(MouseButton::Left, false));
            }
            Role::Drag => {
                out.push(Out::Cursor(at));
                out.push(Out::Mouse(MouseButton::Left, false));
            }
            Role::RightDrag => {
                out.push(Out::Cursor(at));
                out.push(Out::Mouse(MouseButton::Right, false));
            }
            // A swipe let go while still moving glides on; one that stopped first doesn't.
            Role::Scroll if ended => {
                let still = now.saturating_duration_since(f.moved_at) > Duration::from_millis(80);
                if !still && f.speed.abs() > FLING_START {
                    self.fling = Some((f.speed, now));
                }
            }
            // A right click where the first finger rests; that finger is no longer a tap.
            Role::Second if ended => {
                if let Some(first) = self.fingers.iter_mut().find(|f| f.role == Role::Pending) {
                    first.role = Role::Scroll;
                    out.push(Out::Cursor(first.last));
                    out.push(Out::Mouse(MouseButton::Right, true));
                    out.push(Out::Mouse(MouseButton::Right, false));
                }
            }
            Role::Key(i) => {
                if let Some(Key::Keys(keys)) = layout.keys.get(i).map(|k| k.key) {
                    out.push(Out::Keys(keys, false));
                }
            }
            _ => {}
        }
    }

    /// What the fingers hold, from scratch.
    fn hold(&mut self, layout: &Layout) {
        let mut held = PadState::default();
        for f in &self.fingers {
            match f.role {
                Role::Key(i) => match layout.keys.get(i).map(|k| k.key) {
                    Some(Key::Pad(button)) => held.set(button, true),
                    Some(Key::Trigger(t)) => held.triggers[t] = 1.0,
                    Some(Key::Keys(_)) | None => {}
                },
                Role::Stick => {
                    let r = f64::from(layout.stick_radius);
                    let x = ((f.last[0] - f.start[0]) / r) as f32;
                    // Up is positive on a pad, down on the glass.
                    let y = ((f.start[1] - f.last[1]) / r) as f32;
                    let length = x.hypot(y);
                    if length > SPRINT_AT {
                        held.set(Button::LeftStick, true);
                    }
                    let shrink = if length > 1.0 { 1.0 / length } else { 1.0 };
                    held.left = [x * shrink, y * shrink];
                }
                _ => {}
            }
        }
        self.held = held;
    }
}

/// The platform's picture picker (Edit HUD's Picture): the launcher installs its own (iOS'
/// UIImagePickerController, Android's `ACTION_GET_CONTENT`, a desktop file dialog). The default
/// does nothing and logs: the picker itself arrives with stage 2.3.
pub(super) type PicturePicker = fn(&str);
static PICTURE_PICKER: std::sync::OnceLock<PicturePicker> = std::sync::OnceLock::new();

#[allow(dead_code, reason = "the platform launcher installs its picture picker here (stage 2.3)")]
pub(super) fn set_picture_picker(picker: PicturePicker) {
    let _ = PICTURE_PICKER.set(picker);
}

fn ask_picture(id: &str) {
    let picker = PICTURE_PICKER.get_or_init(|| -> PicturePicker {
        |id| log::info!("Horizon: the touch picture picker is not on this platform yet (action {id})")
    });
    picker(id);
}

impl Game<'_> {
    /// Whether the fingers play the world (the game holds the pointer) or work a screen.
    pub(super) fn touch_world(&self) -> bool {
        self.captured && self.title.is_none() && self.screen == Screen::Playing
    }

    fn touch_size(&self) -> Option<[u32; 2]> {
        if let Some(r) = &self.renderer {
            return Some([r.config.width, r.config.height]);
        }
        let size = self.window.as_ref()?.inner_size();
        Some([size.width, size.height])
    }

    fn touch_gui_scale(&self, size: [u32; 2]) -> f32 {
        HudBatch::new(size, self.settings.gui_scale_inputs()).scale
    }

    /// The safe area's insets in the frame's pixels (left, top, right, bottom): the iOS notch
    /// and home indicator (`safe_insets_of`). Android's cutout is not read yet: zero there, and
    /// the layout keeps its own margins.
    pub(super) fn safe_insets(&self) -> [f32; 4] {
        match self.touch_size() {
            Some(size) => safe_insets_of(self.window.as_deref(), size),
            None => [0.0; 4],
        }
    }

    fn touch_layout(&self) -> Option<Layout> {
        let size = self.touch_size()?;
        Some(screen_layout(
            size,
            self.touch_gui_scale(size),
            self.safe_insets(),
            self.touch.hud.current(),
        ))
    }

    /// The default layout of this screen, clear of its safe area.
    fn default_hud(&self, size: [u32; 2]) -> HudLayout {
        screen_layout(size, self.touch_gui_scale(size), self.safe_insets(), None)
            .hud([size[0] as f32, size[1] as f32])
    }

    /// Edit HUD: the layouts where they would be in the world, the tools and the Add list.
    pub(super) fn open_touch_editor(&mut self) {
        let Some(size) = self.touch_size() else {
            return;
        };
        self.touch.load(size, self.touch_gui_scale(size));
        let defaults = self.default_hud(size);
        let layouts = super::touch_editor::slots(&self.touch.hud.layouts, &defaults);
        self.touch.editor = Some(super::touch_editor::Editor::new(layouts, self.touch.hud.active));
    }

    /// A finger in Edit HUD.
    fn touch_editor_event(&mut self, touch: &Touch, at: [f64; 2], size: [u32; 2]) {
        let fsize = [size[0] as f32, size[1] as f32];
        let defaults = self.default_hud(size);
        let Some(editor) = &mut self.touch.editor else {
            return;
        };
        let at = [at[0] as f32, at[1] as f32];
        if let Some((layouts, active)) = editor.touch(touch.id, touch.phase, at, fsize, &defaults) {
            // A layout like the default is no layout: it follows the game's later changes.
            let layouts: Vec<Option<HudLayout>> = layouts
                .into_iter()
                .map(|l| (l != defaults).then_some(l))
                .collect();
            self.touch.hud = HudFile { active, layouts };
            if self.touch.hud.layouts.iter().all(Option::is_none) && active == 0 {
                let _ = std::fs::remove_file(LAYOUT_FILE);
            } else {
                save_layouts(&self.touch.hud);
            }
            self.touch.editor = None;
            return;
        }
        // Picture was tapped: ask the platform's picker for that action's picture; No pic takes
        // the player's own away.
        let request = editor.picture_request.take();
        let clear = editor.picture_clear.take();
        if let Some(id) = request {
            ask_picture(&id);
        }
        if let Some(id) = clear {
            self.remove_picture(&id);
        }
    }

    /// Takes the player's own picture of action `id` away (Edit HUD's No pic).
    fn remove_picture(&mut self, id: &str) {
        let dir = horizon_client::touch_icons::user_dir();
        let note = if horizon_client::touch_icons::remove_user_icon(&dir, id) {
            super::touch_editor::tr("horizon.touch.editor.note.picture_removed").replace("%s", id)
        } else {
            super::touch_editor::tr("horizon.touch.editor.note.no_picture").replace("%s", id)
        };
        if let Some(editor) = &mut self.touch.editor {
            editor.note = Some(note);
        }
    }

    /// Whether the chat screen is open (its buttons show).
    fn chat_open(&self) -> bool {
        self.title.is_none() && self.screen == Screen::Chat
    }

    /// A chat button: Tab, Paste or Enter.
    fn touch_chat_key(&mut self, key: ChatKey) {
        match key {
            ChatKey::Tab => {
                self.touch_key("tab", true);
                self.touch_key("tab", false);
            }
            ChatKey::Enter => {
                self.touch_key("enter", true);
                self.touch_key("enter", false);
            }
            ChatKey::Paste => {
                let text = self.clipboard.text();
                if !text.is_empty() {
                    self.ime_commit(&text.replace(['\r', '\n'], " "));
                }
            }
        }
    }

    /// The first finger turns touch on while options.txt has no `touchscreen` key (a touchscreen
    /// laptop plays like a phone); an explicit off stays off. The Touch Controls option
    /// (Options > Controls) turns it on or off for good.
    fn touch_auto_enable(&mut self) -> bool {
        if self.settings.touchscreen {
            return true;
        }
        if self.settings.file.get("touchscreen").is_some() {
            return false;
        }
        self.settings.touchscreen = true;
        true
    }

    /// A finger on the glass (`WindowEvent::Touch`).
    pub(super) fn touch_event(&mut self, event_loop: &dyn EventLoopCtl, id: WindowId, touch: Touch) {
        if !self.touch_auto_enable() {
            return;
        }
        let Some(size) = self.touch_size() else {
            return;
        };
        self.touch.load(size, self.touch_gui_scale(size));
        let Some(layout) = self.touch_layout() else {
            return;
        };
        let at = touch_pixels(self.window.as_deref(), size, &touch);
        if self.touch.editor.is_some() {
            self.last_input = Instant::now();
            return self.touch_editor_event(&touch, at, size);
        }
        let world = self.touch_world();
        let [inset_left, inset_top, _, _] = self.safe_insets();
        // The chat screen's own buttons.
        if self.chat_open() && touch.phase == TouchPhase::Started {
            let p = [at[0] as f32, at[1] as f32];
            if let Some(&(key, _, _)) = layout.chat_keys.iter().find(|(_, _, r)| inside(*r, p)) {
                self.last_input = Instant::now();
                return self.touch_chat_key(key);
            }
        }
        // Edit HUD's own plate over the title: the touch layer's entry to the editor.
        if !world
            && self.title.is_some()
            && touch.phase == TouchPhase::Started
            && inside(layout.edit, [at[0] as f32, at[1] as f32])
        {
            self.last_input = Instant::now();
            return self.open_touch_editor();
        }
        if !world && touch.phase == TouchPhase::Started {
            let batch = HudBatch::new(size, self.settings.gui_scale_inputs());
            let gui = (
                (at[0] as f32 - inset_left) / batch.scale,
                (at[1] as f32 - inset_top) / batch.scale,
            );
            // A finger landing on a slot with a stack carried slides to split it.
            self.touch.slot_under_next =
                self.carried.is_some() && self.slot_at(&batch, gui.0, gui.1).is_some();
        }
        let outs = self
            .touch
            .touch(touch.id, touch.phase, at, Instant::now(), world, &layout);
        self.last_input = Instant::now();
        self.touch_outs(event_loop, id, outs, [inset_left, inset_top]);
    }

    /// Each frame: a list let go mid-swipe keeps gliding.
    pub(super) fn touch_frame(&mut self, event_loop: &dyn EventLoopCtl) {
        if !self.settings.touchscreen {
            return;
        }
        let outs = self.touch.fling_frame(Instant::now());
        if outs.is_empty() {
            return;
        }
        let Some(id) = self.window.as_ref().map(|w| w.id()) else {
            return;
        };
        self.touch_outs(event_loop, id, outs, [0.0, 0.0]);
    }

    /// Does what the fingers asked: pointer events (moved out of the safe area's `inset`), turns
    /// of the view, Esc and keys.
    fn touch_outs(
        &mut self,
        event_loop: &dyn EventLoopCtl,
        id: WindowId,
        outs: Vec<Out>,
        [inset_left, inset_top]: [f32; 2],
    ) {
        for out in outs {
            let out = match out {
                Out::Cursor([x, y]) => {
                    Out::Cursor([x - f64::from(inset_left), y - f64::from(inset_top)])
                }
                out => out,
            };
            match (out, out.window_event()) {
                (_, Some(event)) => {
                    self.handle_window_event(event_loop, id, event, input_trace::InputOrigin::User);
                }
                // MouseHandler.onMove with the pointer grabbed, as `device_event` does.
                (Out::Look(x, y), None) => {
                    if self.pending_save.is_none() {
                        self.mouse.add_motion(x, y);
                    }
                }
                (Out::Escape, None) => {
                    if let Err(e) = self.pad_back() {
                        log::warn!("Horizon: Esc: {e}");
                    }
                }
                (Out::Keys(keys, down), None) => {
                    let names: Vec<&str> = if down {
                        keys.to_vec()
                    } else {
                        keys.iter().rev().copied().collect()
                    };
                    for name in names {
                        self.touch_key(name, down);
                    }
                }
                _ => {}
            }
        }
    }

    /// A touch button's keyboard key, as the window's own: winit's `KeyEvent` cannot be built
    /// outside winit, so this is the window's key path in its decomposed form — the debug keys
    /// first (F1, F2, F3 and its combinations), the chat screen's raw keys, the open screen's
    /// hooks, the mods' mappings and `input_key`, then the bindings. It works on every build,
    /// with or without the control socket.
    fn touch_key(&mut self, name: &str, down: bool) {
        let Some(code) = horizon_client::keybinds::key_code(&format!("key.keyboard.{name}")) else {
            log::warn!("Horizon: touch button: unknown key {name}");
            return;
        };
        if down {
            self.physical_keys.insert(code);
        } else {
            self.physical_keys.remove(&code);
        }
        // KeyboardHandler.keyPress: the debug keys come before the game's.
        let action = if down {
            horizon_client::debug_keys::KeyAction::Press
        } else {
            horizon_client::debug_keys::KeyAction::Release
        };
        if self.debug_key(code, action) {
            return;
        }
        if down && self.key_press_canceled(code, true, false) {
            return;
        }
        // The chat screen takes its keys raw: Tab completes, Enter sends (ChatScreen.keyPressed).
        if self.screen == Screen::Chat && self.title.is_none() {
            if down {
                self.keys.insert(code);
                self.chat_key_code(code, None);
            } else {
                self.keys.remove(&code);
            }
            return;
        }
        // ScreenEvent.KeyPressed.Pre / .Post: the open screen hears the key before the binds.
        if down && self.screen_key_pressed(code, false) {
            return;
        }
        if self.screen_key_post(code, down, false) {
            return;
        }
        // The mods' mappings and `input_key`, as the window's own key path fires them.
        let key_name = horizon_client::keybinds::key_name(code);
        let layout_key = match code {
            KeyCode::Slash => None,
            _ => key_name,
        };
        let raw_code = horizon_client::keybinds::raw_key_code(PhysicalKey::Code(code));
        let modifiers = super::glfw_modifiers(self.modifiers);
        let action = if down {
            mod_api::KeyAction::Press
        } else {
            mod_api::KeyAction::Release
        };
        self.with_user_input(true, |game| {
            game.key_input(key_name, layout_key, raw_code, action, modifiers);
        });
        if let Err(e) = self.mapped_key_event(code, down, false) {
            log::warn!("Horizon: touch key {name}: {e}");
        }
    }

    /// Edit HUD over the title: the layout where it would be in the world, the tools and the Add
    /// list.
    fn draw_touch_editor(&self, batch: &mut HudBatch) {
        use super::touch_editor::{Sel, Tool, tr};
        let Some(editor) = &self.touch.editor else {
            return;
        };
        let Some(size) = self.touch_size() else {
            return;
        };
        let fsize = [size[0] as f32, size[1] as f32];
        let layout = editor.current();
        let s = batch.scale;
        let atlas = &self.gui.atlas;
        let white = rgba(255, 255, 255, 230);
        let selected = rgba(90, 160, 255, 170);
        let opacity = layout.opacity;
        let fade = |a: u8| (f32::from(a) * opacity).round() as u8;
        let plate = |batch: &mut HudBatch, r: [f32; 4], fill: u32, label: &str| {
            let [x, y, w, h] = r.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgba(0, 0, 0, 120));
            batch.fill(atlas, x, y, w, h, fill);
            let tw = HudBatch::text_width(atlas, label);
            let fit = ((w - 4.0) / tw).min(1.0);
            if fit >= 1.0 {
                batch.text(atlas, label, x + (w - tw) / 2.0, y + (h - 8.0) / 2.0, white, true);
            } else {
                // Drawn at its size, then shrunk about its corner into the plate.
                let (lx, ly) = (x + (w - tw * fit) / 2.0, y + (h - 8.0 * fit) / 2.0);
                let start = batch.vertices.len();
                batch.text(atlas, label, 0.0, 0.0, white, true);
                for v in &mut batch.vertices[start..] {
                    v.pos = [lx + v.pos[0] * fit, ly + v.pos[1] * fit];
                }
            }
        };
        batch.fill(atlas, 0.0, 0.0, fsize[0] / s, fsize[1] / s, rgba(0, 0, 0, 150));
        let stick = layout.stick_rect(fsize);
        let [zx, zy, zw, zh] = stick.map(|v| v / s);
        let zone = if editor.selected == Some(Sel::Stick) {
            rgba(90, 160, 255, 60)
        } else {
            rgba(255, 255, 255, 18)
        };
        batch.fill(atlas, zx, zy, zw, zh, zone);
        let note = tr("horizon.touch.editor.stick_area");
        batch.text(atlas, &note, zx + 6.0, zy + 6.0, rgba(255, 255, 255, 140), true);
        let esc_fill = if editor.selected == Some(Sel::Esc) {
            selected
        } else {
            rgba(0, 0, 0, fade(160))
        };
        plate(batch, layout.esc_rect(fsize), esc_fill, &tr("horizon.touch.esc"));
        for (i, b) in layout.buttons.iter().enumerate() {
            let label = find_action(&b.id).map_or("?".to_owned(), |a| tr(a.label));
            let fill = if editor.selected == Some(Sel::Button(i)) {
                selected
            } else {
                rgba(255, 255, 255, fade(50))
            };
            plate(batch, b.rect(fsize), fill, &label);
        }
        let bar = super::touch_editor::toolbar(fsize);
        if let (Some(top), Some(bottom)) = (
            bar.iter().map(|t| t.2[1]).reduce(f32::min),
            bar.iter().map(|t| t.2[1] + t.2[3]).reduce(f32::max),
        ) {
            let pad = 4.0;
            let (top, bottom) = (top / s - pad, bottom / s + pad + 12.0);
            batch.fill(atlas, 0.0, top, fsize[0] / s, bottom - top, rgba(0, 0, 0, 200));
        }
        for (tool, label, r) in bar {
            let usable = match tool {
                Tool::Remove => matches!(editor.selected, Some(Sel::Button(_))),
                Tool::Smaller | Tool::Bigger => editor.selected.is_some(),
                Tool::Picture | Tool::NoPicture => {
                    matches!(editor.selected, Some(Sel::Button(_) | Sel::Stick))
                }
                _ => true,
            };
            let fill = if tool == Tool::Done {
                rgba(60, 150, 70, 200)
            } else if !usable {
                rgba(40, 40, 40, 240)
            } else {
                rgba(70, 70, 70, 250)
            };
            let label = match tool {
                Tool::Layout => tr("horizon.touch.editor.layout")
                    .replace("%s", &(editor.active + 1).to_string()),
                _ => tr(label),
            };
            plate(batch, r, fill, &label);
        }
        let hint = if let Some(note) = &editor.note {
            note.clone()
        } else if editor.picking {
            tr("horizon.touch.editor.hint.picking")
        } else {
            tr(match editor.selected {
                Some(Sel::Button(_)) => "horizon.touch.editor.hint.button",
                Some(Sel::Stick) => "horizon.touch.editor.hint.stick",
                Some(Sel::Esc) => "horizon.touch.editor.hint.esc",
                None => "horizon.touch.editor.hint.none",
            })
        };
        let hint = hint.as_str();
        let tw = HudBatch::text_width(atlas, hint);
        let hy = super::touch_editor::toolbar_bottom(fsize) / s + 4.0;
        batch.text(atlas, hint, (fsize[0] / s - tw) / 2.0, hy, white, true);
        if editor.picking {
            let (panel, tiles) = super::touch_editor::picker(fsize, &layout.buttons);
            let [px, py, pw, ph] = panel.map(|v| v / s);
            batch.fill(atlas, px - 1.0, py - 1.0, pw + 2.0, ph + 2.0, rgba(255, 255, 255, 90));
            batch.fill(atlas, px, py, pw, ph, rgba(20, 20, 20, 235));
            let title = tr("horizon.touch.editor.add_title");
            let tw = HudBatch::text_width(atlas, &title);
            batch.text(atlas, &title, px + (pw - tw) / 2.0, py + 6.0, white, true);
            for (action, r) in tiles {
                plate(batch, r, rgba(70, 70, 70, 230), &tr(ACTIONS[action].name));
            }
        }
    }

    /// The buttons and the stick over the HUD, while the fingers play the world.
    pub(super) fn draw_touch_controls(&mut self, batch: &mut HudBatch) {
        if !self.settings.touchscreen {
            return;
        }
        let Some(size) = self.touch_size() else {
            return;
        };
        self.touch.load(size, self.touch_gui_scale(size));
        let Some(layout) = self.touch_layout() else {
            return;
        };
        if self.touch.editor.is_some() {
            return self.draw_touch_editor(batch);
        }
        // A controller, keyboard or mouse plays: the buttons step aside until the next touch.
        if self.touch.hardware {
            return;
        }
        let s = batch.scale;
        let atlas = &self.gui.atlas;
        let opacity = layout.opacity;
        let fade = |a: u8| (f32::from(a) * opacity).round() as u8;
        let idle = rgba(255, 255, 255, fade(34));
        let down = rgba(255, 255, 255, fade(96).max(40));
        let edge = rgba(0, 0, 0, fade(70));
        let ink = rgba(255, 255, 255, fade(200).max(60));
        let tr = super::touch_editor::tr;
        {
            let [x, y, w, h] = layout.escape.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
            let plate = if self.touch.escape_held() {
                down
            } else {
                rgba(0, 0, 0, fade(90))
            };
            batch.fill(atlas, x, y, w, h, plate);
            let esc = tr("horizon.touch.esc");
            let tw = HudBatch::text_width(atlas, &esc);
            batch.text(
                atlas,
                &esc,
                x + (w - tw) / 2.0,
                y + (h - 8.0) / 2.0,
                rgba(255, 255, 255, fade(220).max(60)),
                true,
            );
        }
        // Edit HUD's plate over the title, top left.
        if self.title.is_some() {
            let [x, y, w, h] = layout.edit.map(|v| v / s);
            batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
            batch.fill(atlas, x, y, w, h, rgba(0, 0, 0, fade(90)));
            let label = tr("horizon.touch.editor");
            let tw = HudBatch::text_width(atlas, &label);
            let fit = ((w - 4.0) / tw).min(1.0);
            let (lx, ly) = (x + (w - tw * fit) / 2.0, y + (h - 8.0 * fit) / 2.0);
            let start = batch.vertices.len();
            batch.text(atlas, &label, 0.0, 0.0, rgba(255, 255, 255, fade(220).max(60)), true);
            for v in &mut batch.vertices[start..] {
                v.pos = [lx + v.pos[0] * fit, ly + v.pos[1] * fit];
            }
        }
        if self.chat_open() {
            for &(_, label, r) in &layout.chat_keys {
                let [x, y, w, h] = r.map(|v| v / s);
                batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgba(0, 0, 0, 90));
                batch.fill(atlas, x, y, w, h, rgba(0, 0, 0, 150));
                let label = tr(label);
                let tw = HudBatch::text_width(atlas, &label);
                let white = rgba(255, 255, 255, 230);
                batch.text(atlas, &label, x + (w - tw) / 2.0, y + (h - 8.0) / 2.0, white, true);
            }
        }
        if !self.touch_world() || self.ui.hide_gui {
            return;
        }
        for (i, key) in layout.keys.iter().enumerate() {
            let [x, y, w, h] = key.rect.map(|v| v / s);
            // The player's own picture (touch-icons/, Edit HUD's Picture) is the whole button:
            // no plate under it and no frame around it.
            let own = format!("touch/user/{}", key.id);
            if !key.id.is_empty() && atlas.sprites.contains_key(&own) {
                let tint = if self.touch.key_held(i) {
                    rgba(170, 170, 170, fade(255))
                } else {
                    rgba(255, 255, 255, fade(255))
                };
                batch.sprite_scaled(atlas, &own, x, y, w, h, tint);
                continue;
            }
            let icon = format!("touch/{}", key.id);
            let has_icon = !key.id.is_empty() && atlas.sprites.contains_key(&icon);
            if has_icon && atlas.sprites.contains_key("touch/plate") {
                // The rounded plate with its outline (the picture is tinted by the press).
                let tint = if self.touch.key_held(i) {
                    rgba(165, 165, 165, fade(255))
                } else {
                    rgba(255, 255, 255, fade(225))
                };
                batch.sprite_scaled(atlas, "touch/plate", x - 1.0, y - 1.0, w + 2.0, h + 2.0, tint);
            } else {
                batch.fill(atlas, x - 1.0, y - 1.0, w + 2.0, h + 2.0, edge);
                let colour = if self.touch.key_held(i) { down } else { idle };
                batch.fill(atlas, x, y, w, h, colour);
            }
            // A resource pack's picture for the button (`textures/gui/sprites/touch/<id>.png`,
            // the action's id: jump, sneak, use, hit, inventory...) takes the place of its text.
            if has_icon {
                let pad = w.min(h) * 0.17;
                // The face of the button sits a little above its middle (the bevel is below it).
                let lift = h * 0.03;
                batch.sprite_scaled(
                    atlas,
                    &icon,
                    x + pad,
                    y + pad - lift,
                    w - 2.0 * pad,
                    h - 2.0 * pad,
                    rgba(255, 255, 255, fade(235).max(60)),
                );
            } else {
                let label = tr(key.label);
                let tw = HudBatch::text_width(atlas, &label);
                batch.text(atlas, &label, x + (w - tw) / 2.0, y + (h - 8.0) / 2.0, ink, true);
            }
        }
        if let Some((base, knob)) = self.touch.stick() {
            let r = layout.stick_radius / s;
            let [bx, by] = base.map(|v| v as f32 / s);
            let (dx, dy) = (knob[0] as f32 / s - bx, knob[1] as f32 / s - by);
            let length = dx.hypot(dy).max(0.001);
            let reach = length.min(r) / length;
            let k = r * 0.45;
            // The base ring and the ball are pictures (`touch/stick_base`, `touch/stick_knob`)
            // when the atlas has them, else plain squares.
            if atlas.sprites.contains_key("touch/stick_base") {
                batch.sprite_scaled(atlas, "touch/stick_base", bx - r, by - r, r * 2.0, r * 2.0, rgba(255, 255, 255, fade(120).max(40)));
            } else {
                batch.fill(atlas, bx - r - 1.0, by - r - 1.0, r * 2.0 + 2.0, r * 2.0 + 2.0, edge);
                batch.fill(atlas, bx - r, by - r, r * 2.0, r * 2.0, idle);
            }
            let (kx, ky) = (bx + dx * reach - k, by + dy * reach - k);
            if atlas.sprites.contains_key("touch/stick_knob") {
                batch.sprite_scaled(atlas, "touch/stick_knob", kx, ky, k * 2.0, k * 2.0, rgba(255, 255, 255, fade(235).max(60)));
            } else {
                batch.fill(atlas, kx, ky, k * 2.0, k * 2.0, down);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: [f32; 2] = [2556.0, 1179.0];

    fn layout() -> Layout {
        Layout::new(SIZE, 4.0)
    }
    fn centre(l: &Layout, id: &str) -> [f64; 2] {
        let k = l.keys.iter().find(|k| k.id == id).unwrap();
        [
            f64::from(k.rect[0] + k.rect[2] / 2.0),
            f64::from(k.rect[1] + k.rect[3] / 2.0),
        ]
    }

    #[test]
    fn the_buttons_fit_the_screen_and_do_not_overlap() {
        for size in [SIZE, [1334.0, 750.0], [2796.0, 1290.0], [2048.0, 1536.0]] {
            let l = Layout::new(size, 3.0);
            for (i, a) in l.keys.iter().enumerate() {
                let [x, y, w, h] = a.rect;
                assert!(x >= 0.0 && y >= 0.0 && x + w <= size[0] && y + h <= size[1], "{a:?}");
                for b in &l.keys[i + 1..] {
                    let [bx, by, bw, bh] = b.rect;
                    let apart = x + w <= bx || bx + bw <= x || y + h <= by || by + bh <= y;
                    assert!(apart, "{a:?} overlaps {b:?} at {size:?}");
                }
                let [ex, ey, ew, eh] = l.escape;
                let clear = x + w <= ex || ex + ew <= x || y + h <= ey || ey + eh <= y;
                assert!(clear, "{a:?} overlaps Esc at {size:?}");
                // The stick's area holds no button but the hotbar's, which a finger meets first.
                assert!(!inside(l.stick_zone, [x + w / 2.0, y + h / 2.0]) || a.id == "hotbar_previous");
            }
        }
    }

    #[test]
    fn the_default_buttons_keep_clear_of_the_safe_area() {
        // An iPhone held sideways: the Dynamic Island's side, the other side and the home
        // indicator (59, 59 and 21 points at 3x).
        let insets = [177.0, 0.0, 177.0, 63.0];
        let l = Layout::new_in(SIZE, 4.0, insets);
        let inside_safe = |[x, y, w, h]: [f32; 4]| {
            x >= insets[0] && y >= insets[1] && x + w <= SIZE[0] - insets[2] && y + h <= SIZE[1] - insets[3]
        };
        for k in &l.keys {
            assert!(inside_safe(k.rect), "{k:?}");
        }
        assert!(inside_safe(l.escape));
        assert!(inside_safe(l.edit));
        for (_, _, r) in &l.chat_keys {
            assert!(inside_safe(*r), "{r:?}");
        }
        assert!(l.stick_zone[0] >= insets[0]);
    }

    #[test]
    fn esc_works_in_the_world_and_in_screens_and_does_nothing_else() {
        let (l, t0) = (layout(), Instant::now());
        let [x, y, w, h] = l.escape;
        let esc = [f64::from(x + w / 2.0), f64::from(y + h / 2.0)];
        for world in [true, false] {
            let mut c = Controls::default();
            assert_eq!(c.touch(1, TouchPhase::Started, esc, t0, world, &l), [Out::Escape]);
            assert!(c.escape_held());
            let away = [esc[0] + 200.0, esc[1] + 200.0];
            assert!(c.touch(1, TouchPhase::Moved, away, t0, world, &l).is_empty());
            assert!(c.touch(1, TouchPhase::Ended, away, t0, world, &l).is_empty());
            assert_eq!(c.poll(), PadState::default());
        }
    }

    #[test]
    fn a_tap_in_a_screen_is_a_click_where_it_lands() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        assert_eq!(
            c.touch(1, TouchPhase::Started, [300.0, 200.0], t0, false, &l),
            [Out::Cursor([300.0, 200.0])]
        );
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [302.0, 201.0], t0, false, &l),
            [
                Out::Cursor([302.0, 201.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Mouse(MouseButton::Left, false)
            ]
        );
    }

    #[test]
    fn a_quick_vertical_swipe_scrolls_and_never_clicks() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let moved = c.touch(1, TouchPhase::Moved, [305.0, 600.0 + f64::from(l.line) * 2.0], t0, false, &l);
        assert_eq!(moved, [Out::Wheel(2.0)]);
        assert!(c.touch(1, TouchPhase::Ended, [305.0, 900.0], t0, false, &l).is_empty());
    }

    #[test]
    fn a_sideways_or_held_drag_is_a_button_drag() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        assert_eq!(
            c.touch(1, TouchPhase::Moved, [500.0, 610.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Cursor([500.0, 610.0])
            ]
        );
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [520.0, 610.0], t0, false, &l),
            [Out::Cursor([520.0, 610.0]), Out::Mouse(MouseButton::Left, false)]
        );
        // Held still first, then moved down: a drag too.
        c.touch(2, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let later = t0 + HOLD;
        let moved = c.touch(2, TouchPhase::Moved, [300.0, 800.0], later, false, &l);
        assert_eq!(moved[1], Out::Mouse(MouseButton::Left, true));
    }

    #[test]
    fn a_carried_stack_slid_down_from_a_slot_spreads_instead_of_scrolling() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.slot_under_next = true;
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        // Straight down, quickly: a scroll anywhere else, the left button's even split here.
        assert_eq!(
            c.touch(1, TouchPhase::Moved, [300.0, 700.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Cursor([300.0, 700.0])
            ]
        );
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [300.0, 760.0], t0, false, &l),
            [Out::Cursor([300.0, 760.0]), Out::Mouse(MouseButton::Left, false)]
        );
        // The hint is for one finger only: the next swipe scrolls again.
        c.touch(2, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let next = c.touch(2, TouchPhase::Moved, [300.0, 700.0], t0, false, &l);
        assert!(matches!(next[0], Out::Wheel(_)));
    }

    #[test]
    fn a_carried_stack_held_then_slid_puts_one_in_each_slot() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.slot_under_next = true;
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        let moved = c.touch(1, TouchPhase::Moved, [400.0, 600.0], t0 + HOLD, false, &l);
        assert_eq!(moved[1], Out::Mouse(MouseButton::Right, true));
        let lifted = c.touch(1, TouchPhase::Ended, [420.0, 600.0], t0 + HOLD, false, &l);
        assert_eq!(lifted[1], Out::Mouse(MouseButton::Right, false));
        // A screen closing mid-drag lets go of the right button too.
        c.slot_under_next = true;
        c.touch(2, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        c.touch(2, TouchPhase::Moved, [400.0, 600.0], t0 + HOLD, false, &l);
        let closed = c.touch(2, TouchPhase::Moved, [410.0, 600.0], t0 + HOLD, true, &l);
        assert_eq!(closed[0], Out::Mouse(MouseButton::Right, false));
    }

    #[test]
    fn a_tap_on_a_slot_with_a_stack_is_still_a_click() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.slot_under_next = true;
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        assert_eq!(
            c.touch(1, TouchPhase::Ended, [300.0, 600.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Left, true),
                Out::Mouse(MouseButton::Left, false)
            ]
        );
    }

    fn wheel_total(outs: &[Out]) -> f32 {
        outs.iter()
            .map(|o| if let Out::Wheel(lines) = o { *lines } else { 0.0 })
            .sum()
    }

    #[test]
    fn a_flicked_list_glides_then_stops_and_a_finger_stops_it() {
        let (l, t0) = (layout(), Instant::now());
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        for step in 1..=5u64 {
            let y = 600.0 + 40.0 * step as f64;
            c.touch(1, TouchPhase::Moved, [300.0, y], ms(16 * step), false, &l);
        }
        c.touch(1, TouchPhase::Ended, [300.0, 800.0], ms(80), false, &l);
        let first = wheel_total(&c.fling_frame(ms(96)));
        assert!(first > 0.0, "it glides on in the swipe's direction");
        let mut later = 0.0;
        let mut frame = 96;
        let mut stopped = false;
        while frame < 6000 {
            frame += 16;
            let outs = c.fling_frame(ms(frame));
            if outs.is_empty() {
                stopped = true;
                break;
            }
            later = wheel_total(&outs);
        }
        assert!(stopped, "it slows down and stops");
        assert!(later < first, "each frame moves less than the one before");
        // A new finger stops a glide at once.
        c.touch(2, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        for step in 1..=5u64 {
            let y = 600.0 + 40.0 * step as f64;
            c.touch(2, TouchPhase::Moved, [300.0, y], ms(16 * step), false, &l);
        }
        c.touch(2, TouchPhase::Ended, [300.0, 800.0], ms(80), false, &l);
        assert!(!c.fling_frame(ms(96)).is_empty());
        c.touch(3, TouchPhase::Started, [300.0, 600.0], ms(90), false, &l);
        assert!(c.fling_frame(ms(110)).is_empty());
    }

    #[test]
    fn a_swipe_that_stopped_before_lifting_does_not_glide() {
        let (l, t0) = (layout(), Instant::now());
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        for step in 1..=5u64 {
            let y = 600.0 + 40.0 * step as f64;
            c.touch(1, TouchPhase::Moved, [300.0, y], ms(16 * step), false, &l);
        }
        c.touch(1, TouchPhase::Ended, [300.0, 800.0], ms(400), false, &l);
        assert!(c.fling_frame(ms(416)).is_empty());
    }

    #[test]
    fn a_second_finger_right_clicks_at_the_first() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [300.0, 600.0], t0, false, &l);
        assert!(c.touch(2, TouchPhase::Started, [900.0, 600.0], t0, false, &l).is_empty());
        assert_eq!(
            c.touch(2, TouchPhase::Ended, [900.0, 600.0], t0, false, &l),
            [
                Out::Cursor([300.0, 600.0]),
                Out::Mouse(MouseButton::Right, true),
                Out::Mouse(MouseButton::Right, false)
            ]
        );
        // The first finger lifting afterwards does not left click.
        assert!(c.touch(1, TouchPhase::Ended, [300.0, 600.0], t0, false, &l).is_empty());
    }

    #[test]
    fn the_stick_moves_and_sprints_past_its_edge() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        let base = [400.0, 900.0];
        c.touch(1, TouchPhase::Started, base, t0, true, &l);
        let r = f64::from(l.stick_radius);
        c.touch(1, TouchPhase::Moved, [base[0], base[1] - r / 2.0], t0, true, &l);
        let pad = c.poll();
        assert!((pad.left[1] - 0.5).abs() < 1e-4 && pad.left[0] == 0.0, "{pad:?}");
        assert!(!pad.held(Button::LeftStick));
        c.touch(1, TouchPhase::Moved, [base[0], base[1] - r * 2.0], t0, true, &l);
        let pad = c.poll();
        assert!((pad.left[1] - 1.0).abs() < 1e-4);
        assert!(pad.held(Button::LeftStick));
        c.touch(1, TouchPhase::Ended, base, t0, true, &l);
        assert_eq!(c.poll(), PadState::default());
    }

    #[test]
    fn a_button_tapped_between_two_polls_still_counts_once() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        let jump = centre(&l, "jump");
        c.touch(1, TouchPhase::Started, jump, t0, true, &l);
        c.touch(1, TouchPhase::Ended, jump, t0, true, &l);
        assert!(c.poll().held(Button::South));
        assert!(!c.poll().held(Button::South));
        // The triggers too.
        let hit = centre(&l, "hit");
        c.touch(1, TouchPhase::Started, hit, t0, true, &l);
        assert_eq!(c.poll().triggers, [0.0, 1.0]);
        assert_eq!(c.poll().triggers, [0.0, 1.0]);
        c.touch(1, TouchPhase::Ended, hit, t0, true, &l);
        assert_eq!(c.poll().triggers, [0.0, 0.0]);
    }

    #[test]
    fn fingers_off_the_stick_and_on_buttons_turn_the_view() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [1500.0, 500.0], t0, true, &l);
        assert_eq!(
            c.touch(1, TouchPhase::Moved, [1510.0, 480.0], t0, true, &l),
            [Out::Look(10.0 * LOOK_SCALE, -20.0 * LOOK_SCALE)]
        );
        let hit = centre(&l, "hit");
        c.touch(2, TouchPhase::Started, hit, t0, true, &l);
        let moved = c.touch(2, TouchPhase::Moved, [hit[0] + 4.0, hit[1]], t0, true, &l);
        assert_eq!(moved, [Out::Look(4.0 * LOOK_SCALE, 0.0)]);
        // Still held after sliding off the button.
        assert_eq!(c.poll().triggers[1], 1.0);
    }

    #[test]
    fn a_screen_opening_lets_go_of_everything() {
        let (l, t0) = (layout(), Instant::now());
        let mut c = Controls::default();
        c.touch(1, TouchPhase::Started, [400.0, 900.0], t0, true, &l);
        c.touch(1, TouchPhase::Moved, [400.0, 700.0], t0, true, &l);
        c.poll();
        // The inventory opened: the next touch is a screen's.
        let out = c.touch(2, TouchPhase::Started, [800.0, 400.0], t0, false, &l);
        assert_eq!(out, [Out::Cursor([800.0, 400.0])]);
        assert_eq!(c.poll(), PadState::default());
        // The old stick finger lifting does nothing.
        assert!(c.touch(1, TouchPhase::Ended, [400.0, 700.0], t0, false, &l).is_empty());
    }
}
