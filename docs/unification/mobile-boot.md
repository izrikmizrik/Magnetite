# Завершение этапа 1 на мобилах: «первый запуск → вход → файлы игры → игра»

Обозначения: **MAIN** = `horizon-main\horizon-main`, **IOS** = `horizon-ios\Gfhdgcfg-ccr-bb4385db-s67j6f`, **AND** = `Horizon-Android-android`. Пути от соответствующего корня. Только чтение; ничего не изменял.

## 0. Что уже есть в MAIN (фактическая проверка)

- Точки входа этапа 0 работают без экрана: `crates/magnetite-android/src/lib.rs:75-106` (sink в logcat, `set_event_loop_factory`, `ExitOnReturn`, `prepare_with_args` + `main_with`, `--name Player`) и `crates/magnetite-ios/src/main.rs:11-41` (HOME→Documents, `--name Player`, `main_with` прямо в `main`). Boot-loop в `magnetite-ios` **нет** — цикл открывает ядро (`resource_loader.rs:298`), для этапа 0 это легально, для этапа 1 — нет (watchdog).
- Хуки на месте: `entry::prepare_with_args` (`crates/horizon-client/src/entry.rs:38-44`), `window::{set_event_loop_factory,new_event_loop}` (`crates/horizon-client-input/src/window.rs:730,740`), `logging::set_platform_sink` (`crates/horizon-client-render/src/logging.rs:464`), `Game::suspended/resumed` (`crates/horizon-client/src/game/runtime/window_handler.rs:387-392`).
- **Hand-over не сделан**: `run_loading` сам открывает цикл (`crates/horizon-client/src/game/runtime/resource_loader.rs:298`), split-путь тоже (`loading_screen.rs:37`), «нет ресурсов» тоже (`game/runtime/no_resources.rs:45`). `App` приватен (`loading_screen.rs:429` — `pub(super)`). У `Loading`/`App`/`Missing` **нет `fn suspended`** (grep по `game/runtime` — только `window_handler.rs:387`).
- `crates/horizon-account` готов и **никем не используется** (grep по `Cargo.toml` — только workspace member, корневой `Cargo.toml:6`): device code (`microsoft.rs:53`), `Poll` (`:46-51`), `sign_in` с owner-gate (`:358-376`, `finish_sign_in:340-352` → `owns_game:288-300`), Ely authenticate/refresh + 2FA (`ely.rs:14-16,44-62`), `Account::session_json` (`lib.rs:176-188`), `stored_json` без access-токена (`lib.rs:193-202`), `refresh_account` (`lib.rs:240-266`), `FileStore` 0600+атомарная замена (`store.rs:22-88`), client id — конфигурация `MAGNETITE_MSA_CLIENT_ID` (`lib.rs:48-55`), не хардкод как в IOS (`ios_account.rs:20`).
- Ядро сессии умеет оба вида: `auth.rs` принимает `msa`/`elyby` (`crates/horizon-client-render/src/auth.rs:90-100`); `--session-file` читается в `Options::read` (`game/runtime/launch_options.rs:124-135`), есть и `--session-stdin` (`:117-122`). **Важно**: `AuthSession::load` требует возраст ≤120 с и 0600, файл удаляется после чтения (`auth.rs:25,111-134`).
- `docs/unification/mobile-boot.md` пуст; `ios-entry.md` (б.1) уже проектирует `set_app_sink` — ниже это доведено до экрана входа.

---

## 1. Экран до ресурсов

### 1.1. Что MAIN уже умеет рисовать без файлов Minecraft

Да, фундамент полный, новый рендерер не нужен:

- **Атлас без пака**: `HudAtlas::boot()` (`crates/horizon-client-render/src/hud.rs:1076-1120`) — «Nothing from a pack: the boot screen draws with the theme's bundled faces»: белый спрайт + регион `horizon/ui-text` для рантайм-растеризации глифов. Его уже использует `Loading` до прихода пака (`loading_screen.rs:57`).
- **Шрифты**: Geist из `horizon-theme` (`crates/horizon-theme/src/lib.rs:24-27`, `include_bytes!`), раздаются через `horizon_look::give()` (`crates/horizon-client/src/horizon_look.rs:17-68`, вызов в `entry.rs:143`) → `menu_style::install` (OnceLock, повторный вызов безвреден — `menu_style.rs:148-153`) → `ui_theme::Fonts::load` (`crates/horizon-client/src/ui_theme.rs:136-149`). `ui_theme` сам растеризует глифы в `horizon/ui-text` (`ui_theme.rs:190-230`).
- **Готовый каркас экрана**: `game/runtime/no_resources.rs` — `Missing` (`:9-19`) уже создаёт окно+`loading_renderer` (`loading_window.rs:125`), рисует `NoResources` через `ui_theme::Ui` (`:56-72`), клики (`:96-104`), `lang::set(Language::load(&[], …))` (`:24-27`). Строки — в компилируемых `assets/horizon/lang/*.json`, `ru_ru.json` есть. Экран `NoResources` (`crates/horizon-client/src/no_resources.rs:24-190`) — образец layout/draw/click с тестами паритета (`:260-420`).

