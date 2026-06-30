## Why

OpenMux Menubar 的主面板已经逐步变成账号控制台，但 Settings、About 和 footer 仍像工程调试面板：信息层级弱、入口职责混杂、仓库链接口径不一致，且缺少面向 full bundle 分发后的 `omx` Terminal command 配置说明。现在如果直接发布到 GitHub，用户能看到功能，但会在“在哪里配置、如何打开终端命令、如何反馈问题、这个 App 是否需要更新”上产生不必要的疑惑。

这次变更把 Settings/About/Footer 当成发布前用户体验的一部分重新设计。目标不是照搬 CodexBar 的完整 Preferences 体量，而是学习它清晰的顶部 tab、About 可信信息和更新/链接区域，并按 OpenMux 的账号切换产品边界做更小的版本。

## What Changes

- 重构 Menubar Settings 信息架构：从当前 `General / Providers / About` 侧边栏，改为更接近 macOS Preferences 的一级 tab：`General`、`Providers`、`About`（三个 tab），并用原生 grouped `Form` 替换自造 card。
- 不设独立 `Display`/`Tools` tab，也不保留空壳 `General`。按第一性原理，真正的持久用户设置只有 5 个（tray display、hide identifiers、launch at login、provider enabled、source policy），把外观/隐私/开机启动/命令行工具收进有真实内容的 `General`；background refresh cadence 属于产品策略，不暴露成用户选择。
- `General` 新增 launch at login（`SMAppService.mainApp`），这是 menubar app 的标准预期设置；`Providers` 放 provider 可见性、数据源策略、诊断，以及 CLI 已支持的 gateway/profile **只读摘要 + `Manage in dashboard` 跳转**，不在 Settings 重复 import/use/remove。
- 明确 CLI 与 Menubar 共用 provider profile/gateway 配置源：CLI `omx import/use/list/remove` 与 Menubar dashboard 的 profile 管理都通过 Rust control-plane 和 OpenMux state/profile registry，不在 Swift `UserDefaults` 或 Menubar 私有 JSON 中另存一份。
- 区分 gateway/profile 与 HTTP proxy：`OPENAI_BASE_URL`、`ANTHROPIC_BASE_URL` 属于 provider profile；`OMUX_HTTPS_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` 属于网络代理状态。Settings 可以展示和配置 OpenMux 自身网络代理，但不得把 API key 或 raw secret 暴露给 UI。
- 在 `General` 增加 command-line tool 分组，解释 bundled `omx` helper 与 Terminal command 的关系，并提供 `Enable omx command` / `Copy manual command`。这里不是下载 CLI，而是配置 PATH symlink。
- 重设计 `About` 页：居中展示 app icon、OpenMux 名称、版本、build/runtime，再用原生 Form section 展示 repo/docs/issues/release 链接、作者署名链接、版权/许可证、支持信息；仓库 URL 统一为 `https://github.com/hiQianFan/openmux`。schema 数字只进入 `Copy Version Info`，不在可见区域。
- Dashboard footer 从“Manage in CLI + Quit”改成低干扰形态：`状态字符串 + ⋯ 溢出菜单（About / Copy omx command / Open Releases）+ Quit`，复用 header 的 icon-button 词汇；不与 header 的 Settings 入口重复，不抢账号列表焦点。
- 保留 `Copy Redacted Support Report`，但放入 About/Support 区域；避免在 Settings 首页暴露过多 schema/internal 字段。
- 不引入 Sparkle、自动更新、Homebrew、Developer ID notarization 或自动下载另一个产物；更新入口第一阶段只打开 GitHub Releases。
- 不静默修改 `.zshrc`、`.bashrc`、`/usr/local/bin` 或 `/opt/homebrew/bin`；任何 CLI command 配置都必须由用户显式触发。

## Capabilities

### New Capabilities

- `menubar-settings-footer-experience`: Menubar Settings/About/Footer 的信息架构、Terminal command 配置 UX、支持/链接/版本展示和发布前交互约束。

### Modified Capabilities

- 无。当前 repo 尚无已归档 base specs；本 change 会在实现时与未归档的 `refine-menubar-settings-about-experience`、`ship-macos-full-bundle` 和 `redesign-overview-aggregate-view` 对齐。

## Impact

- 影响 Swift Menubar UI：`MenubarSettingsView`、`MenubarSettingsStore`、Dashboard footer、About/Support 页面、新增 `General` command-line tool 与 launch-at-login 相关 view/model。
- 影响 Rust control-plane DTO：可能需要在 `about_view`、settings/profile DTO 或新 support/settings DTO 中增加 `repository_url`、`release_url`、author links、bundled helper path/status、runtime/build metadata、provider profile summary、network proxy status 的前端安全字段。
- 影响文档：README、INSTALL、RELEASE、ROADMAP、Menubar 文档中的 Settings/About/Footer 截图和说明需要统一到 `hiQianFan/openmux` 与 full bundle 口径。
- 不改变 auth snapshot 安全边界或 release workflow 的打包行为；如 Menubar 需要新增 profile 管理 API，应复用现有 registry/profile selector 和 plugin import/use/remove 语义，不创建第二套配置源。
