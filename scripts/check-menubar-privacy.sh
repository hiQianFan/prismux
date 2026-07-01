#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if rg -n 'access_token|refresh_token|api_key|authorization:|bearer |sk-|raw auth|raw log|raw response' \
  apps/prismux-menubar/Sources crates/prismux-menubar-ffi/fixtures; then
  echo "menubar privacy audit failed: sensitive marker found" >&2
  exit 1
fi

if rg -n 'TBCore|tb_core_ffi|TokenBar|tokscale|scanner|pricing|quota fetcher|com\.tokenbar|TBUserDefaults|tokenbar' \
  apps/prismux-menubar/Sources apps/prismux-menubar/Package.swift; then
  echo "menubar TokenBar/data-engine audit failed" >&2
  exit 1
fi

echo "Menubar privacy and TokenBar audit passed"
