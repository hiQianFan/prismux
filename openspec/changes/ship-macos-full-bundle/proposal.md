## Why

OpenMux 现在已经有 Rust CLI、Swift Menubar、embedded Rust FFI 和本地 bundle 脚本，但公开分发口径仍停留在 CLI-only archive。用户如果从 GitHub Releases 下载后还要理解 CLI、Menubar、本地脚本和 PATH 配置，会增加首次使用成本，也容易出现 CLI 与 Menubar 版本分叉。

参考 CodexBar、VS Code、Docker Desktop、1Password 和 JetBrains Toolbox 的安装模式后，本阶段最小可靠方案不是把 CLI 与 Menubar 做成两个互相下载的产物，而是发布一个 macOS full bundle：`OpenMux.app` 内置 Menubar 和同版本 `omx` CLI helper，用户显式点击 `Install CLI` 后只创建 PATH symlink。

这样普通用户下载一个 App 即可使用 Menubar，高级用户仍能把同 bundle 内的 `omx` 安装到 Terminal；后续 Linux/Windows、Homebrew、Sparkle/notarization 和 standalone CLI tarball 可以独立演进，不被 macOS `.app` 结构绑死。

## What Changes

- 将 macOS GitHub Release 主产物从 CLI-only archive 调整为 `OpenMux.app` full bundle。
- 公开发布的 app bundle 名称 SHALL 统一为 `OpenMux.app`；现有本地脚本中的 `OpenMux Menubar.app` 只可作为迁移前内部名称，不进入 release artifact、文档或 smoke test 口径。
- App bundle SHALL 包含 Menubar executable 和同版本 `omx` helper，helper 放在 `OpenMux.app/Contents/MacOS/omx`。
- Menubar SHALL 在首次打开和 Settings 中展示 CLI 安装状态，并提供显式 `Install CLI` 操作。
- `Install CLI` SHALL 默认创建 symlink，不复制 auth/state，不静默修改 shell rc 文件。
- release workflow SHALL 打包、校验并上传 full bundle archive 和 `SHA256SUMS`。
- 文档 SHALL 明确 macOS full bundle 是当前主路径；Linux/Windows packaging 后续独立设计。
- 上线前 SHALL 完成 public GitHub readiness 收口：README/INSTALL/RELEASE/ROADMAP/CHANGELOG 口径一致，贡献、安全、许可证、源码构建、二次开发和第三方来源说明可被外部用户理解。
- release notes SHALL 来自人工维护的 `CHANGELOG.md` 版本段落，不使用 raw git log 作为公告。
- 公开仓库 SHALL 保持 source package 可构建：GitHub tag 自动生成的 source archive 必须包含 Cargo workspace、SwiftPM app、vendor notes、构建脚本、测试命令和必要文档。
- 不改变 registry schema、账号/profile switching 语义或 provider plugin 安全边界。

## Non-Goals

- 不在本变更中实现 Sparkle、appcast、Developer ID signing、notarization 自动化或 Homebrew cask。
- 不在本变更中发布 Linux/Windows official binaries。
- 不实现 CLI 自动下载 Menubar 或 Menubar 自动下载 CLI 二进制。
- 不静默写入 `/usr/local/bin`、`/opt/homebrew/bin`、`.zshrc`、`.bashrc` 或 shell profile。
- 不把 Windows/Linux packaging 设计为 macOS App bundle 的变体。

## Capabilities

### New Capabilities

- `macos-full-bundle-distribution`: macOS full bundle 发布、内置 CLI helper、显式 CLI symlink 安装、Menubar 交互链路和发布验收要求。

### Modified Capabilities

- `github-launch-readiness`: 后续实现时需要从 CLI-only v0.1 artifact 口径调整为 macOS full bundle 主路径。
- `menubar-distribution`: 后续实现时需要从本地 bundle/manual distribution 扩展为 GitHub Release artifact。

## Impact

- 影响 release workflow：需要构建 `omx` CLI、构建 Menubar、组装 `.app`、把 CLI helper 放入 bundle、运行 bundle smoke test、生成 archive/checksum。
- 影响 scripts：`scripts/bundle-menubar.sh` 需要输出 `OpenMux.app`，复制 `target/release/omx` 到 `Contents/MacOS/omx`，并验证两个 executable version 一致。
- 影响 Menubar UI：Settings/About 或 Tools/General 区域需要展示 CLI status、Install CLI/Enable omx command、Copy PATH command 和 release links；若与 `redesign-menubar-settings-footer-experience` 同时实施，以该 change 的 Settings/Footer 信息架构为准，本 change 只要求 full bundle 必须有显式 symlink 配置入口。
- 影响 README、INSTALL、RELEASE、ROADMAP、CHANGELOG、CONTRIBUTING、SECURITY、Menubar 文档和可能新增的 `docs/BUILD.md` / `CODE_OF_CONDUCT.md`。
- 不影响核心账号数据格式；full bundle 仍共享同一个 OpenMux state root。
