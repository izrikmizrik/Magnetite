Предварительный выпуск Magnetite для телефонов: Android и iOS. Сборки для ПК — в следующих выпусках.
A preview release of Magnetite for phones: Android and iOS. PC builds come in later releases.

## Что внутри / What's inside

- Вход через Microsoft (по коду), Ely.by или без входа; при первом запуске игра сама скачивает
  свои файлы (около 530 МБ).
  Sign in with Microsoft (a code), Ely.by or offline; the first start downloads the game's files
  (about 530 MB).
- Лаунчер на телефоне: аккаунты, профили со своими настройками и раскладкой, выбор графики
  (Vulkan / OpenGL ES), разрешения, FPS, чувствительности и языка, моды с площадки Magnetite.
  The phone launcher: accounts, profiles with their own settings and touch layout, graphics
  (Vulkan / OpenGL ES), resolution, FPS, sensitivity and language, mods from Magnetite's platform.
- Сенсорное управление с редактором раскладок, системная клавиатура и буфер обмена.
  Touch controls with a layout editor, the system keyboard and the clipboard.
- Моды: контентные, WebAssembly и Lua. Гайд: `docs/magnetite/modding.md` (English:
  `docs/magnetite/modding.en.md`).
  Mods: content, WebAssembly and Lua. The guide: `docs/magnetite/modding.en.md`.

## Установка / Installing

- **Android** (`Magnetite-*-android-arm64.apk`, Android 9+, arm64): разрешите установку из
  неизвестных источников и откройте файл. Сборка подписана тестовым ключом проекта: следующие
  сборки ставятся поверх.
  Allow installs from unknown sources and open the file. The build is signed with the project's
  test key; later builds install over it.
- **iOS** (`Magnetite-*-ios-unsigned.ipa`, iOS 16+): ставится через SideStore, AltStore или
  TrollStore. Срок подписи показан в лаунчере.
  Install through SideStore, AltStore or TrollStore. The launcher shows when the signature expires.

Это ранняя версия: возможны вылеты. Журнал игры можно отправить из лаунчера («Поделиться
журналом»).
This is an early version and may crash. The launcher's "Share the log" sends the game's log.
