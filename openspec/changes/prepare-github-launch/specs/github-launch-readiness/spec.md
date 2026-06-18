## ADDED Requirements

### Requirement: 公开仓库内容面

OpenMux SHALL 让公开 GitHub 仓库聚焦于产品源码、产品文档、OpenSpec 规划产物，以及构建、测试、发布和贡献所需的 GitHub automation。

#### Scenario: 本地 agent tooling 不被跟踪

- **WHEN** maintainer 在 GitHub 公开发布前审查 tracked files
- **THEN** 不属于构建或使用 OpenMux 所必需的本地 agent/tooling 目录不应作为公开项目源码被跟踪，包括 `.agents/`、`.claude/`、`.codex/`、`.gemini/`、`_bmad/`、`_bmad-output/` 和 `design-artifacts/`
- **AND** `.gitignore` 防止这些本地目录或生成物被意外重新加入。

#### Scenario: 必要项目产物仍被跟踪

- **WHEN** 本地 tooling 目录从公开内容面移除
- **THEN** OpenMux source crates、`docs/`、`openspec/`、`AGENTS.md`、`README.md`、`README.zh-CN.md`、`CONTRIBUTING.md`、`CHANGELOG.md`、`LICENSE`、Cargo manifests、lockfile 和 GitHub workflows 仍对用户和贡献者可用。

### Requirement: 用户文档

OpenMux SHALL 提供英文优先的用户文档，并为关键用户入口提供中文翻译。

#### Scenario: README 语言结构

- **WHEN** 新访问者打开 `README.md`
- **THEN** README 主要使用英文
- **AND** 包含指向 `README.zh-CN.md` 的语言切换链接
- **AND** `README.zh-CN.md` 链接回 `README.md`。

#### Scenario: README 首屏信息

- **WHEN** 新访问者打开 `README.md`
- **THEN** README 包含项目定位、当前发布成熟度、支持平台矩阵，以及安装、quick start、安全、roadmap 和贡献信息入口。

#### Scenario: README 安装路径

- **WHEN** 用户查找安装说明
- **THEN** README 说明 macOS 可从 GitHub Releases 安装
- **AND** 说明 `cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked`
- **AND** 明确 crates.io、Homebrew、Linux official binaries 和 Windows official binaries 是 roadmap 项，而不是 v0.1 支持路径。

#### Scenario: README quick start

- **WHEN** 用户按 Quick Start 操作
- **THEN** 命令覆盖 `omx status`、`omx login codex`、`omx list`、`omx list codex` 和 `omx use codex <selector>`
- **AND** README 中的 Claude 示例明确 account 与 profile 是不同层次。

#### Scenario: CLI 语言范围

- **WHEN** 文档被翻译
- **THEN** CLI 输出、命令名和命令示例保持英文
- **AND** v0.1 不要求 CLI i18n。

### Requirement: 开源维护文档

OpenMux SHALL 包含安全且可维护的开源协作所需的最小文档集合。

#### Scenario: 安全报告

- **WHEN** 用户发现可能的 token 泄露、auth corruption 或 credential handling 漏洞
- **THEN** `SECURITY.md` 说明 public issues 不得包含 tokens、auth payloads、snapshots、backups 或 private account file contents
- **AND** 在可用时推荐使用 GitHub private vulnerability reporting
- **AND** 说明 `auth.json`、Claude Keychain/plaintext credentials、registry metadata、snapshots 和 backups 的安全边界。

#### Scenario: Roadmap 可见

- **WHEN** 贡献者想了解项目方向
- **THEN** `ROADMAP.md` 和 `ROADMAP.zh-CN.md` 标明 v0.1 已完成范围、后续 hardening、Linux official support、Windows official support、Homebrew、crates.io、cargo deny 和 artifact signing/provenance 等 roadmap 项。

#### Scenario: 贡献期望

- **WHEN** 贡献者打开 PR
- **THEN** `CONTRIBUTING.md` 说明 required local checks、GitHub Flow 分支规则、Conventional Commit 期望、squash merge 偏好、stable Rust toolchain 且不承诺 MSRV、auth 文件安全规则，以及行为变更的文档要求
- **AND** 它说明 `main` 应启用 branch protection，包括 required checks、禁止 force push 和禁止删除。

#### Scenario: Issue 与 PR templates