**Вывод: расширять именно `no_resources`-каркас**, а не тащить рендерер IOS. `Missing` превращается в `Boot`-handler; статический `NoResources` остаётся для ПК-пути (`game.rs:11154`).

### 1.2. Что переносить из форков и куда

| Источник | Что | Куда в MAIN |
|---|---|---|
| IOS `game/ios_boot.rs:46-100` | Машина состояний `Account{Choose,Resuming,Ely,Asking,Code,Signed,Failed,OfflineName,Offline}` + `Setup{NotNeeded,Waiting,Running,Failed,Done}` + `Shared{attempt,version}` | Новый `crates/horizon-client/src/game/runtime/boot.rs` (цикл, потоки) |
| IOS `game/ios_boot.rs:127-232` | Потоки входа: счётчик `attempt` (устаревший вход ничего не меняет), retry при «no connection» после Safari (`:227`), forget токена при отказе, но не при отсутствии сети (`:163-168`) | туда же; сети — через `horizon-account` вместо `ios_account` |
| IOS `ios_screen.rs:8-37` + `ios_boot.rs:380-575` | Layout экрана: title/lines/code/progress/buttons, экран кода (`:464-481`), экран загрузки файлов (`:528-551`), «Sign in once first» (`:557-568`) | Новый `crates/horizon-client/src/boot_screen.rs` (pub, по образцу `no_resources.rs`), строки в `assets/horizon/lang/{en_us,ru_ru}.json` |
| IOS `ios_boot.rs:1240-1252` | `WindowEvent::Touch` → pointer + click на `TouchPhase::Ended` | в `Boot::window_event` (в `Missing` сейчас только мышь — `no_resources.rs:146-160`) |
| AND `android/login.rs:27-44` | UX формы Ely (login/password/otp, `auto_open_device`) | только как образец полей; **код не переносить** — он сидит на лаунчерном `crate::account` (`#[path]`-трюк AND) и `ui_theme::active_theme`, которых в MAIN нет |
| AND `android/launcher.rs` | Нативный лаунчер | **не переносить** — этап 5 |
| IOS `LauncherAction`/SwiftUI-мост (`ios_screen.rs:41-77`) | — | **не переносить**: в MAIN форма Ely рисуется в игре, а не в SwiftUI |

Почему экран живёт в `horizon-client`, а не в magnetite-крейте: `ui_theme::Face` — `pub(crate)` (`ui_theme.rs:114`), `loading_renderer`/`UiResources` — `pub(super)` (`loading_window.rs:125`, `resource_tables.rs:131`). Снаружи крейта экран не нарисовать без десятка новых реэкспортов; внутри — всё под рукой, и один код обслуживает Android, iOS и (по флагу, без cfg) ПК без лаунчера.

### 1.3. Hand-over: нужен, и вот где

IOS стартует цикл сразу из-за watchdog (`ios_boot.rs:1-7`), игра стартует внутри цикла: `HANDED`/`hand_over` (`:16-19,34-42`), `Boot::start` (`:1106-1146`) вызывает `run_with`, забирает `App`, делает `app.resumed(event_loop)` + `ControlFlow::Poll`. В MAIN повторяем хуком из `ios-entry.md` (б.1):

```rust
// crates/horizon-client/src/game/runtime/mod.rs
pub fn set_app_sink(sink: fn(App));          // One-shot, как window-фабрика
pub fn take_app_sink() -> Option<fn(App)>;
// App сделать pub (loading_screen.rs:429), реэкспорт horizon_client::game::App
```

Замены (симметрично образцу `new_event_loop`, `window.rs:740`):
- `resource_loader.rs:298` — `match take_app_sink() { Some(sink) => sink(app), None => new_event_loop()?.run_app(&mut app) }`;
- `loading_screen.rs:37` — то же для `Split`;
- `game/runtime/no_resources.rs:45` — тоже через sink (иначе boot-путь при повреждённых ресурсах откроет второй цикл → паника winit).

