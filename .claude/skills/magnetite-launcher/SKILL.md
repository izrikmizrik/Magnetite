---
name: magnetite-launcher
description: Change Magnetite's launcher, the screen before the game on Android, iOS and PC (accounts, sign-in pages, profiles, mods and the mod platform, settings, the game folder, the log). Use for any launcher page, setting, button, string, layout (upright phone or wide PC with a sidebar), input (touch, system keyboard, mouse wheel, Escape), profile or platform hook (orientation, share, open folder, iOS signature date) work.
---

# Magnetite's launcher

The launcher is the boot loop's screen between signing in and the game, drawn with the game's own
UI (`ui_theme::Ui`) on the boot window. One code path for every platform: a phone lays it out upright,
a PC wide with a sidebar. Spec and decisions: `docs/unification/launcher.md`; the plan:
`docs/unification/plan.md`.

## Where things live

| What | File |
| --- | --- |
| Pages, layout, drawing, taps (`Page`, `Action`, `View`, `Flow`, `LauncherScreen`) | `crates/horizon-client/src/launcher_screen.rs` |
| The controller: what the view shows, what an action does, sign-in, jobs, input | `crates/horizon-client/src/game/runtime/boot.rs` (`launcher_view`, `launcher_click`, `key_pressed`, `launcher_back`, `sync_system_keyboard`) |
| The sign-in pages' content (shared with the plain boot screen) | `crates/horizon-client/src/boot_screen.rs` (`Content`, `BootAction`) |
| Profiles (`profiles/<id>/`, `index.json`) | `crates/horizon-client/src/profiles.rs` |
| Installed mods as rows | `crates/horizon-client/src/launcher_mods.rs` |
| The mod platform (index, install) | `crates/horizon-client/src/mod_registry.rs` (`listings_any`, `install_any`) |
| Kept accounts | `crates/horizon-account/src/keyring.rs` |
| Icons (64 px PNGs in the boot atlas) | `crates/horizon-client-render/src/launcher_icons.rs`, `assets/launcher/*.png` |
| Strings | `crates/horizon-client/assets/horizon/lang/{en_us,ru_ru}.json`, keys `horizon.launcher.*` |
| Hosts (the `BootConfig` each platform gives) | `crates/magnetite-android/src/lib.rs`, `crates/magnetite-ios/src/main.rs`, `crates/horizon-game/src/bin/magnetite.rs` (PC) |

## How it fits together

- **View in, Action out.** `boot.rs` builds a `launcher_screen::View` every frame from the controller,
  the keyring, the profiles and the settings (`launcher_view`). `LauncherScreen::draw` lays the page out,
  draws it and keeps the tap targets; `click` answers the `Action` under a point. `launcher_click` acts.
  The screen holds no settings of its own: values live in the game's files (`options.txt`,
  `horizon-graphics.txt` of the **current profile**, `options_path()`).
- **Sign-in pages are a `Flow`** over the pages (`launcher_flow`): the boot screen's `Content` drawn in
  the launcher's look (`draw_flow`). Their buttons send `Action::Boot(BootAction)` to the controller.
- **Layout.** `area_for(ui)` is the page column: below the status bar on a phone; right of the
  `SIDEBAR` (232 dp), at most `CONTENT_MAX` wide, on a window wider than tall and `WIDE_MIN` (640 dp)
  wide. Every page draws in `self.area.get()`; never compute a column from `ui.w`. Pages with a bar
  scroll under it (`content_top`, `scroll_by`); the bar's targets win over rows scrolled under it.
  `density_for(width, height, scale)`: a phone gets width / 420 dp, a PC its system scale.
- **Wide only:** `draw_sidebar` (account plate, Home / Mods / Profiles / Settings, game folder, log,
  version) and `draw_home_wide`. Top pages have no back arrow there. There is no "touch layout" entry on
  a PC: controls are set in the game.
- **Input.** Taps and clicks: `click` on release; a finger that travels past `DRAG_PIXELS` scrolls
  instead. Mouse wheel: `WindowEvent::MouseWheel` → `scroll_by`. Text: on a phone the platform keyboard
  (`system_keyboard::show/poll`, `KeyboardFor`), on a PC the real keyboard (`key_pressed`). Escape
  (and a phone's back) is `launcher_back`: a step back, never quitting on a PC.
- **Profiles.** The main profile is the game folder; another is `profiles/<id>/` and the game gets
  `--player-dir` for it (`logging::set_player_dir`, which can change until the game starts). A new
  profile copies the current one's settings files. Mods are still shared by all profiles: a per-profile
  mod list needs the loader (`horizon-mod-host`, `horizon-mod-wasm`) to read the profile's
  `horizon-mods.txt`.
- **Mod platform.** Magnetite's own index (`mod_registry::INDEX_URL`,
  `izrikmizrik.github.io/magnetite-mods`, Horizon's format), never Horizon's registry. The launcher
  installs content, WebAssembly and Lua mods (`installable_any`); jobs run on a thread
  (`Platform`, `poll_platform` in `about_to_wait`).
- **Platform hooks** are plain `fn` fields of `BootConfig`, each with a no-op default:
  `set_orientation`, `share_log`, `open_folder` (returns false when it could not), `open_link`,
  `keep_awake`, `signature_expires` (iOS, from `embedded.mobileprovision`), `language`.

## Recipes

**A new page.** Add a `Page` variant; draw it in a `draw_*` method using `self.area.get()` and
`self.content_top()`; add its arm in `draw` with its bar title and back action (`top(..)` for a top page);
map it in `draw_sidebar`'s `current` and in `launcher_back`; open it with `Action::Open(Page::X)`.

**A new setting.** Add a `Setting` variant with `label_key` and `icon`, list it in `SECTIONS`, give its
value in `launcher_view`'s `values` (a setting left out of `values` is not listed: per-platform ones).
A pick list: add arms to `choices` and `choose`, and route `Action::Setting(..)` to `Page::Pick`.
Write through `self.settings` and `self.settings.save(&self.options_path())`.

**A new platform hook.** A `fn` field on `BootConfig` with a doc comment and a `no_*` default, the
field and its copy in `Boot`, then every host: Android (JNI, on the UI thread through `ime.rs`'s
command queue if it touches the window: `set_window_flags` deadlocks `poll_events`), iOS (objc2, main
thread), PC (`horizon_client::desktop`).

**Strings.** Every visible string is a `horizon.launcher.*` key in both `en_us.json` and `ru_ru.json`;
`lang::tr` fills `%s`. Russian is the owner's language: write it as natural Russian, not a calque.

## Checks

```sh
cargo check -p horizon-client
cargo clippy -p horizon-client -p horizon-client-lib --lib -- -D warnings
cargo test -p horizon-client-lib --lib -- launcher_screen profiles launcher_mods mod_registry
cargo test -p horizon-client --lib boot
cargo check -p horizon-game --bin magnetite      # the PC host
```

Then look at it for real: on a phone (the `magnetite-builds` skill) or by running `magnetite` on a PC.
Test on a device in batches: run the whole flow, collect every problem, fix them together.
