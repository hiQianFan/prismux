# Menubar v1

OpenMux Menubar v1 是 macOS 14+ Apple Silicon 的原生菜单栏账号控制面板。它显示 active account、账号池、quota/status、手动 refresh、显式 switch，以及最小 today usage 摘要。

公开分发形态采用 macOS full bundle：`OpenMux.app` 内置 Menubar 和同版本 `omx` CLI helper。用户需要 Terminal 命令时，通过 Menubar 显式点击 `Enable omx command` 创建 symlink；App 不静默修改 PATH 或 shell 启动文件。

v1 不提供自动最佳账号选择、account usage attribution、完整 analytics dashboard、Sparkle 更新或 notarization 自动化。低频/高风险管理动作必须由用户显式触发；如果某个动作仍是 CLI-only，Menubar 只提供 CLI handoff 或 copyable command。

技术边界：

- AppKit 管理 `NSStatusItem`、`NSPopover`、accessory lifecycle 和 teardown。
- SwiftUI 只承载 popover content。
- Swift 只通过 `omx_menubar_call` / `omx_menubar_free` 调用 Rust staticlib，不读取 auth、SQLite、usage logs 或 provider endpoint。
- Rust backend 遵守 `OMUX_STATE_ROOT`、`CODEX_HOME` 等 override，方便隔离测试。
- App bundle 内置 helper 路径为 `OpenMux.app/Contents/MacOS/omx`；PATH 中的 `omx` 只应是指向 helper 的 symlink 或用户明确选择的 standalone CLI。
- TokenBar 仅作为 `NSStatusItem` lifecycle、popover chrome 和紧凑状态层级参考；不复制源码、资源、bundle ID、cache、scanner、pricing、quota fetcher 或 report DTO。

## 交互链路

1. 用户从 GitHub Releases 下载 `OpenMux.app` archive。
2. 用户解压并拖到 `/Applications`。
3. 用户打开 `OpenMux.app`，Menubar 显示 dashboard。
4. 如果已有账号/profile，dashboard 展示 Overview、provider tabs、active target、quota/status 和 usage 摘要。
5. 如果没有账号/profile，dashboard 提供 `Sign in`、`Use existing login`、`Import profile` 等 onboarding actions。
6. Settings/General 的 command-line tool 分组显示 CLI 状态：
   - `CLI ready`：PATH 中 `omx` 可用且版本匹配。
   - `CLI not configured`：提供 `Enable omx command` 和 copyable command。
   - `Different omx found`：保留外部 CLI，并显示手动处理说明。
7. 用户点击 `Enable omx command` 时，Menubar 创建：

   ```text
   ~/.local/bin/omx -> /Applications/OpenMux.app/Contents/MacOS/omx
   ```

8. 如果 `~/.local/bin` 不在 PATH，Menubar 显示可复制命令，不自动写 `.zshrc`：

   ```sh
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
   ```

9. 用户在 Terminal 验证：

   ```sh
   omx --version
   omx status
   ```

## 发布验收

- release workflow 上传 `OpenMux-vX.Y.Z-macos-<arch>.zip`，archive 内包含 `OpenMux.app`。
- bundle script 将 `target/release/omx` 放入 `Contents/MacOS/omx` 并校验版本一致。
- Settings/General 展示 bundled helper path、helper version、PATH 中 `omx` 的路径/版本、`Enable omx command`、`Copy PATH command`。
- Dashboard 空状态提供 onboarding actions，footer 用状态串和 `...` 菜单提供 CLI handoff。
- bundle audit 必须确认不含 raw auth、token、API key、raw provider log 或未批准第三方数据引擎。
- Windows/Linux packaging 另开提案，只共享 schema/CLI 语义，不复用 macOS `.app` layout。

本地构建：

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

`scripts/bundle-menubar.sh` 从 Cargo workspace version 生成 `CFBundleShortVersionString`，写入 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`，并执行 ad-hoc codesign。
