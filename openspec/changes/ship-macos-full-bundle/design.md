## Context

当前 OpenMux 已经具备：

- `omx` Rust CLI，覆盖 login/save/list/use/import/alias/remove/doctor/usage。
- `OpenMux Menubar` SwiftPM app，通过 `omx-menubar-ffi` 调用 Rust control-plane。
- `scripts/build-menubar.sh` 和 `scripts/bundle-menubar.sh`，可本地生成 ad-hoc signed `.app`。
- control-plane compatibility model 已经有 `cli_only`、`menubar_only`、`full_bundle` 和 `embedded_staticlib`、`helper_binary`、`installed_cli` 的概念。

缺口是公开分发与首次使用体验：文档仍以 CLI-only archive 为主，Menubar 只作为本地 bundle 验证；用户不知道下载哪个包、CLI 如何进入 PATH、Menubar 与 CLI 是否同版本。

本设计采用一个 macOS full bundle 作为第一公开 GUI 分发路径，避免两个独立包互相下载带来的版本、权限、校验和回滚复杂度。

本次重新审查公开仓库后，还发现上线准备不是单纯 packaging 问题：

- `docs/INSTALL.md` / `docs/RELEASE.md` 已开始描述 full bundle，但 README、ROADMAP、CHANGELOG 仍保留 CLI-only/v0.1 口径。
- `.github/workflows/release.yml` 仍只构建 CLI tarball，没有发布 `OpenMux.app`。
- Menubar 还没有 `Install CLI` / PATH status 交互。
- `crates/omx-app/src/about.rs` 中 repository link 与 README/Cargo metadata 不一致。
- 公开文档已有贡献、安全、许可证和 vendor notes，但缺少面向外部二次开发者的完整 source build 文档，尤其是 Xcode/SwiftPM/Menubar bundle 要求。

对照 OpenSSF Best Practices passing criteria，本阶段必须至少满足：项目用途、获取方式、反馈/贡献方式、许可证、基础文档、外部接口文档、公开可读源码仓库、唯一版本号、release notes、bug/security report 流程、可运行构建系统、自动测试和 CI。OpenMux 已覆盖其中大部分基础文件，剩余风险集中在口径不一致、release artifact 未打通、source build 文档不完整和发布前 secret/license audit 未固化。

## Decisions

### 0. 公开 bundle identity 统一为 `OpenMux.app`

当前本地 bundle script 输出 `OpenMux Menubar.app`，且 `CFBundleExecutable` / `CFBundleName` 使用 `OpenMux Menubar`。这对本地验证可用，但不适合作为公开 release artifact：README、INSTALL、release workflow、smoke test 和用户拖入 `/Applications` 的路径都会与 full bundle 口径分叉。

本 change 将公开 identity 固定为：

```text
OpenMux.app/
  Contents/MacOS/OpenMux
  Contents/MacOS/omx
```

`OmxMenubarApp` 仍可以是 SwiftPM target name，但 bundle assembly SHALL 将可执行文件安装为 `Contents/MacOS/OpenMux`，并把 `CFBundleExecutable`、`CFBundleName`、archive 文件名和文档示例统一到 `OpenMux` / `OpenMux.app`。现有 `OpenMux Menubar.app` 默认路径、audit 脚本和 version check 脚本必须同步迁移；否则 release workflow 即使上传了 app zip，也会继续产出旧产品名。

### 1. macOS 第一公开分发产物是 full bundle

GitHub Release SHALL 提供一个 macOS app archive，例如：

```text
OpenMux-macos-universal-vX.Y.Z.zip
```

第一阶段可以继续由 CI 分别在 Apple Silicon / Intel runner 构建验证；最终 artifact 可以先按架构拆分：

```text
OpenMux-vX.Y.Z-macos-arm64.zip
OpenMux-vX.Y.Z-macos-x86_64.zip
```

如果 universal lipo 合并成本低，再合并为 universal。不要为了文件名一次性引入复杂 universal build pipeline。

### 2. App bundle 内置同版本 CLI helper

Bundle 结构：

```text
OpenMux.app/
  Contents/
    Info.plist
    MacOS/
      OpenMux
      omx
    Resources/
      ...
```

`Contents/MacOS/omx` 是 bundled CLI helper。选择 `Contents/MacOS` 的原因：

- Apple bundle 文档明确将 executable 和 command-line tools 放在 `Contents/MacOS`。
- `omx` 是可执行代码，不是 `Resources`。
- `Contents/Helpers` 是可见的社区惯例，但对 OpenMux 没有必要；少一个自定义目录更好解释。