На ПК sink не ставится — поведение байт-в-байт прежнее.

### 1.4. Поток управления (единый для обеих платформ)

```
magnetite-{android,ios}::main
  → horizon_client::boot::run(BootConfig)          // новый pub-вход в horizon-client
      цикл = window::new_event_loop()              // фабрика Android уже ставится
      экран Boot (HudAtlas::boot + ui_theme)       // watchdog доволен: цикл жив
      потоки: resume сохранённого / device code / Ely / setup
      когда ready (файлы есть + вход/офлайн):
        записать session-файл (свежий, 0600)       // правило 120 с, auth.rs:25
        set_app_sink(...)
        (config.start)(session_path)               // замыкание magnetite-крейта:
                                                   // prepare_with_args(--session-file …) + main_with
        app = take_app_sink() → app.resumed(el), Poll
      события → app; выход app → выход цикла
  → Android: ExitOnReturn (уже есть, lib.rs:63-70)
```

Нюансы, проверенные по коду:
- `entry::start` отработает **внутри** `about_to_wait` (как `run_with` внутри `Boot::start` в IOS): `horizon_look::give()` повторно безвреден (OnceLock), но boot должен вызвать его **сам** до первого окна (шрифты/иконка), иначе экран будет в `NEUTRAL` без шрифтов (`menu_style.rs:144`). Сделать `horizon_look::give` `pub(crate)`-доступным из boot.
- `prepare_with_args` вызывать только **после** готовности ресурсов: `Options::validate` читает метаданные ресурсных маркеров (`entry.rs:25-26`).
- Boot-экрану нужен `suspended`: добавить `fn suspended` в `Loading`/`App` (`loading_screen.rs:433`) и в boot-handler → `renderer.suspend_surface()` (образец `window_handler.rs:387-390`); в `resumed` — ветка восстановления surface.
- **Ввод текста**: device code текста не требует (код + ссылка, `ios_boot.rs:464-481`). Форма Ely и offline-имя — требуют. IOS решал это SwiftUI-лаунчером (`ios_boot.rs:449-455` — «typed in the launcher»), AND — `native_ime` (этап 4). В MAIN есть свой OSK (`crates/horizon-client/src/osk.rs:1-37`), но он **геймпадный** (D-pad/A/X). Для этапа 1 нужен тап-хиттест по его сетке клавиш — отдельная аккуратная работа; без неё Ely-вход на телефоне не выполнить (решение владельца — Ely обязателен, `plan.md:24`).
- **Открытие ссылки**: `desktop::open_uri` на мобилах — no-op (`desktop_off.rs:16`, фича `desktop-opener` — ПК). Хук `open_link: fn(&str)` в `BootConfig`: iOS — `openURL` (образец `horizon-threads/ios_url`), Android — JNI `ACTION_VIEW`. Минимальный запасной вариант — только показать код и URL текстом (работает без хука).
- `keep_awake` на время скачивания — тоже колбэк конфига (IOS: `ios_boot.rs:1266-1269`).

---

## 2. Файлы игры на устройстве

### 2.1. Можно ли вызвать `horizon-prepare` как библиотеку на телефоне — нет

