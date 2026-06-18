## 1. 仓库公开内容整理

- [x] 1.1 审计当前 tracked files，列出 OpenMux public repo 必须保留、可移除、需迁移到 docs 的文件组。
- [x] 1.2 从 git index 移除本地 agent/tooling 目录中不属于 OpenMux 产品交付的文件：`.agents/`、`.claude/`、`.codex/`、`.gemini/`、`_bmad/`、`_bmad-output/`、`design-artifacts/`。
- [x] 1.3 更新 `.gitignore`，忽略本地 agent/tooling 目录、生成物、临时发布产物和本地 smoke test 状态目录。
- [x] 1.4 确认 `AGENTS.md`、`openspec/`、`docs/`、源码 crate、Cargo manifests、GitHub workflows、许可证和 OpenSpec change artifacts 仍被跟踪。
- [x] 1.5 用 `git status --short` 和 `git ls-files` 验证 public repo surface 已收敛。

## 2. 双语用户文档与维护文档

- [x] 2.1 重写 `README.md` 为英文主入口，包含语言切换、项目定位、v0.1 成熟度、macOS 支持矩阵、安装入口、Quick Start 和文档入口。
- [x] 2.2 新增 `README.zh-CN.md`，与英文 README 信息一致，并链接回英文入口。
- [x] 2.3 在 README 中明确 v0.1 安装路径：GitHub Releases macOS binary 和 `cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked`。
- [x] 2.4 在 README 中明确 Homebrew、crates.io、Linux official binary、Windows official binary 均属于 roadmap，不是 v0.1 支持路径。
- [x] 2.5 在 README 中补齐 Quick Start：`omx status`、`omx login codex`、`omx list`、`omx list codex`、`omx use codex <selector>`。
- [x] 2.6 在 README 中明确 Claude account 与 profile 的区别，并避免把 profile 切换描述成 OAuth account 切换。
- [x] 2.7 新增 `docs/INSTALL.md` 和 `docs/INSTALL.zh-CN.md`，覆盖 GitHub Release binary、cargo git install、PATH 配置、版本验证和卸载/清理说明。
- [x] 2.8 新增 `ROADMAP.md` 和 `ROADMAP.zh-CN.md`，列出 v0.1 已完成、v0.1 hardening、Linux/Windows 官方支持、Homebrew、crates.io、cargo deny、artifact signing/provenance。
- [x] 2.9 新增 `SECURITY.md`，说明 GitHub private vulnerability reporting、禁止公开提交 token/auth payload、支持的安全边界和敏感文件类型。
- [x] 2.10 更新 `CHANGELOG.md`，使用 `Unreleased` 和 `vX.Y.Z - YYYY-MM-DD` 格式，补齐 v0.1 release notes 所需内容。
- [x] 2.11 更新 `CONTRIBUTING.md`，同步 GitHub Flow、PR-only、branch protection 建议、squash merge、Conventional Commits、latest stable Rust/no MSRV、PR checklist 和 auth 安全规则。
- [x] 2.12 新增 `.github/ISSUE_TEMPLATE/bug_report.md`、`.github/ISSUE_TEMPLATE/feature_request.md` 和 `.github/pull_request_template.md`。

## 3. CI 工作流增强

- [x] 3.1 将 `.github/workflows/ci.yml` 调整为 v0.1 macOS required baseline。
- [x] 3.2 保留 `cargo fmt --all -- --check`。
- [x] 3.3 保留 `cargo clippy --all-targets --all-features -- -D warnings`。
- [x] 3.4 将 test 命令改为 `cargo test --locked`，确保 CI 使用 lockfile。
- [x] 3.5 在 README 或 CONTRIBUTING 中说明 CI 是 PR merge gate。

## 4. GitHub Release 自动化流水线

- [x] 4.1 新增或重写 `.github/workflows/release.yml`，在 `main` push 后运行 release-on-version-bump 检查。
- [x] 4.2 release workflow 读取 root workspace version，并以 `v<version>` 作为 tag 名。
- [x] 4.3 如果 tag 已存在，workflow 成功退出且不创建 release。
- [x] 4.4 如果 tag 不存在但 `CHANGELOG.md` 缺少对应 `## v<version>` 或 `## v<version> - YYYY-MM-DD` 段落，workflow 失败且不创建 tag。
- [x] 4.5 release workflow 运行必要 preflight：`cargo fmt --all -- --check`、`cargo test --locked`、`cargo clippy --all-targets --all-features -- -D warnings`。
- [x] 4.6 release workflow 构建 macOS Apple Silicon binary。
- [x] 4.7 release workflow 使用原生 macOS x86_64 runner 构建 macOS Intel binary。
- [x] 4.8 每个 release binary 上传前运行 `omx --version` 和隔离目录 `omx status` 自检。
- [x] 4.9 将 `omx` 二进制打包为平台命名的 `.tar.gz` archive。
- [x] 4.10 生成 `SHA256SUMS` 并上传到 GitHub Release。
- [x] 4.11 使用 CHANGELOG 对应版本段落作为 GitHub Release notes。
- [x] 4.12 设置 workflow 最小权限：PR CI 不需要写权限，release workflow 只授予创建 tag/release 所需权限。
- [x] 4.13 新增 `docs/RELEASE.md` 和 `docs/RELEASE.zh-CN.md`，说明版本号、CHANGELOG、自动 tag/release、preflight checks、artifact 自检、手动 smoke test 和失败回滚。

## 5. 安全检查与发布前验证

- [x] 5.1 运行 `cargo fmt --all`。
- [x] 5.2 运行 `cargo test --locked`。
- [x] 5.3 运行 `cargo clippy --all-targets --all-features -- -D warnings`。
- [x] 5.4 运行 `cargo build --release -p omx-cli --locked`。
- [x] 5.5 使用临时目录运行 safe smoke test：`OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- status`。
- [x] 5.6 使用临时目录运行 Codex/Claude list smoke tests：`list`、`list codex`、`list claude`、`doctor`。
- [x] 5.7 运行 gitleaks 或等价 secret scan；本次 gitleaks 不可用，已使用 git grep 手动审计 token/auth payload 风险。
- [x] 5.8 验证文档中的 repository URL、安装命令、release target 名称、checksum 说明和 roadmap 内容一致。
- [x] 5.9 执行 `openspec status --change prepare-github-launch`，确认提案 artifacts 完整。

## 6. 发布策略确认

- [x] 6.1 将 GitHub repository URL 固定为 `https://github.com/hiQianFan/openmux`，并同步 Cargo metadata、README badge 和安装命令。
- [x] 6.2 确认 v0.1 不发布 Linux/Windows official binaries，并在 README/ROADMAP 中明确。
- [x] 6.3 确认 v0.1 不发布 crates.io、不维护 Homebrew tap，并在 README/ROADMAP 中明确。
- [x] 6.4 确认 v0.1 只提供 `SHA256SUMS`，artifact signing/provenance 放入 roadmap。
- [x] 6.5 确认 tag 由 GitHub Actions release workflow 创建，本地手动 tag 不是推荐发布路径。
