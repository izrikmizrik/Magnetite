# Перенос точки входа Android на MAIN: крейт `magnetite-android`

Обозначения: **AND** = `D:\Раст майн\Horizon-Android-android`, **MAIN** = `D:\Раст майн\horizon-main\horizon-main`. Все пути ниже от соответствующего корня. Ничего не изменял — только чтение.

---

## (а) Карта запуска в AND

### Сборка cdylib
- `crates/horizon-android/Cargo.toml:8-10` — `[lib] path = "../horizon-client/src/android_lib.rs"`, `crate-type = ["cdylib"]`. Трюк форка: cdylib **перекомпилирует** исходники `horizon-client` через `include!`, поэтому android-зависимости (`android_logger = "0.15"` :17, `jni = "0.21"` :21, `winit 0.30.13` с `android-native-activity` :37) живут в этом крейте, а не в `horizon-client/Cargo.toml` (там их нет).
- `crates/horizon-client/src/android_lib.rs:5` — `include!("android_main.rs")`; :12-19 — экспортируемый `horizon_on_create` (имя из манифеста `android.app.func_name`).
- `crates/horizon-client/src/android.rs:28-49` — hot-swap нативных модов: `dlopen` другой `libhorizon_android.so` из `files/native/` и передача ему `ANativeActivity_onCreate`. Для MAIN не нужно (этап модов позже).
- `crates/horizon-android/build.rs:24-26` — `-Wl,--build-id=sha1` для tombstone-символизации.

### `android_main(app: AndroidApp)` — `crates/horizon-client/src/android_main.rs:201-283`
Порядок действий до игры:
1. :202 — `mod_api::install_runtime_hooks`; :203-209 — `app.enable_motion_axis` (Vscroll/Hscroll/RelativeX/RelativeY).
2. :210 — `init_logging()` (:157-191): `android_logger` с тегом `Horizon`, обёртка `LocalLogger` дублирует строки в `diagnostics::log_line`; уровень Info, Trace для GLES по env.
3. :211-214 — `home = app.internal_data_path()`; `backend::initialize(home)` (свой OnceLock-«лаунчерный» backend, :71-86).
4. :215-223 — diagnostics, `secure_store::initialize(JavaVM::from_raw(app.vm_as_ptr()))`.
5. :225-233 — `ProcessExitGuard`: на Drop — `diagnostics::finish()`, `platform::finish()`, очистка APP и **`std::process::exit(0)`**. Причина (комментарий :280-282): winit допускает один EventLoop на процесс, а NativeActivity кэширует процесс после Quit — без exit следующий запуск падает «The EventLoop can't be recreated».
6. :235 — `app.asset_manager().open(c"engine-data.zip")`; :236 — `APP` (thread_local `RefCell<Option<AndroidApp>>`, :140-142) = Some(app).
7. :237-241 — `platform::cache_dir()` (JNI `getCacheDir`, `android/platform.rs:281-293`) → `android_runtime::configure_cache_dir` + `official_install::configure_download_cache_dir`.
8. :243-248 — clipboard-хуки через JNI (`platform::clipboard_get/set`, :223-267, :404).
9. :249-254 — `create_dir_all(home)` + **`std::env::set_current_dir(home)`** — вся игра дальше работает от cwd.
10. :258-260 — `crash::install(home, "client")`, `exit_reports::install`.
11. :262-274 — копирование ассета `engine-data.zip` в `home/engine-data.zip` (атомарно через `.partial`+rename). **Распаковка здесь не делается** — её делает установщик ресурсов: `official_install.rs:479` `resource_install::extract(home/engine-data.zip → stage)` + :481 `android_resources::rebuild`, результат активируется в `home/.local/horizon-client/<VANILLA_VERSION>` (:497-515).
12. :276-279 — `Launcher::new(home)` и `android_event_loop()?.run_app(&mut launcher)`.

### Передача AndroidApp в winit
- `android_main.rs:144-155` — `android_event_loop()`: `EventLoop::builder().with_android_app(APP.clone()).build()` (`EventLoopBuilderExtAndroid`).
- `game.rs:21222-21227` — `new_event_loop()`: на Android вызывает `crate::android_event_loop()`, иначе `EventLoop::new()`.
- JNI-доступ к activity из любого потока: `android/platform.rs:201-221` `with_context` — `JavaVM::from_raw(app.vm_as_ptr())`, `attach_current_thread`, `with_local_frame`, `JObject::from_raw(app.activity_as_ptr())`.

