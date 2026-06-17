# OpenMux

Local Account Manager for AI Coding Tools.

Manage, unify, and switch accounts and subscriptions across AI coding tools.
Codex is supported today; Claude Code, Gemini CLI, and everything else can be added through plugins.
CLI first, with room for menu bar and GUI workflows.

OpenMux 是 AI coding tools 的本地账号和订阅管理器。

它的核心心智不是“管理 auth 文件”，而是：用户在 Codex、Claude Code、Gemini CLI 以及其他 plugin-supported tools 里有多个账号和订阅来源，希望能通过一个统一的本地 workflow 添加账号、查看平台账号池总览，并用平台内编号快速切换账号。

> 当前状态：早期 Rust 实现。Codex 插件已经支持 `omx login codex`、`omx login codex --device-auth`、可选 alias、编号账号池、Codex account/plan metadata、重复 auth hash 检测、`omx list [platform]`、`omx use codex <number|alias>`、`omx alias`、`omx save codex` 保存当前 active auth 和基础 doctor。Claude Code 已支持中转/API profile 导入与切换，并支持 OAuth account snapshot 导入与切换；macOS 使用 Keychain backend，非 macOS 或显式 `CLAUDE_CONFIG_DIR` 使用 plaintext `.credentials.json` backend。

## 目标

- 用简单 CLI 管理多个 AI coding 平台的本地账号池。
- 普通用户添加账号时走 `omx login <platform>`，由 OpenMux 调用平台官方登录流程并自动记录账号。
- 不强制用户给账号取 alias；平台内编号是默认 selector。
- 全局 `omx list` 是跨平台 overview，只展示平台、当前 active 账号、账号数、账号池总体可用概览、账号池 5h 剩余额度和收敛后的状态。
- 平台明细 `omx list <platform>` 再展示每个账号的编号、active、alias、account、plan 和 availability。
- Codex account/plan 从官方 auth JWT claims 中提取 email / plan；availability 使用 Codex backend usage endpoint 做 best-effort 查询，失败时显示 `unknown`。
- 平台行为放在 plugin crate 中，CLI 保持薄层。
- 替换 active auth 前备份，写入使用原子文件操作，并避免打印 raw auth payload。

## 非目标

- OpenMux 不是 API gateway、model router 或 provider marketplace。
- 当前阶段不实现 GUI、daemon、watcher 或动态插件加载。
- 当前阶段不读取 provider 私有 API 获取额度。
- alias 不是账号导入的必要条件。

## Workspace

```text
openmux/
├── Cargo.toml
├── crates/
│   ├── omx-core/
│   ├── omx-plugin-codex/
│   ├── omx-plugin-claude/
│   └── omx-cli/
├── docs/
└── openspec/
```

- `crates/omx-core`：共享领域类型、错误、报告、profile/account capability 和安全 storage helper。
- `crates/omx-plugin-codex`：Codex 平台适配器，负责路径解析、官方登录包装、账号池 registry、auth snapshot 和切换。
- `crates/omx-plugin-claude`：Claude Code 平台适配器，负责 profile settings env 切换，以及 Keychain/plaintext OAuth account snapshot 导入/恢复。
- `crates/omx-cli`：`omx` 命令行入口，只负责命令解析和输出展示。
- `docs/PRD.md`：产品路线和用户心智。
- `docs/ARCHITECTURE.md`：当前 monorepo 技术架构。
- `openspec/changes/account-pool-numbered-import/`：账号池提案、设计、任务和验收规格。

## CLI

普通添加账号：

```sh
omx login codex
```

远程、SSH、容器或无浏览器环境：

```sh
omx login codex --device-auth
```

登录时顺手设置 alias，或登录后立即切换：

```sh
omx login codex --alias work
omx login codex --use
omx login claude --alias work
omx login claude --use
```

查看全局总览：

```sh
omx list
```

`omx list` 展示的是平台级 overview，而不是单个账号列表。每个平台一行，用于快速判断当前选择的账号是谁、整个账号池还剩多少总体用量、关键窗口（例如 Codex 的 `5h`）是否紧张，以及是否有账号进入 limited/exhausted 状态。

查看某个平台账号池：

```sh
omx list codex
omx list claude
```

切换账号：

