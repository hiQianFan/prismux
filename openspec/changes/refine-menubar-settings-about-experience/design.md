# 设计：Menubar Settings 与 About

## CodexBar 调研结论

本地 CodexBar 代码中的关键结构：

- `PreferencesView.swift`：`TabView` 承载 `GeneralPane`、`ProvidersPane`、`DisplayPane`、`AdvancedPane`、`AboutPane`、`DebugPane`，并用 `PreferencesTabLayoutCoordinator` 处理 tab 切换和窗口 resize。
- `PreferencesGeneralPane.swift`：系统设置、语言、terminal、launch at login、usage、refresh cadence、notifications、Quit。
- `PreferencesProvidersPane.swift`：左侧 provider sidebar，右侧 provider detail；支持搜索、排序、enable toggle、refresh、错误展开、provider-specific settings descriptor。
- `ProviderSettingsDescriptors.swift`：provider 返回 `ProviderSettingsToggleDescriptor`、`ProviderSettingsFieldDescriptor`、`ProviderSettingsPickerDescriptor`、`ProviderSettingsActionDescriptor`、`ProviderSettingsTokenAccountsDescriptor` 等数据描述。
- `ProviderImplementation.swift`：provider 只提供行为和 descriptor，不拥有 Settings UI。
- `PreferencesAboutPane.swift`：icon、app name、version/build、build timestamp、project links、update controls。
- `PreferencesDebugPane.swift`：日志、fetch strategy、debug toggles、cache 操作等低频诊断能力。

OpenMux 的可复用原则：

1. 独立 Settings window 是对的。
2. Provider settings descriptor 是长期方向，但第一阶段不需要先建通用 descriptor 系统。
3. About/Support 要显示版本和诊断是对的。
4. Debug 能力必须隔离是对的。
5. 但 OpenMux 不复制 CodexBar 的 Swift provider 业务层；业务语义仍在 Rust control-plane。

## CodexBar 前后端架构参考

CodexBar 不是 Web 式前后端，而是 SwiftPM 多 target 的本地分层：

```text
CodexBar.app / CodexBarCLI / CodexBarWidget
  -> CodexBarCore
  -> provider descriptors / fetch plans / config / keychain / logging / redaction
```

从代码结构看：

- `Package.swift` 暴露 `CodexBarCore` library、`CodexBarCLI` executable、macOS `CodexBar` executable、Widget、watchdog 和 web probe。
- `Sources/CodexBarCore` 放 provider descriptor、fetcher、config store、keychain/cache、HTTP transport、logging/redaction、usage/cost models。
- `Sources/CodexBar` 是 App 前端，但不只是纯 view；它有 `UsageStore`、`SettingsStore`、`ProviderRefreshCoordinator`、`ProviderRuntime`、provider implementation registry 和 Preferences UI。
- `Sources/CodexBarCLI` 直接复用 `CodexBarCore`，并提供 `usage`、`cost`、`config`、`cache`、`diagnose`、`serve`。其中 `serve` 是 localhost-only HTTP server，提供 `/health`、usage/cost JSON、last-good/stale TTL 和 request timeout。
- Settings 持久化使用 `CodexBarConfigStore`，支持 `CODEXBAR_CONFIG`、`XDG_CONFIG_HOME`、normalized config、atomic write 和 `0600` 权限。
- Provider 扩展分两层：`CodexBarCore.ProviderDescriptor` 定义 provider metadata/fetchPlan/CLI config；App 层 `ProviderImplementation` 提供 presentation、settings descriptors、runtime hooks 和 menu entries。

可参考的架构模式：

1. **共享核心库是事实源**：CLI 和 App 不应该各自解释 provider enablement、source preference、diagnostics 和 config schema。
2. **设置是 typed config，不是零散 UserDefaults**：provider enablement/source/config 应有 schema、normalize、validate、atomic write 和 private permission。
3. **provider UI descriptor 化应后置**：provider 不能随意塞自定义 UI；当 provider-specific 设置超过固定字段时，再输出 toggle、picker、action、info 等可审查 descriptor。
4. **refresh runtime 要有 generation/coalescing/cancel**：CodexBar 的 `ProviderRefreshCoordinator` 证明长期运行的 menubar 需要防止旧请求覆盖新状态。
5. **diagnostics/redaction 必须在核心层**：support/debug 信息不能由前端拼 raw log。
6. **CLI 是共享入口，不是 GUI 的复制对象**：CodexBar CLI 复用核心能力，GUI 只暴露高频或安全入口。

OpenMux 不应照搬的部分：