### Запуск игры (без лаунчерного argv)
- `android/game_launch.rs:5-41` — `LaunchRequest{resources, game_home, session_file, offline_name, server, control_socket, extra_arguments, open_gl, gpu_batching}`; `arguments()` собирает **`--resources <abs> --game-dir <abs> [--session-file|--name] [--server] [--control-socket]`**.
- `android/game_launch.rs:47-66` — `start()`: проверка абсолютности путей, `set_current_dir(game_home)`, `android_runtime::configure_graphics(open_gl)` / `set_prefer_vulkan(true)` / `configure_gpu(batching)`, `android_reset_bundled_mods()`, затем `game::android_game(arguments)`.
- `game.rs:21163-21186` — `android_game`: `auth::clear_current`, фабрика тач-раскладок, **`Options::read_arguments(arguments)`** (явный `Vec<String>`, никакого `std::env::args`), `Settings::load(cwd/options.txt)`, `horizon_threads::put_helpers_behind`, `Behaviours::set_cache_dir(cwd/.horizon-cache)`, `loading_app(...)` → `Box<dyn ApplicationHandler>`.
- Лаунчер передаёт управление: `android/launcher.rs:625-651` — собирает `LaunchRequest` (resources = `home/.local/horizon-client/<ver>`), `game_launch::start`, **обнуляет свои window/renderer** (одно native-окно), `game.resumed(events)`. Возврат в лаунчер — флаг `android_runtime::return_to_launcher/take_launcher_return` (`horizon-client-render/src/android_runtime.rs:193-197`, `launcher.rs:266-300`).

### Жизненный цикл
- `launcher.rs:700-714` — `resumed`/`suspended` пробрасываются в игру.
- `game.rs:16207-16221` — `Game::suspended` (cfg android): отпустить тач-клавиши, `capture(false)`, очистить keys/pad, если Playing → Paused, **`renderer.suspend_surface()`**.
- `game.rs:16223-16243` — `Game::resumed` (android-ветка): `display::prepare`, **`renderer.recreate_surface()`**, `resize(inner_size)`, сброс `previous`.
- `horizon-client-render/src/gfx.rs:4030-4039` — `suspend_surface()`: `suspended=true`, очистить in-flight, `device.poll`, `surface=None`, выбросить spare. Комментарий: Android уничтожает native-window при фоне; IME тоже пересоздаёт поверхность.

### `android_runtime` (найден)
`crates/horizon-client-render/src/android_runtime.rs`, реэкспорт `horizon_client::android_runtime` через `crates/horizon-client/src/lib.rs:176`. Чистое состояние без OS: input/gamepad/keypad, `configure_cache_dir` :9, `configure_graphics/set_prefer_vulkan/graphics_api` :158-190, `configure_gpu/gpu_batching/compatibility` :181-190, worker/cache-бюджеты :100-104, :246-256, `graphics_startup` (recovery после вылета Vulkan) :3.

### Упаковка AND
- `packaging/android/AndroidManifest.xml` — кастомная `HorizonActivity` (наследник NativeActivity) ради IME/SAF/hot-swap; `android.app.lib_name=horizon_android`, `func_name=horizon_on_create`; `sensorLandscape`, `configChanges=orientation|screenSize|keyboardHidden|keyboard|density|screenLayout`, `extractNativeLibs=true`; alias `com.google.androidgamesdk.GameActivity` с LAUNCHER.
- `packaging/android/build.gradle` — AGP 8.11.1, compileSdk/targetSdk 35, minSdk 28 (property), `jniLibs` из `.local/android/gradle-jni`, assets из `.local/android/gradle-assets`, debug-keystore, `useLegacyPackaging=true`.
- `tools/android_build.py` — NDK-окружение :150-157 (`CC_<triple>`/`CXX`/`AR`/`CARGO_TARGET_<TRIPLE>_LINKER` = `aarch64-linux-android28-clang`), minSdk 28 :141, `cargo build -p horizon-android` :176, `llvm-strip --strip-unneeded` :186, symbols-архив :188-190, `engine_data()` :64-93 (zip таблиц из `.local/horizon-client/<ver>`), ассеты :196-210, keytool :212-214, Gradle `assembleDebug/Release` :222, `apksigner verify` :226.
- CI-образец: `packaging/android/dev-builds.yml` — ubuntu-24.04, temurin JDK 25, Gradle 8.14, `sdkmanager 'platforms;android-35' 'build-tools;35.0.0' 'ndk;28.2.13676358'`, `rustup target add`, кэш cargo, `android_build.py --public --profile release --ndk-version 28.2.13676358`, артефакт APK.

