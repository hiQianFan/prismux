## Why

OpenMux 仍处于开发阶段，尚未投入真实用户使用，现在是一次性完成品牌重命名的最低成本窗口。`Prismux` 能同时表达 prism 的“多色折射/收束”和 mux 的“多路选择/统一出口”，比 `OpenMux` 更适合作为面向 AI coding tools 的长期产品名。

同时，短命令 `pmx` 虽然顺手，但 GitHub、npm、PyPI 和 crates.io 已存在明显占用，其中 crates.io 上的 `pmx` 已是同类 “Claude/Codex profiles” CLI。为避免发布名被外部包管理器卡死，本变更将 `prismux` 作为唯一正式命令和发行名，`pmx` 只作为随包安装的短二进制入口，不保留 `omx` 兼容入口。

## What Changes

- **BREAKING** 将产品品牌从 `OpenMux` 改为 `Prismux`，中文短名定为“棱镜”。
- **BREAKING** CLI 正式命令从 `omx` 改为 `prismux`。
- 新增短命令入口 `pmx`，与 `prismux` 执行同一套 CLI 行为；帮助文本和文档必须以 `prismux` 为主，`pmx` 为短入口。
- **BREAKING** 移除 `omx` 二进制、Menubar helper、安装说明、release artifact 和用户文档中的公开入口，不提供迁移期兼容。
- 将 macOS app、bundle、release artifact、README、安装文档、release 文档、roadmap、PRD/architecture 中的外部品牌改为 `Prismux`。
- 将 Rust workspace 内部 crate/package/module 中的 `omx-*` 和 `OpenMux` 命名一并改为 `prismux-*` / `Prismux`，不使用 `pmx-*` 作为内部包名，不保留旧命名。
- 将用户可配置的 state/env 前缀从 `OMUX_*` 改为 `PRISMUX_*`，开发阶段不提供旧环境变量兼容。
- 将 GitHub repository 从 `hiQianFan/openmux` 改为 `hiQianFan/prismux`，并同步 Cargo metadata、README、安装文档、release workflow 和远端 URL。
- 将 `CHANGELOG.md` 纳入硬切，不保留 `OpenMux`、`openmux`、`omx` 或 `OMUX` 历史称呼，避免开发期品牌沿革造成用户理解歧义。
- 不改变 Codex/Claude/Gemini account/profile 业务语义。

## Capabilities

### New Capabilities

- `prismux-brand-and-cli-entrypoints`: 定义 Prismux 品牌、`prismux` 主命令、`pmx` 短命令、`omx` 移除、macOS bundle/release/documentation 命名和发布边界。

### Modified Capabilities

- `macos-full-bundle-distribution`: 将 macOS app、helper、archive、CLI 安装和 Menubar CLI 状态从 `OpenMux`/`omx` 改为 `Prismux`/`prismux`/`pmx`。
- `github-launch-readiness`: 将公开仓库文档、安装路径、release automation、artifact self-test 和 smoke test 示例从 `OpenMux`/`omx` 改为 `Prismux`/`prismux`/`pmx`。

## Impact

- 影响 CLI：`crates/omx-cli` 需要重命名为 `crates/prismux-cli`，binary 声明、Clap command name/about、测试中的帮助文本和文档示例都要同步。
- 影响 macOS Menubar：bundle/app 名称、helper 二进制名、symlink 创建逻辑、Settings 中的 command-line tool 文案、打包脚本和 bundle audit 脚本。
- 影响 release：artifact 文件名、archive 内容、`SHA256SUMS` 说明、GitHub Release notes、安装文档。
- 影响文档：`README*`、`docs/INSTALL*`、`docs/RELEASE*`、`docs/BUILD.md`、`docs/PRD.md`、`docs/ARCHITECTURE.md`、`docs/menubar-v1.md`、`CHANGELOG.md`、`CONTRIBUTING.md`、`ROADMAP*`。
- 影响 GitHub 远端：实现完成后需要执行 `gh repo rename prismux`，再执行 `git remote set-url origin https://github.com/hiQianFan/prismux.git` 或对应 SSH URL。
- 不影响 Prismux 管理的账号池数据结构、auth snapshot 内容、provider plugin 行为和现有业务命令语义；但 FFI/schema/package 中的 `omx` 命名需要同步改掉。
- 实现必须遵循仓库 `AGENTS.md`：使用 stable Rust 验证链路，手动 CLI 检查使用隔离的 `PRISMUX_STATE_ROOT`/`CODEX_HOME`，不打印 token 或 raw auth payload，CLI 保持薄展示层，provider/path/snapshot 行为继续归属 plugin/core crates。
