# Release Guide

[简体中文](RELEASE.zh-CN.md)

OpenMux v0.1 uses release-on-version-bump automation. Maintainers do not create
release tags manually in the normal path.

## Normal Release Flow

1. Open a release PR from a short-lived branch.
2. Update the workspace version in `Cargo.toml`.
3. Promote `CHANGELOG.md` notes from `## Unreleased` to:

   ```md
   ## vX.Y.Z - YYYY-MM-DD
   ```

4. Leave a fresh `## Unreleased` section.
5. Merge the PR after CI passes.
6. The release workflow detects that `vX.Y.Z` does not exist, extracts the
   matching changelog section, creates the tag, builds macOS artifacts, runs
   self-tests, generates checksums, and creates the GitHub Release.

## Release Preconditions

Run locally before merging a release PR:

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release -p omx-cli --locked
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

The Menubar build requires a full Xcode installation. Command Line Tools alone
do not provide the SwiftUI macro toolchain used by the app.

Run safe smoke tests with isolated state:

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- status
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list codex
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list claude
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- doctor
```

Do not run real `login`, `use`, or credential-switching smoke tests against a
real tool home unless that is the explicit intent.

## macOS Artifacts

- `OpenMux-vX.Y.Z-macos-arm64.zip` and `OpenMux-vX.Y.Z-macos-x86_64.zip`
  archives, each containing `OpenMux.app`
- bundled CLI helper at `OpenMux.app/Contents/MacOS/omx`
- `SHA256SUMS`

The macOS app bundle is the preferred distribution path. It includes Menubar and
the same-version CLI helper. Users who want Terminal access install a symlink
from a PATH directory to the bundled helper; the release does not copy auth/state
files and does not modify shell startup files.

First public app bundles do not publish Linux binaries, Windows binaries,
Homebrew formulae, crates.io packages, Sparkle updates, Developer ID
notarization automation, independent signatures, or provenance attestations.

## Bundle Layout

```text
OpenMux.app/
  Contents/
    MacOS/
      OpenMux
      omx
    Resources/
      ...
```

`Contents/MacOS/omx` is executable code, not a resource. Release validation must
check:

- `OpenMux.app` has `LSUIElement=true` and `LSMinimumSystemVersion=14.0`.
- `CFBundleShortVersionString` matches the Cargo workspace version.
- `OpenMux.app/Contents/MacOS/omx --version` reports the same version.
- bundled `omx status` passes with isolated `OMUX_STATE_ROOT`, `CODEX_HOME`, and
  `CLAUDE_CONFIG_DIR`.
- unpacking the release zip preserves `Contents/MacOS/OpenMux` and
  `Contents/MacOS/omx` as executable files.
- bundle privacy/audit scripts do not find raw auth, tokens, API keys, raw logs,
  or excluded third-party engines.

## Source Builds

GitHub's generated source archive is usable for development. `cargo install
--git https://github.com/hiQianFan/openmux -p omx-cli --locked` installs only the
CLI. Building the full Menubar app from source requires macOS with full Xcode:

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

See [Build from source](BUILD.md) for the longer path.

## Artifact Capability Scope

| Artifact | Capabilities | Platform | Notes |
| --- | --- | --- | --- |
| `OpenMux.app` full bundle | Menubar dashboard, refresh, explicit account/profile activation, onboarding actions, bundled `omx` helper, shared state root | macOS 14+ | Preferred public macOS artifact. |
| Standalone CLI tarball | CLI-only account/profile management and scripts | Later | Add only when there is real standalone demand. |
| Windows/Linux packages | Platform-specific CLI/app packaging | Later | Separate proposals; do not reuse macOS `.app` layout. |

## Rollback

If a release workflow fails before creating a tag, fix the PR or workflow and
merge another change.

If a release is created with bad artifacts, delete the release artifacts, publish
a patch version, and document the issue in `CHANGELOG.md`.
