## Context

当前代码质量门禁已经能在本地通过：`cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features` 和 `cargo build --release -p omx-cli` 均可执行成功。但从 GitHub 初始开源版本的角度看，仓库还存在几个发布工程问题：

- 仓库跟踪了大量本地 agent/BMad 工具链文件，例如 `.agents/`、`.claude/`、`.codex/`、`.gemini/`、`_bmad/`，这些不是 OpenMux 用户安装或贡献所必需的源码 surface。
- README 解释了核心功能，但缺少面向第一次访问 GitHub 的安装、Quick Start、支持矩阵和安全边界。
- `CHANGELOG.md`、`CONTRIBUTING.md` 与当前 Claude/Codex 实现状态不完全同步。
- CI 只有 Ubuntu 上的基础 Rust 检查，没有覆盖 v0.1 正式支持的 macOS release artifact 构建。
- release 仍依赖人工理解 tag、版本号和公告内容，缺少“合并版本 PR 后自动发布”的低心智负担路径。

本设计把上线前整备拆成四个层面：仓库卫生、开源文档、质量门禁、发布分发。它不改变 OpenMux 的账号切换业务逻辑。

## Goals / Non-Goals

**Goals:**

- 让 public GitHub 仓库只暴露 OpenMux 项目需要维护和用户需要理解的内容。
- 让新用户能在 README 中完成“理解项目、安装、试运行、查看安全边界”的闭环。
- 让贡献者能通过 CONTRIBUTING 和 CI 明确 PR 需要满足的质量门槛。
- 让维护者通过“版本号 + CHANGELOG 版本段落”的 PR 自动触发 GitHub Release 二进制和 checksum 生成。
- 让 v0.1 明确支持 macOS binary 和 `cargo install --git`，把 Homebrew、crates.io、Linux/Windows 官方二进制放入 roadmap。

**Non-Goals:**

- 不实现新的 Codex/Claude/Gemini 账号切换功能。
- 不在本变更中发布到 crates.io、Homebrew 或任何外部包管理源。
- 不在 v0.1 承诺 Linux/Windows 官方二进制。
- 不引入复杂 release train 或长期 `develop` 分支。
- 不在 CI 中执行真实 Codex/Claude 官方登录流程；真实登录仍通过手动 smoke test 覆盖。

## Decisions

### 1. 将本地 agent 工具链从 public repo surface 中移出

发布仓库应以 OpenMux 产品源码、文档、OpenSpec 规划和 GitHub automation 为主体。`.agents/`、`.claude/`、`.codex/`、`.gemini/`、`_bmad/` 等目录可以在开发者本机存在，但不应默认出现在开源仓库中，除非某个文件明确是项目贡献协议的一部分。

备选方案是保留这些目录并在 README 解释用途，但这会让新贡献者误以为必须安装特定 agent 工具链才能贡献，也会放大误提交私有配置或生成物的风险。因此选择默认移除/忽略，并保留 `AGENTS.md` 作为给本仓库协作 agent 的精简指导。

### 2. README 面向用户，docs 面向细节

README 第一屏应覆盖：

- OpenMux 是什么；
- 当前支持什么平台和能力；
- 安装方式；
- Quick Start；
- 安全边界；
- 文档入口。

长篇产品背景、架构细节和 release 流程放入 `docs/`。这样 README 不会变成 PRD，同时用户不会为了找到安装命令而读完整产品设计。

文档语言策略采用“用户入口双语、CLI 英文单语”：

- `README.md` 为英文主入口，顶部链接 `README.zh-CN.md`；
- 关键用户文档提供中文版本，例如 `docs/INSTALL.zh-CN.md`、`docs/RELEASE.zh-CN.md`、`ROADMAP.zh-CN.md`；
- CLI 输出、命令名和错误消息 v0.1 只维护英文，不做 i18n；
- OpenSpec 仍按项目规则使用中文。

### 3. GitHub Flow 与受保护 main

OpenMux v0.x 使用 GitHub Flow，不引入长期 `develop`：

- `main` 是唯一长期分支；
- 所有代码、文档和 release 准备变更都通过短分支 PR 合并到 `main`；
- 默认 squash merge，squash commit 使用 Conventional Commit；
- `main` 应启用 branch protection：required status checks、禁止 force push、禁止删除，必要时要求 PR review；
- release tag 只能由 release workflow 基于通过检查的 `main` commit 创建。

这种模型比 Git Flow 更适合当前小团队/早期 CLI 项目，避免长期分支、release branch 回合并和多环境部署心智负担。

### 4. CI 与 release 分离

