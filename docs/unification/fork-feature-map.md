# Слияние форков Horizon: платформенный аудит MAIN / iOS (Ferrite) / Android

Пути ниже — от `D:\Раст майн`. Сокращения: **MAIN** = `horizon-main\horizon-main`, **IOS** = `horizon-ios\Gfhdgcfg-ccr-bb4385db-s67j6f`, **AND** = `Horizon-Android-android`. Строки указаны по фактическому чтению; «н/д» = размер не замерялся.

**Ключевые находки до таблиц:**
- MAIN — не «чистый ПК»: в нём уже есть тач-база, адаптированная из форка Starling (ветка android) — `MAIN\crates\horizon-client-render\src\touch.rs:1-14` (629 стр.) + `touch\layout.rs` (534), `keypad.rs`, `android_gamepad.rs`, `android_runtime.rs` (167), battery mode (`graphics.rs:942-948`), `dynamic_resolution.rs`, android-cfg в `horizon-client-input` (`keybinds.rs:304,323`, `clipboard.rs:155`) и зависимость winit `android-native-activity` (`horizon-client\Cargo.toml:122-123`). Но **нет** редактора HUD, кастомных кнопок, пресетов, входа в игре, Ely.by, RAM-губернатора, crash watcher, голосового чата.
- Структурный риск: у IOS и AND монолитный `game.rs` (17 000+ / 21 000+ строк), у MAIN игра разнесена по `game\*` (напр. `game\runtime\split\pads.rs`, `game\interface\pad_menu.rs`). Переносить нужно **фичами, не файлами**.
- AND живёт на **патченом вендоренном winit**: `AND\Cargo.toml:78` (`winit = { path = "third_party/winit" }`, `[patch.crates-io]` :77-80) с API `native_content_rect`, `native_device_id`, `set_native_gamepads`, `set_native_content_insets`. MAIN и IOS используют стоковый winit 0.30.13.
- AND уже решил задачу «вход в игре без лаунчера» переиспользованием кода лаунчера: `AND\crates\horizon-client\src\android_main.rs:17-18` — `#[path = "../../../launcher/src/account.rs"] mod account;`. Это готовый паттерн для MAIN.
- В MAIN `horizon-threads\src\capped.rs:3` ссылается на `crate::memory_limit`, но модуля `memory_limit.rs` в MAIN **нет** (проверено glob по `horizon-threads\src\*.rs`) — битая ссылка, губернатор памяти надо переносить.

---

## 1. Таблица фич, отсутствующих (или неполных) в MAIN