---

## (б) Изменения в MAIN: чего не хватает и минимальные хуки

Цепочка MAIN сегодня: `horizon-game/src/main.rs:9-13` → `horizon_game::main()` (`crates/horizon-game/src/lib.rs:28-34`) → `prepare_main_stack` + `exit_if_compile_child` + `entry::prepare()` + `horizon_mod_host::main_with(Vec::new(), &[], &[], startup)` → `entry::start` (`crates/horizon-client/src/entry.rs:96`) → `game::run_with` (`crates/horizon-client/src/game.rs:11095`) → `run_loading` (`game.rs:11247-11248` → `game/runtime/resource_loader.rs:256-298`) → **`EventLoop::new()`** (:298).

Android-код в MAIN уже есть: `winit android-native-activity` (`crates/horizon-client/Cargo.toml:122-123`), вендоренный winit (`third_party/winit/PATCHES.md` — input/safe-area/aspect), `android-activity 0.6.1` (`Cargo.lock:114`), `android_runtime`/`android_gamepad`/`keypad`/`touch` (`crates/horizon-client-render/src/lib.rs:4-5,70,166`), фича `mobile` (`horizon-game/Cargo.toml:33`, `horizon-client/Cargo.toml:139`), GLES-фолбэк (`gfx/device.rs:88-110`: Android Auto → `[VULKAN, GL]`), in-memory clipboard на android (`horizon-client-input/src/clipboard.rs:133-189`). `docs/unification/android-entry.md` — пустой плейсхолдер; ориентиры — `docs/unification/plan.md:6-16` (Vulkan первым, minSdk 28) и `mobile-trim-plan.md`.

**Блокеры и хуки (по файлам):**

1. **`crates/horizon-guest-stack/src/lib.rs` — главный блокер.** `prepare_main` (:40-45) → `prepare()` для `not(linux)` (:47-51) → `inspect_current()` на Android возвращает `Err("…unsupported on this platform")` (:218-219) → **`exit(1)` до любой игры**. Требование стека 128+8 MiB (`guest_stack_constants.rs:3-5`). Linux-ветка (:53-113) делает re-exec — на Android невозможен. Тот же `prepare_main` вызывается внутри `exit_if_compile_child` (`crates/horizon-mod-host/src/lib.rs:17-22`). additionally `check_current` (:122) отказывает guest-входу wasm-модов (`crates/horizon-mod-wasm/src/runtime.rs:938`) и `debug_assert` в `spawn` (:20).
   **Хук (cfg допустим — это платформенный крейт):** android-ветка `prepare()` = проверка без re-exec; `inspect_current()` для android — через `pthread_getattr_np` (bionic поддерживает), как не-main-ветка Linux (:186-208). Рабочие потоки, созданные `builder()` (136 MiB), проверку пройдут; поток `android_main` (стек по умолчанию ~2 MiB у android-activity — **проверить при реализации**) останется не-guest-capable → wasm-моды на нём拒аются, для этапа 0 (без модов) приемлемо. Альтернатива-минимум: вообще не вызывать `prepare_main`/`exit_if_compile_child` из `magnetite-android` (см. п.2) и собирать release/fast.

2. **`crates/horizon-client/src/entry.rs:27-49` — argv.** `prepare()` строит `Command::new(current_exe())` + `std::env::args_os().skip(1)` (:30-32). На Android argv — мусор от zygote/app_process → `LaunchArguments::parse` упадёт «missing value» → `exit(USAGE)`.
   **Хук (без cfg):**
   ```rust
   /// Startup from explicit arguments; the process argv is not read.
   pub fn prepare_with_args(args: Vec<String>) -> Startup
   ```
   Реализация: `Command::new("horizon").args(args)` → существующие `LaunchArguments::for_command` (`crates/horizon-mod-package/src/selection/context.rs:76-103`; `parse` :117-170 уже публичный и принимает cwd/home явно) → `Options::validate`. `prepare()` рефакторится в обёртку над ней. `current_exe()` внутри `from_launch` (`game/runtime/launch_options.rs:182-186`, sibling `resources/`) на Android вернёт `/system/bin/app_process64` — не упадёт, fallback `.local/...` перекрывается `--resources <abs>`. `main_with` тоже читает `current_exe` (`crates/horizon-mod-host/src/loading.rs:31-38`), но при `built=&[]` `Runtime::load` ничего не читает (`horizon-mod-package/src/runtime.rs:27-58`) — хук не нужен.

