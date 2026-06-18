## Why

OpenMux 的核心账号/profile 能力已经进入可发布的早期版本，但当前仓库还没有达到高质量 GitHub 开源项目的上线标准：本地 agent/BMad 工具链文件被跟踪、安装路径不清晰、CI 只覆盖基础检查、没有自动 release 打包，且 README/CHANGELOG/CONTRIBUTING 与当前实现状态不完全一致。

上线前需要把“用户能理解、能安装、能试用、能贡献、能从 tag 获得可验证二进制”作为独立交付目标处理，否则即使代码可用，开源体验也会显得不可信且难维护。

## What Changes

- 整理仓库公开内容，明确哪些目录属于 OpenMux 源码/产品文档，哪些属于本地 agent 工具链或生成物，并从 public repo surface 中移除或忽略无关内容。
- 更新 README，使英文为主入口，并提供中文切换链接；关键用户文档维护中英双语版本。
- 新增或更新开源协作文件：`SECURITY.md`、`ROADMAP.md`、issue/PR templates、release/安装说明，并修正 `CHANGELOG.md`、`CONTRIBUTING.md`、License 说明。
- 增强 CI：在 PR/push 上运行 Rust fmt、clippy、test；v0.1 以 macOS 支持为主，Linux/Windows 进入后续平台验证路线。
- 新增 GitHub Release 自动化流水线：当合并到 `main` 的 PR 修改 workspace version 且 `CHANGELOG.md` 存在对应版本段落时，workflow 自动创建 tag、构建 macOS `omx` 二进制、运行 artifact 自检、打包 archive、生成 checksum，并创建 GitHub Release。
- 明确安装路径：v0.1 支持 GitHub Releases macOS binary 和 `cargo install --git`；Homebrew、crates.io、Linux/Windows 官方二进制放入 roadmap。
- 明确 release notes 维护方式：`CHANGELOG.md` 的版本段落是 GitHub Release 公告来源，`Unreleased` 用于日常积累，发版 PR 将其提升为 `vX.Y.Z - YYYY-MM-DD`。
- 保留现有账号切换业务行为，不在本变更中修改 Codex/Claude account/profile 功能。

## Capabilities

### New Capabilities

- `github-launch-readiness`: GitHub 开源上线前的仓库卫生、用户文档、CI/CD、release 打包、安装路径和维护协作要求。

### Modified Capabilities

- 无。

## Impact

- 影响仓库根目录文档：`README.md`、`README.zh-CN.md`、`CONTRIBUTING.md`、`CHANGELOG.md`、`LICENSE` 引用说明，以及新增 `SECURITY.md`、`ROADMAP.md`、`ROADMAP.zh-CN.md`、`docs/INSTALL.md`、`docs/INSTALL.zh-CN.md`、`docs/RELEASE.md`、`docs/RELEASE.zh-CN.md` 等。
- 影响仓库公开文件清单和 `.gitignore`，可能需要移除已跟踪的 `.agents/`、`.claude/`、`.codex/`、`.gemini/`、`_bmad/`、`_bmad-output/`、`design-artifacts/` 中不属于 OpenMux 产品交付的文件。
- 影响 CI/CD：新增或调整 `.github/workflows/ci.yml`、`.github/workflows/release.yml`。
- 影响 Cargo metadata：至少保证 `cargo install --git <repo> -p omx-cli --locked` 可作为开发者安装路径；crates.io 发布所需的进一步 metadata/package 工作进入 roadmap，除非后续明确提前执行。
- 不影响 OpenMux registry schema、auth snapshot 格式、账号/profile selector 语义和现有 CLI 命令行为。