发布脚本 SHALL 验证：

- `OpenMux.app/Contents/MacOS/OpenMux` 可执行。
- `OpenMux.app/Contents/MacOS/omx` 可执行。
- `OpenMux.app` 的 `CFBundleShortVersionString` 等于 Cargo workspace version。
- `OpenMux.app/Contents/MacOS/omx --version` 输出同一版本。
- 打包后的 zip 解压后仍保留两个 executable 的权限、bundle layout 和 symlink target。

### 3. PATH 安装只创建 symlink，且必须由用户显式触发

Menubar SHALL NOT 首次启动静默安装 CLI，也 SHALL NOT 自动修改 `.zshrc`、`.bashrc` 或 shell profile。

Settings/Tools 中的 `Install CLI` 默认创建：

```text
~/.local/bin/omx -> /Applications/OpenMux.app/Contents/MacOS/omx
```

如果 `~/.local/bin` 不在当前 login shell PATH，UI 显示可复制命令：

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

高级安装位置可以后续支持：

```text
/opt/homebrew/bin/omx
/usr/local/bin/omx
```

这些位置不可写时只展示 guidance；需要管理员权限的 AppleScript/Authorization Services 安装不是第一阶段必需项。

第一阶段固定只写 `~/.local/bin/omx`。如果该路径已经存在且不是指向 bundled helper 的 symlink，Menubar SHALL NOT overwrite；UI 只显示不同 `omx` 已存在和手动处理 guidance。创建 symlink 时可以先确保 `~/.local/bin` 存在，但不得复制 credential、state、snapshot 或 backup 文件。

### 4. 不做互相自动下载

CLI 中不实现“自动下载 Menubar”；Menubar 中也不下载 standalone CLI。原因：

- 需要解析 release API、架构、checksum、权限和回滚。
- 网络失败或 GitHub rate limit 会把首次使用链路复杂化。
- full bundle 已经包含 CLI，下载器当前没有价值。

CLI 可以提供低风险入口，例如后续 `omx app` 打开 release URL 或打印安装说明；第一阶段文档即可。

### 5. Menubar 交互链路

Menubar 初次使用链路：

1. 用户下载 `OpenMux.app` archive。
2. 用户解压并拖到 `/Applications`。
3. 用户打开 App，系统可能提示未公证/未知开发者；文档说明 v0.x 暂不提供 notarization。
4. Menubar 打开 dashboard：
   - 有账号：展示 Overview、provider tabs、active target、quota/status、usage summary。
   - 无账号：展示 onboarding actions，例如 `Sign in`、`Use existing login`、`Import profile`。
5. Dashboard footer 或 Settings/Tools 展示 CLI status：
   - `CLI ready`：PATH 中 `omx` 指向 bundled helper 或同版本 binary。
   - `CLI not installed`：显示 `Install CLI`。
   - `CLI version mismatch`：显示 `Update CLI link` 或 guidance。
6. 用户点 `Install CLI` 后，Menubar 创建 symlink 到 bundled helper。
7. 用户在 Terminal 运行：

   ```sh
   omx --version
   omx status
   ```

8. 高风险/低频管理动作仍可通过 CLI 完成；Menubar 可以复制命令或打开 Terminal，但不隐式执行用户未确认的 credential switching。

### 6. Settings/UI 需要的最小内容

新增或调整 Settings 区域：

- `Tools` 或 `CLI` section：
  - Bundled CLI path。
  - Installed CLI path/status。
  - Installed CLI version。
  - `Install CLI` / `Update CLI link`。
  - `Copy PATH command`。
- `About` section：
  - App version。
  - CLI helper version。
  - State root。
  - Release/Docs links。

Dashboard 空状态需要减少纯说明文案，改为操作入口：

- Codex 无账号：`Sign in`、`Use existing login`。
- Provider 无 profile：`Import profile`。
- CLI 未安装：footer/Tools 显示 `Install CLI`，不要在主 dashboard 挤占账号操作空间。

### 7. Cross-platform boundary

本变更只设计 macOS package。Windows/Linux 后续 SHALL 使用独立 packaging proposal。它们只需要共享：

- product version。
- state schema。
- account/profile selector semantics。
- CLI command behavior。
- safe diagnostics 和 auth redaction policy。

它们不需要共享 `.app/Contents/MacOS` layout。

### 8. Public GitHub readiness 是 release gate

OpenMux 首个 full bundle release 不能只靠“代码能跑”。release PR 合并前 SHALL 有一个 public readiness checklist，覆盖：