3. **EventLoop — `EventLoop::new()` без AndroidApp на Android паникует.** Места: `game/runtime/resource_loader.rs:298`, `game/runtime/no_resources.rs:45`, `game/runtime/loading_screen.rs:36` (split-путь; для mobile не активен, но заменить заодно).
   **Хук в оконном glue `crates/horizon-client-input/src/window.rs` (без cfg, type-erased):**
   ```rust
   pub type EventLoopFactory =
       Box<dyn FnOnce() -> Result<winit::event_loop::EventLoop<()>, winit::error::EventLoopError> + Send>;
   pub fn set_event_loop_factory(factory: EventLoopFactory); // OnceLock
   pub fn new_event_loop() -> Result<winit::event_loop::EventLoop<()>, winit::error::EventLoopError>;
   // factory.take()() иначе EventLoop::new()
   ```
   `magnetite-android` ставит замыкание с `EventLoop::builder().with_android_app(app.clone()).build()`. Три вызова `EventLoop::new()` заменяются на `horizon_client::window::new_event_loop()`. (Допустимая альтернатива — cfg-android OnceLock<AndroidApp> прямо в window.rs, как `new_event_loop` в AND `game.rs:21222`; фабрика чище и тестируется на ПК.)

4. **Жизненный цикл — в MAIN нет ни одного `fn suspended`** (grep по `horizon-client/src`: только `resumed`). При фоне Android уничтожает поверхность; `Game::resumed` (`game/runtime/window_handler.rs:385`) рано выходит при `window.is_some()` → рендер в мёртвую поверхность.
   **Хуки:**
   - `crates/horizon-client-render/src/gfx.rs`: добавить `pub fn suspend_surface(&mut self)` — порт AND `gfx.rs:4030-4039` без cfg (на ПК harmless). Поля/методы уже есть: `suspended` :1855, `recreate_surface` :4848, `resize` :4943-4946, пропуск кадра :7373, автореконнект при потере поверхности :7462-7475.
   - `game/runtime/window_handler.rs` (Game), `game/runtime/loading_screen.rs` (Loading :93 и App :434): добавить `fn suspended` → `renderer.suspend_surface()`; в `resumed` — **платформенно-нейтральная** ветка «окно есть, но `renderer.suspended` → `recreate_surface()` + `resize(inner_size)` + сброс таймера» вместо раннего return (cfg не нужен). Пауза игры при фоне — существующая обработка `Focused(false)`; отдельный cfg-android Pause как в AND `game.rs:16215-16217` — опционально позже.

5. **Логгер — `crates/horizon-client-render/src/logging.rs`.** `init` (:655-687) ставит свой файловый логгер через `log::set_logger` (один раз). Если `magnetite-android` поставит `android_logger` глобально первым — файловый лог молча потеряется.
   **Хук (без cfg):**
   ```rust
   /// Secondary sink for the platform (logcat); called for every kept line.
   pub fn set_platform_sink(sink: fn(log::Level, &str, &str)); // OnceLock
   ```
   Вызов в `Logger::log`/`send` (~:560-600). `magnetite-android` регистрирует sink на `android_logger::AndroidLogger`. Файловый лог (`logs/latest.log`), flood-лимиты и crash-recent остаются нетронутыми. `set_game_home` (:37-41) уже есть — вызывается из `entry::start` (`entry.rs:152`) значением `--home`.

6. **Реэкспорты (для следующих этапов, не блокер):** `crates/horizon-client/src/client_lib.rs:25-45` не реэкспортирует `android_runtime`, `android_gamepad`, `keypad`, `touch` (в AND — `lib.rs:176`). Добавить строкой в `pub use horizon_client_render::{...}`.