- **WHEN** 贡献者打开 issue 或 pull request
- **THEN** 仓库提供轻量 bug report、feature request 和 pull request templates
- **AND** bug report template 提醒用户不要粘贴 tokens、`auth.json`、`.credentials.json`、snapshots、backups 或 private account file contents。

#### Scenario: Changelog 准确

- **WHEN** maintainer 准备 release PR
- **THEN** `CHANGELOG.md` 包含 `## Unreleased`
- **AND** release PR 将累计 notes 提升到 `## vX.Y.Z - YYYY-MM-DD`
- **AND** 版本段落在适用时包含 Codex account/profile support、Claude profile support、Claude OAuth account snapshot support、安全行为、CI/release changes 和 known limitations。

### Requirement: CI 质量门禁

OpenMux SHALL 在 pull requests 和 main branch updates 上运行自动质量检查。

#### Scenario: Pull request CI

- **WHEN** pull request 被创建或更新
- **THEN** GitHub Actions 运行 Rust formatting check
- **AND** 使用 `-D warnings` 运行 all targets 和 all features 的 clippy
- **AND** 使用 lockfile 运行 tests
- **AND** 在 PR 可被视为 merge-ready 前报告失败。

#### Scenario: macOS v0.1 CI baseline

- **WHEN** v0.1 的 Rust workspace CI 运行
- **THEN** CI 在 macOS 上运行
- **AND** Linux 和 Windows CI 被记录为 roadmap 或 optional validation，而不是 v0.1 support guarantee。

### Requirement: Version-bump release automation

OpenMux SHALL 提供自动 GitHub Release workflow，在 version-bump PR 合并到 `main` 后发布 v0.1 macOS binaries。

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

OpenMux SHALL 只为 v0.1 支持的 macOS targets 发布官方 binaries。

#### Scenario: Release target set

- **WHEN** v0.1 release workflow 构建 artifacts
- **THEN** 构建 macOS Apple Silicon
- **AND** 使用原生 x86_64 GitHub runner 构建 macOS Intel
- **AND** 不发布 Linux 或 Windows official binaries。

#### Scenario: Release artifact self-test

- **WHEN** 每个 release binary 构建完成
- **THEN** workflow 运行 `omx --version`
- **AND** workflow 使用隔离 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 `CLAUDE_CONFIG_DIR` 运行 `omx status`
- **AND** self-test 失败时不上传 artifact。

#### Scenario: Checksums

- **WHEN** release artifacts 被打包
- **THEN** workflow 生成 `SHA256SUMS`
- **AND** 将 checksums 上传到 GitHub Release
- **AND** 文档说明 v0.1 不提供独立 signing 或 provenance。

### Requirement: 安装路径

OpenMux SHALL 说明 v0.1 安装路径，且不夸大未支持的 package managers。

#### Scenario: 用户安装指南

- **WHEN** 用户想安装 OpenMux
- **THEN** `docs/INSTALL.md` 和 `docs/INSTALL.zh-CN.md` 说明 macOS GitHub Release binary 安装
- **AND** 说明 `cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked`
- **AND** 说明如何验证 `omx --version` 和运行 `omx status`。

#### Scenario: 未支持的 package managers

- **WHEN** 安装文档提及 Homebrew 或 crates.io
- **THEN** 文档明确这些路径 planned but not available in v0.1。

### Requirement: 手动 smoke test 指南

OpenMux SHALL 说明安全的手动 smoke tests，避免修改贡献者真实 AI tool credentials。

#### Scenario: 隔离 status 和 list checks

- **WHEN** maintainer 执行 pre-release smoke testing
- **THEN** 文档使用临时 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 `CLAUDE_CONFIG_DIR`
- **AND** smoke tests 包含 `omx status`、`omx list`、`omx list codex`、`omx list claude` 和 `omx doctor`
- **AND** 明确标记可能触碰真实 credentials 的命令需要用户显式意图。

### Requirement: 仓库安全扫描

OpenMux SHALL 包含 release-preparation check，用于查找意外跟踪的 secrets 或 auth payloads。

#### Scenario: 首次公开发布前 secret scan

- **WHEN** maintainers 准备 GitHub 公开上线
- **THEN** tasks 包含对当前 tree 运行 gitleaks 或等价 secret scan
- **AND** tasks 包含确认 staged changes 不含 raw tokens、auth payloads、snapshots、backups 或 private account file contents。