- README 首屏：项目定位、macOS full bundle 下载、CLI helper 安装、支持平台、支持 provider、安全边界、文档入口。
- INSTALL：下载 app、拖到 `/Applications`、Gatekeeper/未公证说明、`Install CLI`、PATH symlink、卸载和 state 清理。
- RELEASE：version bump、CHANGELOG 版本段落、workflow 产物、bundle layout、smoke tests、失败回滚。
- ROADMAP：macOS full bundle 已完成/进行中，Homebrew、Sparkle、Developer ID/notarization、Linux/Windows、standalone CLI tarball 明确为 later。
- CHANGELOG：使用 `## Unreleased` 和 `## vX.Y.Z - YYYY-MM-DD`，版本段落说明 Menubar/full bundle、CLI helper、已知限制和安全行为。
- CONTRIBUTING：Rust + Swift/Menubar build/test 命令、GitHub Flow、PR checklist、credential safety、文档更新要求。
- SECURITY：private vulnerability reporting、禁止公开粘贴 auth/token/snapshot/backups、支持版本和响应预期。
- LICENSE：根目录保留 MIT license，Cargo metadata 保持一致。
- Vendor/third-party notes：vendored tokscale 和参考项目边界清楚；CodexBar 只作为 packaging/UX 参考，不复制源码或资源。
- Source build：外部贡献者能从 GitHub source archive 执行 Rust checks、Swift build、bundle smoke；如果需要 full Xcode，文档必须写明。

不为第一版增加大型治理文件。`CODE_OF_CONDUCT.md` 可以采用 Contributor Covenant 或先在 CONTRIBUTING 中声明基本行为期望；如果开放外部社区贡献，独立文件更清晰。`SUPPORT.md`、funding、website、badges、OpenSSF badge、cargo deny、artifact signing/provenance 都是后续增强，不阻塞第一个可用 release。

### 9. Release workflow 必须和文档一致

当前 release workflow 构建 CLI-only tarball；这与 full bundle 文档冲突。第一版 public release SHALL 选择一种口径并让 workflow 实际产出相同 artifact。本 change 固定为 full bundle：

- preflight 运行 Rust fmt/test/clippy。
- build job 构建 `omx`、构建 Menubar、运行 Swift contract tests；runner 必须具备 full Xcode，不能只依赖 CommandLineTools。
- bundle job 组装 `OpenMux.app`，把 `omx` 放入 `Contents/MacOS/omx`。
- smoke job 解压 release zip 后，使用 isolated state 运行 bundled helper `omx --version` 和 `omx status`，并验证 `Contents/MacOS/OpenMux` 可执行。
- package job 上传 `OpenMux-vX.Y.Z-macos-arm64.zip` 和 `OpenMux-vX.Y.Z-macos-x86_64.zip`，或在 universal build 被证明简单后上传 universal zip。
- publish job 上传 `SHA256SUMS`，GitHub Release notes 使用 CHANGELOG 对应版本段落。

GitHub 自动生成的 source archive 不需要额外上传，但 release notes 和 INSTALL SHALL 明确“from source”路径：`cargo install --git ... -p omx-cli --locked` 只安装 CLI；完整 Menubar source build 使用 `scripts/build-menubar.sh` / `scripts/bundle-menubar.sh`。

## Risks / Trade-offs

- [Risk] 用户以为安装 App 后 Terminal 自动有 `omx`。  
  Mitigation: 首次打开和 Settings 明确显示 CLI status，并提供一键 symlink。

- [Risk] `~/.local/bin` 不在 PATH。  
  Mitigation: 显示 `Copy PATH command`，不自动改 shell config。

- [Risk] v0.x 未公证导致 macOS Gatekeeper friction。  
  Mitigation: 文档明确当前 release 不含 notarization；正式广泛分发前再做 Developer ID/notarization。

- [Risk] App 内 helper 与 PATH 中旧 CLI 版本分叉。  
  Mitigation: Menubar 检查 installed `omx --version`，发现 mismatch 显示 `Update CLI link`。

## Open Questions

- 第一版是否做 universal app，还是先发布 arm64/x86_64 两个 app zip。
- 未公证 App 的安装说明放 README 还是 INSTALL；建议 INSTALL 详细写，README 只给入口。
- 是否新增独立 `docs/BUILD.md`；建议新增，避免 README/CONTRIBUTING 承载过多 source build 细节。
- 是否新增 `CODE_OF_CONDUCT.md`；建议在接受外部 PR 前新增轻量版本，第一版至少在 CONTRIBUTING 中写明基本行为期望。
