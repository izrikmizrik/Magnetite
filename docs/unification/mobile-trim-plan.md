# Мобильная выемка Horizon: карта зависимостей, классификация, механизм гейтинга

> **Поправка оркестратора (проверено по коду и устройству wasmtime).** Пункт 13 и раздел 3 ниже предлагают убрать
> из wasmtime фичу `cranelift` на iOS. Это неверно: Pulley — только исполнитель, а wasm в байткод Pulley компилирует
> тот же Cranelift. Без `cranelift` движок умеет лишь загружать заранее собранные `.cwasm`. Правильно: cranelift
> оставить везде, а там, где JIT недоступен (iOS), выбирать цель `pulley64` в `engine_config()`
> (`crates/horizon-mod-wasm/src/runtime.rs`). Это правка ядра по требованию 3 ТЗ, а не выемка, и делается отдельным шагом.
>
> Сборка на Windows из `D:\Раст майн\...` падает в `libmimalloc-sys` (cl.exe и кириллица в пути). Собирать с
> `CXXFLAGS=-utf-8 CFLAGS=-utf-8` (флаг через дефис: Git Bash портит `/utf-8`) и `CARGO_TARGET_DIR=D:/hz-target`.

> **Решение владельца (2026-11-21): шейдер-API на мобилах — обязательно последней версии.** `horizon-shaders`,
> `horizon-shader-api`, `horizon-optifine` и naga `glsl-in` в мобильной сборке **остаются** (пункт 17 и вопрос
> «Шейдеры OptiFine» в разделе 4 закрыты). Версия берётся из main: там ~40 тыс. строк против ~25 тыс. в форках
> iOS и Android. Отдельно нужно проверить, что шейдер-паки работают на бэкендах GLES (Android) и Metal (iOS).

## 0. Что такое «бинарник игры»

Цепочка сборки исполняемого файла:

- `crates/horizon-game/src/main.rs` → `crates/horizon-game/src/lib.rs:29-32` (вызывает `horizon_mod_host::exit_if_compile_child()` и `horizon_mod_host::main_with(...)`) → `horizon-mod-host` → `horizon-client` (lib+bin, `crates/horizon-client/Cargo.toml`, `[[bin]]` в `horizon-game/Cargo.toml:24-27`) → `horizon-client-lib` (`[lib] path = "../horizon-client/src/client_lib.rs"`, `crates/horizon-client-lib/Cargo.toml:14`) → `horizon-client-render` / `horizon-gameplay` / `horizon-client-gameplay` → `horizon-world/core/protocol/...`.
- Официальный exe — **другой** бинарь: `crates/horizon-official/Cargo.toml:15-25` (heartbeat, discord, official-servers, friends-join, perf-share). Мобильная сборка его не строит — эти крейты в игру не попадают автоматически.
- `horizon-ctl` — отдельный bin в пакете `horizon-client-lib` (`crates/horizon-client-lib/Cargo.toml:173-175`); в игровой exe не линкуется.
- Вне дерева игры вообще (отдельныеWorkspaces/бинарники, трогать нечего): `launcher/`, `crates/horizon-telemetry-server`, `crates/horizon-relay`, `crates/horizon-bots`, `crates/cargo-horizon-mod`, `crates/horizon-server`, `crates/horizon-server-launch`, `crates/horizon-prepare`, `crates/horizon-port-pack*`, `crates/horizon-perf` (в игре только dev-dep), `crates/horizon-release-sign`, `xtask`, `crates/horizon-mod-registry`. `horizon-game-discovery` — только build-dependency (`crates/horizon-client/Cargo.toml:11-13`, `horizon-client-lib/Cargo.toml:11`), в бинарь не входит.

## 1. Workspace-крейты в бинарнике игры — классификация

### Нужно на мобиле (ядро, оставить как есть)
`horizon-core`, `horizon-world`, `horizon-protocol`, `horizon-items`, `horizon-command`, `horizon-client-core`, `horizon-client-input`, `horizon-client-render`, `horizon-client-lib`, `horizon-client-gameplay`, `horizon-gameplay`, `horizon-assets`, `horizon-hooks`, `horizon-engine-slots`, `horizon-mod-vocab`, `horizon-mod-api-macros`, `horizon-mod-runtime` (чистые таблицы, без wasm), `horizon-guest-stack`, `horizon-threads`, `horizon-system`, `horizon-theme`, `horizon-java-properties`, `horizon-server-files` (форматы saves/eula — используются и клиентом), `horizon-chat-filter` (ставится в `entry.rs:109`, маленький), `horizon-addresses` (косвенно через cosmetics/telemetry/friends), `horizon-cosmetics` (поведенчески выключается `cfg!(horizon-extras)`, `game/interface/cosmetics/mod.rs:334`).

