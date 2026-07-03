# Release Guide

[简体中文](RELEASE.zh-CN.md)

Prismux v0.1 uses release-on-version-bump automation. Maintainers do not create
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
cargo build --release -p prismux-cli --locked
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

The Menubar build requires a full Xcode installation. Command Line Tools alone
do not provide the SwiftUI macro toolchain used by the app.

Run safe smoke tests with isolated state:

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- status
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list codex
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list claude
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- doctor
```

Do not run real `login`, `use`, or credential-switching smoke tests against a
real tool home unless that is the explicit intent.

## macOS Artifacts

- `Prismux-vX.Y.Z-macos-arm64.zip`, containing `Prismux.app`
- `prismux-cli-vX.Y.Z-macos-arm64.tar.gz`, containing standalone `prismux` and
  `pmx` commands plus `install.sh`
- bundled CLI helper at `Prismux.app/Contents/SharedSupport/bin/prismux`
- `SHA256SUMS`

The macOS app bundle is the preferred UI distribution path. It includes Menubar
and the same-version CLI helper. Users who want Terminal access can either
install a symlink from a PATH directory to the bundled helper or download the
standalone CLI package. The release does not copy auth/state files and does not
modify shell startup files.

First public app bundles do not publish Linux binaries, Windows binaries,
Homebrew formulae, crates.io packages, Sparkle updates, Developer ID
notarization automation, independent signatures, or provenance attestations.

## Bundle Layout

```text
Prismux.app/
  Contents/
    MacOS/
      Prismux
      prismux
    Resources/
      ...
```

`Contents/SharedSupport/bin/prismux` is executable code, not a resource. Release validation must
check:

- `Prismux.app` has `LSUIElement=true` and `LSMinimumSystemVersion=14.0`.
- `CFBundleShortVersionString` matches the Cargo workspace version.
- `Prismux.app/Contents/SharedSupport/bin/prismux --version` reports the same version.
- bundled `prismux status` passes with isolated `PRISMUX_STATE_ROOT`, `CODEX_HOME`, and
  `CLAUDE_CONFIG_DIR`.
- unpacking the release zip preserves `Contents/MacOS/Prismux` and
  `Contents/SharedSupport/bin/prismux` as executable files.
- bundle privacy/audit scripts do not find raw auth, tokens, API keys, raw logs,
  or excluded third-party engines.

## Source Builds

GitHub's generated source archive is usable for development. `cargo install
--git https://github.com/hiQianFan/prismux -p prismux-cli --locked` installs only the
CLI. Building the full Menubar app from source requires macOS with full Xcode:

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

See [Build from source](BUILD.md) for the longer path.

## Artifact Capability Scope

| Artifact | Capabilities | Platform | Notes |
| --- | --- | --- | --- |
| `Prismux.app` full bundle | Menubar dashboard, refresh, explicit account/profile activation, onboarding actions, bundled `prismux` helper, shared state root | macOS 14+ | Preferred public macOS artifact. |
| `prismux-cli-vX.Y.Z-macos-arm64.tar.gz` | CLI-only account/profile management and scripts | macOS arm64 | Includes `prismux`, `pmx`, `install.sh`, and package README. |
| Windows/Linux packages | Platform-specific CLI/app packaging | Later | Separate proposals; do not reuse macOS `.app` layout. |

## Rollback

If a release workflow fails before creating a tag, fix the PR or workflow and
merge another change.

If a release is created with bad artifacts, delete the release artifacts, publish
a patch version, and document the issue in `CHANGELOG.md`.
