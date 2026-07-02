# AGENTS.md

## Project

Prismux is a Rust CLI and macOS Menubar app for local account switching across
AI coding tools. Codex and Claude Code are implemented; Gemini CLI is planned
after the core account-switching flow is reliable.

## Commands

Use the stable Rust toolchain. If `cargo` is not on PATH in this environment,
try:

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

Run before finishing code changes:

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```

For manual CLI checks, isolate state with temporary directories:

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- status
```

## Architecture

- `crates/prismux-core`: shared types, reports, errors, and plugin trait.
- `crates/prismux-plugin-codex`: Codex path detection and account switching.
- `crates/prismux-plugin-claude`: Claude Code profile and OAuth account handling.
- `crates/prismux-cli`: thin command-line presentation layer.
- `crates/prismux-app` and `crates/prismux-menubar-ffi`: control-plane runtime
  and Swift bridge for the macOS Menubar app.
- `apps/prismux-menubar`: SwiftPM macOS Menubar app.
- `docs/PRD.md`: product scope.
- `docs/ARCHITECTURE.md`: technical design.

## Documentation

- Project proposals and OpenSpec artifacts under `openspec/changes/**` must be
  written in Chinese, including `proposal.md`, `design.md`, `tasks.md`, and
  capability `spec.md` files.
- Keep commands, code identifiers, file paths, crate names, and protocol terms
  in their original English when translating would reduce clarity.
- User-facing product reasoning should prefer concise Chinese descriptions with
  concrete CLI examples.

## Safety Rules

- Do not print tokens, raw auth payloads, or private account files.
- Do not store auth payloads in registry metadata.
- Back up active auth before replacing it.
- Use atomic writes for registry and auth replacement.
- Keep permissions private for state directories and auth snapshots where the
  platform supports it.
- Do not add API calls to private or undocumented endpoints without an explicit
  product decision.

## Implementation Preferences

- Keep CLI logic thin; platform behavior belongs in plugin crates.
- Prefer small, testable filesystem operations over hidden global state.
- Keep registry schemas versioned and reject unsupported future versions.
- Add tests around import/switch behavior before expanding provider support.
- Treat `CODEX_HOME` as the primary override for Codex path discovery.
