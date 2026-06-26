#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release -p omx-menubar-ffi
swift build --package-path apps/omx-menubar -c release
swift run --package-path apps/omx-menubar -c release OmxMenubarContractTests
"$ROOT/scripts/check-menubar-privacy.sh"