1. **不要把 provider 业务逻辑搬进 Swift App**。CodexBar App 里的 `UsageStore` 很重，符合它自己的 Swift-only 架构；OpenMux 已有 Rust core/plugin/control-plane，provider 语义应继续留在 Rust。
2. **不要为了 Settings 引入 localhost server**。CodexBar `serve` 服务 usage/cost JSON 和外部分发场景；OpenMux Menubar 当前 embedded Rust FFI 已足够，新增 HTTP server 会扩大安全和生命周期面。
3. **不要复制 CodexBar 的 provider registry 规模**。OpenMux 当前核心是账号/profile 管理，不是几十个 usage provider 的配置中心。
4. **不要复制 WebKit/cookie/private endpoint 能力**。这些需要独立产品和安全决策。
5. **不要复制 Sparkle/Widget/watchdog 发布体系**。它们和 Settings/About 第一阶段无关。

OpenMux 对应架构决策：

```text
omx-cli / omx-menubar Swift
  -> omx-menubar-ffi JSON envelope
  -> crates/omx-app control-plane
  -> crates/omx-core + provider plugins + state store
```

Swift Menubar 应保留轻量前端状态：

- 当前 popover page / selected provider。
- Settings window selected tab。
- Settings sidebar selected provider。
- in-flight UI affordance。
- purely visual local preferences。

Rust control-plane 应成为共享产品后端：

- provider enablement/source preference/settings schema。
- dashboard/provider/settings/about/support DTO。
- active target、diagnostics、action eligibility。
- settings persistence、atomic writes、schema validation。
- support report redaction。

这条边界比 CodexBar 更适合 OpenMux：CodexBar 用 Swift core 统一 App/CLI；OpenMux 应用 Rust control-plane 统一 CLI/Menubar。Settings/About 设计必须服务这个边界，而不是诱导 Swift 重新实现后端。

## 用户旅程与配置筛选原则

Settings 的入口来自主 popover，但它不是主任务流。用户打开 Settings 通常只有三类动机：

1. “这个状态为什么不对？”需要检查 provider 是否启用、数据来源是否可用、最近刷新是否失败。
2. “我想减少干扰或保护隐私。”需要控制刷新频率、是否隐藏个人身份信息。
3. “我要反馈问题。”需要版本、schema、runtime、redacted support report。

因此第一阶段只接受满足以下任一条件的配置：

- 影响账号/用量状态是否被正确展示。
- 影响后台刷新频率或资源消耗。
- 影响隐私暴露。
- 影响排障和支持。

不满足这些条件的内容暂不进入 Settings。典型例子：

- `Quit`：生命周期操作，留在 popover/footer，不放 Settings。
- `Launch at Login`：需要 ServiceManagement 决策，暂不放占位项。
- `keyboard shortcut`：不是账号切换核心问题，等用户明确需要再做。
- `Open Logs` / debug log viewer：风险高，第一阶段只提供 Rust redacted support report。
- 大量 display 微调：先用产品默认值，超过 5 个稳定需求再拆 `Display` tab。

## 信息架构

第一阶段只做三个 tab：

```text
Settings
  General
  Providers
  About
```

暂不做 `Display`、`Advanced`、`Debug` 独立 tab。原因是 OpenMux 当前设置量少，拆太多会制造空页面。后续触发条件：

- `Display`：tray/menu/content display preference 超过 5 个独立项。
- `Advanced`：CLI helper、keyboard shortcut、dangerous recovery 等低频功能进入 GUI。
- `Debug`：support report、redacted logs、operation history 足够成熟，需要单独调试面板。

### General

承载会长期影响主旅程的全局设置：

- Background refresh cadence：`manual`、`5m`、`15m`、`30m`。
- Privacy：hide personal identifiers in UI。

暂不放：

- Tray display mode：它是 menubar 视觉偏好，不影响 CLI 或后端数据口径。第一阶段保留在 Swift local preference，必要时仍可在 popover 菜单中提供。
- Launch at login：未选择 ServiceManagement API 前不放。
- Quit OpenMux：保留在 popover 低频操作区。

### Providers

承载 provider 级设置和状态：

- 左侧 provider list：Codex、Claude、Gemini，展示 enabled、status、last refresh。
- 右侧 provider detail：
  - provider overview：status、enabled、active target、target count、last refresh。
  - source preference：第一阶段只允许 `auto`、`local_only`，并且只在 backend 声明 provider 支持时可编辑。`remote_only` 只有在对应 provider 的远端数据源经过产品和安全决策后才能出现。
  - account/profile target policy：说明一个 provider 全局只能 active 一个 target，不管 target 是 account 还是 profile。
  - provider actions：Refresh provider、Open CLI help。
  - diagnostics：安全错误、recovery action、复制 redacted diagnostic。