| Фича | iOS (файлы, ~строк) | Android (файлы, ~строк) | Аналог в MAIN | Категория | Сложн. | Заметки |
|---|---|---|---|---|---|---|
| Вход Microsoft device code прямо в игре | `IOS\crates\horizon-client\src\game\ios_account.rs` (523), `ios_boot.rs` (1303), `ios_screen.rs` (422) | `AND\crates\horizon-client\src\android\login.rs` (951) + `launcher\src\account.rs` через `#[path]` (`android_main.rs:17`) | Только лаунчер: `MAIN\launcher\src\account.rs` (2586); игра читает session-file (`auth.rs:1-16`) | платформа | L | iOS: Azure app Cazza `967c7b5b-…` (`ios_account.rs:20`); MAIN: `HORIZON_MSA_CLIENT_ID` (`account.rs:44-50`) |
| Ely.by логин/пароль (+2FA) | `ios_account.rs:31-32` (authenticate/refresh), `horizon-client-render\src\auth.rs:16-22,31,65,503,559`, `skins.rs:27-38` | `AND\launcher\src\account\ely.rs` (311), `login.rs` (поля ely/username/password/otp) | **Нет** (grep `ely` в `MAIN\...\auth.rs` — 0) | правка ядра + платформа | M | Ядро: `AuthSession` должен стать enum MS/Ely; join через `authserver.ely.by/session/join`; чат Ely — без подписи (`auth.rs:503`) |
| Несколько аккаунтов, offline-имя | `ios_boot.rs` / `ios_screen.rs` (account picker), CHANGELOG 2026.11.11 | `login.rs` (`saved_accounts`, `offline`) | `accounts.json` в лаунчере (`account.rs:5-6`) | платформа | M | Хранение: iOS `Library/Application Support/horizon/account.json` (приватно), Android secure_store |
| Тач: мир + экраны (жесты-мышь) | `IOS\...\game\touch.rs` (1916): tap=клик, swipe=скролл, hold-drag, 2-й палец=ПКМ; плавающий стик | `AND\crates\horizon-client-render\src\touch.rs` (3022) + `touch_ui.rs` (8900) | `MAIN\...\touch.rs` (629) + `touch\{layout,aim,draw}.rs` — база Starling, 14 фикс. кнопок, 4 схемы | платформа | L | У MAIN нет экранного «пальца-мыши» уровня iOS (drag/scroll/right-click в инвентарях) |
| Редактор HUD (двигать/добавлять/удалять/resize) | `game\touch_editor.rs` (597): Add/Remove/Smaller/Bigger/Picture/No pic/Layout/Reset/Done | `touch\editor.rs` (975, `EditorDragKind::Move/Resize`) + `touch\custom.rs` (787, `CustomButton{key,…}`) | **Нет** (grep `touch_editor\|touch-controls.json` в MAIN — 0) | платформа | L | Паритет владельца: до 3 раскладок в `touch-controls.json` |
| Кнопки-клавиши (F1–F3, F3+B…, 1–9, /, моды) | `touch.rs:64-100` `Key::Keys(&["f3","g"])`, путь `horizon-ctl key` | `custom.rs` `CustomButton.key` + `osk.rs` (н/д) | `keypad.rs` (профиль клавиш, Starling) — частично | платформа | M | В MAIN `keypad` есть, но нет привязки к экранным кнопкам |
| Свои картинки кнопок | `touch-icons/<action>.png` + `packaging\ios\swift\ImagePicker.swift` + спрайты пака `gui/sprites/touch/<action>.png` (docs/ios.md); 256×256 обработка | галерея: `android\gallery.rs` (н/д); кастом-кнопки в `custom.rs` | **Нет** | платформа | M | Логика загрузки иконок платформенно-нейтральна; пикер — платформенный |
| Пресеты/стили (MCPE Bedrock, GoyControl, виртуальная мышь, floating H-меню) | 3 раскладки (проще) | `touch\layout.rs:126,708-709,936-943,976-1199` (`horizon-touch.json`, `controls/<style>/<id>.json`, защищённый `default.json`), `touch_ui.rs` (8900: хаб, слайдеры, virtual mouse) | **Нет** | платформа | L | Android-доп. по правилу владельца; не конфликтует с 3 раскладками, если пресеты — надмножество |
| Гироскоп | **Нет** | `android\device_gyro.rs` (212, ASensorManager на своём looper) + слайдер `gyro_sensitivity_dragging` (`touch_ui.rs:41`) | **Нет** | платформа | M | Точка применения к камере не зафиксирована (не в `touch.rs`/`game.rs`; вероятно android-входной слой) |
| Геймпад/клавиатура/мышь на мобиле | `horizon-threads\ios_gamecontroller.rs` (287, GCController/GCKeyboard/GCMouse), `game\ios_input.rs` (133), `game\pad.rs` | `android\platform.rs:696` (`set_native_gamepads`), патченый winit (`native_device_id`, `game.rs:16298-16443`), `game\pad.rs`, `game\android_input.rs` | `android_gamepad.rs` + `android_runtime.rs` (167, чистое состояние, тестируется везде) — частично | платформа | L | iOS: gilrs не работает, winit iOS не шлёт клавиши/мышь — всё через GameController (`ios_gamecontroller.rs:1-6`) |
| Защита памяти + плавная дальность прорисовки | `horizon-client\src\ram_limit.rs` (505: Governor, `/ramlimit`, phone_step_up 8→29 за ~21 с), `game\ram_limit.rs` (103), `horizon-threads\memory_limit.rs` (462, `os_proc_available_memory` :323-332) | **Нет губернатора** (grep `ram_limit` — 0); вместо этого dynamic resolution + exit reports | `crash.rs:611-614` `report_memory_limit` (хук есть), `capped.rs`; модуля `memory_limit` нет | правка ядра | L | Перенос почти целиком платформенно-нейтрален; iOS-специфика — только источник «сколько осталось» |
| Ловец вылетов | `game\crash_watch.rs` (119): `home/running.json` каждые 5 с, `last-crash.txt`, 400 строк лога, показ на Play-вкладке (`ios_boot.rs:845,913,1043,1088`) | `android\exit_reports.rs` (235, ApplicationExitInfo через JNI), `native_tombstone.rs` (н/д), `horizon-android\build.rs:23-26` (`--build-id=sha1` для символизации) | `crash.rs` (обычные crash-reports) | платформа | M | Два互补 подхода: iOS — «маркер живого запуска», Android — системный журнал выходов |
| Battery mode | `graphics.rs:687-690,895` (60 fps мир / 30 меню) | Battery Saver 60 FPS VSync (README:31) | **Есть**: `MAIN\graphics.rs:942-948,1200` | правка ядра | S | Уже в MAIN; проверить только моб. дефолты |
| Профили настроек (PvP/survival) | `game\ios_instances.rs` (136): `instances/<name>`, `instance.txt`, копия options/graphics/servers/hotbar; `logging::set_player_dir` | не обнаружено (пресеты тача — другое) | нет (лаунчерные инстансы — другое) | игровая фича | M | Миры/паки/моды/вход общие; наличие `set_player_dir` в MAIN не проверено |
| Обновление в приложении | `game\ios_update.rs` (102): `altstore-source.json`, SideStore/AltStore install-links | `android\updater.rs` (291): GitHub Releases, changelog, установка APK | нет | платформа | M | Разные каналы; общая абстракция «проверить/скачать/установить» |
| Переподпись в приложении (7 дней) | `crates\ferrite-signer\src\lib.rs` (839, isideload/idevice, SRP+2FA, LocalDevVPN 10.7.0.1, pairing file, increased-memory-limit), `swift\Signing.swift` | не нужно | нет | платформа (iOS-only) | L | Только iOS; пароль Apple ID не хранится (`lib.rs:9-12`); не отзывать сертификаты (:14-16) |
| Нативный лаунчер в игре | SwiftUI: `packaging\ios\swift\` (11 файлов: Launcher/PlayView/Signing/Skins/SkinModel3D/TitleBackground/…), мост `horizon-threads\ios_launcher.rs` (73, `hz_launcher_*`, cfg `horizon_swift_ui`) | Rust Material3: `android\launcher.rs`, `launcher_pages.rs`, `launcher_gesture.rs` (н/д) | десктопный GPUI-лаунчер (`MAIN\launcher\`) | платформа | L | Android-лаунчер на Rust — переносимее; iOS SwiftUI остаётся платформенным слоем |
| Speed Trade | `game\trading.rs:22,56-58,542-546` + настройки `ios_extras.rs:36-40,58-60` (4/10/20 в с, Hold/Tap) | нет | нет | игровая фича | S | Кнопка «>>» у результата сделки; работает и на серверах |
| Инвентарь пальцем (drag-split, fling списков) | CHANGELOG Unreleased («Inventory by finger»); реализация в тач/экранных слоях `game\touch.rs`/экраны (точное место не зафиксировано) | жесты в `touch_ui.rs` (частично) | нет | игровая фича | M | Разделение стака протяжкой, инерция списков (JEI, creative) |
| Голосовой чат (Simple Voice Chat) | `crates\voicechat-engine\src\lib.rs` (1266 + audio/buf/…), `game\voice_chat.rs` (281), кнопки `vc.menu/talk/mute` (`touch.rs:100+`) | не обнаружено | нет (только тесты канала `voicechat:secret`, `raw_payloads\raw_channel_tests.rs:9`) | игровая фича | L | Движок платформенно-нейтрален; аудио-бэкенд iOS/Android отдельный |
| Файловые провайдеры / SAF | Documents виден в Files.app; `ios_picker.rs`, `ios_url.rs` (н/д); папки-кнопки открывают Files | `android\documents.rs`, `document_store.rs`, `document_provider.rs`, `document_bridge.rs` (н/д; `content://com.goydevv.horizon.documents`, README:64), мини-файлменеджер, import/export zip | нет | платформа | L | Android SAF — самый объёмный кусок; iOS — почти бесплатно (Documents) |
| IME / экранная клавиатура | `horizon-threads\ios_keyboard.rs` (н/д), чат над клавиатурой (CHANGELOG 2026.11.12) | `android\native_ime.rs` (867: системный EditText на UI-looper, JNI+UnixStream), `osk.rs` (н/д), `set_native_content_insets` (:688) | нет | платформа | M | iOS — системный UITextField; Android —.stock EditText без Java-классов проекта |
| Буфер обмена | `ios_pasteboard.rs` (н/д) | через `horizon-client-input` | **Есть** android-cfg: `MAIN\...\clipboard.rs:155` | платформа | S | iOS-бэкенд добавить |
| Ресурсы без компьютера (загрузка на устройстве) | `game\ios_setup.rs` (374): tables-релиз + jar Mojang, 8 потоков, resume, sha1/sha256 | лаунчерные загрузки переиспользуются (`browser.rs`, `game_launch.rs` через `#[path]`); `--public` качает на телефоне (CONTRIBUTING_ANDROID) | подготовка только на ПК (`horizon-prepare`) | платформа | M | Таблицы, требующие JVM, публикуются релизом — ключевой трюк iOS |
| Оптимизации: PowerVR→GLES, GLES-quirks, fast chunk upload, culling-дефолты | culling/кэши по умолчанию (CHANGELOG 2026.11.13), mimalloc | `gfx.rs:1363-1380` (is_powervr), :1597-1599 (Vulkan→GLES fallback), :4930 (base-vertex rebase), :2656,:2723 (GLES HUD/uniforms), `gfx\frame_scale.rs:59` | gles-fallback есть (`horizon-client\Cargo.toml:127-130`), `gpuCulling`/`parallelTick` (`graphics.rs:1237`), `dynamic_resolution.rs`; PowerVR-детекта нет | правка ядра | M | Переносится в `gfx.rs` MAIN точечно |
| Safe Area / keep awake / фото тайтла | `ios_extras.rs` (340, `ios-settings.txt`: SafeArea, Opacity, HideStick, TitlePhoto, LookSpeed, KeepAwake…), `TitleBackground.swift` | `native_content_rect` (`gfx.rs:6071`, `hdr.rs:561`), insets IME | нет | платформа | S | Extra Settings — отдельная страница Options, не трогает `options.txt` |
| Скины/гардероб в лаунчере | `swift\Skins.swift`, `SkinModel3D.swift`; Ely-скины `auth.rs:559`, `skins.rs:109` | `android\wardrobe.rs` (н/д) | лаунчерный skin changer | платформа | M | Ely-текстуры — часть правки ядра auth/skins |
| Perf-отчёты / диагностика / DNS | Send logs (лаунчер) | `android\performance.rs` (318, opt-in, 8 МБ/1800 с), `diagnostics.rs`, `dns.rs`, `device_runtime.rs`, `device_preferences.rs` (н/д) | `horizon-perf`, `horizon-telemetry` | платформа | S | Android-специфика (DNS обход, device quirks) |

