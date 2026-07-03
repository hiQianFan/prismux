# Prismux CLI Guide

[简体中文](CLI.zh-CN.md)

The `prismux` CLI is the terminal entry point for checking account state,
adding accounts, listing saved accounts, and switching the active account for a
supported AI coding tool.

## Install the CLI

Download the standalone CLI package from GitHub Releases:

```text
prismux-cli-vX.Y.Z-macos-arm64.tar.gz
```

Then install:

```sh
tar -xzf prismux-cli-vX.Y.Z-macos-arm64.tar.gz
cd prismux-cli-vX.Y.Z-macos-arm64
./install.sh
```

The package installs `prismux` and `pmx` to `$HOME/.local/bin` by default. To
choose a different directory:

```sh
PRISMUX_INSTALL_DIR=/usr/local/bin ./install.sh
```

## Check Status

```sh
prismux status
```

Use this first. It shows which tool homes Prismux can detect and what account
state is currently active.

## Add Accounts

Add a Codex account through the official Codex login flow:

```sh
prismux login codex
```

For remote machines or browserless environments:

```sh
prismux login codex --device-auth
```

Add a Claude Code account and give it an alias:

```sh
prismux login claude --alias work
```

## List Accounts

List all supported account pools:

```sh
prismux list
```

List a single tool:

```sh
prismux list codex
prismux list claude
```

## Switch Accounts

Switch by number:

```sh
prismux use codex 2
```

Switch by alias:

```sh
prismux use codex work
prismux use claude work
```

## Set Aliases

```sh
prismux alias codex 2 work
```

Aliases are useful when the numbered order changes or when you want scripts to
refer to a stable account name.

## Claude Code Notes

Prismux supports two Claude Code account styles:

- **OAuth accounts**: snapshots of official Claude.ai or Console login state.
- **Gateway/API profiles**: environment settings written to Claude Code
  `settings.json`.

Most users should start with OAuth accounts:

```sh
prismux login claude --alias work
prismux list claude
prismux use claude work
```

Gateway/API profiles are useful when your Claude Code setup uses a custom base
URL, auth token, or model setting:

```sh
prismux import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=<your-token>
ANTHROPIC_MODEL=sonnet
"
prismux use claude gateway-work
```

Prismux wraps the official Claude Code login flow or imports local official
credential artifacts. It does not implement its own Anthropic OAuth token
exchange and does not call private Anthropic endpoints.

## Manual Checks with Isolated State

When testing commands that touch tool homes, isolate state with temporary
directories:

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home prismux status
```
