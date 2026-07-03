# Prismux CLI 指南

[English](CLI.md)

`prismux` CLI 是 Prismux 的 Terminal 入口，可用于查看账号状态、添加账号、列出账号池，以及切换支持工具的当前账号。

## 安装 CLI

从 GitHub Releases 下载独立 CLI 包：

```text
prismux-cli-vX.Y.Z-macos-arm64.tar.gz
```

然后安装：

```sh
tar -xzf prismux-cli-vX.Y.Z-macos-arm64.tar.gz
cd prismux-cli-vX.Y.Z-macos-arm64
./install.sh
```

默认会把 `prismux` 和 `pmx` 安装到 `$HOME/.local/bin`。如果要指定其他目录：

```sh
PRISMUX_INSTALL_DIR=/usr/local/bin ./install.sh
```

## 查看状态

```sh
prismux status
```

建议先运行这个命令。它会展示 Prismux 能识别到哪些 tool home，以及当前账号状态。

## 添加账号

通过 Codex 官方登录流程添加账号：

```sh
prismux login codex
```

远程机器或无浏览器环境：

```sh
prismux login codex --device-auth
```

添加 Claude Code 账号并设置 alias：

```sh
prismux login claude --alias work
```

## 查看账号

查看所有支持工具的账号池：

```sh
prismux list
```

查看单个工具：

```sh
prismux list codex
prismux list claude
```

## 切换账号

按编号切换：

```sh
prismux use codex 2
```

按 alias 切换：

```sh
prismux use codex work
prismux use claude work
```

## 设置 Alias

```sh
prismux alias codex 2 work
```

当账号编号变化，或你希望脚本使用稳定账号名时，alias 会更方便。

## Claude Code 说明

Prismux 支持两类 Claude Code 账号配置：

- **OAuth account**：官方 Claude.ai 或 Console 登录状态的快照。
- **Gateway/API profile**：写入 Claude Code `settings.json` 的环境配置。

大多数用户应先使用 OAuth account：

```sh
prismux login claude --alias work
prismux list claude
prismux use claude work
```

如果你的 Claude Code 使用 custom base URL、auth token 或 model setting，可以导入 gateway/API profile：

```sh
prismux import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=<your-token>
ANTHROPIC_MODEL=sonnet
"
prismux use claude gateway-work
```

Prismux 只包装官方 Claude Code 登录流程，或导入本机已有官方 credential artifact。它不实现自己的 Anthropic OAuth token exchange，也不调用 Anthropic 私有 endpoint。

## 使用隔离状态做手动检查

测试会触碰 tool home 的命令时，建议使用临时目录隔离状态：

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home prismux status
```