---

## 2. Вход и тач: iOS vs Android, единый дизайн для MAIN

### 2.1. Как авторизован MAIN сейчас
- **Лаунчер** `MAIN\launcher\src\account.rs` (2586): OAuth device-code → XBL → XSTS → Minecraft services, проверка владения игрой (owner gate, :10-13), профиль. Секреты — в keychain ОС (`SERVICE = "dev.playhorizon.launcher"`, :27) или приватном файле; `accounts.json` — только имена/UUID. Azure client id — конфигурация `HORIZON_MSA_CLIENT_ID` (:44-50). Игра получает сессию **файлом** `--session-file` (:6-8), подмодуль `account\session.rs`.
- **Игра** `MAIN\crates\horizon-client-render\src\auth.rs` (953): `AuthSession{name,uuid,xuid,access_token}` (:18-23), `parse` принимает только `user_type:"msa"` (:75-77), файл старше 120 с отвергается (:15-16), `joinServer` через discovery-документ (:1-5). Токен только в памяти. **Ely.by нет, входа в игре нет.**

### 2.2. iOS
- Флоу: `ios_boot.rs` (1303) поднимает event loop сразу с собственным экраном `ios_screen.rs` (422, рисуется бандл-шрифтами до наличия ресурсов), вход и распаковка ресурсов идут на параллельных потоках, игра стартует внутри уже работающего loop (`ios_boot.rs:1-7`).
- `ios_account.rs` (523): тот же device-code, но client id Cazza зашит (:20); **плюс Ely.by** — `ELY_AUTH_URL/ELY_REFRESH_URL` (:31-32), `Signed{ely, client_token,…}` (:60-70), `session_json()` отдаёт `user_type:"elyby"` (:74-80). Хранение: только MS refresh-token + имя/uuid в `Library/Application Support/horizon/account.json` (не в Documents, :10-12); MC access-token в памяти.
- Ядро IOS уже расширено: `horizon-client-render\src\auth.rs:16-22` (`ELY_JOIN_URL`, `ELY_PROFILE_URL`, `ELY_SKIN_URL`, `ELY_USER_TYPE`), enum-сессия MS/Ely (:31,65), Ely join через свой session-сервер, чат без подписи (:503), свои текстуры (:559); `skins.rs:27-38` — allowlist URI Ely.by.
- Лаунчер (SwiftUI) показывает аккаунты/скины/профили; мост `ios_launcher.rs` (73) передаёт действия строками.

