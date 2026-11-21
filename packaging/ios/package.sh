#!/usr/bin/env bash
# Wraps Magnetite iOS's executable in Magnetite.app and zips it as an .ipa with an ad hoc
# signature: SideStore and AltStore re-sign it with the player's own Apple ID, TrollStore installs
# it as is. The .ipa holds no Minecraft file (AGENTS.md); the game's prepared files come later.
# Usage: packaging/ios/package.sh <path to the magnetite executable> <output .ipa>
set -euo pipefail
binary="$1"
out="$2"
root="$(cd "$(dirname "$0")/../.." && pwd)"
work="$(mktemp -d)"
app="$work/Payload/Magnetite.app"
mkdir -p "$app"
cp "$binary" "$app/magnetite"
chmod 755 "$app/magnetite"
cp "$root/packaging/ios/Info.plist" "$app/Info.plist"
# Each build its own version, so AltStore and SideStore see a new one as an update.
if [ -n "${MAGNETITE_IOS_VERSION:-}" ]; then
  plutil -replace CFBundleShortVersionString -string "$MAGNETITE_IOS_VERSION" "$app/Info.plist"
fi
if [ -n "${MAGNETITE_IOS_BUILD:-}" ]; then
  plutil -replace CFBundleVersion -string "$MAGNETITE_IOS_BUILD" "$app/Info.plist"
fi
printf 'APPL????' > "$app/PkgInfo"
cp "$root/LICENSE" "$root/THIRD_PARTY_NOTICES.md" "$app/"
plutil -lint "$app/Info.plist"
codesign --force --sign - --timestamp=none "$app" || true
mkdir -p "$(dirname "$out")"
out="$(cd "$(dirname "$out")" && pwd)/$(basename "$out")"
rm -f "$out"
(cd "$work" && zip -qry "$out" Payload)
ls -lh "$out"
file "$app/magnetite"
