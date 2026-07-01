#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-"$ROOT/target"}"

VERSION="$(awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.toml)"
APP_DIR="$ROOT/target/menubar/Prismux.app"
CONTENTS="$APP_DIR/Contents"
MACOS="$CONTENTS/MacOS"
RESOURCES="$CONTENTS/Resources"
APP_EXECUTABLE="PrismuxMenubar"
APP_ICON="Prismux.icns"

cargo build --release -p prismux-cli
"$ROOT/scripts/build-menubar.sh"

rm -rf "$APP_DIR"
mkdir -p "$MACOS"
mkdir -p "$RESOURCES"
cp "$ROOT/apps/prismux-menubar/.build/release/PrismuxMenubarApp" "$MACOS/$APP_EXECUTABLE"
cp "$CARGO_TARGET_DIR/release/prismux" "$MACOS/prismux"
cp "$CARGO_TARGET_DIR/release/pmx" "$MACOS/pmx"
cp "$ROOT/assets/prismux-icon/prismux-mac-icon.icns" "$RESOURCES/$APP_ICON"
for products_dir in \
  "$ROOT/apps/prismux-menubar/.build/release" \
  "$ROOT/apps/prismux-menubar/.build/"*-apple-macos*/release \
  "$ROOT/apps/prismux-menubar/.build/out/Products/Release"; do
  if [[ -d "$products_dir" ]]; then
    find "$products_dir" -maxdepth 1 -name '*PrismuxMenubarCore.bundle' -exec cp -R {} "$RESOURCES/" \;
  fi
done
if ! find "$RESOURCES" -maxdepth 1 -name '*.bundle' | rg -q .; then
  echo "error: Swift resource bundle not found; Prismux.app would exit on launch." >&2
  exit 1
fi

cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$APP_EXECUTABLE</string>
  <key>CFBundleIdentifier</key>
  <string>dev.prismux.menubar</string>
  <key>CFBundleName</key>
  <string>Prismux</string>
  <key>CFBundleIconFile</key>
  <string>$APP_ICON</string>
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

codesign --force --sign - "$MACOS/$APP_EXECUTABLE"
codesign --force --sign - "$MACOS/prismux"
codesign --force --sign - "$MACOS/pmx"
codesign --force --sign - "$APP_DIR"
codesign --verify "$APP_DIR"
"$ROOT/scripts/check-menubar-version.sh" "$APP_DIR"
"$ROOT/scripts/check-menubar-privacy.sh"
"$ROOT/scripts/audit-menubar-bundle.sh" "$APP_DIR"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP_DIR"
echo "$APP_DIR"
