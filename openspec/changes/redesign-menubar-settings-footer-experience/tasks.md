## 1. 后端链接与 About DTO

- [x] 1.1 将 `crates/omx-app/src/about.rs` 中的 repository/docs URL 统一为 `https://github.com/hiQianFan/openmux`。
- [x] 1.2 在 About DTO 中补齐 Issues、Releases 链接，保持 Swift 只渲染 backend 输出的 links。
- [x] 1.3 在 About DTO 中新增 author links：GitHub `https://github.com/hiQianFan`、Website `https://blog.mapin.net/`、Twitter/X `https://x.com/hiQianFan`；email 未配置时不输出空链接。
- [x] 1.4 确认 `Copy Version Info` 和 `Copy Redacted Support Report` 包含 version/schema/runtime/state root，但不暴露 raw auth、token、API key、snapshot、backup 或 private account file content。

## 2. Settings 信息架构

- [x] 2.1 将 `MenubarSettingsView` 从 `NavigationSplitView` + sidebar 改为顶部 tab 风格导航，三个 pane 统一用原生 `Form` + `.formStyle(.grouped)`，删除自造的 `SettingsSection`/`SettingsKeyValueRow`/大部分 `PathRow`。
- [x] 2.2 固定 tab：`General`、`Providers`、`About`；不设独立 `Display`/`Tools` tab，也不保留空壳 `General`。
- [x] 2.3 从 Settings UI 移除 background refresh cadence 分段控件，刷新策略改为产品默认和 backend cooldown/TTL/backoff。
- [x] 2.4 在 `General` 组建分组：Appearance（tray display）、Privacy（hide personal identifiers）、Startup（launch at login）、Command-line tool。
- [x] 2.5 在 `General` Startup 分组实现 launch at login，使用 `SMAppService.mainApp` register/unregister，反映真实注册状态。
- [x] 2.6 `Providers` 展示 provider enabled、source policy、status 和 diagnostics；source policy 仅在用户能理解差异时显示，解释文案收进 `?` help popover 而非常驻正文。
- [x] 2.7 在 `Providers` 中**只读**展示 provider profile/gateway 脱敏摘要（`LabeledContent`）：active profile、profile count、base URL host、model、auth type。
- [x] 2.8 `Providers` 提供 `Manage in dashboard →` 跳转；import/use/remove 等 mutation 不在 Settings 重复实现（已在 dashboard `ProfileTargetRow` + import overlay 完成）。
- [x] 2.9 移除或避免可见展示 control-plane/state/settings schema 数字；它们只进入 support/version copy 内容。

## 2a. Shared Profile / Proxy Source

- [ ] 2a.1 为 Menubar 增加或复用 Rust control-plane API，读取与 CLI `omx list` 相同来源的 `ConfigProfile` metadata。
- [ ] 2a.2 Menubar profile import/use/remove 由 dashboard 现有入口承担，复用 plugin `import_config`、`use_target`、`remove_target` 语义；Settings 不另起 mutation 路径，CLI 与 Menubar 操作后看到的 profile 列表必须一致。
- [x] 2a.3 Swift DTO 只包含脱敏字段：name、provider id、base URL、model、auth type、active、display/reveal path；不得包含 API key、auth token、snapshot bytes 或 raw profile content。
- [x] 2a.4 区分 provider gateway/profile 与 HTTP network proxy：UI 文案中 `Gateway profile` / `API profile` 放在 `Providers`，`Network proxy` 放在 `General` 的 command-line tool 分组。
- [x] 2a.5 `General` 的 command-line tool 分组至少展示当前有效 network proxy 来源：shared settings、`OMUX_HTTPS_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` 或 none。
- [ ] 2a.6 如实现持久 network proxy 编辑，必须通过 Rust `settings_view/update_settings` 或等价 shared settings schema；不得只写 Menubar `UserDefaults`。

## 3. Command-line tool 分组 UX（在 General 内，非独立 tab）

- [x] 3.1 在 `General` 增加 Command-line tool 分组，显示 bundled helper 路径 `OpenMux.app/Contents/MacOS/omx`。
- [x] 3.2 检测 bundled helper 是否存在、是否可执行，并显示 helper version 或 unavailable 状态。
- [x] 3.3 检测 PATH 中的 `omx`，区分 `Ready`、`Not configured`、`Different omx found`。
- [x] 3.4 提供 `Copy manual command`，复制 `mkdir -p "$HOME/.local/bin"` 和 `ln -sf ... "$HOME/.local/bin/omx"`。
- [x] 3.5 当 `~/.local/bin` 不在 PATH 时显示 `Copy PATH command`；不得自动修改 shell rc 文件。
- [ ] 3.6 如实现 `Enable omx command`，仅创建 `~/.local/bin/omx` symlink；不得覆盖非 symlink 或版本不同的外部 `omx`。

## 4. About 页面重构

- [x] 4.1 About 顶部居中展示 OpenMux icon/name/version/runtime 摘要和一句产品说明（macOS About-panel 心智）。
- [x] 4.2 About 用原生 `Form` section 展示 GitHub、Documentation、Issues、Releases 链接，统一指向 `hiQianFan/openmux`。
- [x] 4.3 About 展示 Author 区域：GitHub、Twitter/X、个人网站和 email 中已配置的链接。
- [x] 4.4 About 保留 Support 区域：`Copy Version Info`、`Copy Redacted Support Report`。
- [x] 4.5 About 底部展示 MIT License / copyright，不引入 Sparkle 自动更新控件。

## 5. Dashboard Footer

- [x] 5.1 将 Dashboard footer 从 `Manage in CLI` 主按钮改成 `状态字符串 + ⋯ 溢出菜单 + Quit` 布局，复用 header 的 28pt icon-button 词汇。
- [x] 5.2 Footer 左侧复用 header 的 freshness/status 文案，不超过一行；CLI 未配置时显示 `CLI not configured`。
- [x] 5.3 Footer 的 `⋯`（NSMenu）提供 About、Copy omx command、Open Releases；为 icon button 提供 tooltip/accessibility label。Settings 入口不在 footer 重复（已在 header）。
- [x] 5.4 当 Terminal command 未配置时，`⋯` 的 Copy omx command 兜底，必要时引导到 Settings → General 的 command-line tool 分组。
- [x] 5.5 Footer 不执行未确认的 credential-changing CLI command，`Copy omx command` 只复制不自动开终端。

## 6. 文档与发布口径

- [ ] 6.1 更新 README / README.zh-CN 中 Menubar Settings、Terminal command 和 repo URL 口径。
- [ ] 6.2 更新 docs/INSTALL 和 docs/INSTALL.zh-CN，避免将 bundled CLI helper 描述为另一个需要下载的 CLI。
- [ ] 6.3 更新 docs/menubar-v1.md，记录新的 Settings/About/Footer 结构。
- [ ] 6.4 如截图进入文档，替换旧 sidebar Settings 截图，避免与新 UI 冲突。

## 7. 验证

- [x] 7.1 运行 Swift contract tests，确认 Settings/About DTO decode 仍兼容。
- [x] 7.2 运行 `scripts/build-menubar.sh`。
- [x] 7.3 运行 `scripts/bundle-menubar.sh`。
- [x] 7.4 运行 `cargo fmt --all`、`cargo test --locked`、`cargo clippy --all-targets --all-features -- -D warnings`。
- [ ] 7.5 手工检查 Settings 三个 tab（General/Providers/About）、About 链接、command-line tool copy command 和 Dashboard footer 在浅色/深色模式下无文本溢出。