7. **Не нужно для этапа 0 (есть или не требуется):** выбор бэкенда Vulkan→GLES (`gfx/device.rs:88-110` уже Android-aware); clipboard (in-memory, `clipboard.rs:133-189`); кэш-папка — MAIN кладёт кэши в cwd (`--game-dir`), `getCacheDir`-хук можно отложить; `configure_graphics/gpu_batching/atlas_limit/worker_limit` из AND `android_runtime` — отсутствуют в MAIN, понадобятся на этапе тюнинга GPU (см. риски); `crash::install` уже вызывается в `entry.rs:163`.

---

## (в) Новый крейт `crates/magnetite-android`

Обычный cdylib поверх публичных API MAIN + хуки (без `include!`-трюка AND — он нужен только для доступа к приватным модулям, которых у нас нет).

| Файл | ~строк | Содержимое |
|---|---|---|
| `Cargo.toml` | 45 | ниже |
| `build.rs` | 8 | `-Wl,--build-id=sha1` при `CARGO_CFG_TARGET_OS=android` (порт AND `horizon-android/build.rs:24-26`) |
| `src/lib.rs` | 300-380 | `android_main`, пути, распаковка engine-data, аргументы, хуки, exit-guard |
| `src/resources.rs` | 80-120 | копирование ассета `engine-data.zip` → `home/engine-data.zip` (атомарно, как AND `android_main.rs:262-274`) + распаковка zip в `home/resources` (проверка `horizon-resources.json`, защита от zip-slip — по образцу AND `official_install.rs:448-479`) |

`Cargo.toml` (версии как в AND `crates/horizon-android/Cargo.toml`; workspace MAIN — edition 2024):
```toml
[package]
name = "magnetite-android"
version = "0.1.0"
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
horizon-game = { path = "../horizon-game", default-features = false, features = ["mobile"] }
horizon-client = { path = "../horizon-client", default-features = false, features = ["mobile"] }
horizon-mod-host = { path = "../horizon-mod-host", default-features = false, features = ["mobile"] }
horizon-guest-stack = { path = "../horizon-guest-stack" }
winit = { version = "0.30.13", features = ["android-native-activity"] } # патч third_party/winit
android_logger = "0.15"
jni = "0.21"
log = "0.4"
zip = { version = "9.1.3", default-features = false, features = ["deflate"] }

[lints]
workspace = true
```
+ добавить `"crates/magnetite-android"` в `members` корневого `Cargo.toml`. `android-activity 0.6.1` уже в `Cargo.lock:114` (тянется winit).

Скелет `src/lib.rs` (ключевые решения):
```rust
#![cfg(target_os = "android")]
use winit::platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp};

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // 1. Лог: sink в logcat (файловый лог MAIN останется).
    let logger = android_logger::AndroidLogger::new(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("Magnetite"));
    horizon_client::logging::set_platform_sink(move |lvl, target, msg| { /* logger.log(...) */ });

    // 2. Пути без argv.
    let home = app.internal_data_path().expect("data dir");
    let game_dir = home.join("game");
    let resources = home.join("resources");
    std::fs::create_dir_all(&game_dir).unwrap();
    resources::install(&app, &home, &resources); // engine-data.zip → resources/

    // 3. EventLoop-фабрика с AndroidApp (хук б.3).
    let app2 = app.clone();
    horizon_client::window::set_event_loop_factory(Box::new(move || {
        EventLoop::builder().with_android_app(app2).build()
    }));

    // 4. Запуск БЕЗ prepare_main/exit_if_compile_child (гостевой стек, хук б.1):
    //    compile-child на Android невозможен, argv не читаем.
    let args = vec![
        "--resources".into(), resources.to_string_lossy().into_owned(),
        "--game-dir".into(), game_dir.to_string_lossy().into_owned(),
        "--home".into(), home.join("horizon").to_string_lossy().into_owned(),
        "--name".into(), "Player".into(),
    ];
    let startup = horizon_client::entry::prepare_with_args(args);
    // ProcessExitGuard как в AND android_main.rs:225-233 (exit(0) на Drop).
    let _guard = ProcessExitGuard;
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
    // main_with сам не возвращается при ошибке (exit), при нормальном выходе — guard.
}
```
Нюансы: `GAME_IDENTITY`-маркер не нужен (нет лаунчер-импорта); `Options::read` сам сделает `set_current_dir(game_dir)` (`launch_options.rs:142-147`); early-логи до `logging::init` писать прямо в `AndroidLogger::log`. Один EventLoop на процесс → после выхода `exit(0)` (AND `android_main.rs:280-283`).

