# 任务

## 1. Contract 与后端

- [x] 1.1 扩展 `BackendClient.Payload`，新增 `settings_view`、`update_settings`、`about_view`、`support_report` op。
- [x] 1.2 在 `crates/omx-menubar-ffi` 接入上述 op，保持 versioned envelope。
- [x] 1.3 在 `crates/omx-app/src/settings.rs` 将 `SettingsView` 调整为 General/Providers/Privacy 分组，只包含 refresh cadence、provider enablement、provider source preference、privacy。
- [x] 1.4 实现 Rust settings 持久化：schema version、atomic write、private perms、future schema fail closed、parse failure safe diagnostic。
- [x] 1.5 新增 `AboutView` DTO，暴露 app version、schema versions、runtime mode（embedded staticlib/unavailable）、state root display/reveal path、links。
- [x] 1.6 复用 `support.rs`，确保 support report 统一 redaction。
- [x] 1.7 不新增通用 provider settings descriptor；第一阶段使用固定 provider settings DTO。

## 2. Swift DTO 与 Store

- [x] 2.1 新增 Swift `SettingsView`、`AboutView`、`SupportReport` DTO，并保持在现有 `DTO.swift` 聚合文件中。
- [x] 2.2 新增 `MenubarSettingsStore`，负责 load/update settings、load about、copy support report。
- [x] 2.3 将 menubar 当前 `@AppStorage` 的 refresh cadence 迁移到 backend settings，tray visual display mode 继续保留为 Swift local preference。
- [x] 2.4 保留 selected tab/provider selection 为 Swift-local UI state。

## 3. Settings UI

- [x] 3.1 新增 `MenubarSettingsWindowController`，保证单实例 window。
- [x] 3.2 新增 `MenubarSettingsView`，使用 macOS sidebar 分组：General、Providers、About。
- [x] 3.3 新增 General pane：refresh cadence、privacy toggle、tray display local preference。
- [ ] 3.4 新增 Provider pane：provider status、enabled、source preference、diagnostics；refresh action、CLI help 暂未落地。
- [x] 3.5 新增 About pane：版本、schema、runtime、state root、links、copy version info、copy redacted support report。
- [x] 3.6 新增局部 shared row 组件：`SettingsSection`、`SettingsKeyValueRow`、`PathRow`、`SettingsBanner`。

## 4. Popover 入口

- [x] 4.1 顶部齿轮按钮打开 Settings window，而不是只展示小 `Menu`。
- [ ] 4.2 Settings menu 至少提供 `Settings...` 和 `About OpenMux...`。
- [ ] 4.3 About 入口打开 Settings window 并选中 About tab。
- [x] 4.4 Quit 保持在 popover/footer，不放入 Settings。
- [x] 4.5 隐私开关同步影响 overview active target、account/profile row、delete confirmation 的显示标签。

## 5. 验证

- [x] 5.1 Rust 添加 settings validation/persistence tests。
- [ ] 5.2 Rust 添加 about/support redaction tests。
- [ ] 5.3 Swift 添加 Settings DTO decode contract tests。
- [ ] 5.4 Swift 添加 MenubarSettingsStore update failure rollback tests。
- [x] 5.5 运行 `swift build`。
- [x] 5.6 若修改 Rust，运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`。
