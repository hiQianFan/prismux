#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${1:-"$ROOT/target/menubar/Prismux.app"}"

if [[ ! -d "$APP_DIR" ]]; then
  echo "bundle not found: $APP_DIR" >&2
  exit 1
fi
EXECUTABLE="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP_DIR/Contents/Info.plist")"
ICON_FILE="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$APP_DIR/Contents/Info.plist")"
for file in "$APP_DIR/Contents/MacOS/$EXECUTABLE" "$APP_DIR/Contents/MacOS/prismux" "$APP_DIR/Contents/MacOS/pmx"; do
  if [[ ! -x "$file" ]]; then
    echo "menubar bundle audit failed: missing executable $file" >&2
    exit 1
  fi
done
if [[ ! -f "$APP_DIR/Contents/Resources/$ICON_FILE" ]]; then
  echo "menubar bundle audit failed: missing icon $APP_DIR/Contents/Resources/$ICON_FILE" >&2
  exit 1
fi

if find "$APP_DIR" -type f | rg -n 'TokenBar|TBCore|tb_core_ffi|tokscale|scanner|pricing|appcast|Sparkle'; then
  echo "menubar bundle audit failed: forbidden resource name found" >&2
  exit 1
fi

if find "$APP_DIR" -type f \( -name '*.png' -o -name '*.gif' -o -name '*.mov' -o -name '*.mp4' -o -name '*.json' \) | rg .; then
  echo "menubar bundle audit failed: unreviewed asset/resource found" >&2
  exit 1
fi

if strings "$APP_DIR/Contents/MacOS/$EXECUTABLE" | rg -n 'TBCore|tb_core_ffi|TokenBar|com\.tokenbar|Sparkle|SUUpdater|appcast'; then
  echo "menubar bundle audit failed: forbidden linked symbol/string found" >&2
  exit 1
fi

echo "Menubar bundle audit passed"