`ci.yml` 负责 PR 和 `main` push 的快速反馈：

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --locked`
- v0.1 必须覆盖 macOS；Linux/Windows 可以作为 roadmap 中的平台验证探针逐步加入

`release.yml` 负责版本发布，不由维护者手工打 tag。它在 `main` push 后检查 workspace version：

- 如果 `v<version>` tag 已存在，则退出；
- 如果 tag 不存在，则要求 `CHANGELOG.md` 包含对应 `## v<version>` 段落；
- 重新执行必要 preflight checks；
- 构建 macOS arm64 和 macOS x86_64 release binaries；
- 对每个 artifact 运行 `omx --version` 和隔离目录 `omx status` 自检；
- 生成 archive 与 `SHA256SUMS`；
- 创建 tag、GitHub Release，并用 CHANGELOG 对应版本段落作为 release notes。

这样发布开关只有两个：版本号变化和 CHANGELOG 版本段落。日常 PR 不改版本号就不会发布；release PR 合并后自动发布。

### 5. v0.1 分发策略

OpenMux 是 CLI 工具，v0.1 的正式安装路径只保留两条：

- GitHub Releases macOS binary：普通用户首选；
- `cargo install --git <repo> -p omx-cli --locked`：已有 Rust 环境的开发者/备用路径。

Homebrew 和 crates.io 不进入 v0.1 必做项。原因是 Homebrew 需要稳定 release 之后维护 tap；crates.io 会过早锁定 workspace 内部 crate 的公开名称和 API。两者进入 roadmap。

### 6. 支持矩阵必须诚实

v0.1 只承诺 macOS：

- macOS Apple Silicon：supported；
- macOS Intel：supported；
- Linux：planned，源码构建可能可用，但官方 binary 和完整行为验证后再 release；
- Windows：planned，源码构建需要验证；Windows credential、权限、外部 CLI、文件替换行为通过测试后再 release。

Rust 跨平台编译不等于 OpenMux 完整产品行为跨平台。OpenMux 触碰 credential storage、外部 CLI、文件权限和 active auth 替换，必须通过平台 smoke test 后才能承诺支持。

## Risks / Trade-offs

- [Risk] 移除已跟踪 agent/BMad 文件可能影响当前开发者本机工作流。  
  Mitigation: 只从 git index/public repo surface 移除，不删除本机文件；在 `.gitignore` 中忽略这些本地工具目录；保留必要的 `AGENTS.md`。

- [Risk] release workflow 的跨平台 target 过多会增加维护成本。  
  Mitigation: v0.1 只发布 macOS arm64 和 macOS x86_64；Linux/Windows、Homebrew、crates.io、cargo deny、artifact signing/provenance 进入 roadmap。

- [Risk] release-on-version-bump 可能意外发布。  
  Mitigation: workflow 同时要求 tag 不存在、版本号变化、`CHANGELOG.md` 有对应版本段落、preflight checks 通过；没有版本段落则 fail，不创建 release。

- [Risk] README 过度承诺 Claude/Codex 外部行为。  
  Mitigation: 明确 OpenMux 包装官方 CLI，不保证 provider 私有状态；usage 查询 best-effort；真实登录建议使用隔离目录 smoke test。

- [Risk] 不发布 Linux/Windows binary 会让部分用户失望。  
  Mitigation: README 和 ROADMAP 明确 Linux/Windows 计划；文档提供 `cargo install --git` 作为源码构建备用路径，但不宣称官方支持。

## Migration Plan

1. 先更新 `.gitignore` 和 git index，移除不应公开跟踪的本地 agent/tooling 目录。
2. 补齐英文 README、中文 README、SECURITY、ROADMAP、INSTALL、RELEASE、issue/PR templates，并同步 CHANGELOG/CONTRIBUTING。
3. 更新 CI，验证本地格式/测试/clippy不受影响。
4. 新增 release-on-version-bump workflow，通过版本号和 CHANGELOG 段落自动创建 tag/release。
5. release workflow 构建 macOS arm64/x86_64 artifact，生成 `SHA256SUMS`，并运行 `omx --version` 与隔离目录 `omx status` 自检。
6. 在 release 文档中记录手动 smoke test：隔离 `OMUX_STATE_ROOT`、`CODEX_HOME`、`CLAUDE_CONFIG_DIR`，运行 `status/list/doctor`。

回滚策略：如果 release workflow 或 packaging 产生问题，可以先保留 CI 和文档修复，只暂缓 tag 发布；不需要回滚业务代码。

## Resolved Decisions

- v0.1 默认 GitHub repository URL 为 `https://github.com/hiQianFan/openmux`；后续如果迁移到组织账号，再通过独立变更更新 metadata、README badge 和安装命令。
- v0.1 macOS Intel artifact 使用 GitHub 原生 x86_64 runner 构建，避免在第一版引入交叉编译复杂度。
