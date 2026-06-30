#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-"$ROOT/target"}"

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
export HOME="${OMUX_SWIFT_HOME:-"$ROOT/target/swift-home"}"
export XDG_CACHE_HOME="${XDG_CACHE_HOME:-"$ROOT/target/swift-cache"}"
export CLANG_MODULE_CACHE_PATH="${CLANG_MODULE_CACHE_PATH:-"$ROOT/target/swift-module-cache"}"
SWIFT_CONFIG_PATH="${OMUX_SWIFT_CONFIG_PATH:-"$ROOT/target/swift-config"}"
SWIFT_SECURITY_PATH="${OMUX_SWIFT_SECURITY_PATH:-"$ROOT/target/swift-security"}"
mkdir -p "$HOME" "$XDG_CACHE_HOME" "$CLANG_MODULE_CACHE_PATH" "$SWIFT_CONFIG_PATH" "$SWIFT_SECURITY_PATH"
swift build \
  --package-path apps/omx-menubar \
  --cache-path "$XDG_CACHE_HOME" \
  --config-path "$SWIFT_CONFIG_PATH" \
  --security-path "$SWIFT_SECURITY_PATH" \
  --disable-sandbox \
  -Xswiftc -disable-sandbox \
  -debug-info-format none \
  -c release
swift run \
  --package-path apps/omx-menubar \
  --cache-path "$XDG_CACHE_HOME" \
  --config-path "$SWIFT_CONFIG_PATH" \
  --security-path "$SWIFT_SECURITY_PATH" \
  --disable-sandbox \
  -Xswiftc -disable-sandbox \
  -debug-info-format none \
  -c release \
  OmxMenubarContractTests
"$ROOT/scripts/check-menubar-privacy.sh"