### Только ПК → вырезать из мобильной сборки
| # | Что | Где в бинаре | Ключевые точки вызова (gate-места) | Объём работ |
|---|---|---|---|---|
| 1 | **Агентский контроль** (control-сокет + все ctl-команды) | `src/control.rs` 954 стр. (модуль `client_lib.rs:65`); `game/runtime/control.rs` 2046; `game/runtime/agent/mod.rs` 2505; `ui_control.rs` 241; `saves_control.rs`, `station_control.rs`, `blockctl.rs`, `repro.rs`, `input_trace.rs`, `memory.rs`, `replay.rs` | pole: `game/runtime/window_handler.rs:336` (`poll_control`), `game/runtime/loading_window.rs:347` (`start_agent_control`), `game/runtime/agent_settings.rs:31-55` (запуск сервера), `game.rs:1665` (поле `control`), `game.rs:9247`, `game/interface/options.rs:383` (синхр. настройки), страница `Page::Agent` (`runtime/control.rs:238-241`), `settings.extras.agent_control` | Средний-высокий: ~9–10 тыс. стр. собственного кода, но все входы в ~10 точек; шаблон вырезки — stub `friends_off.rs` |
| 2 | **MCP-сервер** | **Не входит в игровой бинарник.** `horizon-mcp` — зависимость пакета lib (`horizon-client-lib/Cargo.toml:33`), используется только bin: `src/bin/horizon-ctl/mcp.rs:13-14`. Сам крейт = serde_json (`horizon-mcp/Cargo.toml:10`) | Для вырезки: не билдить `horizon-ctl` (bin в списке пакета) и перенести dep в bin-секцию; текст MCP-конфига в `agent_settings.rs:117-131` уйдёт вместе с п.1 | Нулевой-низкий |
| 3 | **`--benchmark` + `horizon-bench`** | `src/benchmark.rs` 1364 стр.; dep `horizon-client-lib/Cargo.toml:87` (crate лёгкий: serde + `horizon-system`, `horizon-bench/Cargo.toml:12-18`) | `game/runtime/launch_options.rs` (парсинг `--benchmark`), `loading_window.rs:328-369` (запуск `benchmark::start` :352), `resource_load.rs:57`, `activity.rs:13`, `game/world/integrated.rs:524`, `interface/profile_capture.rs:560`, `telemetry.rs:161,405`, `server_perf/system.rs:27,33,145-165`, `client_perf.rs:287-292` | Низкий-средний (~40 вызовов `horizon_bench::` в игре) |
| 4 | **Telemetry-накопитель `horizon-telemetry`** | `src/telemetry.rs` 547 стр. (модуль в `client_lib.rs`); dep `horizon-client/Cargo.toml:~112`, `horizon-client-lib/Cargo.toml:86` | `entry.rs:120` (`mark_start`), `entry.rs:122` (`crash::on_report(telemetry::keep_crash)`), `game/world/world_session.rs:246` (`note_display`), `game.rs:11121-11122` (consent для Perf), `benchmark.rs:344` | Низкий, но менять надо с осторожностью: без лаунчера на мобиле отправка всё равно невозможна — realistic-вариант вырезать накопитель целиком (547 стр. + 5 мест) |
| 5 | **Перф-захваты `.hzperf`** (`client_perf` 322 + `server_perf/*` + `perf_net.rs` 590 + `perf_overlay` + `interface/profile_capture.rs` (1060+ стр.) + `game/world/integrated/capture.rs`, dep `horizon-perf-format`) | узлы: `game.rs:1737-1738` (поле `profile`), `game.rs:10421`, `game.rs:8755`, `runtime/control.rs:217` (`poll_profile` — внутри полла контроля, уйдёт с п.1), `game/world/levels.rs:1923`, `interface/dedicated/report.rs:9-15`, `game/world/integrated/capture.rs:6-10` | Средний: ~3–4 тыс. стр.; «всегда-on» health-history (`client_perf.rs:2`) завязан на autotune — оставить минимальный ринг или заглушку |
| 6 | **Dedicated-сервер в игровом exe** | `game/interface/dedicated/` (8 файлов, `mod.rs` ~1500+ стр.), `game/dedicated.rs` | вход: `runtime/launch_options.rs:58` (`--dedicated`), ветка в `entry.rs:139-147` | Низкий: один аргумент + один модуль; LAN-host при этом остаётся |
| 7 | **Studio** (recorder/timeline/camera path) | `src/studio/` (9 файлов) + `game/interface/studio.rs` + `render/src/studio_ui.rs`; группа `studio` в ctl (`runtime/control.rs:22-25`) | `game/interface/studio.rs:30` (`EDITOR = cfg!(horizon-extras)`) — сейчас только поведенческий гейт; для выемки — модульный | Низкий-средний |
| 8 | **Splitscreen** | `game/runtime/split/` (в т.ч. `players.rs:472`) | аргументы окон/мониторов, страница настроек Splitscreen | Средний (много ссылок на окна/профили), но на сенсере бессмысленно — вырезать |
| 9 | **`desktop.rs`** (открыть папку/URI, xdg-open/ShellExecute) | `src/desktop.rs` 217 стр. | `title/world_management.rs:121`, `game/world/packs/mod.rs:38,44` — кнопки «Open Folder» | Низкий: 3 места + кнопка |
| 10 | **`horizon-anticheat`** | `src/lan/anticheat.rs` (+tests), `lan/guest.rs:312`, `lan/recording.rs:5`; dep `horizon-client-lib/Cargo.toml:52` | Это серверная сторона LAN-host'а. Мобильный игрок может хотеть хостить LAN → **неясно**; крейт маленький, тяжёлых dep нет. Держать до решения | Низкий в любом случае |
| 11 | **`horizon-dualsense` + `gilrs`** | Уже optional за `controllers` (`horizon-client/Cargo.toml:93-96,150-152`; `horizon-client-lib/Cargo.toml:91`) | Мобиле: просто не включать `controllers` | Нулевой |
| 12 | **`horizon-narrator`** | ~44 места (chat, options_ui, startup_menu, screen_narrator.rs) | Accessibility-мост на мобильные TTS нужен в будущем; без бэкенда `is_active()==false` (windows/linux/macos-only backend, `horizon-narrator/Cargo.toml:9-24`). **Оставить** | Нулевой |
| 13 | **Мод-рантайм wasmtime** | `horizon-game → horizon-mod-host (Cargo.toml:18) → horizon-mod-wasm → wasmtime 50` с **одновременно** `cranelift` и `pulley` (`horizon-mod-wasm/Cargo.toml:22`) | Для iOS: убрать feature `cranelift` (JIT-память запрещена) — `Engine::new` в `horizon-mod-wasm/src/runtime.rs:173,178-192` сам сядет на Pulley (заметка `runtime.rs:158-160`: threads и так выключены, Pulley-ограничение не мешает). Также desktop-only: `runtime-process` (`horizon-mod-package/Cargo.toml:66`: os_pipe/spawn компайл-чалда; включён у `horizon-client/Cargo.toml` строка mod-package с `features = ["index","runtime-process"]`), `junction` (Windows junction, `horizon-client-lib/Cargo.toml:61`) и **нативные моды** = `libloading` (`horizon-mod-host/Cargo.toml:22`, `src/native_libraries.rs:86-94`; `horizon-game/src/lib.rs:29`) — на iOS невозможны в принципе | Средний: feature-параметризация wasmtime + stub нативных мод; код-мод-хук места в `entry.rs` (`load(&native, ...)`) гейтить |
| 14 | **mimalloc** | `horizon-threads/Cargo.toml:9-10` (+патч `third_party/libmimalloc-sys`); глобальный alloc в `crates/horizon-client/src/lib.rs:16-19` | Уже есть готовый рычаг `system-allocator` (`horizon-client/Cargo.toml:163`, cfg в `lib.rs:16`). Работает и на Android, и на iOS; решение — маркетинговое, не блокирующее | Нулевой |
| 15 | **Аудио (`cpal`, `symphonia`)** | `horizon-client-render/Cargo.toml:41,58` — optional за `audio` (`:114`) | cpal поддерживает AAudio/AudioUnit → **оставить** `audio` в мобильном профиле | Нулевой |
| 16 | **HDR (`horizon-hdr`)** | `horizon-client-render/Cargo.toml:24`; Vulkan `ash` + wayland (linux-only, `horizon-hdr/Cargo.toml:8-17`) | Android HDR через Vulkan работает; iOS EDR — отдельная дорожка. **Неясно**, по умолчанию оставить | Нулевой |
| 17 | **Форматы паков: `horizon-optifine`, `horizon-shaders` + `horizon-shader-api`, `horizon-polytone`** | `horizon-client/Cargo.toml:38,62-66`; `horizon-client-render/Cargo.toml:31-37` (включая naga `glsl-in`) | Это контент-совместимость паков, а не «мусор». На мобиле шейдер-паки OptiFine всё равно почти не используются; решение за владельцем. Отдельно: naga `hlsl-out` в普通-deps (`horizon-client/Cargo.toml:88`, `horizon-client-render/Cargo.toml:53`) нужен **только тестам** — кандидат на удаление из всех сборок | По решению; выемка `shaders`+`glsl-in` — самый «жирный» выигрыш после wasmtime |
| 18 | **`horizon-friends` + relay-клиент** | Уже optional за `friends` (`horizon-client/Cargo.toml:71-72,155-157`), модуль гейтится `// game-module-cfg: feature = "friends"` (`game/network/friends.rs:1`) + заглушка `game/network/friends_off.rs`; join-сторона только в `horizon-official` | Мобиле: не включать `friends` | Нулевой |
| 19 | **`horizon-heartbeat` / `horizon-discord` / `horizon-official-servers` / `horizon-perf-share`** | **В игровой бинарник не входят** — только `crates/horizon-official/Cargo.toml:17-24`. `perf-share` — хук «Open/Share .hzperf» (`horizon-perf-share/src/lib.rs:1-4`) | Нечего гейтить; мобильный exe строится из `horizon-game` (или своего «mobile-official») | Нулевой |
| 20 | **`horizon-game-discovery`, `cargo-horizon-mod`, `horizon-bots`, `horizon-relay`, `horizon-telemetry-server`, launcher** | Вне дерева игры | Просто не билдить | Нулевой |

