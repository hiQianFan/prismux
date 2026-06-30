## Context

当前 Menubar Settings 已经有独立窗口，但信息架构仍偏工程内部：

- `General` 同时承载 tray display、background refresh 和 privacy，概念混在一起；其中 refresh cadence 对普通用户不是明确选择，反而制造干扰。
- `About` 以 schema/runtime/storage/support 为主，缺少面向用户的产品身份、项目链接、release 入口和更新说明。
- Dashboard footer 只有 `Manage in CLI` 和 `Quit`，像调试工具条，不像菜单栏产品的稳定底部导航。
- `crates/omx-app/src/about.rs` 中 repository URL 仍指向 `Sitoi/OpenMux`，与公开仓库 `hiQianFan/openmux` 不一致。
- full bundle 提案已经确定 App 内置 `omx` helper，但 UI 还没有解释“CLI 已随 App 安装，Terminal command 只是 PATH/symlink 配置”。

CodexBar 的 Settings/About 截图给出一个有价值的方向：顶部 tab 更像 macOS Preferences，About 页先展示产品身份和版本，再展示作者/项目链接与更新；高级/支持信息不应该压过普通用户信息。OpenMux 只采纳这个信息架构，不复制 CodexBar 的 Sparkle、provider 设置体量或多语言资源体系。

## Goals / Non-Goals

**Goals:**

- 把 Settings 重构成顶部 tab preferences：`General`、`Providers`、`About`（三个 tab，用原生 `Form` + `.grouped`）。
- 不设 `Display`/`Tools` 独立 tab，也不保留空壳 `General`；把外观、隐私、开机启动、命令行工具收进有真实内容的 `General`。
- 让 About 面向用户：OpenMux icon/name/version/build/runtime、GitHub/Docs/Issues/Releases、license、support。
- 让 About 明确作者署名：作者 GitHub、Twitter/X、个人网站等链接在产品链接附近展示。
- 让 `General` 的 command-line tool 分组面向操作：Terminal command 状态、bundled helper path、`Enable omx command`、manual command。
- 让 Dashboard footer 变成低干扰工具栏：状态字符串 + `⋯` 溢出菜单 + Quit，不抢主内容焦点，也不与 header 的 Settings 入口重复。
- 统一公开仓库 URL 为 `https://github.com/hiQianFan/openmux`。

**Non-Goals:**

- 不实现 Sparkle、自动更新、appcast、更新频道或后台下载。
- 不实现 Developer ID notarization 或 Gatekeeper 处理 UI。
- 不做完整 CLI 下载器；full bundle 已内置 `omx`。
- 不自动修改 shell rc 文件或覆盖外部 `omx` 安装。
- 不做 provider-specific 私有设置 schema；provider profile/gateway 通过现有 Rust plugin 和 OpenMux state registry 暴露脱敏摘要。
- 不把 HTTP proxy 与 provider gateway 混成一个设置。`OPENAI_BASE_URL` / `ANTHROPIC_BASE_URL` 属于 provider profile；`OMUX_HTTPS_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` 属于网络代理。

## Decisions

### 1. Settings 使用顶部 toolbar tab，而不是左侧 sidebar

第一版 Settings 当前窗口较宽，但内容量少，左侧 sidebar 占空间且让“General / Providers / About”像管理后台。改成顶部 toolbar tab 更符合 macOS Preferences 心智：用户先选择设置类别，内容区居中展示。当前 `MenubarSettingsView.swift` 用 `NavigationSplitView` + 150pt sidebar 承载这点内容是横向空间浪费。

Tab 固定为 **三个**，不是四个：

```text
General | Providers | About
```

- `General`: appearance（tray display）、privacy（hide personal identifiers）、startup（launch at login）、command-line tool（`omx` symlink 配置 + 只读 network proxy 状态）。
- `Providers`: provider enablement、source policy、diagnostics、只读 gateway/profile 摘要 + “在 dashboard 管理”跳转。
- `About`: 产品身份、版本、链接、author、support、license。

不设 `Display` 和 `Tools` 作为独立 tab。按第一性原理盘点真正的“设置”（会长期改变核心旅程的持久用户选择）只有 5 个：tray display、hide identifiers、launch at login、provider enabled、source policy。把它们摊到 4 个顶层 tab 会让 `Display`（2 个 toggle）和 `Tools`（一次性 symlink 动作）成为半空壳，结构比内容重，用户也要多猜一层“设置在哪个 tab”。

`General` 不是“想到什么放什么”的杂物间，而是“这个 App 在这台 Mac 上如何运行”的连贯分组：外观、隐私、开机启动、命令行工具。这四组都不足以单独成 tab，但合在一起是一个用户能理解的心智单元。之前反对 `General` 是因为它当时是空壳；加入 launch at login 和 `omx` 配置后它有了真实内容。

