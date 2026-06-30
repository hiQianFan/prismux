#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${1:-"$ROOT/target/menubar/OpenMux.app"}"

if [[ ! -d "$APP_DIR" ]]; then
  echo "bundle not found: $APP_DIR" >&2
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

if strings "$APP_DIR/Contents/MacOS/OpenMux" | rg -n 'TBCore|tb_core_ffi|TokenBar|com\.tokenbar|Sparkle|SUUpdater|appcast'; then
  echo "menubar bundle audit failed: forbidden linked symbol/string found" >&2
  exit 1
fi

echo "Menubar bundle audit passed"