### Тяжёлые внешние зависимости в мобильном бинаре (что реально «весит»)
- `wgpu` (vulkan+metal+gles+dx12 по cfg, `horizon-client/Cargo.toml:76,127-139`) — остаётся вся; на мобиле gles/vulkan/metal нужны.
- `naga` с `wgsl-in, glsl-in, hlsl-out` (`horizon-client/Cargo.toml:88`, `horizon-client-render/Cargo.toml:53`) — `glsl-in` живёт ради шейдер-паков, `hlsl-out` — ради тестов.
- `wasmtime` + `cranelift-codegen` (+ `pulley`) — через mod-host, см. п.13; самый большой кандидат на сокращение.
- `gilrs`+`horizon-dualsense` — уже optional.
- `cpal`+`symphonia` — аудио, оставить.
- `ureq` (rustls) — скачивание скинов/паков/Realms/реестра (`server_packs/download.rs:237`, `realms/client.rs:1203`, `mod_registry.rs:83,267`) — оставить.
- `mimalloc` — см. п.14; `rayon` (`horizon-client-lib/Cargo.toml:41`), `mio` (`:82`, сетевые потоки LAN) — оставить; `cap-std`/`cap-fs-ext` (`horizon-client/Cargo.toml:131-133`, `game/modding/native_config_files.rs:4`, `horizon-world/Cargo.toml:28-29`) — уйти вместе с нативными модами; `fs4` (`save.rs:355`) — оставить; `zip`, `flate2`, `image-webp`, `zune-jpeg`, `ab_glyph`, `chrono`, `serde*` — оставить.
- windows/macos-only (`open`, `dunce`, `objc2*`, `windows-sys`) уже изолированы `[target.'cfg(...)']`.