---

## (г) Упаковка APK и CI

**Что переносится из AND почти как есть:**
- `packaging/android/build.gradle` — целиком (AGP 8.11.1, compileSdk/targetSdk 35, minSdk 28, staging `jniLibs`/`assets`, debug-keystore, `useLegacyPackaging=true`); заменить namespace/applicationId на `dev.magnetite`, имя `.so` на `libmagnetite_android.so`, выкинуть dev-канал/`HORIZON_ANDROID_KEYSTORE`-ветку до релизного этапа.
- `packaging/android/res/` — иконка/баннер/`styles.xml` (тема fullscreen) как временный плейсхолдер.
- `tools/android_build.py` — упрощённый порт: NDK-окружение (:150-157), `cargo build -p magnetite-android --target aarch64-linux-android --profile fast --no-default-features --features mobile`, `llvm-strip`, staging, Gradle, `apksigner verify`. Выкинуть `--public`/hot-swap/bedrock-controls.
- **Манифест — упростить до стокового NativeActivity** (кастомная `HorizonActivity` AND нужна для IME/SAF/hot-swap — этапы 4-5):
```xml
<activity android:name="android.app.NativeActivity" android:exported="true"
    android:screenOrientation="sensorLandscape"
    android:configChanges="orientation|screenSize|keyboardHidden|keyboard|density|screenLayout"
    android:theme="@style/MagnetiteGameTheme">
  <meta-data android:name="android.app.lib_name" android:value="magnetite_android"/>
  <intent-filter><action android:name="android.intent.action.MAIN"/>
    <category android:name="android.intent.category.LAUNCHER"/></intent-filter>
</activity>
```
`extractNativeLibs="true"`, permissions INTERNET/VIBRATE, `uses-feature vulkan required=false`, `glEsVersion 0x00030000 required=true` (как AND). `ANativeActivity_onCreate` экспортирует сам `android-activity` — свой `func_name` не нужен. Gradle, не cargo-apk/xbuild: у AND уже отлаженная связка Gradle+staging, cargo-apk не поддерживает кастомные ассеты/манифест так гибко.

**engine-data.zip:** таблицы генерировать **из MAIN** (`horizon-prepare`, `.local/horizon-client/26.3`), НЕ брать zip AND (форки расходятся в таблицах/structure-transforms; `android_resources::rebuild` — AND-специфика). На CI — порт `tools/android_prepare.py` (официальные загрузки Mojang + экспортёры MAIN); в git ничего Mojang не коммитить (правило `AGENTS.md`).

**CI (`.github/workflows/android.yml`, ubuntu-24.04)** — по образцу AND `dev-builds.yml`:
1. `actions/checkout`; `actions/setup-java` (temurin 17+; AND uses 25); `gradle/actions/setup-gradle@v4` с `gradle-version: 8.14`.
2. NDK: `sdkmanager "$ANDROID_HOME/..." 'platforms;android-35' 'build-tools;35.0.0' 'ndk;28.2.13676358'` (пин как AND; `$ANDROID_NDK_LATEST_HOME` на раннере есть, но версия плавающая — для воспроизводимости лучше пин).
3. `rustup target add aarch64-linux-android` (toolchain из `rust-toolchain.toml` MAIN).
4. Env: `CC_aarch64_linux_android=$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android28-clang`, аналогично `CXX_`/`AR_`/`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`; `CARGO_TARGET_DIR=.local/ci-target`; кэш `~/.cargo` + target (ключ по `Cargo.lock`).
5. Подготовка ресурсов (Java + horizon-prepare) → `engine-data.zip`.
6. `cargo build -p magnetite-android --target aarch64-linux-android --profile fast` (на старте; release+ThinLTO — позже, ~150 мин таймаут как AND).
7. `llvm-strip --strip-unneeded` → staging `jniLibs/arm64-v8a/libmagnetite_android.so`; ассеты (engine-data.zip, LICENSE, THIRD_PARTY_NOTICES).
8. `gradle -p packaging/android -Pabi=arm64-v8a -PminSdk=28 assembleDebug`; `apksigner verify`; `upload-artifact`.
Подпись: debug-keystore генерируется keytool на месте (AND `android_build.py:212-214`); релизный ключ — secrets позже.

---

## (д) Риски и порядок работ

