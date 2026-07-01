## Context

当前公开身份是 `OpenMux` / `omx`。项目仍在开发阶段，没有真实用户迁移负担，因此不需要保留 `omx` 兼容入口。

命名调研截至 2026-07-01：

- `Prismux`：GitHub 仓库仅发现 `deepikad04/PrismUX`，1 star；GitHub `PRISMUX` organization 与 `prismux.com` 已被占用；npm、PyPI、crates.io 未发现精确包占用；`prismux.dev` 未注册。
- `pmx`：GitHub、npm、PyPI 均拥挤；crates.io 已有同类 CLI `pmx = 0.1.0`，描述为管理 Claude/Codex profiles。
- `PrismX`：GitHub 已有 `yqcs/prismx`，约 826 stars，中文描述为“棱镜 X”，且 npm 已有多个 `prismx` scope/package；中文语境撞名明显。

`Prismux` 的主要歧义是尾部 `UX`：如果写成 `PrismUX`，容易被理解为 UX/design 产品。采用 `Prismux` 统一大小写可以降低这个问题。

## Goals / Non-Goals

**Goals:**

- 将公开品牌硬切为 `Prismux`。
- 将正式 CLI 命令硬切为 `prismux`。
- 提供 `pmx` 短命令入口，但不把 `pmx` 作为包发布名或唯一文档入口。
- 完全移除公开 `omx` 入口，不保留迁移期兼容。
- 同步 macOS bundle、release artifact、安装文档、README、release 自检和 Menubar CLI 安装体验。

**Non-Goals:**

- 不重写账号/profile 业务逻辑。
- 不在本变更中发布到 npm、PyPI、crates.io 或 Homebrew。
- 不改变账号/profile 业务语义、auth snapshot 内容或 provider plugin 行为。
- 不提供 `omx`、`OpenMux` 或 `OMUX_*` 兼容入口。
- 不在 `CHANGELOG.md` 中保留旧品牌历史叙述。

## Decisions

### 1. 使用 `Prismux`，不使用 `PrismUX` 或 `PrismX`

`Prismux` 保留 `prism + mux` 的语义，技术和品牌都能解释。展示时统一写作 `Prismux`，避免 `UX` 大写造成“用户体验工具”的误读。

`PrismX` 更短，但现有中文/安全工具项目占用强，且“X”语义泛化，不如 `mux` 贴合当前产品。

### 2. `prismux` 是正式命令，`pmx` 是同包短入口

`pmx` 好记，但不能作为正式发布名：crates.io 已有同类 `pmx`，npm/PyPI 也有历史包。正式文档、release artifact 和安装说明以 `prismux` 为准；`pmx` 作为同一 binary 的短入口，适合日常输入。

内部 crate/package/module 使用 `prismux-*`，不使用 `pmx-*`。`pmx` 只存在于二进制命令入口层，避免把高冲突短名扩散到包发布和源码结构。

实现上优先用 Cargo 多 binary name 指向同一 CLI 入口，避免复制逻辑。帮助文本以被调用的 binary 名称显示，若实现成本过高，则统一显示 `prismux`。

### 3. 不保留 `omx`

项目尚未投入使用，保留 `omx` 会让文档、Menubar helper、release artifact 和安装脚本出现三套入口。删除 `omx` 是当前最低成本路径。

内部 crate、module、FFI symbol、schema name、state env 也同步从 `omx` / `OpenMux` / `OMUX` 改到 `prismux` / `Prismux` / `PRISMUX`。因为项目仍处于开发阶段，不保留旧命名兼容。

`CHANGELOG.md` 也按当前品牌重写，不保留旧品牌名作为历史记录。项目尚未公开投入使用，历史旧称比沿革说明更容易让用户误以为存在两个产品或旧命令兼容。

### 4. 中文短名使用“棱镜”

中文短名定为“棱镜”。常用展示采用：

- `棱镜（Prismux）`
- `Prismux 棱镜`
- 必要时补充“AI coding tools 的账号与 profile 切换器”

不采用“棱枢”“光枢”“棱切”等生造短名；它们要么拗口，要么语义成本高。`棱镜` 足够直观，也能直接承接 prism 的品牌隐喻。

### 5. GitHub repository 改为 `hiQianFan/prismux`

实现完成并验证后，GitHub repository SHALL 从 `hiQianFan/openmux` 重命名为 `hiQianFan/prismux`。代码中的 repository metadata、安装命令和 release 链接先同步到新 URL，再执行远端 rename 和本地 `origin` 更新。

## Risks / Trade-offs

- [Risk] `Prismux` 的 `UX` 被误读为设计产品。  
  Mitigation: 全部文档和 UI 使用 `Prismux`，不使用 `PrismUX`；定位文案明确 `prism + mux`。

- [Risk] `pmx` 与外部命令冲突。  
  Mitigation: `pmx` 只作为可选短入口；正式命令和文档主路径使用 `prismux`；Menubar 安装时检测已有 `pmx`，不静默覆盖非本包 symlink。

- [Risk] 完全移除 `omx` 会让本地开发脚本失效。  
  Mitigation: 同一变更中更新脚本、docs、tests 和 release workflow；不保留过渡脚本。

- [Risk] 全仓重命名触碰面大，容易漏掉 build scripts、FFI symbol 或文档示例。  
  Mitigation: 使用 `rg -n "OpenMux|openmux|OMUX|omx"` 作为收尾门禁，只允许历史 changelog 或明确第三方 URL 中保留旧词。

## Migration Plan

1. 重命名 workspace package/crate/module 路径，从 `omx-*` 改为 `prismux-*`。
2. 调整 CLI binary 声明，生成 `prismux` 和 `pmx`，删除 `omx`。
3. 更新 Clap command name/about/help tests。
4. 更新 state/env 前缀为 `PRISMUX_*`，删除 `OMUX_*` 支持。
5. 更新 Menubar bundle：`Prismux.app`、`Contents/MacOS/Prismux`、`Contents/MacOS/prismux`、`Contents/MacOS/pmx`。
6. 更新 Menubar Settings 的 CLI install/status 逻辑，创建/检测 `prismux` 与 `pmx`，不再处理 `omx`。
7. 更新 build/bundle/audit/release 脚本和 GitHub workflow。
8. 更新 README、安装文档、release 文档、PRD、architecture、roadmap、changelog；`CHANGELOG.md` 不保留旧品牌历史命名。
9. 将 repository metadata 和安装命令改为 `https://github.com/hiQianFan/prismux`。
10. 运行 Rust 与 Menubar 验证。
11. 验证通过后执行 `gh repo rename prismux`，再更新本地 `origin` remote。

Rollback 策略：开发阶段无需用户迁移；如命名最终反悔，按同一清单再做一次硬切，不引入兼容层。