## 2. Как гейтится сейчас и где дырка

1. **Cargo-фичи по-настоящему вырезают** только: `audio` (→cpal), `controllers` (→gilrs/dualsense), `friends` (→крейт + модуль целиком, через прецедент `game-module-cfg` + `friends_off.rs`). Серверная сборка = `--no-default-features` (`horizon-client/Cargo.toml:145-152`).
2. **`horizon-extras` — не вырезает, а глушит**: гейты это `cfg!(feature = ...)` — обе ветки компилируются и линкуются (`docs/forking.md`, «both sides compile in every build»; места: `entry.rs:116`, `game/interface/cosmetics/mod.rs:334`, `game/interface/studio.rs:30`). Для мобильной экономии веса непригоден.
3. Агентский контроль, benchmark, telemetry-накопитель, perf-захваты, dedicated, studio, splitscreen, `desktop.rs` — **нигде не гейтятся** (docs/forking.md прямо: «Not behind the feature: Agent control...»).
4. Ограничение ТЗ «в логике ядра нельзя `#[cfg(target_os)]`» в коде уже нарушено точечно (в input-слое: `horizon-client-input/src/clipboard.rs:7,133-180`, `keybinds.rs:304-323`, android-cfg `winit` в `horizon-client/Cargo.toml:136-137`) — это упаковочные/платформенные исключения, новые гейты делать фичами.

## 3. Предлагаемый механизм: фича `desktop` + mobile-профиль

Наиболее чистый вариант, совпадающий с существующей архитектурой:

