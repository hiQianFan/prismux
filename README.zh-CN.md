<div align="center">

<img src="assets/prismux-icon/prismux-mac-icon-1024.png" width="128" alt="Prismux" />

# Prismux

**🔀 面向 AI coding tools 的本地账号 & profile 切换工具**

[English](README.md) | 简体中文

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg?logo=rust)](rust-toolchain.toml)
[![Platform](https://img.shields.io/badge/macOS-Apple%20Silicon-black.svg?logo=apple)](#-支持平台)
[![Status](https://img.shields.io/badge/status-early%20v0.x-blue.svg)](ROADMAP.zh-CN.md)

</div>

Prismux 帮助你在 Codex、Claude Code 等工具中管理多个本地账号，查看账号池状态，并通过平台内编号或 alias 快速切换账号，减少重复浏览器登录。🚀

> ⚠️ **当前状态：** 早期 v0.x。v0.1 官方下载路径是 macOS full app bundle。Linux 和 Windows 会在平台凭据、权限和外部 CLI 行为验证后加入官方支持。

## 🖥️ 支持平台

| 平台 | v0.1 状态 | 说明 |
| --- | --- | --- |
| macOS Apple Silicon | 支持 | 官方 `Prismux.app` GitHub Release target。 |
| macOS Intel | 不计划 | 源码构建可能可用；不发布官方 app bundle。 |
| Linux | 计划中 | 源码构建可能可用；v0.1 不提供官方 binary。 |
| Windows | 计划中 | 需要验证 credential、权限和外部 CLI 行为。 |

## 🧰 支持工具

| 工具 | 状态 | 能力 |
| --- | --- | --- |
| Codex | 已实现 | 官方 login 包装、device auth、编号账号池、alias、save、list、switch、profile import、额度/限流展示。 |
| Claude Code | 已实现 | Gateway/API profile 导入与切换、OAuth account snapshot 导入与切换、macOS Keychain、非 macOS plaintext fallback。 |
| Gemini CLI | 计划中 | 尚未实现。 |

## 📦 安装

### GitHub Releases

v0.1 从 GitHub Releases 下载 macOS app archive：

```text
https://github.com/hiQianFan/prismux/releases
```

解压后把 `Prismux.app` 拖到 `/Applications`，再从 Finder 打开。App 内置同版本 `prismux` CLI helper；如果希望在 Terminal 使用 `prismux`，在 Settings 中点击 `Enable prismux command`，然后验证：

```sh
prismux --version
prismux status
```

### 从 Git 安装 Cargo package

如果本机已有 Rust：

```sh
cargo install --git https://github.com/hiQianFan/prismux -p prismux-cli --locked
prismux --version
```

Homebrew 和 crates.io 是后续计划，不是 v0.1 安装路径。

## ⚡ 快速开始

查看已识别的工具 home：

```sh
prismux status
```

通过 Codex 官方登录流程添加账号：

```sh
prismux login codex
```

远程机器或无浏览器环境：

```sh
prismux login codex --device-auth
```

查看全平台账号池：

```sh
prismux list
```

查看单个平台明细：

```sh
prismux list codex
```

按编号或 alias 切换：

```sh
prismux use codex 2
prismux use codex work
```

设置 alias：

```sh
prismux alias codex 2 work
```

## 🤖 Claude Code account 与 profile

Prismux 中 Claude Code 有两个不同层次：

- **OAuth account**：官方 Claude.ai/Console 登录快照。
- **Profile**：写入 Claude Code `settings.json` env 的 gateway/API 配置。

导入 gateway/API profile：

```sh
prismux import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=<your-token>
ANTHROPIC_MODEL=sonnet
"
prismux use claude gateway-work
```

登录并记录 Claude OAuth account：

```sh
prismux login claude --alias work
prismux list claude
prismux use claude work
```

Prismux 不实现自己的 Anthropic OAuth token exchange，也不调用 Anthropic 私有 endpoint。它只包装官方 Claude Code CLI 登录流程，或导入本机已有官方 credential。

## 🔒 安全模型

- 不打印 token 或 raw auth payload。
- registry 只保存 metadata 和 hash，不保存 raw auth。
- 替换 active credential 前会备份。
- 在平台支持时，snapshot 和 registry 使用私有权限文件。
- 切换前校验 snapshot hash。
- 遇到未来 registry schema 会拒绝修改。

疑似 credential 处理漏洞请私密报告，见 [SECURITY.md](SECURITY.md)。

## 📚 文档

- [安装指南](docs/INSTALL.zh-CN.md)
- [发布指南](docs/RELEASE.zh-CN.md)
- [源码构建](docs/BUILD.md)
- [路线图](ROADMAP.zh-CN.md)
- [贡献指南](CONTRIBUTING.md)
- [架构](docs/ARCHITECTURE.md)
- [产品范围](docs/PRD.md)

## 🛠️ 开发

Prismux 使用 `rust-toolchain.toml` 选择的 stable Rust。当前不承诺具体 MSRV。

```sh
rustup default stable
rustup component add rustfmt clippy
```

提交前检查：

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```

从源码运行：

```sh
cargo run -p prismux-cli -- status
cargo run -p prismux-cli -- list
cargo run -p prismux-cli -- list codex
```

涉及工具 home 的手动检查建议隔离状态：

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- status
```

## 📄 License

MIT

## ⭐ Star History

<div align="center">

<a href="https://star-history.com/#hiQianFan/prismux&Date">
  <img src="https://api.star-history.com/svg?repos=hiQianFan/prismux&type=Date" alt="Star History Chart" width="600" />
</a>

</div>