Background refresh cadence 不作为用户设置。Menubar 采用产品默认策略：打开 popover 和手动点击 Refresh 走 interactive refresh，后台刷新由后端 cooldown/TTL/backoff 控制。用户要的是“信息是否新鲜”和“能不能手动刷新”，不是在 5/15/30 分钟之间做系统调度选择。

### 1.0 用原生 `Form` + `.grouped`，不自造 card

当前 Settings 用自定义 `SettingsSection`（`Color.primary.opacity(0.045)` 圆角矩形）+ `SettingsKeyValueRow` + `PathRow` 手搓分组卡片，这与 DESIGN.md “prefer dense native macOS controls”“不嵌套 card” 的立场冲突，也和 macOS System Settings 的 inset-grouped form 观感不一致。

第一阶段改用 SwiftUI 原生：

```swift
Form {
    Section("Appearance") { … }
    Section("Startup") { … }
}
.formStyle(.grouped)
```

`.grouped` form 自带正确的分组缩进、分隔线、label/control 对齐和深浅色表面，立即读作原生 preferences 窗口，并删除 `SettingsSection` / `SettingsKeyValueRow` / 大部分 `PathRow`。这是“最省事”和“最贴合 DESIGN.md”同时成立的写法，比 tab 结构调整带来的观感提升更大。Provider 摘要用 `LabeledContent`，路径行保留 reveal 按钮但收进 form row。

### 1.1 Settings 字段边界

第一阶段 Settings 只放这些字段，分到三个 tab：

`General`:

- appearance：tray display `Icon and summary` / `Icon only`
- privacy：hide personal identifiers
- startup：launch at login（`SMAppService.mainApp`，一行 API，是 menubar app 的标准预期设置，价值高于 network proxy）
- command-line tool：bundled helper path/version、Terminal command status、`Enable omx command`、`Copy manual command`、`Copy PATH command`、reveal state/settings folder、只读 network proxy 状态

不放 refresh interval、动画偏好、复杂主题、默认 usage period；没有明确用户收益就不进 Settings。

`Providers`（**只读 + 跳转，不在 Settings 里做 mutation**）:

- provider display name / icon
- enabled toggle，用于隐藏不使用的 provider
- source policy：`Auto` / `Local only`，**仅在该 provider 有可理解的数据源差异时显示**；解释文案收进 `?` help popover，不在每张 provider 区块下常驻一段正文
- status badge
- redacted diagnostics / recovery hint
- profile/gateway summary：active profile、profile count、base URL host、model、auth type 等脱敏信息，用 `LabeledContent` 紧凑展示
- 一个 `Manage in dashboard →` 跳转

profile/account 的 import / use / remove 已经在 dashboard 完整实现（`TargetRows.swift` 的 `ProfileTargetRow` + `importProfileOverlay`）。profile 和 account 在 registry 里同属 target，若在 Settings 再做一套 mutation 入口，会出现“切账号在主面板、切 profile 在设置里”的双入口困惑，并要重写一遍导入 UI。因此 Settings → Providers 只做只读摘要 + 跳转；所有 mutation 留在 dashboard。账号登录、删除、alias、switch 同理留在 dashboard/provider context。任何 raw API key、auth token、snapshot content 都不得进入 Swift 持久化或普通文本展示。

### 2. About 先展示产品身份，再展示内部状态

About 页布局：

```text
[App Icon]
OpenMux
Version 0.x.y
Built ... / Runtime embedded_staticlib
Local account switcher for AI coding tools

GitHub
Documentation
Issues
Releases

Author
GitHub
Website
Twitter/X

Support
Copy Version Info
Copy Redacted Support Report

MIT License
```

Control-plane schema、state schema、settings schema **不再出现在 About 任何可见区域**，只进入 `Copy Version Info` 的复制内容。当前 `MenubarSettingsView.swift` 把三个 schema 数字放在 About 首屏第一组，对用户零价值且方向相反。普通用户打开 About 首先需要确认“这是什么版本、谁维护、从哪里更新、到哪里反馈”，不是 schema 数字。

布局沿用 macOS 原生 About-panel 心智：居中 icon → name → version 摘要在顶部；链接 / author / support 在下方用原生 `Form` section 承载（`LabeledContent` + link row），而不是当前一串松散的 `Button(.link)`。分组 row 给这些信息正确的对齐和权重。

### 3. 命令行工具配置进 General，不单独成 Tools tab，不叫安装 CLI

Full bundle 已经安装了 CLI helper：

```text
OpenMux.app/Contents/MacOS/omx
```

