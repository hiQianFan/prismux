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
```

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

## v0.1 Artifacts

- macOS Apple Silicon archive
- macOS Intel archive
- `SHA256SUMS`

v0.1 does not publish Linux binaries, Windows binaries, Homebrew formulae,
crates.io packages, independent signatures, or provenance attestations.

## Rollback

If a release workflow fails before creating a tag, fix the PR or workflow and
merge another change.

If a release is created with bad artifacts, delete the release artifacts, publish
a patch version, and document the issue in `CHANGELOG.md`.