### 2.3. Android
- `android_main.rs:17-18` включает **лаунчерный** `account.rs` прямо в игровой процесс (`#[path]`), рядом `browser.rs`, `proxy.rs`. Т.е. вся машинерия device-code/токенов/обновления — общая с десктопом.
- UI: `android\login.rs` (951) — нативная панель на `ui_theme`: выбор провайдера, сохранённые аккаунты, offline, Ely (username/password/**otp**), `auto_open_device` (:27-44).
- Ely: `launcher\src\account\ely.rs` (311) — Yggdrasil authenticate/refresh, 2FA-сообщение (:18-20), проверка `clientToken` (:44-49), `Secrets{refresh_token=clientToken, minecraft_token, expires=3600}` (:55-66). Токены — в `android\secure_store.rs`.
- В `auth.rs` самого AND Ely **нет** (grep 0) — Ely-сессия живёт на уровне лаунчерного аккаунта; при переносе на MAIN ядро всё равно надо расширить по образцу IOS.

### 2.4. Единый дизайн входа (куда встаёт Ely.by)
1. **Ядро сессии**: расширить `MAIN\crates\horizon-client-render\src\auth.rs` до enum `{Microsoft, Ely}` — почти дословный перенос IOS `auth.rs:16-22,31,65,91,256,503,559` + allowlist в `skins.rs`. `user_type:"elyby"` в session-файле; `parse` принимает оба.
2. **Общий крейтflows**: вынести device-code/XBL/XSTS/MC + Ely Yggdrasil из `launcher\src\account.rs` в общий крейт (новый `crates\horizon-account` либо `pub mod account` в `horizon-client-render`; лаунчер и игра зависят от него). Android уже доказал работоспособность переиспользования (`#[path]`), но общий крейт чище. `Endpoints`-моки MAIN (`account.rs:53-60`) сохранить для тестов.
3. **Хранение**: trait `SecretStore` с тремя бэкендами: keychain (ПК, уже есть), приватный `account.json` (iOS, образец `ios_account.rs:10-12`), `secure_store` (Android). В игре — только имя/uuid/refresh, access-token в памяти.
4. **UI в игре**: экран входа на `hud`/`ui_theme` MAIN по образцу `ios_screen.rs` (рисуется до ресурсов) + панель провайдеров по образцу `login.rs` (Microsoft / Ely.by / offline). SwiftUI-лаунчер iOS остаётся платформенной надстройкой, не источником истины.
5. Ely.by при этом: вход — в общем крейте (authenticate/refresh/2FA из `ely.rs`), join/скины/чат — в `auth.rs`/`skins.rs` ядра (из IOS). Владение игрой (owner gate) проверять только для MS; Ely — без Mojang-сервисов (`IOS auth.rs:256`).

### 2.5. Тач: форматы и флоу
| | iOS | Android | MAIN (Starling-база) |
|---|---|---|---|
| Файл | `touch-controls.json` в папке игры (`touch.rs:194`) | `horizon-touch.json` (`layout.rs:709,943`) + пресеты `controls/<style>/<id>.json`, защищённый `default.json` (`layout.rs:976-1103`) | `Settings` serde (`touch\layout.rs`), файл настроек Starling |
| Структура | `{active, layouts:[{buttons:[{id,x,y,size}], stick:[x,y,w,h], esc:[x,y], esc_scale, opacity}\|null ×3]}` (`touch.rs:135-194,264-283`); центр кнопки в долях экрана, размер — доля короткой стороны; миграция старого одно-раскладочного файла (:245-262) | `Mode{Crosshair,Tap,Dpad}` + `split_controls`, 14 фикс. `Control`, `Placement`, `ControlStyle{MCPE,GoyControl}`, `JoystickStyle/MoveArea`, `PresetMeta` (`layout.rs:6-126`), `CustomButton{key,…}` serde (`custom.rs:7-8`); размеры — доля высоты | те же `Mode/Control/Placement/Settings` (kebab-case serde, `layout.rs:16-80`), 4 схемы, раскладка на каждый Mode; **без** кастом-кнопок/пресетов/редактора |
| Действия | строковые id ~40 действий: `Key::Pad(Button)`, `Trigger(0/1)`, `Keys(&["f3","g"])` (`touch.rs:34-100`), включая vc.* | фикс. `Control` + произвольная клавиша `CustomButton.key` | `Control` enum + `keypad` |
| Редактор | `touch_editor.rs` (597): 9 инструментов, Sel Button/Stick/Esc, размеры 0.06–0.30 | `editor.rs` (975): Move/Resize drag, `PickerStep`; UI-хаб в `touch_ui.rs` (8900) | нет |
| Картинки | `touch-icons/<action>.png` (пикер, 256×256), спрайты пака `gui/sprites/touch/<action>.png`, `stick_knob.png`, `title-background.png` | галерея (`gallery.rs`) | нет |
| particularities | палец-мышь в экранах (tap/swipe/hold-drag/2-й палец=ПКМ), Esc-кнопка, 3 раскладки | виртуальная мышь, гироскоп, floating-меню, слайдеры чувствительности/размера указателя/резolution | роли пальцев Move/Look, hold=добыча, double-tap sneak (`touch.rs:27-50`) |

**Единый дизайн (крейт `horizon-client-render`, модуль `touch`):**
- База — уже лежащие в MAIN `touch.rs` + `touch\layout.rs` (родословная Starling/Android совпадает с AND — диффы минимальны).
- `layout.rs`: добавить (а) свободное размещение кнопок в стиле iOS (`{id,x,y,size}` — поверх `Placement`), (б) `CustomButton` из AND `custom.rs`, (в) реестр действий iOS `Key{Pad,Trigger,Keys}` как `Control::Custom(action_id)`, (г) хранилище пресетов из AND `layout.rs:976-1199` с защищённым default.
- Канонический файл по правилу владельца — **`touch-controls.json`**: `{active, layouts[≤3], presets,…}`; импортёр из `horizon-touch.json` и из старого iOS-формата (образец миграции `IOS touch.rs:245-262`).
- Новый `touch\editor.rs` в MAIN: перенос AND `editor.rs` (975) + инструменты iOS (Picture/No pic/Layout×3/Reset) из `touch_editor.rs` (597). Иконки: загрузка `touch-icons/` и спрайтов пака в `touch\draw.rs` (платформенно-нейтрально), пикер — через платформенный слой (`ios_picker` / `gallery`).
- Экранный «палец-мышь» iOS (жесты в инвентарях/меню) — отдельный подмодуль `touch\screen.rs`, перенос из IOS `game\touch.rs`; в MAIN сейчас этого нет.
- Гироскоп: trait сенсора в `horizon-platform` (MAIN уже имеет крейт-фасад `horizon-platform\src\lib.rs:1-15`) или `horizon-threads`; бэкенд — AND `device_gyro.rs`; применение к камере — в `touch\aim.rs`. Android-доп. (пресеты, gyro, virtual mouse) включать feature-флагами, чтобы не конфликтовать с iOS-паритетом.
- `touch_ui.rs` (8900) переносить **не целиком**: хаб/слайдеры — отдельная опциональная панель поверх единого `Settings`.

---

## 3. Правки общих крейтов MAIN для мобил

**wgpu / GLES / Metal (`horizon-client-render\src\gfx.rs`):**
- В MAIN `order()` (`gfx.rs:12579-12615`) нет веток android/ios (grep `"android"|"ios"` — 0). Добавить: iOS → Metal (feature `metal`, образец `IOS\crates\horizon-client\Cargo.toml:46`); Android → Vulkan, GLES-fallback.
- Из AND `gfx.rs`: детект PowerVR/IMG (`:1363-1380`), принудительный fallback Vulkan→GLES (`:1597-1599`), GLES 3.0 без base-vertex — rebase атрибутов (`:4930`), GLES HUD-рендерер и uniforms (`:2656,:2723`). В MAIN gles-feature уже подключён для не-macOS (`horizon-client\Cargo.toml:127-130`), патч `wgpu-hal`/`naga` уже в `third_party` (`MAIN\Cargo.toml:111-112`).

**winit glue:**
- iOS: стоковый winit 0.30.13 достаточно; атрибуты окна `WindowAttributesExtIOS`/ориентации/`ScreenEdge` (`IOS game.rs:17137`). Вход — «loop сразу, boot-экран, передача app» (`ios_boot.rs:1-7`).
- Android: MAIN уже объявил `winit … android-native-activity` (`Cargo.toml:122-123`), но AND требует **вендоренный патченый winit** (`AND\Cargo.toml:78`): `native_content_rect` (`gfx.rs:6071`, `gfx\hdr.rs:561`, `gfx\frame_scale.rs:59`), `native_device_id`/`native_pointer_device` (`game.rs:16298-16443`), `set_native_gamepads` (`platform.rs:696`), `set_native_content_insets` (`native_ime.rs:688`). Решение: вендорить `third_party/winit` в MAIN + `[patch.crates-io]` (или upstream этих API). Точка входа — `android_main.rs` (292, `android_main(app: AndroidApp)` :201, `OnCreateState` :126), lifecycle/save-recovery.

**Лимиты памяти (`horizon-threads`, `horizon-client`):**
- Перенести `IOS\crates\horizon-threads\src\memory_limit.rs` (462: watcher, GRACE 20 с, `can_shrink`, `os_proc_available_memory` :323-332) — в MAIN модуля нет, `capped.rs:3` ссылается вхолостую. Android-источник давления — через JNI (`device_runtime.rs`) / `/proc` (`MAIN\horizon-threads\process_memory.rs:56,112` уже умеет linux/android).
- Перенести Governor `IOS\crates\horizon-client\src\ram_limit.rs` (505: `HORIZON_MEMORY_LIMIT_MB`, `/ramlimit`, phone_step_up «треть пути» 8→29 ~21 с, URGENT_KEEP 0.6) + оконную часть `game\ram_limit.rs` (103). Хук отчёта в MAIN уже есть: `crash.rs:611-614`.

**Пути и хранение:**
- iOS: папка игры = Documents (видна в Files), секреты = приватный Application Support (`ios_account.rs:10-12`); профили через `logging::set_player_dir` (`ios_instances.rs:4-6`) — наличие этой функции в MAIN проверить при переносе.
- Android: SAF (`document_store/provider/bridge`), `secure_store`, каталоги через `android\platform.rs`.
- `horizon-client-input`: android-cfg уже есть (`keybinds.rs:304,323`, `clipboard.rs:155`); добавить iOS-бэкенды (`ios_pasteboard`, `ios_keyboard`).

**`horizon-world` / `horizon-core`:** в просмотренных слоях платформенных правок не обнаружено — всё мобильное сосредоточено в `horizon-client`, `horizon-client-render`, `horizon-threads`, `horizon-client-input`. (Полный grep `target_os` по этим двум крейтам форков не выполнен — см. пробелы.)

**Прочее:** mimalloc (`third_party\libmimalloc-sys` во всех трёх), `--build-id=sha1` для tombstone (`AND\crates\horizon-android\build.rs:23-26` — крейт-пустышка, только build.rs), packaging (`packaging/ios` + swift, `packaging/android` + `tools/android_build.py`), CI `ios.yml`.

---

## 4. Рекомендуемый порядок переноса

| Этап | Содержание | Объём (оценка) |
|---|---|---|
| 0. Фундамент | Вендор патченого winit в MAIN + patch; ветки Metal/GLES/PowerVR в `gfx.rs`; скелеты входов iOS (boot-loop) и Android (`android_main`); `cargo check` обоих таргетов | 1–2 нед |
| 1. Вход (паритет №1) | Enum-сессия MS/Ely в `auth.rs` + skins-allowlist; общий крейт flows из `launcher\account.rs`; `SecretStore` ×3; экран входа в игре (ios_screen + login.rs); Ely 2FA | 1–2 нед |
| 2. Тач (паритет №2) | На Starling-базу MAIN: кастом-кнопки, editor.rs, `touch-controls.json` ≤3 раскладок + импортёры, иконки/пикер, кнопки-клавиши, экранный палец-мышь iOS | 2–3 нед |
| 3. Память и вылеты | `memory_limit.rs` + Governor `ram_limit` + плавная дальность; `crash_watch` (iOS) + `exit_reports`/tombstone (Android) | ~1 нед |
| 4. Платформенные сервисы | IME/OSK, clipboard, picker/SAF/галерея, keep-awake/safe-area, GameController (iOS) + native gamepads (Android, частично уже в MAIN), загрузка ресурсов на устройстве (`ios_setup`) | 2–3 нед |
| 5. Лаунчер/профили/обновления | Rust-лаунчер в игре (образец AND), профили `ios_instances`, updater ×2, ferrite-signer (iOS-only, последний — самый рискованный) | 2–4 нед |
| 6. Игровые фичи и Android-доп. | Speed Trade (S), инвентарь пальцем/fling, voicechat-engine + аудио-бэкенды, гироскоп/пресеты/virtual mouse (feature-флаги) | 2–3 нед |

Итого ориентировочно **10–16 недель** на одного опытного разработчика; этапы 1–3 дают владельческий паритет, 4–6 — полноту.

**Проверка (end-to-end):**
- iOS: `cargo check --target aarch64-apple-ios -p horizon-client --no-default-features --features audio,system-allocator` (docs/ios.md), сборка `packaging/ios/package.sh` + прогон на устройстве; тесты serde-круга `touch-controls.json` и миграций, моки `Endpoints` для auth.
- Android: `python tools/android_build.py --abi arm64-v8a --clippy` (CONTRIBUTING_ANDROID), тесты ввода/состояния по правилу форка; образец тест-хранилища MAIN — `horizon-client-render\tests\pad_settings_store.rs`.
- Регресс ПК: существующие тесты MAIN (touch-база «ничего не запускает без touch surface», `touch.rs:7-8`).

**Незакрытые пробелы (честно):** не замерены размеры `ios_keyboard/picker/pasteboard/url/display/bundle`, AND `launcher*.rs`, `secure_store`, `documents*`, `osk.rs`, `device_runtime`; точные места реализации inventory-drag/fling (iOS) и точки применения гироскопа к камере (AND) не зафиксированы; отсутствие voice chat в AND не подтверждено полным grep; `horizon-world`/`horizon-core` форков не сканировались на `target_os` целиком; наличие `logging::set_player_dir` в MAIN не проверено. На выводы по архитектуре это не влияет, но перед этапом 2/6 эти места стоит дочитать.