因此 UI 文案用 `Enable omx command`，避免用户误解为下载/安装另一个 CLI。这组配置是一次性 setup，不是反复调整的“设置”，所以放进 `General` tab 的 command-line tool 分组，而不是单独占一个 `Tools` 顶层 tab。

command-line tool 分组状态：

- `Bundled helper`: 显示 bundle 内 helper path 和 version。
- `Terminal command`: `Ready` / `Not configured` / `Different omx found`。
- `Enable omx command`: 创建 `~/.local/bin/omx` symlink。
- `Copy manual command`: 复制 mkdir + ln 命令。
- `Copy PATH command`: 仅在 `~/.local/bin` 不在 PATH 时显示。

如果 PATH 中已有非 symlink 或不同版本 `omx`，OpenMux 只提示，不覆盖。用户可以手动处理。

### 3.1 Provider profile/gateway 与 network proxy 使用同一个后端源

CLI 目前已有两条相关路径：

- provider profile/gateway：`omx import <provider>` 可以导入 Codex/Claude 的 OpenAI/Anthropic-compatible 配置，例如 `OPENAI_BASE_URL`、`ANTHROPIC_BASE_URL`、model 和 auth env key；`omx list` 会展示 profile name、base URL、model、auth type；`omx use` 会应用 profile。
- refresh network proxy：`omx refresh` 的 Codex usage 请求会读取 `OMUX_HTTPS_PROXY`、`HTTPS_PROXY` 或 `ALL_PROXY`。

Settings 应把它们拆开呈现：

```text
Providers (Settings — 只读摘要 + 跳转)
  Codex
    Enabled
    Source: Auto
    Active target: Account #1 / Profile gateway
    Gateway profiles (read-only summary)
      gateway-api     gateway.example.com     gpt-5.5     active
      api-apikey-fun  api.apikey.fun          gpt-5.5
    Manage in dashboard →

General (Settings)
  Command-line tool
  Network proxy
    Current: System env HTTPS_PROXY
    Effective for: refresh usage requests
    (read-only first phase; Copy env command)
```

Import / Use / Remove 在 dashboard 完成，不在 Settings 重复。

共享源原则：

- Menubar profile list/use/import/remove 必须复用 Rust `PlatformPlugin::list_configs`、`import_config`、`use_target`、`remove_target` 或对应 control-plane wrapper。
- CLI 与 Menubar 看到的 profile metadata 必须来自同一个 OpenMux state registry；Codex 的 profile 文件和 Claude 的 profile snapshot 仍由 plugin 管理。
- Swift 只接收脱敏 DTO：profile name、provider id、base URL、model、auth type、active flag、config path display/reveal path。Raw API key、auth token、snapshot bytes 不进入 Swift model。
- 如需要持久 network proxy，新增 shared settings 字段，例如 `network.proxy.mode`、`network.proxy.url`、`network.proxy.no_proxy`，由 Rust `settings_view/update_settings` 读写。Menubar 不得只写本地偏好；CLI 后续也应能读取同一字段。
- 第一阶段可以先只展示环境变量代理状态，并提供 `Copy env command`。只有在确定要持久化代理时，再引入 `network.proxy` schema。

### 4. Dashboard footer 是工具栏，不是主操作区

Footer 只服务两个职责：**显示状态** 和 **承载低频退出**。其余入口都已经可达——Settings 已在 header 的 `gearshape` icon button（DESIGN.md Header 规范），账号登录/导入/切换在 provider 页面和 account/profile rows。footer 不应再放醒目的 `Manage in CLI` 主按钮，也不应把 Settings / About 做成与 header 重复、与账号列表争视觉的一排按钮。

footer 形态：状态字符串 + 一个溢出菜单 + Quit。

```text
Last updated 2m ago · CLI ready                    [ ⋯ ] [ Quit ]
```

- 左侧：复用 header 的 freshness/status 文案，不超过一行；CLI 未配置时显示 `CLI not configured`。
- `⋯`（NSMenu）：About、Copy omx command、Open Releases —— 这些都是低频动作，收进溢出菜单不抢焦点。复用 header 已有的 28pt `icon-button` 词汇，而不是引入一排 `CompactCommandButton`。
- `Quit` 保持可见，但不是唯一明显动作。
- `omx` 的**配置**是低频 setup，归 Settings → General 的 command-line tool 分组；footer 不放独立 Terminal 按钮。CLI 未配置时，状态串提示 + `⋯` 菜单的 `Copy omx command` 兜底，必要时引导到 Settings → General。

### 5. Repository/link 口径从 backend 输出

Repo/docs/issues/releases 链接应由 Rust `about_view` 输出，Swift 只渲染。这样 CLI/Menubar/About 的链接不会分叉。