Provider 页不直接编辑 secret。登录、导入、alias、删除等危险或高复杂动作第一阶段仍跳转 CLI help。

暂不放：

- Open state folder：state root 附近可能存在 registry、backup 和 auth snapshot。About 可显示 state root，并提供受控 Reveal；Provider 页不放。
- Provider order：当前只有少量 provider，且主 popover 已有 provider selector。等 provider 数量和排序诉求明确后再做。
- Secret/API key/Cookie fields：第一阶段不做图形化 secret 输入。

### About

承载产品与支持信息：

- App icon、OpenMux、version/build。
- Backend/control-plane schema version、state schema version、settings schema version。
- Runtime mode：embedded staticlib / unavailable。
- State root：只显示路径，不显示 auth 文件内容；提供受控 Reveal in Finder。
- Project links：GitHub、Docs、Issues。
- Support actions：Copy Redacted Support Report、Copy Version Info。
- Privacy note：support report 已 redacted，不包含 tokens、raw auth payload、Cookie、Authorization header。

不做自动更新 UI。OpenMux 当前发布策略还没引入 Sparkle；About 只显示版本和链接。

## 前后端架构

OpenMux 的“后端”不是远端服务，而是 Rust control-plane：

```text
Swift Menubar frontend
  -> BackendClient.swift
  -> omx_menubar_call JSON envelope
  -> crates/omx-menubar-ffi
  -> crates/omx-app control-plane
  -> crates/omx-core / provider plugins / state store
```

Swift 是前端：

- 负责窗口、tab、表单、按钮、loading/error。
- 保存 frontend-local UI state，例如当前选中的 Settings tab、provider sidebar selection。
- 保存纯视觉偏好，例如 `trayDisplayMode`，除非未来证明 CLI/其他 frontend 也需要同一语义。
- 不读取 auth、SQLite、usage logs、provider endpoint。
- 不推断 provider health、action eligibility、compatibility。

Rust 是产品后端：

- 负责 settings schema、validation、persistence。
- 负责 provider settings 固定 DTO；通用 provider descriptor/action descriptor 后置。
- 负责 About/compatibility/support report DTO。
- 负责 redaction。
- 负责 state root、version、schema、runtime capability 的统一口径。

共享设置与本地偏好的边界：

```text
Rust shared settings:
  refresh cadence
  provider enabled
  provider source preference
  privacy hide personal identifiers

Swift local preferences:
  selected settings tab
  selected provider in settings sidebar
  tray visual display mode
```

## Rust 模块设计

最小改动优先，复用当前 `crates/omx-app/src/settings.rs` 和 `support.rs`：

```text
crates/omx-app/src/
  settings.rs             # SettingsView / UpdateSettingsCommand / validation / persistence
  about.rs                # AboutView / version/runtime/state/support links
  support.rs              # SupportReport / redaction
  compatibility.rs        # schema/runtime compatibility
```

第一阶段不创建 `provider_settings.rs`，也不引入通用 `ProviderSettingsDescriptor`。固定字段足够覆盖当前旅程。等 provider-specific 设置超过 3 种且无法用固定字段表达时，再拆 `provider_settings.rs`。

建议 DTO：

```rust
pub struct SettingsView {
    pub schema_version: u32,
    pub general: GeneralSettings,
    pub providers: Vec<ProviderSettingsView>,
    pub privacy: PrivacySettings,
}

pub struct GeneralSettings {
    pub refresh_cadence_seconds: Option<u64>,
}

pub struct ProviderSettingsView {
    pub provider: String,
    pub display_label: String,
    pub enabled: bool,
    pub status: ProviderSettingsStatus,
    pub source_preference: SourcePreference,
    pub source_options: Vec<SettingsPickerOption>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct PrivacySettings {
    pub hide_personal_identifiers: bool,
}

pub struct AboutView {
    pub schema_version: u32,
    pub app_version: String,
    pub bundle_version: Option<String>,
    pub build_timestamp: Option<String>,
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub settings_schema_version: u32,
    pub runtime_mode: RuntimeMode,
    pub state_root_display: String,
    pub state_root_reveal_path: Option<String>,
    pub links: Vec<AboutLink>,
}
```

第一阶段避免动态 `support_actions`。About 页面固定渲染 `Copy Version Info` 和 `Copy Redacted Support Report`，减少 Swift 动态 action 分发。

