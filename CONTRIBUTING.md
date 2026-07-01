# Contributing

Prismux is an early-stage Rust CLI and macOS Menubar app. Keep changes small,
reviewable, and explicit about credential safety.

## Branch Model

Prismux uses GitHub Flow:

- `main` is the only long-lived branch.
- Create short-lived branches from `main`.
- Open pull requests back into `main`.
- Delete feature branches after merge.

Recommended branch names:

```text
feature/<description>
bugfix/<description>
hotfix/<description>
refactor/<description>
docs/<description>
chore/<description>
```

Examples:

```text
docs/prepare-github-launch
chore/release-workflow
bugfix/claude-account-rollback
feature/gemini-plugin
```

`main` should be protected in GitHub:

- require pull requests before merge
- require CI status checks
- block force pushes
- block branch deletion
- prefer squash merge

## Commit Style

Use Conventional Commits where practical:

```text
feat(codex): add account import
fix(claude): roll back credentials on settings failure
docs(readme): add macos install guide
chore(ci): add release workflow
refactor(core): centralize target resolution
```

## Versioning

Prismux uses Semantic Versioning.

During `0.x`, the CLI and internal plugin API may still change:

- `0.MINOR.0`: meaningful feature milestone or breaking CLI/core change
- `0.MINOR.PATCH`: bug fix, documentation, CI, or packaging fix
- `1.0.0`: stable CLI behavior and stable core plugin API

The release workflow publishes when a PR changes the workspace version and
`CHANGELOG.md` contains the matching version section.

Current roadmap shape:

- `v0.1.0`: macOS public release, Codex + Claude account/profile support
- `v0.2.0`: Linux validation and official Linux binaries
- `v0.3.0`: Windows validation and official Windows binaries
- `v0.4.0`: Gemini or broader provider/profile support
- `v1.0.0`: stable CLI and plugin API

## Rust Toolchain

Use the stable Rust toolchain selected by `rust-toolchain.toml`. Prismux does not
currently guarantee a minimum supported Rust version.

```sh
rustup default stable
rustup component add rustfmt clippy
```

## Local Checks

Before finishing a code change:

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```

Run from source:

```sh
cargo run -p prismux-cli -- status
cargo run -p prismux-cli -- list
cargo run -p prismux-cli -- list codex
```

Use isolated state for manual checks:

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- status
```

## Menubar and Bundle Checks

The macOS Menubar app requires a full Xcode installation. Command Line Tools
alone do not include the SwiftUI macro toolchain used by the app.

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

The bundle script creates `target/menubar/Prismux.app` with the bundled helper at
`Prismux.app/Contents/MacOS/prismux`. Use isolated state for helper smoke tests:

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home target/menubar/Prismux.app/Contents/MacOS/prismux status
```

See [docs/BUILD.md](docs/BUILD.md) for source-build details.

## OpenSpec Changes

Use `openspec/changes/<change-name>/` for product or architecture changes before
implementation. Proposal, design, tasks, and capability specs under
`openspec/changes/**` are written in Chinese; keep commands, code identifiers,
file paths, crate names, and protocol terms in English when that is clearer.

## Pull Request Checklist

- CI passes.
- Local checks have been run or the PR explains why not.
- README/docs are updated when behavior changes.
- `CHANGELOG.md` is updated for user-visible changes.
- Menubar, release, and install docs are updated when bundle behavior changes.
- No tokens, auth payloads, snapshots, backups, or private credential files are committed.
- Changes touching auth replacement explain backup/rollback behavior.

## Safety Rules

Prismux operates on auth files and local account state. Be conservative:

- Never print raw tokens.
- Never store raw auth material in registry metadata.
- Back up active auth state before replacing it.
- Use atomic writes for active auth files.
- Verify snapshot hashes before switching.
- Roll back when a switch fails halfway.
- Keep diagnostics useful without exposing secrets.

## Community Conduct

Be direct, specific, and respectful. Assume good intent, keep reviews focused on
the code and user impact, and never ask contributors to paste tokens, raw auth
payloads, snapshots, backups, or private account files into public issues or PRs.