**Риски расхождения форков:**
1. **Гостевой стек 136 MiB vs поток `android_main` (~2 MiB).** Главный риск. Этап 0 обходит `prepare_main`; wasm-моды на render-потоке будут отказаны (`mod-wasm/runtime.rs:938`), пока не будет android-ветки guest-stack или патча android-activity (вендорить по образцу winit и задать `stack_size` потоку). Размер стека потока android-activity 0.6.1 проверить по исходнику в `~/.cargo/registry` на этапе реализации (доступ вне проекта в этой сессии запрещён).
2. **Рендер GLES/PowerVR.** MAIN `backend_order` уже даёт Vulkan→GL, но в MAIN нет AND-тюнингов: `configure_gpu/gpu_batching`, `atlas_limit`, `graphics_startup::recover` (авто-GLES после вылета Vulkan), PowerVR-воркараунды в шейдерах. Ожидаемы проблемы компиляции WGSL→GLSL на GLES-драйверах; `check_layers` (`gfx/device.rs:186-196`) уже учитывает GL-лимиты массивов текстур. План владельца: Vulkan первым везде (`plan.md:11-13`).
3. **Ресурсы:** таблицы AND ≠ таблицы MAIN; нужен свежий `engine-data.zip` из MAIN-подготовки, иначе `missing_resources`/реестры не сойдутся.
4. **mimalloc:** MAIN ставит глобальный аллокатор (`horizon-client/src/lib.rs:16-19`), AND — нет. Патченый `libmimalloc-sys` под android собирается (check проходит); запасной рычаг — фича `system-allocator` (`horizon-client/Cargo.toml:163`).
5. **`current_exe` = app_process:** безвредно для `from_launch`/`Runtime::load` при пустых модах, но `mod_content.rs:3402` (meta exe) и `lod/cache.rs:185` используют его как ключ — проверить на этапе контент-модов.
6. **Ввод:** игра MAIN не читает `android_runtime/keypad/touch` (grep — ноль вызовов), тач-события winit не обработаны → этап 0 = «запускается и рисует титульный экран», играть нельзя до этапа тача. Это соответствует ТЗ.
7. **Жизненный цикл:** без хука б.4 — краш при сворачивании; IME-ресайз тоже пересоздаёт поверхность (AND `game.rs:16209`).
8. **Один EventLoop на процесс** — обязателен `exit(0)`-guard, иначе второй запуск из кэшированного процесса падает.
9. Аудио cpal 0.16 (AAudio) и ureq/rustls под android собираются с NDK CC (AND подтверждает); wasmtime/cranelift на Android легален (JIT разрешён, в отличие от iOS).

**Порядок малых коммитов (каждый проверяем на ПК):**
1. MAIN: `entry::prepare_with_args` + рефактор `prepare` + тест паритета argv.
2. MAIN: `window::{set_event_loop_factory,new_event_loop}` + замена 3 вызовов + тест фабрики.
3. MAIN: `logging::set_platform_sink` + тест.
4. MAIN: `gfx::suspend_surface` + `suspended`/recovery-ветка `resumed` (Game/Loading/App) + тест.
5. MAIN: android-ветка `horizon-guest-stack` (cfg в платформенном крейте) + реэкспорты `client_lib.rs`.
6. `crates/magnetite-android` + workspace member; `cargo check -p magnetite-android --target aarch64-linux-android --no-default-features --features mobile`.
7. `packaging/android` (манифест/gradle/res) + порт `tools/android_build.py`; локальный APK.
8. CI `android.yml` + артефакт APK.
9. Позже: graphics-recovery, GPU-тюнинги android_runtime, тач, IME, лаунчер.

**Верификация end-to-end:** `cargo check` обоих таретов (android/ios) и ПК-default не сломан; `cargo test -p horizon-client` (entry/logging); ПК-смоук `cargo run --profile fast -p horizon-game -- --offscreen --frames 60 --resources .local/horizon-client/26.3`; на устройстве: `adb install` → logcat тег `Magnetite` → титульный экран рендерится (Vulkan, при отказе — GLES) → свернуть/развернуть (surface recreate) → выход и повторный запуск (exit-guard); `python3 tools/pr_ready.py` по правилам MAIN.

**Остаточные непроверенные пункты:** размер стека потока `android_main` в android-activity 0.6.1 и точное содержимое `packaging/android/res/values/styles.xml` AND (тема) — проверить при реализации; на план не влияют.
