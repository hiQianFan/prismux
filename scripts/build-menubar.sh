#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# SwiftUI macros (@State etc.) need the full Xcode toolchain; CommandLineTools
# ships no SwiftUIMacros plugin and the build fails on SDK 27. Pin DEVELOPER_DIR
# to a full Xcode unless the caller already set it or the active toolchain is one.
if [ -z "${DEVELOPER_DIR:-}" ] && ! xcode-select -p 2>/dev/null | grep -q "Xcode.*\.app"; then
  xcode_dir=""
  for app in $(ls -d /Applications/Xcode*.app 2>/dev/null | sort -r); do
    # Skip Xcodes.app (the version manager) and anything lacking a toolchain.
    case "$app" in */Xcodes.app) continue;; esac
    if [ -d "$app/Contents/Developer" ]; then xcode_dir="$app"; break; fi
  done
  if [ -z "$xcode_dir" ]; then
    echo "error: full Xcode not found in /Applications — SwiftUI macros need it." >&2
    echo "       install Xcode, or run: sudo xcode-select -s /path/to/Xcode.app" >&2
    exit 1
  fi
  export DEVELOPER_DIR="$xcode_dir/Contents/Developer"
  echo "using $DEVELOPER_DIR"
fi

cargo build --release -p omx-menubar-ffi
swift build --package-path apps/omx-menubar -c release
swift run --package-path apps/omx-menubar -c release OmxMenubarContractTests
"$ROOT/scripts/check-menubar-privacy.sh"
