#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${1:-"$ROOT/target/menubar/OpenMux Menubar.app"}"
VERSION="$(awk '/^version = / { gsub(/"/, "", $3); print $3; exit }' "$ROOT/Cargo.toml")"

CLI_VERSION="$(cargo run -q -p omx-cli -- --version | awk '{print $2}')"
if [[ "$CLI_VERSION" != "$VERSION" ]]; then
  echo "omx --version mismatch: Cargo=$VERSION CLI=$CLI_VERSION" >&2
  exit 1
fi

if [[ -d "$APP_DIR" ]]; then
  BUNDLE_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_DIR/Contents/Info.plist")"
  if [[ "$BUNDLE_VERSION" != "$VERSION" ]]; then
    echo "bundle version mismatch: Cargo=$VERSION bundle=$BUNDLE_VERSION" >&2
    exit 1
  fi
fi

if [[ -n "${RELEASE_TAG:-}" && "${RELEASE_TAG#v}" != "$VERSION" ]]; then
  echo "release tag mismatch: Cargo=$VERSION tag=$RELEASE_TAG" >&2
  exit 1
fi

echo "OpenMux version consistent: $VERSION"