- В `horizon-client`, `horizon-client-lib`, `horizon-mod-host`, `horizon-game` добавить:
  - `default = ["audio", "controllers", "horizon-extras", "desktop"]` — **ПК-сборка графически идентична сегодняшней** (просто `desktop` включена по умолчанию);
  - `desktop = ["agent-control", "benchmark", "telemetry-store", "perf-capture", "dedicated", "studio", "splitscreen", "desktop-opener", "native-mods", "mod-jit"]` (или атомарно, без группы — см. ниже);
  - `mobile = ["audio"]` — профиль для `--no-default-features --features mobile` (по образцу серверного `--no-default-features`).
- Техника гейта — **модульная, по прецеденту friends** (`// game-module-cfg: feature = "friends"` в `game/network/friends.rs:1` + зеркальная заглушка `friends_off.rs`): для каждого удаляемого подсистемы — `#[cfg(feature = "...")] mod control;` + stub. Это сохраняет компилируемость обеих сторон и не размазывает `cfg!` по телу `Game`.
- Атодарные фичи вместо одной `desktop` предпочтительнее, чтобы мобильные платформы将来 могли вернуть себе кусок (напр. `perf-capture` для отладки), и чтобы CI серверной сборки не сдвинулся.
- Точки среза минимальны (см. таблицу п.1): поллю `poll_control` в `window_handler.rs:336`, старт в `loading_window.rs:328-352`, поля `Game` в `game.rs:1665,1737-1738`, `entry.rs:109,116-122`, `horizon-game/src/lib.rs:29-32` (mod-host).
- Wasmtime: в `horizon-mod-wasm/Cargo.toml:22` сделать `cranelift` опциональной фичей крейта (`jit = ["wasmtime/cranelift", "wasmtime/parallel-compilation"]`), mobile: только `runtime+pulley` — `Engine::new` (`runtime.rs:173`) легально падает на интерпретатор; threads отключены и так (`runtime.rs:158-168`). `runtime-process`/`junction`/`libloading`-нативные моды — за `native-mods`-гейт (на iOS dlopen запрещён политикой).
- `telemetry::keep_crash` в `entry.rs:122` при вырезании телеметрии заменить no-op-ом в stub-модуле (crash-репорт останется локальным файлом — `crash::install`, `entry.rs:163`).

## 4. Неясные решения, которые нужны от владельца (предположения по умолчанию)

| Вопрос | Мой дефолт в плане |
|---|---|
| LAN-хостинг с телефона (значит `lan/hub`, `anticheat`, `mio`) | оставить (это и есть сервер-тред сингла) |
| Play with Friends на мобиле | выключить (`friends` не включать; join всё равно в `horizon-official`) |
| Шейдеры OptiFine / `horizon-shaders` / naga `glsl-in` | оставить v1, вынести в отдельную фичу `shader-packs` решением |
| `horizon-hdr` | оставить |
| mimalloc на iOS | оставить (работает); рычаг `system-allocator` есть |
| Splitscreen, Studio, dedicated, benchmark, ctl/агент, MCP-бинарь, telemetry, perf-capture, desktop-opener | вырезать (согласно запросу) |

## 5. Проверка результата

1. `cargo tree -p horizon-game --no-default-features --features mobile -d` — убедиться, что `wasmtime` без `cranelift`, нет `gilrs`, `horizon-mcp` не в graph либы.
2. Границы ПК-сборки не сдвинули: `cargo tree -p horizon-game` до/после — идентичный graph (default теперешний + `desktop`).
3. Компиляция обеих сторон гейтов: `cargo check -p horizon-client -p horizon-client-lib -p horizon-game` (default) и тот же `--no-default-features --features mobile`; серверный путь `--no-default-features` не должен сломаться (`docs/forking.md`: CI билдит только default, серверная сборка — best effort, так что mobile-профиль надо добавить в CI-матрицу самому).
4. `python3 tools/pr_ready.py` + затронутые харнессы; ран-смоук: `cargo run --profile fast -p horizon-game -- --offscreen ...` и `python3 tools/lan_e2e.py --check chest`, чтобы stub-модули не разъехались с реальным поведением.
5. Размер: сравнить `ls -l target/*/horizon-client` на aarch64 до/после (главный выигрыш ожидается от cranelift + naga glsl-in + ctl-модулей).

Итого по объёму: ~15–20 тыс. стр. гейтимого кода, но ~90% точек входа локализованы в 12 файлах (pole-места в таблице п.1); самый большой одиночный выигрыш по весу — `cranelift` в wasmtime, самая высокая связность — агентский контроль (его поллинг оплетает весь кадр), самый спорный — perf-capture (связан с always-on health-history и autotune в `client_perf.rs:2`).
