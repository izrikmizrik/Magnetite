# Этап 2: тач как в iOS (Android и iOS, один код)

## Что есть сейчас (разведка 2026-11-22)

- В MAIN лежит тач-база Starling: `horizon-client-render/src/touch.rs` (`TouchControls` → `PadState`),
  `touch/{layout,aim,draw}.rs`, `keypad.rs`. **К игре она не подключена**: в `horizon-client` нет ни одного
  `WindowEvent::Touch`, кроме экрана входа (`game/runtime/boot.rs`). Значит, текущий APK/IPA проходит вход и
  загрузку файлов, но в самой игре (меню, мир) пальцы ничего не делают.
- iOS-форк (эталон владельца): `horizon-client/src/game/touch.rs` (1915), `touch_editor.rs` (596),
  `horizon-client-render/src/touch_icons.rs` (502). Модель простая и проверена на устройстве: касание →
  `touch::Out` → синтетические `WindowEvent` (мышь, колесо, клавиши) и пад (`PadState`, `pad.rs` сливает
  его как ещё один геймпад). Логика — данные на входе и выходе, тестируется без окна.
- Android-форк: `touch.rs` (3022) + `touch_ui.rs` (8900), `editor.rs` (975), `custom.rs` (787), пресеты
  `controls/<style>/<id>.json`, гироскоп `android/device_gyro.rs` (212).

## Решение

База — **модель iOS-форка** (её владелец взял за эталон), перенесённая в MAIN без `cfg(target_os)` в логике:
1. `game/touch.rs` из iOS: экран — палец-мышь (тап = клик, вертикальный свайп = скролл, протяжка/удержание =
   перетаскивание, второй палец = ПКМ); мир — плавающий стик слева, кнопки = пад, остальное — камера.
   Включается, когда пришло первое касание (ПК с сенсорным экраном тоже работает) или по настройке.
2. `game/touch_editor.rs` из iOS: Add/Remove/Smaller/Bigger/Picture/No pic/Layout×3/Reset/Done,
   `touch-controls.json` в папке игры: `{active, layouts[≤3]}` + миграция старого формата.
3. `touch_icons.rs` из iOS: `touch-icons/<action>.png` и спрайты пака `gui/sprites/touch/<action>.png`.
   Выбор картинки — платформенный хук (как `open_link`): iOS — UIImagePickerController, Android — Intent
   `ACTION_GET_CONTENT`; ПК — диалог файла.
4. Кнопки-клавиши: `Key::Keys(&["f3","g"])` из iOS + любые keybind'ы, включая клавиши модов.
5. Starling-база MAIN (`touch.rs`, `layout.rs`, `aim.rs`) — что из неё полезно (прицел по тапу,
   `item_uses_on_tap`, `hotbar_slot`), используется из нового слоя; остальное, если дублирует iOS, удаляется
   после переноса (с проверкой тестов).
6. **Касания для модов** (решение владельца, `plan.md` → «Площадка модов»): общий слой касаний отдаёт событие
   сначала экрану мода, потом HUD модов, потом тач-управлению игры; мод может забрать касание. API — по шагу 7 ТЗ;
   на этом этапе — внутренняя точка маршрутизации и событие в каталоге хуков.
7. Только Android, следом: гироскоп (`device_gyro.rs` → камера, слайдер чувствительности) и пресеты раскладок
   Android-форка как надмножество трёх раскладок iOS.

## Порядок работ

| # | Что | Откуда | Кому |
|---|---|---|---|
| 2.1 | Подключить касания к игре: `touch.rs` iOS, палец-мышь в экранах, стик/камера/кнопки в мире | iOS `game/touch.rs`, `game.rs:17222,21243` | Qwen 4 max (сложная склейка с `Game`) |
| 2.2 | Редактор и `touch-controls.json` (≤3 раскладки) | iOS `touch_editor.rs` | DeepSeek / Mistral Large |
| 2.3 | Иконки кнопок + хук выбора картинки на Android/iOS | iOS `touch_icons.rs`, `ImagePicker.swift` | DeepSeek |
| 2.4 | Маршрутизация касаний для модов | новое | я + Qwen 4 max |
| 2.5 | Гироскоп и пресеты Android | AND `device_gyro.rs`, `layout.rs` | DeepSeek |

Строки интерфейса — ключи `horizon.touch.*` в `en_us.json`/`ru_ru.json` (правило AGENTS.md: без захардкоженных строк).
