# OpenMux

OpenMux is a local account manager for AI coding tools.

It is being designed around one small, reliable core: import the current login
state from a tool such as Codex, store it locally under a friendly alias, and
switch between accounts without making the user remember where each tool keeps
its auth files.

The first milestone is a Rust core plus CLI. GUI, tray, daemon, and dynamic
plugins can be added later without moving account switching logic out of the
core crates.

> Status: early scaffold. The workspace compiles and exposes the initial CLI
> shape, but real Codex auth import/switching is not implemented yet.

## Goals

- Provide a simple CLI for account switching across AI coding tools.
- Keep platform-specific behavior isolated in plugin crates.
- Store sensitive auth material through a vault abstraction instead of printing
  or scattering tokens in plain text.
- Make switching safe: detect paths, back up current state, write atomically,
  and roll back on failure.
- Leave room for a future macOS/Windows tray app and GUI without rewriting the
  core.

## Non-goals

- OpenMux is not an API gateway or model router.
- OpenMux is not a provider marketplace.
- The first version will not try to support every AI tool at once.
- Linux GUI/tray support is intentionally out of scope for the early desktop
  plan, though the CLI should remain portable.

## Workspace

```text
openmux/
├── Cargo.toml
├── crates/
│   ├── omx-core/
│   ├── omx-plugin-codex/
│   └── omx-cli/
└── rust-toolchain.toml
```

- `crates/omx-core`: shared domain types, errors, reports, and platform plugin
  interfaces.
- `crates/omx-plugin-codex`: Codex platform adapter. This is currently a
  scaffold and will become the first real platform implementation.
- `crates/omx-cli`: `omx` command line frontend. It should stay thin and call
  core/plugin APIs instead of owning business logic.

Future crates may include:

- `omx-vault`: system keychain / credential manager integration.
- `omx-plugin-claude`: Claude Code adapter.
- `omx-plugin-gemini`: Gemini CLI adapter.
- `omx-daemon`: optional local service for tray/GUI clients.
- `apps/desktop`: optional Tauri or native desktop frontend.

## Architecture

OpenMux follows a small layered design:

```text
CLI / future GUI / future tray
        |
        v
platform plugins
        |
        v
omx-core
```

The core crate owns shared concepts such as:

- platform identity
- account references
- account status
- switch reports
- doctor reports
- platform plugin trait

Each platform plugin should own only the behavior specific to that tool:

- detect whether the tool is installed
- locate config/auth paths
- import the currently active account
- list stored accounts
- switch to an account
- run diagnostics

## CLI

Current scaffolded commands:

```sh
omx list
omx current codex
omx status
omx import codex work
omx use codex work
omx doctor
```

The intended command style is:

```sh
omx import <platform> <alias>
omx use <platform> <alias>
omx current <platform>
```

Examples:

```sh
omx import codex work
omx use codex personal
omx current codex
```

## Development Setup

Install Rust with `rustup`, then use the stable toolchain:

```sh
rustup default stable
rustup component add rustfmt clippy
```

This repository includes `rust-toolchain.toml` so Cargo should select stable
automatically.

Build and test:

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features
cargo fmt --all
```

Run the CLI during development:

```sh
cargo run -p omx-cli -- list
cargo run -p omx-cli -- status
cargo run -p omx-cli -- doctor
```

If `cargo` is not on your PATH after installing Homebrew `rustup`, make sure the
active toolchain bin directory is available in your shell:

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

## Safety Principles

OpenMux will deal with auth files and tokens, so the core should be conservative:

- Do not print tokens or raw auth payloads.
- Do not store sensitive auth material in the registry.
- Prefer system credential stores for secrets.
- Use atomic writes when replacing a tool's active auth file.
- Back up existing active auth before switching.
- Roll back if a switch operation fails partway through.
- Keep `doctor` output useful without leaking private data.

## Roadmap

1. Implement Codex detection and auth path discovery.
2. Add local account registry.
3. Add import/list/use/current for Codex.
4. Add backups, atomic writes, and rollback.
5. Add a vault abstraction for sensitive auth material.
6. Add `doctor` checks for common path and permission problems.
7. Add Claude Code as the second platform plugin.
8. Decide whether the first desktop frontend should be Tauri or native
   macOS/Windows apps.

## License

MIT, pending repository setup.