固定 URL：

```text
https://github.com/hiQianFan/openmux
https://github.com/hiQianFan/openmux/tree/main/docs
https://github.com/hiQianFan/openmux/issues
https://github.com/hiQianFan/openmux/releases
```

作者链接也从 backend 输出，避免 Swift 硬编码。第一版至少需要：

```text
Author GitHub: https://github.com/hiQianFan
Author Website: https://blog.mapin.net/
Author Twitter/X: https://x.com/hiQianFan
```

未配置的 author link 不渲染空占位。Email 第一版可以不配置。

### 6. 不把 Settings 做成营销页

Settings 是工作型工具界面，不做 hero、插画或过大装饰。OpenMux 保持紧凑、清晰、可扫描。用原生 `Form` + `.grouped` section，不做自造卡片，更不做卡片套卡片。

## Risks / Trade-offs

- [Risk] 删除 refresh cadence 让少数用户无法调低后台活动 → Mitigation: 后端继续有 cooldown/TTL/backoff；如果真实反馈需要节能模式，再加一个 `Background refresh: Automatic / Manual only`，不加分钟级旋钮。
- [Risk] toolbar tab 重构扰动当前 Settings 状态绑定 → Mitigation: 保留 `MenubarSettingsStore`，先替换 shell/navigation，再迁移具体 pane。
- [Risk] `Enable omx command` 涉及文件写入和 PATH 判断 → Mitigation: 第一阶段只支持 `~/.local/bin`，不碰系统目录，不覆盖非 symlink。
- [Risk] About 隐藏 schema 后调试信息不易找 → Mitigation: `Copy Version Info` 和 support report 仍包含 schema/runtime/state root。
- [Risk] Footer 砍掉 `Manage in CLI` 后高级用户找不到 CLI handoff → Mitigation: `⋯` 菜单的 `Copy omx command` 和 General → command-line tool 提供命令复制；provider 空状态仍给具体 login/import action。
- [Risk] 用户把 gateway profile 叫成 proxy，UI 如果直接使用 `Proxy` 会误导为 HTTP proxy → Mitigation: Providers 使用 `Gateway profile` / `API profile`，General → command-line tool 的网络设置才使用 `Network proxy`。
- [Risk] Menubar profile import 会接触 API key/token → Mitigation: import dialog 只把内容提交给 Rust control-plane；Swift 不缓存、不写 UserDefaults、不出现在 support report。

## Migration Plan

1. 修正 `about_view` 链接为 `hiQianFan/openmux`，补齐 issues/releases links 和 author links。
2. 在 Swift 中把 Settings 从 `NavigationSplitView` + sidebar 改为顶部 tab shell，三个 pane（General/Providers/About）统一用原生 `Form` + `.grouped`，删除自造的 `SettingsSection`/`SettingsKeyValueRow`/大部分 `PathRow`。
3. 组建 `General` pane：tray display、hide identifiers、launch at login（`SMAppService.mainApp`）、command-line tool 分组、只读 network proxy 状态；不迁移 background refresh cadence。
4. 为 `Providers` pane 增加只读 profile/gateway summary（`LabeledContent`）+ `Manage in dashboard →` 跳转；source policy 仅在有意义时显示，解释收进 help popover。不在 Settings 做 import/use/remove。
5. `General` command-line tool 分组的只读检测和 copy manual command；`Enable omx command` 可作为后续实现任务，但 UI 文案先定。
6. command-line tool 分组增加 network proxy 状态。第一阶段只读展示 env proxy 状态 + `Copy env command`；持久 proxy 需要 shared settings schema 后再开启编辑。
7. 重构 Dashboard footer 为状态字符串 + `⋯` 溢出菜单（About/Copy omx command/Open Releases）+ Quit，复用 header 的 icon-button 词汇。
8. 更新 README/INSTALL/Menubar 文档中的 Settings/About/Footer 截图和说明。

回滚策略：如果 toolbar tab 体验不稳定，可以保留旧 Settings window 作为 fallback；不涉及 state schema 或 auth 数据迁移。

## Open Questions

- App icon 是否已有最终资源；如果没有，About 第一版可使用现有 symbol 占位，但 release 前应补真实 app icon。
- 作者链接第一版固定为 GitHub `https://github.com/hiQianFan`、Website `https://blog.mapin.net/`、Twitter/X `https://x.com/hiQianFan`；Email 可暂不配置。
- `Enable omx command` 第一版是否只创建 symlink，还是先只提供 `Copy manual command`；建议最小实现先 copy，随后加 symlink。
- 已定：footer 用 `状态 + ⋯ + Quit`；`⋯` 的 `Copy omx command` 复制命令（不自动开终端），避免焦点和权限问题。