Privacy 的展示口径不能由 Swift 临时正则遮挡 email。后端应在 dashboard/settings/about DTO 中提供可安全展示的 label，或明确返回 `hide_personal_identifiers` 后 Swift 只做整体隐藏/替换，不做敏感信息解析。

FFI ops：

```text
settings_view
update_settings
about_view
support_report
```

已有 `dashboard`、`refresh`、`switch`、`remove` 不变。

## Swift 模块设计

建议新增：

```text
apps/omx-menubar/Sources/OmxMenubarCore/
  Features/Settings/
    SettingsWindowController.swift
    SettingsView.swift
    SettingsSelection.swift
    GeneralSettingsPane.swift
    ProviderSettingsPane.swift
    AboutSettingsPane.swift
    SettingsRows.swift

  Backend/
    SettingsDTO.swift
    AboutDTO.swift
```

最小组件：

- `SettingsView`：`TabView`，只含 `General`、`Providers`、`About`。
- `SettingsWindowController`：由 `Shell/StatusItemController` 持有，复用/聚焦单一 settings window，避免每点一次齿轮开一个窗口。
- `SettingsSelection`：保存当前 tab。
- `GeneralSettingsPane`：Picker/Toggle 表单。
- `ProviderSettingsPane`：左 sidebar + 右 detail。
- `AboutSettingsPane`：版本、schema、state root、链接、support report。
- `SettingsSection`、`SettingsToggleRow`、`SettingsPickerRow`、`SettingsActionRow`：小组件，避免每页重复 HStack。

Popover 顶部齿轮按钮行为：

- 点击齿轮打开 Settings window。
- 可在 Settings 菜单中提供 `Settings...` 和 `About OpenMux...` 两个入口。
- About 不是 popover 内卡片，进入 Settings window 的 About tab。

## 数据流

打开 Settings：

```text
user taps gear
  -> SettingsWindowController.show(tab: .general)
  -> SettingsStore.load()
  -> BackendClient.call(.settingsView)
  -> render SettingsView
```

修改设置：

```text
user changes picker/toggle
  -> Swift updates local draft
  -> Save/apply immediately or debounce
  -> BackendClient.call(.updateSettings)
  -> Rust validates + persists
  -> returns SettingsView
  -> Swift replaces draft with backend result
  -> AppStore refresh cadence observes result
```

第一阶段用即时保存，不做 Apply/Cancel。失败时 Swift 必须回滚到 backend 返回前的已保存值，并在当前 row 下方显示 backend-provided error。不能出现“UI 显示已保存但后端未保存”的状态。

About support report：

```text
user clicks Copy Support Report
  -> BackendClient.call(.supportReport)
  -> Rust redacts
  -> Swift copies JSON/text to pasteboard
```

## 持久化边界

当前 `settings_view()` 返回默认值，`update_settings()` 只 validate 后回传。此提案要求真正持久化：

- Rust settings 文件进入 OpenMux state root，例如 `<state_root>/settings.omx.json`。
- schema version 必须写入文件。
- future schema fail closed，不静默覆盖。
- 原子写入。
- 文件权限保持私有。
- parse failure 时保留损坏文件并返回 safe diagnostic，不能静默重置。
- 不存 auth payload、token、Cookie。

Swift `@AppStorage` 只保留纯 UI 偏好并逐步迁移：

- `selectedSettingsTab` 可以留在 Swift。
- `trayDisplayMode` 留在 Swift local preference。
- `backgroundRefreshCadence` 应迁移到 Rust shared settings。

## HIG/交互标准

- Settings 使用原生 window，不嵌在 popover。
- Tab label 使用 SF Symbols：`gearshape`、`square.grid.2x2`、`info.circle`。
- About 页面不做营销页，只做可信产品信息和支持动作。
- Provider 页使用 sidebar + detail，信息密度高但不堆卡片。
- destructive action 必须二次确认；第一阶段 Settings 不放 destructive account action。
- icon-only button 必须 `.help()` 和 accessibility label。
- Provider sidebar、copy buttons、Reveal in Finder 和每个 picker/toggle 必须有可读 accessibility label。

## 验收标准

- 用户能从 popover 打开 Settings 和 About。
- Settings window 只有一个实例，重复点击聚焦已有窗口。
- General 的 refresh cadence 和 privacy 设置经 Rust shared settings 持久化，重启后保持。
- Providers 页展示 provider enablement/status/source preference，字段来自 Rust DTO，且第一阶段不出现未经安全决策的 remote-only source。
- About 页显示 app/backend/schema/state root 和支持动作。
- support report 经 Rust redaction，不包含 token/raw auth/Cookie/Authorization。
- Swift 不读取 OpenMux SQLite/auth/usage logs。
