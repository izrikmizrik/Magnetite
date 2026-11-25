---
name: magnetite-builds
description: Build, install and release Magnetite for Android (APK), iOS (IPA) and PC, and debug the build when it breaks. Use for the APK or IPA pipeline, testing on a phone over adb, the android-apk / ios-ipa / ci workflows, a v* release tag, LuaJIT or NDK build errors, signing, or the build variables the game reads (Microsoft client id, the tables release, the mod index).
---

# Building and releasing Magnetite

Three products from one workspace: PC (`horizon-game`, bins `horizon-client` and `magnetite`), Android
(`crates/magnetite-android`, a cdylib in `packaging/android`) and iOS (`crates/magnetite-ios`, packed by
`packaging/ios/package.sh`). Phones build with the `mobile` profile features and the `fast` Cargo profile.

## Build variables (repository variables, not secrets)

Read at compile time with `option_env!`; a build without them simply lacks the feature.

| Variable | What |
| --- | --- |
| `MAGNETITE_MSA_CLIENT_ID` | the Azure app for Microsoft sign-in |
| `MAGNETITE_TABLES_URL`, `MAGNETITE_TABLES_SHA256` | the `magnetite-tables` release the game downloads with its files (re-release it when `horizon-prepare` or the resource format changes) |
| `MAGNETITE_MOD_INDEX` | optional: another mod index than `izrikmizrik.github.io/magnetite-mods` |

Locally: `export V="$(gh variable get V)"` for each before building. Secrets: the test signing key,
`MAGNETITE_DEBUG_KEYSTORE` (base64 PKCS12, alias `magnetite`) and `MAGNETITE_DEBUG_KEYSTORE_PASSWORD`;
Gradle reads the file path from `MAGNETITE_DEBUG_KEYSTORE_FILE`. Never commit a keystore or its password.

## Android

```sh
# NDK clang wrappers as CC/CXX/AR/linker for aarch64-linux-android (see android-apk.yml)
cargo build --locked -p magnetite-android --target aarch64-linux-android --profile fast
llvm-strip --strip-unneeded -o .local/android/jni/arm64-v8a/libmagnetite_android.so target/aarch64-linux-android/fast/libmagnetite_android.so
gradle -p packaging/android -PversionName=X -PversionCode=N assembleDebug
adb install -r .local/android/gradle-build/outputs/apk/debug/MagnetiteAndroid-debug.apk
```

- JDK 21 for Gradle (25 fails: "Unsupported class file major version 69"); build-tools and platform 36,
  NDK 28.
- A checkout path with non-ASCII characters needs `-Pandroid.overridePathCheck=true`.
- On Windows the build host needs `CFLAGS_x86_64_pc_windows_msvc=-utf-8`; Java's Unix sockets need a
  short `TEMP` (and `-Djdk.net.unixdomain.tmpdir`).
- Wireless adb: `adb mdns services` finds the phone again; its port changes when wireless debugging is
  turned off and on.
- Every APK is signed with the kept test key so it installs over the last one. Another key means
  uninstalling, which loses the player's files: ask first.
- Runtime gotchas already solved, don't undo them: the guest stack size is set in an `.init_array`
  constructor (`RUST_MIN_STACK`, before android-activity's threads), and window flags go through the
  IME's UI-thread queue (`set_window_flags` deadlocks `poll_events`).

## iOS

`cargo build --locked -p magnetite-ios --target aarch64-apple-ios --profile fast` on macOS, then
`packaging/ios/package.sh <binary> <ipa>` (ad hoc signature; SideStore, AltStore or TrollStore re-sign it).
LuaJIT needs `TARGET_SYS=iOS`, and its host tools must build for the Mac: with
`IPHONEOS_DEPLOYMENT_TARGET` set, clang aims them at iOS and macOS kills them (`Killed: 9`), so the
workflow sets `HOST_CFLAGS`/`HOST_LDFLAGS=-mmacosx-version-min=12.0`. The `ci` workflow's iOS job is a
`cargo check` without the deployment target: only `ios-ipa` catches a full-build failure.

- Runtime gotchas already solved, don't undo them: UIKit's main thread runs the boot loop and the
  whole game, and iOS gives it 1 MiB of stack, which the hand-over from the loading screen to the
  game overflowed (the app closed with nothing in the log). `magnetite-ios` moves the thread onto a
  guest-sized `horizon_guest_stack::OwnStack` before `boot::run`; `-Wl,-stack_size` is not relied on.
- The IPA carries `horizon-wasm-std.wasm` beside the executable (`package.sh`'s third argument,
  built by `cargo-horizon-mod wasm-std` in `ios-ipa`): without it every wasm mod is refused.

## PC

`cargo build -p horizon-game --bin magnetite --profile fast` gives the launcher-first executable; it keeps
its files in the platform data folder's `magnetite/` and downloads the game files after sign-in, as a
phone does. On Windows LuaJIT builds with `msvcbuild.bat`: run Cargo from a Visual Studio developer
prompt (`vcvars64.bat`). `msvcbuild.bat` runs `minilua` from the current folder, which fails when
`NoDefaultCurrentDirectoryInExePath` is set: clear it for the build.

## Releases

Push an annotated tag `vX.Y.Z` (`e4mc-vX.Y.Z` for the e4mc channel). `android-apk` and `ios-ipa` run on
it, take the version from the tag, make the tag's pre-release with `packaging/release-notes.md` if it is
not there yet, and attach `Magnetite-<version>-android-arm64.apk` / `-ios-unsigned.ipa`. A failed build
is fixed on `main` and released under a new tag: never move or delete a published tag. Run a workflow by
hand on `main` (`gh workflow run ios-ipa.yml --ref main`) to check a fix first.
