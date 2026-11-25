 Точка входа iOS на MAIN: крейт `crates/magnetite-ios` + `packaging/ios`

**MAIN** = `D:\Раст майн\horizon-main\horizon-main`, **IOS** = `D:\Раст майн\horizon-ios\Gfhdgcfg-ccr-bb4385db-s67j6f`. Ничего не изменял — только чтение.

Замечание по репозиторию: `docs/unification/ios-entry.md` существует, но **пуст** (0 строк), а `docs/unification/android-entry.md:1-251` — единственный образец. `crates/magnetite-ios` в MAIN **отсутствует**, `crates/magnetite-android` уже есть (3 файла, `crates/magnetite-android/src/lib.rs:1-110`). `packaging/ios` в MAIN нет (`packaging/{android,macos,linux,server,windows,icons}`, `packaging/executable_link.rs`). CI: `.github/workflows/ci.yml:63-74` уже проверяет `cargo check -p horizon-game --target aarch64-apple-ios --no-default-features --features mobile`; APK-образец — `.github/workflows/android-apk.yml:1-45`.

---

## Ответы на особые вопросы (кратко; подробности в (б)/(в)/(д))

**1. Главный поток и стек гостя.** IOS **не решал** эту задачу: WASM-модов там нет, `horizon-guest-stack` в форке не используется, гость = статически вшитый нативный Rust (`.github/workflows/ios.yml:120-135`, `docs/ios.md` «What does not work: Native Rust mods…»). В MAIN главный поток — это поток event loop: winit на iOS требует `EventLoop::new()` на main thread (`third_party/winit/src/platform_impl/ios/event_loop.rs:161-163`) и `run_app` вызывает `UIApplicationMain` (`:282-306`), поэтому «увести движок на свой поток» нельзя без переписывания winit. Гость с главного потока вызывается: кадровые хуки (`crates/horizon-client/src/game/interface/extras.rs:72` → `game/native_hooks/render_frame_pre.rs:23-30`) и интерфейсные хуки, зарегистрированные в `crates/horizon-client/src/entry.rs:install_client_hooks` — их листенеры идут через `mod_api::dispatch_*` на этом же потоке; проверка стека стоит в `crates/horizon-mod-wasm/src/runtime.rs:938` и `:1128`, требование — `crates/horizon-guest-stack/src/guest_stack_constants.rs:3-5` (128+8 МиБ).
Рекомендация: **не опираться на `-Wl,-stack_size`** (ld64 принимает флаг и пишет `LC_MAIN.stacksize`, но для UIKit-приложения главный поток создаётся SpringBoard'ом до `main`, надёжного подтверждения роста на iOS нет; в MAIN флаг и так выдаётся только для macOS/windows — `packaging/executable_link.rs:4-16`). Рабочий путь: **вызовы гостя только с рабочих потоков** `horizon_guest_stack::builder()` — в MAIN туда уже уходит почти весь guest-трафик (`crates/horizon-client/src/game/world/integrated.rs:55-62,588` — серверный поток, `crates/horizon-client/src/game/interface/startup_menu.rs:418` — загрузчик, `crates/horizon-client/src/game/runtime/resource_load.rs:93,779,909`, `crates/horizon-client/src/game/world/world_loading.rs:108,398,458,533,596`, `crates/horizon-client/src/vanillagen.rs:805`, `crates/horizon-client/src/lan/io_pool.rs:337`). Остаток — кадровые/UI-хуки: на iOS они получат отказ `GuestStackUnavailable` (`runtime.rs:938`) и мод просто не будет вызван — для этапа 0 (без модов) это штатно, для этапа модов это отдельная работа (маршалинг хуков кадра на guest-поток или политика «этот хук на iOS не обслуживается»). Этап 0 **не должен** вызывать `prepare_main`/`exit_if_compile_child` — как Android (`crates/magnetite-android/src/lib.rs:101-107`), иначе `crates/horizon-guest-stack/src/lib.rs:40-45` → `inspect_current` apple-ветка (`:239-252`) вернёт отказ при 1 МиБ и процесс умрёт на `exit(1)`.

> **Решено позже.** Стек главного потока (1 МиБ) переполняла не только гость, но и сам движок: передача от экрана загрузки к игре (`loading_screen.rs` `finish` → `build_renderer`, `start_game`/`Game::new`) роняла приложение без строки в логе. `crates/magnetite-ios/src/main.rs` переводит главный поток на `horizon_guest_stack::OwnStack` (стек гостевого размера, `psm::on_stack`) до `boot::run`, так что весь цикл UIKit и игра идут на нём; внутри `check_current()` пускает гостей, и кадровые/UI-хуки на iOS больше не получают отказ.

**2. Как собрать .app на CI.** IOS собирает **чистый Rust-бинарник**, не staticlib+Xcode: `cargo build --release --target aarch64-apple-ios -p horizon-client --bin horizon-client` (`crates/horizon-client/src/main.rs:18-21`), затем `packaging/ios/package.sh` кладёт его в `Payload/Horizon.app` с `Info.plist`, `PkgInfo`, иконками и делает ad-hoc `codesign --force --sign - --timestamp=none`, затем `zip` (`packaging/ios/package.sh:1-45`). Swift в форке — **только SwiftUI-лаунчер** (не точка входа): `packaging/ios/swift/build.sh:1-40` компилирует `libHorizonUI.a`, а `crates/horizon-threads/build.rs:1-50` линкует её в Rust-бинарник через `HORIZON_IOS_SWIFT_LIB` — это нужно экранам Ferrite, на этапе 0 не нужно. Итог: **проще и надёжнее — Rust-бинарник + `Info.plist`**; Xcode-проект не нужен ни для сборки, ни для подписи (SideStore/AltStore переподпишут `.ipa`, TrollStore — поставит без подписи). Проверено работающим `ios.yml` (`:118-135`, `:136-146`).

**3. Чего не хватает в MAIN.** Один настоящий блокер архитектуры: **игра не умеет стартовать внутри уже работающего event loop** — на iOS это обязательно (один `EventLoop` на процесс, `event_loop.rs:167-172`; второй `run_app` → паника «cannot be run after a call to UIApplicationMain», `:280-286`). В MAIN `run_loading` сам открывает цикл (`crates/horizon-client/src/game/runtime/resource_loader.rs:298`), split — тоже (`crates/horizon-client/src/game/runtime/loading_screen.rs:36`), «нет ресурсов» — тоже (`crates/horizon-client/src/game/runtime/no_resources.rs:45`). В форке это решено передачей `App` (hand-over): `IOS/crates/horizon-client/src/game/ios_boot.rs:16-18,36-44` (`HANDED`) и `:312-364` (`run_after`), старт игры внутри `about_to_wait` (`:1136-1146`). Нужен тонкий хук в MAIN (п.(б) 1). Остальные хуки уже сделаны: `entry::prepare_with_args` (`crates/horizon-client/src/entry.rs:41-46`), `window::{set_event_loop_factory,new_event_loop}` (`crates/horizon-client-input/src/window.rs:718-752`), `logging::set_platform_sink` (`crates/horizon-client-render/src/logging.rs:459-472`), `Renderer::suspend_surface` + `Game::suspended/resumed` (`crates/horizon-client/src/game/runtime/window_handler.rs:376-403`).

---

## (а) Карта запуска в IOS

**Точка входа и «boot-loop»**
- `crates/horizon-client/src/main.rs:18-21` — `fn main()` → `horizon_client::entry::main()`.
- `crates/horizon-client/src/entry.rs:5-7` — `main()` → `main_with(Default::default())`.
- `crates/horizon-client/src/entry.rs:18-57` — `ios_folders()`: контейнер из `HOME`, `Documents` как **игровая папка** (`set_current_dir`), `home` = `Documents/home` (логи), `resources.zip` из бандла → распаковка в `Documents/resources` с маркером `.unpacked-from`; приложение стартует в `/`, бандл read-only (комментарий `:9-17`).
- `crates/horizon-client/src/entry.rs:99-140` — `ios_unpack_resources`: пофайловая распаковка в `resources.unpacking` + переименование, прогресс через `game::ios_boot::UNPACKED`.
- `crates/horizon-client/src/entry.rs:308-313` — iOS-ветка: `game::ios_boot::run_after(unpacking, setup)`; иначе `game::run_with()`.
- `crates/horizon-client/src/game/ios_boot.rs:1-6` — **почему цикл стартует сразу**: «The system ends an app that takes too long to start, and the first start unpacks the resources… for longer than that»; поэтому экран свой, распаковка/вход на потоках, игра стартует **внутри** цикла.
- `crates/horizon-client/src/game/ios_boot.rs:312-364` — `run_after()`: `EventLoop::new()?.run_app(&mut boot)`; распаковка — `std::thread::Builder` (`:330-338`), вход — `spawn` (`:221-226`).
- `crates/horizon-client/src/game/ios_boot.rs:36-44` — `hand_over(app) -> Option<App>`: цикл уже есть → `None`, иначе вернуть app.
- `crates/horizon-client/src/game/ios_boot.rs` `Boot::start` (≈`:1100-1150`) — `run_with()` → `HANDED.take()` → `app.resumed(event_loop)` + `ControlFlow::Poll`.
- `crates/horizon-client/src/ios_screen.rs:1-5` — экран загрузки/входа (свой рендерер, свои шрифты), плюс `LauncherAction` (`:44-88`) для SwiftUI-лаунчера.

**Где создаётся `EventLoop`, где пути, логи**
- `EventLoop::new()`: `game/ios_boot.rs:362`, `game.rs:21088,21659`, `game/no_resources.rs:35` — все на главном потоке, один цикл.
- Пути: `Documents` = игра, `Library/Application Support/horizon/account.json` = секреты (не видны в Files) — `IOS/docs/ios.md` «Where things are», «Signing in»; бандл `Info.plist` через `NSBundle` — `crates/horizon-threads/src/ios_bundle.rs:1-40`.
- Логи: файловый лог MAIN-типа `home/logs/latest.log`, crash-reports в `Documents/home/crash-reports`; **`os_log`/`NSLog` в форке не используются вовсе** (grep = 0), диагностика через файл + `ios-resources-error.txt`.
- Первый запуск без ресурсов: скачивание с Mojang на экране загрузки — `crates/horizon-client/src/game/ios_setup.rs`; не переносим.

**Платформенные модули (в `horizon-threads`)**
`crates/horizon-threads/src/lib.rs:7-22` — `ios_bundle`, `ios_display`, `ios_gamecontroller`, `ios_keyboard`, `ios_launcher`, `ios_picker`, `ios_pasteboard`, `ios_url`; память — `crates/horizon-threads/src/memory_limit.rs:114,284,318-342` (iOS-ветки). GameController — `ios_gamecontroller.rs` (в `docs/ios.md`: «A controller paired with the phone is not read yet (gilrs has no iOS backend)»).

**Упаковка (IOS)**
- `packaging/ios/package.sh:1-45` — `Payload/Horizon.app`, бинарник 755, `Info.plist` (+ подмена версии `HORIZON_IOS_VERSION/BUILD`), `PkgInfo`, `ru.lproj`, LICENSE/THIRD_PARTY_NOTICES, иконки через `sips`, `plutil -lint`, ad-hoc `codesign --force --sign - --timestamp=none`, `zip -qry`.
- `packaging/ios/Info.plist:1-110` — `CFBundleExecutable=horizon-client`, `MinimumOSVersion=16.0`, `UIDeviceFamily=[1,2]`, landscape-only, `UILaunchScreen` пустой (иначе letterbox), `UIFileSharingEnabled`+`LSSupportsOpeningDocumentsInPlace` (Documents в Files), `CADisableMinimumFrameDurationOnPhone` (120 Гц), `UIRequiredDeviceCapabilities=[arm64,metal]`, `GCSupportsControllerUserInteraction`.
- Swift: `packaging/ios/swift/build.sh` + `swift/*.swift` — SwiftUI-лаунчер (Ferrite), линкуется статически; **не нужен** на этапе 0.
- `packaging/ios/touch-pack/*`, `FerriteTouchIcons*.zip` — тач-иконки, этап 2.

**CI (IOS)**
- `.github/workflows/ios.yml:1-60` — `macos-15`, Xcode 26 при наличии, `rustup target add aarch64-apple-ios`, Opus, horizon-mixins, нативные моды.
- `.github/workflows/ios.yml:118-146` — `cargo build --release --target aarch64-apple-ios -p horizon-client --bin horizon-client --no-default-features --features audio`, `CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS=-C target-cpu=apple-a11y`, `package.sh` → `Horizon-unsigned.ipa`.
- `.github/workflows/ios.yml:200-260` — симулятор: `x86_64-apple-ios`, `simctl boot/install/launch`, скриншоты, `log show`.

**Не переносим сейчас:** `crates/ferrite-signer` (переподпись на устройстве, LiveContainer-подход), SwiftUI-лаунчер, `ios_setup` (скачивание), `ios_update`, `ios_instances`, `ios_account` (вход — этап 1), `game/touch.rs` (этап 2), `ios_screen`.

---

## (б) Изменения в MAIN (минимум; без `#[cfg(target_os)]` в логике ядра)

1. **Hand-over приложения вместо второго event loop — единственный настоящий блокер.**
   Файл: `crates/horizon-client/src/game/runtime/loading_screen.rs:433` — `App` (`Loading`/`Ready`) уже реализует `ApplicationHandler`; сделать его `pub` (или `pub struct RunningApp`, реэкспорт `horizon_client::game::App`) и добавить в `crates/horizon-client/src/game/runtime/mod.rs`:
   ```rust
   /// Where the app goes when the caller owns the loop (a phone starts one cycle for the process).
   pub fn set_app_sink(sink: fn(App));
   pub fn take_app_sink() -> Option<fn(App)>;   // берётся один раз
   ```
   Замены (три места, симметрично образцу хука `new_event_loop`):
   - `game/runtime/resource_loader.rs:298` → `match take_app_sink() { Some(sink) => { sink(app); Ok(()) } None => new_event_loop()?.run_app(&mut app) }`
   - `game/runtime/loading_screen.rs:36` — то же для `App::Ready(split)`.
   - `game/runtime/no_resources.rs:45` — оставить (нет ресурсов = игры нет; на iOS до этого не доходим, см. (в)).
   На ПК sink не ставится → поведение байт-в-байт прежнее; тест: sink получает `App` и `run_app` не вызывается.

2. **Гостевой стек: не убивать процесс на iOS.** `crates/horizon-guest-stack/src/lib.rs:40-45` (`prepare_main` → `exit(1)`), `:47-51` (`prepare` для `not(linux)`), `:122-129` (`check_current`), `:239-252` (apple-ветка `pthread_get_stacksize_np`). Достаточно **не вызывать** `prepare_main`/`exit_if_compile_child` из `magnetite-ios` (как `crates/magnetite-android/src/lib.rs:101-107`; `crates/horizon-game/src/lib.rs:28-34` и `crates/horizon-mod-host/src/lib.rs:17-22` остаются для ПК). Если хочется явности — сделать `prepare()` публичной и вернуть `Result` вместо `exit`; это платформенный крейт, cfg там допустим. `check_current()` не менять: отказ на главном потоке iOS — желаемое поведение.

3. **Ресурсы: `engine-data.zip` → `Documents/resources` до старта игры.** В MAIN распаковщика нет (это делает `horizon-prepare` на ПК); Android копирует asset и отдаёт установщику (`docs/unification/android-entry.md`, п.(в)). В `magnetite-ios` — свой `resources.rs` (zip-slip guard, атомарный `.partial`+rename), по образцу `IOS/crates/horizon-client/src/entry.rs:99-140`. В MAIN менять нечего.

4. **Логи.** `logging::set_platform_sink` уже есть (`crates/horizon-client-render/src/logging.rs:459-472`, вызов из `Logger::log`). На iOS sink пишет в `stderr` (Xcode/devicectl) — файловый лог `Documents/home/logs/latest.log` остаётся главным источником; `os_log` можно добавить позже без изменений MAIN.

5. **Жизненный цикл.** Уже готово: `Game::suspended` (`window_handler.rs:376-384`, `renderer.suspend_surface()`), `Game::resumed` с веткой «окно есть, surface suspended → `recreate_surface()` + `resize`» (`:385-403`), `Renderer::suspend_surface` (`crates/horizon-client-render/src/gfx.rs`; в android-entry — `:4030-4039`). winit iOS шлёт `Resumed`/`Suspended` из нотификаций (`third_party/winit/src/platform_impl/ios/event_loop.rs:190-220`). Ничего не добавлять.

6. **Графика/потоки.** `backend_order` уже знает iOS: Metal-only (`crates/horizon-client-render/src/gfx/device.rs:88-96`). Apple-ветки потоков и памяти на месте: `crates/horizon-threads/src/lib.rs:194,281,356,450,514`, `crates/horizon-threads/src/process_memory.rs:67-88` (iOS — `task_info`), `crates/horizon-threads/src/stack_dump.rs:16,44`. `raise_open_file_limit` (`entry.rs`, конец) под `#[cfg(unix)]` — на iOS `setrlimit` может отказать, это только warn.

7. **Чего НЕ нужно для этапа 0:** реэкспорты `android_runtime`/`keypad`/`touch` (`crates/horizon-client/src/client_lib.rs:25-45`), выбор Vulkan→GLES, `getCacheDir`-аналог, `configure_graphics/gpu_batching`, правки `packaging/executable_link.rs` (флаг `-stack_size` не добавляем — см. ответ 1).

---

## (в) Крейт `crates/magnetite-ios`

Обычный **bin**-крейт (не cdylib): iOS запускает Mach-O executable из `.app`; `main` вызывает `EventLoop::new().run_app(...)` → `UIApplicationMain` (winit). Без `include!`-трюка, без Swift.

| Файл | ~строк | Содержимое |
|---|---|---|
| `Cargo.toml` | 30 | ниже |
| `src/main.rs` | 60-90 | `fn main()`: `HOME`→`Documents`, `set_platform_sink`, выбор boot-режима, `prepare_with_args` + `main_with` |
| `src/paths.rs` | 40 | контейнер (`HOME`), `Documents`, `game/`, `resources/`, `Library/Application Support/horizon` |
| `src/resources.rs` | 100-140 | `engine-data.zip` (из бандла) → `Documents/resources`: `.partial`+rename, проверка `horizon-resources.json`, zip-slip |
| `src/boot.rs` | 200-260 | `ApplicationHandler`: цикл сразу, распаковка на потоке, затем `main_with`, `take_app_sink` → держит `App`, пересылает события |

`Cargo.toml` (образец — `crates/magnetite-android/Cargo.toml:1-27`, workspace MAIN edition 2024):
```toml
[package]
name = "magnetite-ios"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true
publish = false

[[bin]]
name = "magnetite"          # CFBundleExecutable в Info.plist
path = "src/main.rs"

[target.'cfg(target_os = "ios")'.dependencies]
horizon-client  = { path = "../horizon-client",  default-features = false, features = ["mobile"] }
horizon-mod-host = { path = "../horizon-mod-host", default-features = false, features = ["mobile"] }
winit = { version = "0.30.13" }        # патч third_party/winit (ios-платформа)
log = "0.4"
zip = { version = "9.1.3", default-features = false, features = ["deflate"] }

[lints]
workspace = true
```
+ `"crates/magnetite-ios"` в `members` корневого `Cargo.toml:4-8` (рядом с `magnetite-android`).

Скелет `src/main.rs` (ключевые решения):
```rust
//! Magnetite iOS: the app starts the shared core itself, on the one event loop iOS gives a
//! process; there is no launcher, no argv and no writable app folder.
#![cfg(target_os = "ios")]

fn main() {
    // The core's file log and crash reports; every kept line also to stderr (Xcode/devicectl).
    horizon_client::logging::set_platform_sink(|lvl, target, line| {
        eprintln!("[{lvl}][{target}] {line}");
    });
    let Some(container) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        eprintln!("iOS gave the app no container");
        return;
    };
    let paths = paths::Paths::new(&container);
    // Stage 0: the app's own event loop first (boot.rs), the game starts inside it.
    boot::run(paths);
}
```
`src/boot.rs` — порядок, повторяющий `IOS/crates/horizon-client/src/game/ios_boot.rs`, но без экранов:
```rust
impl ApplicationHandler for Boot {
    fn resumed(&mut self, el: &ActiveEventLoop) { /* окно (белое) или сразу unpack-поток */ }
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.unpacking.is_none() && !self.started {
            self.started = true;
            // внутри уже работающего цикла: hand-over (хук б.1)
            let args = vec!["--game-dir".into(), self.paths.game_dir.display().to_string(),
                            "--resources".into(), self.paths.resources.display().to_string(),
                            "--home".into(), self.paths.home.display().to_string(),
                            "--name".into(), "Player".into()];
            let startup = horizon_client::entry::prepare_with_args(args);
            horizon_mod_host::main_with(Vec::new(), &[], &[], startup); // вернётся после sink(App)
        }
        if let Some(app) = &mut self.app {
            app.about_to_wait(el);            // App: ApplicationHandler из loading_screen.rs:433
        }
    }
    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, ev: WindowEvent) { /* → app */ }
}
```
Ключевые правила, которые нельзя нарушить:
- `EventLoop::new()` — ровно один, на главном потоке, и **до** `UIApplicationMain` (winit: `event_loop.rs:161-163, 282-306`); `set_event_loop_factory` на iOS **не ставить**.
- `prepare_with_args` — до/внутри цикла, но **не** `horizon_game::main()` (там `prepare_main` → `exit(1)` на 1 МиБ): `crates/horizon-game/src/lib.rs:28-34`.
- `with_icon`/`set_icon_png` не нужны (иконка в `.app`), `--name Player` — до этапа входа.
- «Нет ресурсов» на этапе 0 — штатный исход: `run_with` покажет окно `no_resources` (`crates/horizon-client/src/game.rs:11159`, `game/runtime/no_resources.rs:1-45`); это и есть smoke-тест, как в android-entry.

---

## (г) Упаковка `.ipa` и CI (GitHub Actions, `macos-latest`)

**`packaging/ios/` (порт `IOS/packaging/ios`, минус Ferrite-специфика):**

| Файл | Что делать |
|---|---|
| `Info.plist` | взять `IOS/packaging/ios/Info.plist:1-110`, заменить: `CFBundleExecutable=magnetite`, `CFBundleName/DisplayName=Magnetite`, `CFBundleIdentifier=dev.magnetite.ios`, `CFBundleShortVersionString/Version` из CI; оставить `MinimumOSVersion=16.0`, `UILaunchScreen` (пустой), `UIDeviceFamily`, landscape-only, `UIFileSharingEnabled`+`LSSupportsOpeningDocumentsInPlace`, `CADisableMinimumFrameDurationOnPhone`, `UIRequiredDeviceCapabilities=[arm64,metal]`, `ITSAppUsesNonExemptEncryption=false`, `NSLocalNetworkUsageDescription`; убрать `LSApplicationQueriesSchemes` (sidestore/altstore) и микрофон — до этапов обновлений/голоса |
| `package.sh` | `IOS/packaging/ios/package.sh:1-45` как есть: `Payload/Magnetite.app`, `chmod 755`, plist, `PkgInfo`, иконки `sips` из `packaging/icons/hicolor/512x512/apps/horizon.png` (`:29-33`), LICENSE/THIRD_PARTY_NOTICES, `plutil -lint`, `codesign --force --sign - --timestamp=none`, `zip -qry` → `Magnetite-unsigned.ipa` |
| `README.md` | 10 строк: как собрать, что ресурсов внутри нет (правило `AGENTS.md:11` — файлы Mojang не коммитить) |

Названия бандла/исполняемого файла — `Magnetite.app` / `magnetite`; bundle id оставить `dev.magnetite.ios`, чтобы `magnetite.e4mc`-канал (`docs/unification/plan.md`, «Потом») стал отдельным id позже.

**CI `.github/workflows/ios-ipa.yml`** (образцы: `.github/workflows/ios.yml:1-146` + `.github/workflows/android-apk.yml:1-45`):
1. `runs-on: macos-latest`, `timeout-minutes: 180`, `workflow_dispatch` + `push: tags: ['v*','e4mc-v*']` (как APK).
2. `actions/checkout@v4`; Xcode: `xcode-select` на `/Applications/Xcode_*.app` при наличии + `xcodebuild -version` (`ios.yml:44-52`).
3. `rustup toolchain install stable --profile minimal --target aarch64-apple-ios` + `rustup default stable`.
4. `Swatinem/rust-cache@v2` (`key: ios-ipa`).
5. `env: IPHONEOS_DEPLOYMENT_TARGET: "16.0"`; сборка: `cargo build --locked -p magnetite-ios --target aarch64-apple-ios --profile fast` (на старте `fast`; `release`+ThinLTO — после зелёного смоука; `CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS=-C target-cpu=apple-a11` как `ios.yml:126`).
6. `HORIZON_IOS_VERSION=$(date -u +%Y.%-m.%-d) HORIZON_IOS_BUILD=${{ github.run_number }} bash packaging/ios/package.sh target/aarch64-apple-ios/fast/magnetite dist/Magnetite-unsigned.ipa`.
7. `actions/upload-artifact@v4` (`if-no-files-found: error`).
8. Плюс в `.github/workflows/ci.yml:63-74` добавить `cargo check --locked -p magnetite-ios --target aarch64-apple-ios` — крейт собирается всегда, ловит регрессии хуков.

**Ресурсы:** `engine-data.zip` генерировать из MAIN (не брать zip форков — таблицы расходятся); на CI первый шаг можно пропустить — без ресурсов игра покажет `no_resources`-окно, это ожидаемый smoke-тест этапа 0. В git — ничего Mojang.

**Подпись:** ad-hoc в `package.sh` (`codesign --sign -`), для установки — SideStore/AltStore (переподпись своим Apple ID, 7 дней) или TrollStore (без подписи). `ferrite-signer` — только упомянуть как этап 5 (`docs/unification/plan.md`, п.5).

---

## (д) Риски и порядок малых коммитов

**Риски**
1. **Watchdog на первом старте** — главный. IOS начал цикл сразу именно из-за этого (`game/ios_boot.rs:1-6`): распаковка ресурсов дольше лимита системы. Поэтому хук б.1 обязателен, а не «потом»; вариант «распаковать синхронно в `main`» допустим только для маленького `engine-data.zip` и только как временный.
2. **Стек гостя (ответ 1).** Этап 0 — без `prepare_main`, кадровые/UI-хуки на iOS получат `GuestStackUnavailable` (`mod-wasm/runtime.rs:938,1128`). Пока моды не включены — не проявляется; для этапа модов нужен маршалинг хуков кадра на guest-поток или явная политика отказа.
3. **WASM на iOS:** Cranelift JIT запрещён → Pulley (`docs/unification/mobile-trim-plan.md`, поправка в шапке; `crates/horizon-mod-wasm/src/runtime.rs:173-192`). Этап 0 модов не грузит (`main_with(Vec::new(), &[], &[])` — как Android).
4. **mimalloc на iOS:** grep по IOS-ветке `mimalloc` в этой сессии не завершён; рычаг — `horizon-game`/`horizon-client` фича `system-allocator` (`crates/horizon-client/Cargo.toml:163-165`, `crates/horizon-client/src/lib.rs:16-19`). Если C-сборка падает — собрать с `system-allocator` и вернуться позже.
5. **Metal/wgpu:** `third_party/wgpu-hal` уже с apple-ветками; шейдер-паки GLSL→MSL на iOS не проверены (в плане — отдельный пункт).
6. **Играть нельзя:** тач не перенесён (`game/touch.rs` в IOS), геймпада нет (`docs/ios.md`), IME/буфер обмена отсутствуют. Этап 0 = «запускается и рисует титульный экран».
7. **Пути:** процесс стартует в `/`, бандл read-only (`IOS/crates/horizon-client/src/entry.rs:9-17`) — `--game-dir`/`--home` обязательны, иначе `Options::read` (`crates/horizon-client/src/game/runtime/launch_options.rs:142-147`) сделает `set_current_dir("/")` и всё сломается.
8. **Симулятор:** wgpu/Metal в симуляторе не умеет indirect draw (форк это обходит: `ios_boot.rs` `target_abi = "sim"` → только лаунчер). Для нас симулятор годится лишь как проверка старта/путей; экран будет пустым.
9. **`current_exe`** внутри `.app` корректен (в отличие от Android); `Runtime::load` при `built=&[]` ничего не читает (`docs/unification/android-entry.md`, п.2).
10. **7-дневная подпись** и переустановки — организационный риск, не технический.

**Порядок малых коммитов (каждый проверяется на ПК):**
1. MAIN: `game::{set_app_sink, take_app_sink}` + `App` публичный + замены в `resource_loader.rs:298` и `loading_screen.rs:36` + тест «sink получает App, loop не открывается».
2. MAIN: сделать `horizon_guest_stack::prepare()` возвращающей `Result` (без `exit`) — или оставить как есть и не вызывать; тест на отказ главного потока уже есть (`crates/horizon-guest-stack/src/lib.rs:464-480`).
3. `crates/magnetite-ios` (main/paths/resources/boot) + workspace member; `cargo check -p magnetite-ios --target aarch64-apple-ios` (плюс строка в `ci.yml`).
4. `packaging/ios` (`Info.plist`, `package.sh`, README); локально на Mac `.ipa` собирается, ставится через SideStore.
5. CI `ios-ipa.yml` → артефакт `Magnetite-unsigned.ipa`.
6. Позже: экран загрузки/входа (порт `ios_screen`+`ios_account`), тач, GameController, `os_log`, `engine-data.zip` в CI, JIT/Pulley для модов, `ferrite-signer`.

**Верификация end-to-end:** `cargo check` трёх целей — ПК-default, `--target aarch64-linux-android`, `--target aarch64-apple-ios` (`--no-default-features --features mobile`) — и `-p magnetite-ios`; `cargo test -p horizon-client` (entry/logging) и новый тест sink; ПК-смоук `cargo run --profile fast -p horizon-game -- --offscreen --frames 60 --resources .local/horizon-client/26.3` (поведение без sink не изменилось); на устройстве: установка `.ipa` → `Documents/home/logs/latest.log` содержит `Horizon <версия>` → титульный экран (Metal) → свернуть/развернуть (surface recreate, `window_handler.rs:385-403`) → выход и повторный запуск; `python3 tools/pr_ready.py` по правилам MAIN.
