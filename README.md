# OpenMux

English | [简体中文](README.zh-CN.md)

OpenMux is a local account and profile switcher for AI coding tools.

It helps you keep multiple local accounts for tools such as Codex and Claude
Code, inspect the current account pool, and switch accounts by a platform-local
number or alias without repeatedly walking through browser login flows.

> Status: early v0.x. v0.1 targets macOS full app bundles for official
> downloads. Linux and Windows are planned after platform-specific credential
> behavior is tested and documented.

## Supported Platforms

| Platform | v0.1 status | Notes |
| --- | --- | --- |
| macOS Apple Silicon | Supported | Official `OpenMux.app` GitHub Release target. |
| macOS Intel | Supported | Official `OpenMux.app` GitHub Release target. |
| Linux | Planned | Source builds may work; no official v0.1 binary. |
| Windows | Planned | Requires credential, permission, and external CLI validation. |

## Supported Tools

| Tool | Status | Capabilities |
| --- | --- | --- |
| Codex | Implemented | Official login wrapper, device auth, numbered account pool, aliases, save, list, switch, profile import, best-effort usage. |
| Claude Code | Implemented | Gateway/API profile import and switch, OAuth account snapshot import and switch, macOS Keychain support, plaintext fallback outside macOS. |
| Gemini CLI | Planned | Not implemented yet. |

## Install

### GitHub Releases

For v0.1, download the macOS app archive from:

```text
https://github.com/hiQianFan/openmux/releases
```

Unpack it, move `OpenMux.app` to `/Applications`, and open it from Finder.
The app contains the matching `omx` CLI helper. In Settings, use
`Enable omx command` if you want `omx` available in Terminal, then verify:

```sh
omx --version
omx status
```

### Cargo from Git

For developers with Rust installed:

```sh
cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked
omx --version
```

Homebrew and crates.io distribution are planned, but are not v0.1 install paths.

## Quick Start

Inspect detected tool homes:

```sh
omx status
```

Add a Codex account through the official Codex login flow:

```sh
omx login codex
```

For remote machines or browserless environments:

```sh
omx login codex --device-auth
```

List all platform pools:

```sh
omx list
```

List one platform in detail:

```sh
omx list codex
```

Switch by number or alias:

```sh
omx use codex 2
omx use codex work
```

Set an alias:

```sh
omx alias codex 2 work
```

## Claude Code Accounts and Profiles

Claude Code has two distinct layers in OpenMux:

- **OAuth accounts** are official Claude.ai/Console login snapshots.
- **Profiles** are gateway/API settings written to Claude Code `settings.json`
  environment keys.

Import a gateway/API profile:

```sh
omx import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=<your-token>
ANTHROPIC_MODEL=sonnet
"
omx use claude gateway-work
```

Login and record a Claude OAuth account:

```sh
omx login claude --alias work
omx list claude
omx use claude work
```

OpenMux does not implement its own Anthropic OAuth token exchange and does not
call private Anthropic endpoints. It wraps the official Claude Code CLI login
flow or imports local official credential artifacts.

## Safety Model

- OpenMux does not print raw tokens or raw auth payloads.
- Registry files store metadata and hashes, not raw auth material.
- Active credentials are backed up before replacement.
- Snapshot and registry writes use private files where the platform supports it.
- Snapshot hashes are verified before switching.
- Future registry schema versions are rejected instead of being modified.

Report suspected credential handling vulnerabilities privately. See
[SECURITY.md](SECURITY.md).

## Documentation

- [Install guide](docs/INSTALL.md)
- [Release guide](docs/RELEASE.md)
- [Build from source](docs/BUILD.md)
- [Roadmap](ROADMAP.md)
- [Contributing](CONTRIBUTING.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Product scope](docs/PRD.md)

## Development

OpenMux uses the stable Rust toolchain selected by `rust-toolchain.toml`. The
project does not currently guarantee a minimum supported Rust version.

```sh
rustup default stable
rustup component add rustfmt clippy
```

Before finishing changes:

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```

Run from source:

```sh
cargo run -p omx-cli -- status
cargo run -p omx-cli -- list
cargo run -p omx-cli -- list codex
```

Use isolated state for manual checks that touch tool homes:

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- status
```

## License

MIT