- Подготовка гоняет **vanilla-экспортёры на Java**: `crates/horizon-prepare/src/lib.rs:1-8` («runs the vanilla exporters … which call Mojang's own code and so stay Java»), `exporters.rs:121` (`java::find`), `java.rs` — `javac`/`java` через `std::process::Command`. На iOS дочерние процессы запрещены, на Android нет JDK. Это не обходится фичами: Java-шаги — источник半数 таблиц.
- Цена на ПК: JVM-и вместе 1.25–2 ГБ, 63–105 с (`java.rs:18-24`) — телефонный бюджет это не переживёт даже с JDK.
- Зависимости крейта лёгкие (`horizon-prepare/Cargo.toml` — sha1/sha2/zip/regex), но это не помогает: блокер — Java, а не Rust.

### 2.2. Рабочий поток — трюк IOS, таблицы из MAIN

IOS (`game/ios_setup.rs:1-9`): Java-таблицы публикуются **один раз релизом** (`tables-<rev>`, без файлов Mojang), остальное — собственный Minecraft игрока с серверов Mojang после MS-входа. Что именно скачивается (`ios_setup.rs:169-303`):

1. `tables.zip` (наш релиз, sha256 зашит в билд — `:19-20`) → внутри `ios-setup.json`: `client{url,sha1,size}`, `asset_index{url,sha1}`, `objects[]`, `jar_files[]` — **version manifest не ходит**, URL/хеши Mojang запинены в таблицах;
2. `client.jar` с piston-data (sha1) → распаковка только `jar_files` (assets/ и data/);
3. `asset-index.json` → объекты с `resources.download.minecraft.net` (8 потоков, `:24`);
4. staging → проверка `horizon-resources.json` → атомарная активация (`move_dir` с кросс-девайс-копией, `:307-325`).

AND делает то же иначе: живой `version_manifest_v2` + пины `research-lock.json` (`official_install.rs:19-20,326-342`), allowlist хостов (`:113-127`), распаковка assets/+data/ из jar (`:449-465`), затем `resource_install::extract(engine-data.zip)` (`:481`, `resource_install.rs:112`) и **`android_resources::rebuild`** (`:483`, `android_resources.rs:67-95`) — агрегация worldgen JSON чистым Rust, потому что их engine-data.zip беднее. Плюс тач-иконки (`controls_with_progress`, `android_resources.rs:134`) — этап 2, не сейчас.

**Предложение для MAIN (поток):**

1. **CI (ПК-раннер)**: полная подготовка `horizon-prepare` из MAIN → релиз `tables-<rev>`: zip всех **не-Mojang** файлов подготовленной папки (включая worldgen-агрегации — они тогда не нужны на устройстве, `rebuild` не переносим) + `setup.json` (client/asset_index/objects/jar_files, по образцу `ios-setup.json`). Таблицы форков **не брать** (`android-entry.md:209,229` — расходятся). В git/APK/IPA — ничего Mojang (`AGENTS.md:11-13`).
2. **Устройство**: порт `ios_setup::run` в MAIN (модуль `game/runtime/setup.rs` или отдельный `crates/horizon-setup`; зависимости уже есть в `horizon-client/Cargo.toml:69-98` — ureq/zip/sha1/sha2). Улучшения из AND: потоковая запись на диск вместо `Vec` whole-file (`ios_setup.rs:107-119` буферизует весь файл в RAM — client.jar ~30–60 МБ терпимо, но лучше стримить), allowlist хостов (`official_install.rs:113-127`), пины sha1 client jar в билде (аналог `research-lock.json` MAIN).
3. **Вход → файлы**: `Setup::Waiting` закрывается **только Microsoft-аккаунтом**: owner-gate уже встроен в `finish_sign_in` (`microsoft.rs:345`) — успешный MS-вход = владение доказано. Ely владения не имеет (`ely.rs:1-5`) и offline тоже: до появления файлов показывать «Sign in once first» (`ios_boot.rs:557-568`). **Расхождение с IOS-кодом, которое надо исправить сознательно**: IOS запускает setup при любом `Account::Signed` (`ios_boot.rs:1261`), т.е. Ely-вход фактически тоже начинает скачивание без проверки владения; MAIN должен гейтить по `Kind::Microsoft` явно. После первого скачивания файлы общие: Ely и offline играют без MS (правило «файлы приходят с Microsoft-аккаунтом, который владеет игрой, один раз» — `ios_account.rs:6-7`).
4. **Объёмы**: ~520 МБ загрузка (`ios_boot.rs:534`), тест IOS говорит ~600 МБ (`ios_setup.rs:356-358`); пик диска ~2× во время staging+активации; RAM при стриминге — десятки МБ; время — минуты на Wi-Fi, resume по sha1 (`ios_setup.rs:74-80`) переживает обрывы и смерть процесса. Android: staging в internal storage (кэш `getCacheDir` — потом, хук `configure_download_cache_dir`-аналог); iOS: work в `Library/Caches/horizon-setup` (`ios_boot.rs:627-630`).
5. `engine-data.zip` в бандле (путь AND) на этапе 1 **не нужен** — один код-путь «таблицы с релиза» для обеих платформ; бандл-вариант можно добавить позже как оптимизацию первого старта.

---

## 3. Секреты на телефонах

- **iOS**: `FileStore` (`horizon-account/src/store.rs:22-88`) в `Library/Application Support/horizon/account.json` — приватно, не видно в Files (образец IOS `ios_account.rs:10-12,369,379,433` — 0600). `FileStore` уже делает 0600 + tmp+rename. Хранится только `stored_json` (refresh-секрет, kind, name, uuid — **без access-токена**, `lib.rs:193-202`).
- **Android, этап 1**: `FileStore` в `internal_data_path()/horizon/account.json` — **достаточно**: папка приватна песочницей приложения, 0600 работает (unix-ветка `store.rs:44-48`). AND использует Keystore AES/GCM через JNI (`secure_store.rs:13,31-186`, AAD=ключ аккаунта, ciphertext `.gcm`, `:187-235`) — это этап 5: нужен `jni`+`JavaVM` (в `magnetite-android/Cargo.toml` их сейчас нет), и trait AND keyed (`get/set/delete(key)`, `persistent()`) против одноаккаунтного `load/save/clear` MAIN (`store.rs:13-17`) — мультиаккаунт потребует расширения trait. Известный риск Keystore — restore на другом устройстве (`secure_store.rs:184`), у FileStore его нет.
- **Передача сессии игре**: boot пишет session-файл **в момент старта игры** (не в момент входа): `AuthSession::load` отвергает файл старше 120 с (`auth.rs:25,129`), а скачивание файлов идёт минутами. Файл 0600, рядом с секретами, `--session-file` в аргументах; ядро само удалит его после чтения (`auth.rs:131-133`) и поставит chat-ключи (`launch_options.rs:131-135`). Альтернатива `--session-stdin` (`launch_options.rs:117-122`) не нужна.
- **Повторный запуск**: `FileStore::load` → `Account::from_stored_json` → `refresh_account` на потоке (`lib.rs:240-266`); отказ токена — забыть, отсутствие сети — сохранить (образец IOS `ios_boot.rs:163-168`). `needs_renewal` (`microsoft.rs:378-380`) — для обновления перед join.

---

## 4. Минимальные изменения по файлам, малые коммиты

| # | Коммит | Файлы / сигнатуры | Кто делает |
|---|---|---|---|
| 1 | **App sink (hand-over)** | `game/runtime/mod.rs`: `pub fn set_app_sink(fn(App))`, `pub fn take_app_sink() -> Option<fn(App)>`; `App` pub (`loading_screen.rs:429`); замены `resource_loader.rs:298`, `loading_screen.rs:37`, `no_resources.rs:45`; тест «sink получает App, цикл не открывается» | Средняя модель (механика, образец — `window.rs:730-745`) |
| 2 | **Жизненный цикл экрана** | `fn suspended` в `Loading`/`App` (`loading_screen.rs:433`) и forwarding в `Split`; `suspend_surface` уже есть (`window_handler.rs:389`) | Средняя модель |
| 3 | **Подключение horizon-account** | зависимость в `horizon-client/Cargo.toml`; `horizon_look::give` доступ из boot; **проверить `tools/check_layers.py`** (правило слоёв `AGENTS.md:31-35` — новый крейт должен лечь в разрешённый слой) | Средняя модель, но layers-проверка обязательна |
| 4 | **Экран входа (статика)** | `crates/horizon-client/src/boot_screen.rs` по образцу `no_resources.rs` (layout/draw/click + тесты паритета); состояния Choose/Code/Signed/Failed/Offline; строки в `assets/horizon/lang/{en_us,ru_ru}.json`; Touch-клик (`ios_boot.rs:1240-1252`) | Средняя модель под ревью (образец с тестами есть) |
| 5 | **Форма Ely + offline-имя + тап-OSK** | поля по образцу AND `login.rs:27-44`; тап-хиттест сетки `osk.rs`; 2FA через `NEEDS_2FA` (`ely.rs:16`) | **Аккуратно** (ввод текста на мобиле без IME — новая механика) |
| 6 | **Boot-цикл** | `game/runtime/boot.rs`: `pub fn run(BootConfig)`; `BootConfig{game_dir, resources, secrets, work, tables_pin, open_link: fn(&str), keep_awake: fn(bool), start: Box<dyn FnOnce(Option<PathBuf>) -> Result<()>>}`; потоки входа со счётчиком attempt (`ios_boot.rs:127-135`), запись session-файла перед `start`, `app.resumed` + Poll (`ios_boot.rs:1128-1134`) | **Аккуратно** (конкурентность, один цикл на процесс, watchdog) |
| 7 | **Установщик файлов** | порт `ios_setup.rs:169-325` (таблицы→jar→index→objects, resume, staging+rename) + стриминг на диск и allowlist из AND (`official_install.rs:113-127`); тесты: mock-сервер, zip-slip (`ios_setup.rs:367-373`), resume, битый sha1 | Средняя модель для download/extract (тесты-образцы есть); **аккуратно** — активация/откат (`official_install.rs:497-520`) |
| 8 | **magnetite-android** | `lib.rs`: вместо прямого `main_with` — `boot::run`; пути (`internal_data_path`); `open_link` через JNI Intent (+`jni` в Cargo.toml); `ExitOnReturn` остаётся | Средняя модель после #1-7 |
| 9 | **magnetite-ios** | `main.rs`: `boot::run`; секреты — `Library/Application Support/horizon`, work — `Library/Caches`; `open_link`/`keep_awake` (objc, главный поток) | **Аккуратно** (UIKit-правила, watchdog) |
| 10 | **CI таблиц** | workflow: ПК-подготовка MAIN → `tables-<rev>` релиз (zip + setup.json), sha256 в билд через env; `MAGNETITE_MSA_CLIENT_ID` в сборку; ничего Mojang в git | **Аккуратно** (релизный пайплайн, пины) |
| 11 | **Документация** | заполнить `docs/unification/mobile-boot.md` | Средняя модель |

### Риски

1. **Watchdog iOS** — главный: цикл обязан жить до распаковки/скачивания (`ios_boot.rs:1-7`); поэтому #6 не «потом», а до первого device-теста. Текущий `magnetite-ios/src/main.rs` синхронен — первый же длительный старт будет убит системой.
2. **Один EventLoop на процесс**: любой путь, открывающий второй цикл (`no_resources.rs:45` при повреждённых ресурсах после boot) — паника; #1 закрывает все три места. Android — `ExitOnReturn` уже есть.
3. **120 с session-файла** (`auth.rs:25`): писать только перед стартом игры; вход задолго до файлов — штатен.
4. **Текстовый ввод**: без тап-OSK Ely-форма и offline-имя мертвы на телефоне; winit iOS клавиатуру не шлёт (IOS обошёл SwiftUI, AND — IME). Это самый недооценённый кусок этапа 1.
5. **Таблицы форков ≠ таблицы MAIN** (`android-entry.md:229`): только свежая подготовка MAIN, иначе реестры не сойдутся; `android_resources::rebuild` не переносить — агрегации уже в таблицах CI.
6. **Слои** (`AGENTS.md:31-35`, `check_layers.py`): зависимость `horizon-client` → `horizon-account` может нарушить правило «Horizon не в engine»; при отказе — экран/вход за фичу `horizon-extras` или отдельный крейт-платформа.
7. **Фон во время скачивания**: iOS suspend/resume + retry «no connection» (`ios_boot.rs:227`); Android — смерть процесса: resume по sha1 и staging+rename (#7) обязательны.
8. **Место на диске**: пик ~2× объёма ресурсов при активации; при нехватке — понятная ошибка и retry, не молчаливый обрыв.
9. **Билд без `MAGNETITE_MSA_CLIENT_ID`** (`lib.rs:39`): MS-вход недоступен → файлы не получить (Ely/offline не качают); экран должен объяснить, а не циклиться.
10. **Гостевой стек не трогается**: boot-потоки — обычные `std::thread`, guest-вызовов нет; `prepare_main` по-прежнему не вызывается (как сейчас в обоих крейтах).

### Верификация end-to-end

- ПК: `cargo test -p horizon-account` (mock-эндпоинты уже есть, `tests.rs`), тесты sink/suspended/экрана (паритет draw/click по образцу `no_resources.rs:260-420`), `cargo check -p magnetite-android --target aarch64-linux-android` и `-p magnetite-ios --target aarch64-apple-ios` (строки в `ci.yml` уже есть для этапа 0).
- Устройство, первый запуск: экран входа сразу (< лимита watchdog) → device code → «Getting the game files» с прогрессом → титульный экран; session-файл удалён ядром; в `account.json` нет access-токена.
- Второй запуск: resume без кода; авиарежим → аккаунт сохранён, offline-имя играет; Ely-вход (с 2FA и без) → join Ely-сервера; свернуть/развернуть приложение на экране загрузки и в игре — без краша surface.
- Сверка папки ресурсов с ПК-подготовкой MAIN файл-в-файл (образец — `tools/compare_prepare.py`, упомянут в `horizon-prepare/src/lib.rs:10-12`).

**Остаточные вопросы к владельцу (не блокеры, решения приняты по умолчанию в пользу строгости):** (а) гейт скачивания строго на MS (как в документации IOS) против фактического поведения IOS-кода, пускающего Ely тоже; (б) тап-OSK этапа 1 против переноса Ely-формы на этап 4 (IME) — в плане выше выбран тап-OSK, т.к. решение владельца требует Ely прямо в игре.
