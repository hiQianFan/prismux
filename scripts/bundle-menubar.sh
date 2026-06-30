#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="$(awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.toml)"
APP_DIR="$ROOT/target/menubar/OpenMux.app"
CONTENTS="$APP_DIR/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"

cargo build --release -p omx-cli
"$ROOT/scripts/build-menubar.sh"

rm -rf "$APP_DIR"
mkdir -p "$MACOS"
mkdir -p "$RESOURCES"
cp "$ROOT/apps/omx-menubar/.build/release/OmxMenubarApp" "$MACOS/OpenMux"
cp "$ROOT/target/release/omx" "$MACOS/omx"
find "$ROOT/apps/omx-menubar/.build/out/Products/Release" -maxdepth 1 -name '*.bundle' -exec cp -R {} "$RESOURCES/" \;

cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>OpenMux</string>
  <key>CFBundleIdentifier</key>
  <string>dev.openmux.menubar</string>
  <key>CFBundleName</key>
  <string>OpenMux</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

codesign --force --sign - "$MACOS/OpenMux"
codesign --force --sign - "$MACOS/omx"
codesign --force --sign - "$APP_DIR"
codesign --verify "$APP_DIR"
"$ROOT/scripts/check-menubar-version.sh" "$APP_DIR"
"$ROOT/scripts/check-menubar-privacy.sh"
"$ROOT/scripts/audit-menubar-bundle.sh" "$APP_DIR"
echo "$APP_DIR"
