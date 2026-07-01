## MODIFIED Requirements

### Requirement: 用户文档

Prismux SHALL 提供英文优先的用户文档，并为关键用户入口提供中文翻译。

#### Scenario: README 语言结构

- **WHEN** 新访问者打开 `README.md`
- **THEN** README 主要使用英文
- **AND** 包含指向 `README.zh-CN.md` 的语言切换链接
- **AND** `README.zh-CN.md` 链接回 `README.md`。

#### Scenario: README 首屏信息

- **WHEN** 新访问者打开 `README.md`
- **THEN** README 包含 `Prismux` 项目定位、当前发布成熟度、支持平台矩阵，以及安装、quick start、安全、roadmap 和贡献信息入口
- **AND** README SHALL NOT present `OpenMux` as the current public brand.

#### Scenario: README 安装路径

- **WHEN** 用户查找安装说明
- **THEN** README 说明 macOS 可从 GitHub Releases 安装 `Prismux.app`
- **AND** 说明 `cargo install --git https://github.com/hiQianFan/prismux -p prismux-cli --locked`
- **AND** 说明安装后的正式命令是 `prismux`，短命令是 `pmx`
- **AND** 明确 crates.io、Homebrew、Linux official binaries 和 Windows official binaries 是 roadmap 项，而不是 v0.1 支持路径.

#### Scenario: README quick start

- **WHEN** 用户按 Quick Start 操作
- **THEN** 命令覆盖 `prismux status`、`prismux login codex`、`prismux list`、`prismux list codex` 和 `prismux use codex <selector>`
- **AND** README 中的 Claude 示例明确 account 与 profile 是不同层次
- **AND** README MAY mention that `pmx` can replace `prismux` for day-to-day use.

#### Scenario: CLI 语言范围

- **WHEN** 文档被翻译
- **THEN** CLI 输出、命令名和命令示例保持英文
- **AND** v0.1 不要求 CLI i18n。

### Requirement: Version-bump release automation

Prismux SHALL 提供自动 GitHub Release workflow，在 version-bump PR 合并到 `main` 后发布 v0.1 macOS binaries。

#### Scenario: Version bump 触发 release

- **WHEN** commit 进入 `main`
- **AND** root workspace version 解析为 `X.Y.Z`
- **AND** tag `vX.Y.Z` 不存在
- **AND** `CHANGELOG.md` 包含匹配的 `## vX.Y.Z` 或 `## vX.Y.Z - YYYY-MM-DD` section
- **THEN** GitHub Actions 运行 release preflight checks
- **AND** 创建 tag `vX.Y.Z`
- **AND** 使用匹配 CHANGELOG section 作为 release notes 创建 GitHub Release。

#### Scenario: 非 release main push

- **WHEN** commit 进入 `main`
- **AND** 当前 workspace version 对应的 tag `vX.Y.Z` 已存在
- **THEN** release workflow 成功退出，不创建新 tag 或 release。

#### Scenario: 缺少 changelog section 阻止 release

- **WHEN** commit 进入 `main`
- **AND** tag `vX.Y.Z` 不存在
- **AND** `CHANGELOG.md` 不包含匹配的 version section
- **THEN** release workflow 在创建 tag 或 release 前失败。

#### Scenario: Release permissions 最小化

- **WHEN** release workflow 运行
- **THEN** workflow permissions 限制在读取 contents 和创建 tags/releases 所需的最小范围
- **AND** pull request CI 不需要 release write permissions。

### Requirement: macOS release packaging

Prismux SHALL 只为 v0.1 支持的 macOS targets 发布官方 binaries。

#### Scenario: Release target set

- **WHEN** v0.1 release workflow 构建 artifacts
- **THEN** 构建 macOS Apple Silicon `Prismux.app`
- **AND** 使用原生 x86_64 GitHub runner 构建 macOS Intel `Prismux.app`
- **AND** 不发布 Linux 或 Windows official binaries。

#### Scenario: Release artifact self-test

- **WHEN** 每个 release app 构建完成
- **THEN** workflow 运行 `prismux --version` 和 `pmx --version`
- **AND** workflow 使用隔离 `PRISMUX_STATE_ROOT`、`CODEX_HOME` 和 `CLAUDE_CONFIG_DIR` 运行 `prismux status`
- **AND** self-test 失败时不上传 artifact.

#### Scenario: Checksums

- **WHEN** release artifacts 被打包
- **THEN** workflow 生成 `SHA256SUMS`
- **AND** 将 checksums 上传到 GitHub Release
- **AND** 文档说明 v0.1 不提供独立 signing 或 provenance。

### Requirement: 安装路径

Prismux SHALL 说明 v0.1 安装路径，且不夸大未支持的 package managers。

#### Scenario: 用户安装指南

- **WHEN** 用户想安装 Prismux
- **THEN** `docs/INSTALL.md` 和 `docs/INSTALL.zh-CN.md` 说明 macOS GitHub Release binary 安装
- **AND** 说明 `cargo install --git https://github.com/hiQianFan/prismux -p prismux-cli --locked`
- **AND** 说明如何验证 `prismux --version`、`pmx --version` 和运行 `prismux status`。

#### Scenario: 未支持的 package managers

- **WHEN** 安装文档提及 Homebrew 或 crates.io
- **THEN** 文档明确这些路径 planned but not available in v0.1。

### Requirement: 手动 smoke test 指南

Prismux SHALL 说明安全的手动 smoke tests，避免修改贡献者真实 AI tool credentials。

#### Scenario: 隔离 status 和 list checks

- **WHEN** maintainer 执行 pre-release smoke testing
- **THEN** 文档使用临时 `PRISMUX_STATE_ROOT`、`CODEX_HOME` 和 `CLAUDE_CONFIG_DIR`
- **AND** smoke tests 包含 `prismux status`、`prismux list`、`prismux list codex`、`prismux list claude` 和 `prismux doctor`
- **AND** 明确标记可能触碰真实 credentials 的命令需要用户显式意图。
