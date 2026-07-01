#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${1:-"$ROOT/target/menubar/Prismux.app"}"
VERSION="$(awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' "$ROOT/Cargo.toml")"

if [[ -d "$APP_DIR" ]]; then
  APP_EXEC="$APP_DIR/Contents/MacOS/$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP_DIR/Contents/Info.plist")"
  HELPER="$APP_DIR/Contents/MacOS/prismux"

  if [[ ! -x "$APP_EXEC" ]]; then
    echo "bundle executable missing or not executable: $APP_EXEC" >&2
    exit 1
  fi

  if [[ ! -x "$HELPER" ]]; then
    echo "bundled prismux helper missing or not executable: $HELPER" >&2
    exit 1
  fi

  BUNDLE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_DIR/Contents/Info.plist")"
  if [[ "$BUNDLE_VERSION" != "$VERSION" ]]; then
    echo "bundle version mismatch: Cargo=$VERSION bundle=$BUNDLE_VERSION" >&2
    exit 1
  fi

  CLI_VERSION="$("$HELPER" --version | awk '{print $2}')"
else
  CLI_VERSION="$(cargo run -q -p prismux-cli -- --version | awk '{print $2}')"
fi

if [[ "$CLI_VERSION" != "$VERSION" ]]; then
  echo "prismux --version mismatch: Cargo=$VERSION CLI=$CLI_VERSION" >&2
  exit 1
fi

if [[ -n "${RELEASE_TAG:-}" && "${RELEASE_TAG#v}" != "$VERSION" ]]; then
  echo "release tag mismatch: Cargo=$VERSION tag=$RELEASE_TAG" >&2
  exit 1
fi

echo "Prismux version consistent: $VERSION"
