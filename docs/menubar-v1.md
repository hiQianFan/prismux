# Menubar v1

OpenMux Menubar v1 是 macOS 14+ Apple Silicon 的原生菜单栏账号控制面板。它显示 active account、账号池、quota/status、手动 refresh、显式 switch，以及最小 today usage 摘要。

v1 不提供登录、导入、删除、alias 编辑、自动最佳账号选择、account usage attribution、完整 analytics dashboard、Sparkle 更新或 notarization 自动化。这些管理动作继续使用 CLI，例如 `omx login codex`、`omx import codex`、`omx use codex <selector>`、`omx usage`。

技术边界：

- AppKit 管理 `NSStatusItem`、`NSPopover`、accessory lifecycle 和 teardown。
- SwiftUI 只承载 popover content。
- Swift 只通过 `omx_menubar_call` / `omx_menubar_free` 调用 Rust staticlib，不读取 auth、SQLite、usage logs 或 provider endpoint。
- Rust backend 遵守 `OMUX_STATE_ROOT`、`CODEX_HOME` 等 override，方便隔离测试。
- TokenBar 仅作为 `NSStatusItem` lifecycle、popover chrome 和紧凑状态层级参考；不复制源码、资源、bundle ID、cache、scanner、pricing、quota fetcher 或 report DTO。

本地构建：

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

`scripts/bundle-menubar.sh` 从 Cargo workspace version 生成 `CFBundleShortVersionString`，写入 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`，并执行 ad-hoc codesign。