```sh
omx use codex 2
omx use codex work
```

给账号设置 alias：

```sh
omx alias codex 2 work
```

保存当前 active auth：

```sh
omx save codex
omx save codex --alias work
```

导入外部中转站/env 配置：

```sh
omx import codex --name apikey-fun "
model_provider = \"codex\"
model = \"gpt-5.5\"

[model_providers.codex]
name = \"codex\"
base_url = \"https://api.apikey.fun\"
wire_api = \"responses\"
requires_openai_auth = true
"
```

也可以导入 OpenAI-compatible KV，OpenMux 会生成 Codex profile 文件且不会把 raw API key 写入 profile：

```sh
omx import codex "
OPENAI_API_KEY=sk-xxx
OPENAI_BASE_URL=https://api.example.com/v1
OPENAI_MODEL=gpt-5
"
```

Claude Code 的中转/API profile 与 OAuth account 是两个不同层次。profile 写入 `settings.json` 的 `env`，不会替换 Claude.ai 登录凭据：

```sh
omx import claude --name gateway-work "
ANTHROPIC_BASE_URL=https://gateway.example.com
ANTHROPIC_AUTH_TOKEN=secret
ANTHROPIC_MODEL=sonnet
"
omx use claude gateway-work
```

Claude OAuth account 也走统一的 `claude` 入口。`omx login claude` 会调用官方 Claude Code 登录流程，登录成功后自动导入 account snapshot；`omx import claude` 有配置内容时导入中转/API profile，没有配置内容时导入本机已有官方登录产物。`omx list claude` 会分组展示 accounts 和 profiles，但使用同一组连续选择编号；例如两个 account 后面的第一个 profile 会显示为 `#3`。切换时 `omx use claude <selector>` 会在当前列表编号、account alias 和 profile name 中自动推断；如果 alias/name 同时命中两者，会拒绝并要求改成唯一名称。列表编号是当前展示编号，不是底层持久 ID；脚本中建议优先使用稳定 alias/profile name。OpenMux 不实现自己的 Anthropic OAuth token exchange，也不调用 Anthropic 私有 endpoint；macOS 默认读写 Keychain，非 macOS 或显式 `CLAUDE_CONFIG_DIR` 读写 `<claude-home>/.credentials.json`：

```sh
omx login claude --alias work
omx import claude --name work
omx list claude
omx use claude 3
omx use claude work
```

诊断：

```sh
omx status
omx doctor
omx doctor codex
```

## Development

安装 Rust stable toolchain：

```sh
rustup default stable
rustup component add rustfmt clippy
```

如果当前 shell 找不到 `cargo`，可以临时加入 stable toolchain：

```sh
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
```

常用检查，提交代码前至少跑一遍：

```sh
cargo fmt --all
cargo test
cargo clippy --all-targets --all-features
```

开发阶段直接运行 CLI：

```sh
cargo run -p omx-cli -- list
cargo run -p omx-cli -- list codex
cargo run -p omx-cli -- status
```

涉及 Codex auth 的手动检查建议隔离状态目录，避免误写真实账号文件：

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home cargo run -p omx-cli -- status
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home cargo run -p omx-cli -- list codex
OMUX_STATE_ROOT=/tmp/openmux-state CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list claude
```

本地编译 debug binary：

```sh
cargo build -p omx-cli
./target/debug/omx status
```

打包 release binary：

```sh
cargo build --release -p omx-cli
./target/release/omx status
```

把当前 workspace 的 `omx` 安装到 Cargo bin 目录，方便像普通 CLI 一样执行：

```sh
cargo install --path crates/omx-cli --locked
omx status
```

## Safety

- 不打印 token 或 raw auth payload。
- registry 只保存 metadata，不保存 raw auth。
- auth snapshot、backup、registry 写入时使用私有权限。
- 切换 active auth 前先备份已有 auth 文件。
- 替换 auth 文件和 registry 时使用原子写入。
- Claude profile 和 Claude OAuth account 底层分开管理，但 CLI 使用统一 `claude` 入口；selector 唯一命中 profile 时只修改 settings env，唯一命中 account 时才替换 OAuth credential。
- 遇到未来版本 registry schema 时拒绝继续写入。

## License

MIT, pending repository setup.
