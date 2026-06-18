# OpenMux

[English](README.md) | 简体中文

OpenMux 是面向 AI coding tools 的本地账号和 profile 切换工具。

它帮助你在 Codex、Claude Code 等工具中管理多个本地账号，查看账号池状态，并通过平台内编号或 alias 快速切换账号，减少重复浏览器登录。

> 当前状态：早期 v0.x Rust CLI。v0.1 官方二进制只面向 macOS。Linux 和 Windows 会在平台凭据、权限和外部 CLI 行为验证后加入官方支持。

## 支持平台

| 平台 | v0.1 状态 | 说明 |
| --- | --- | --- |
| macOS Apple Silicon | 支持 | v0.1 官方 GitHub Release target。 |
| macOS Intel | 支持 | v0.1 官方 GitHub Release target。 |
| Linux | 计划中 | 源码构建可能可用；v0.1 不提供官方 binary。 |
| Windows | 计划中 | 需要验证 credential、权限和外部 CLI 行为。 |

## 支持工具

| 工具 | 状态 | 能力 |
| --- | --- | --- |
| Codex | 已实现 | 官方 login 包装、device auth、编号账号池、alias、save、list、switch、profile import、best-effort usage。 |
| Claude Code | 已实现 | Gateway/API profile 导入与切换、OAuth account snapshot 导入与切换、macOS Keychain、非 macOS plaintext fallback。 |
| Gemini CLI | 计划中 | 尚未实现。 |

## 安装

### GitHub Releases

v0.1 从 GitHub Releases 下载 macOS archive：

```text
https://github.com/hiQianFan/openmux/releases
```

将 `omx` 放到 `PATH` 后验证：

```sh
omx --version
omx status
```

### 从 Git 安装 Cargo package

如果本机已有 Rust：

```sh
cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked
omx --version
```

Homebrew 和 crates.io 是后续计划，不是 v0.1 安装路径。

## 快速开始

查看已识别的工具 home：

```sh
omx status
```

通过 Codex 官方登录流程添加账号：

```sh
omx login codex
```

远程机器或无浏览器环境：

```sh
omx login codex --device-auth
```

查看全平台账号池：

```sh
omx list
```

查看单个平台明细：

```sh
omx list codex
```

按编号或 alias 切换：

```sh
omx use codex 2
omx use codex work
```

设置 alias：

```sh
omx alias codex 2 work
```

## Claude Code account 与 profile

OpenMux 中 Claude Code 有两个不同层次：

- **OAuth account**：官方 Claude.ai/Console 登录快照。
- **Profile**：写入 Claude Code `settings.json` env 的 gateway/API 配置。

导入 gateway/API profile：

```sh
omx import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=<your-token>
ANTHROPIC_MODEL=sonnet
"
omx use claude gateway-work
```

登录并记录 Claude OAuth account：

```sh
omx login claude --alias work
omx list claude
omx use claude work
```

OpenMux 不实现自己的 Anthropic OAuth token exchange，也不调用 Anthropic 私有 endpoint。它只包装官方 Claude Code CLI 登录流程，或导入本机已有官方 credential。

## 安全模型

- 不打印 token 或 raw auth payload。
- registry 只保存 metadata 和 hash，不保存 raw auth。
- 替换 active credential 前会备份。
- 在平台支持时，snapshot 和 registry 使用私有权限文件。
- 切换前校验 snapshot hash。
- 遇到未来 registry schema 会拒绝修改。

疑似 credential 处理漏洞请私密报告，见 [SECURITY.md](SECURITY.md)。

## 文档

- [安装指南](docs/INSTALL.zh-CN.md)
- [发布指南](docs/RELEASE.zh-CN.md)
- [路线图](ROADMAP.zh-CN.md)
- [贡献指南](CONTRIBUTING.md)
- [架构](docs/ARCHITECTURE.md)
- [产品范围](docs/PRD.md)

## 开发

OpenMux 使用 `rust-toolchain.toml` 选择的 stable Rust。当前不承诺具体 MSRV。

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
cargo run -p omx-cli -- status
cargo run -p omx-cli -- list
cargo run -p omx-cli -- list codex
```

涉及工具 home 的手动检查建议隔离状态：

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- status
```

## License

MIT
