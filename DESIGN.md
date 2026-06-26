---
version: "alpha"
name: "OpenMux Menubar"
description: "macOS menu bar account switcher design system for OpenMux."
platform:
  os: "macOS 14+"
  shell: "AppKit NSStatusItem + NSPopover"
  content: "SwiftUI"
  standard: "Apple Human Interface Guidelines"
colors:
  text-primary: "labelColor"
  text-secondary: "secondaryLabelColor"
  text-tertiary: "tertiaryLabelColor"
  background: "windowBackgroundColor"
  surface: "controlBackgroundColor"
  separator: "separatorColor"
  accent: "controlAccentColor"
  status-success: "systemGreen"
  status-warning: "systemOrange"
  status-danger: "systemRed"
  status-stale: "systemYellow"
typography:
  title:
    fontFamily: "system"
    fontSize: "title3"
    fontWeight: "semibold"
  section:
    fontFamily: "system"
    fontSize: "headline"
    fontWeight: "semibold"
  body:
    fontFamily: "system"
    fontSize: "body"
    fontWeight: "regular"
  label:
    fontFamily: "system"
    fontSize: "caption"
    fontWeight: "regular"
spacing:
  xs: 4
  sm: 8
  md: 12
  lg: 16
  xl: 20
rounded:
  sm: 4
  md: 6
  lg: 8
layout:
  popoverWidth: 392
  popoverMinHeight: 560
  popoverMaxHeight: 640
  rowMinHeight: 44
  iconButtonSize: 28
components:
  status-item:
    icon: "SF Symbols"
    title: "short aggregate signal"
    accessibilityLabel: "OpenMux account switcher"
  popover:
    width: "{layout.popoverWidth}"
    backgroundColor: "{colors.background}"
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
    padding: "{spacing.md}"
  icon-button:
    size: "{layout.iconButtonSize}"
    rounded: "{rounded.md}"
  provider-selector:
    style: "segmented"
    rounded: "{rounded.lg}"
---

## Overview

OpenMux Menubar 是一个 macOS 原生菜单栏控制面板，不是营销页，也不是完整 analytics dashboard。第一屏只服务三个高频任务：

1. 看清当前 active account/profile。
2. 判断当前 provider/account 是否可用、是否 stale、是否接近 quota 风险。
3. 安全切换到另一个 account/profile，并看到后端确认后的结果。

设计文件采用 Google `DESIGN.md` 的结构：顶部 YAML 是机器可读设计令牌，正文解释如何使用这些令牌。OpenMux 不引入 Google 的工具链；格式只用于让后续 agent 和实现代码有稳定 UI 约束。

Apple HIG 是界面标准来源：优先使用 macOS 系统颜色、系统字体、SF Symbols、标准控件语义、accessibility label、动态深浅色适配和原生 popover 行为。OpenMux 不做自定义主题系统，除非系统控件无法表达状态。

## Sources

- Google Labs Code `design.md`: https://github.com/google-labs-code/design.md
- Apple Human Interface Guidelines: https://developer.apple.com/design/human-interface-guidelines/
- Apple AppKit `NSStatusItem`: https://developer.apple.com/documentation/appkit/nsstatusitem
- Apple AppKit `NSPopover`: https://developer.apple.com/documentation/appkit/nspopover
- Apple AppKit `NSColor`: https://developer.apple.com/documentation/appkit/nscolor
- Apple SF Symbols: https://developer.apple.com/sf-symbols/

## Product Shape

Menubar 的交互模型是“短暂打开、快速确认、立即关闭”。内容必须密集但可扫读：

- Status item 展示一个短信号：`Codex 42%`、`2 alerts`、`OpenMux stale` 或 icon-only。
- Popover 固定窄宽度，纵向滚动；不使用独立 dashboard 窗口。
- Header 展示 OpenMux、刷新状态和全局操作。
- Provider selector 使用 segmented control 语义：`Overview | Codex | Claude | Gemini`。
- Overview 只显示聚合健康信号和入口。
- Provider page 才显示可切换 targets、quota windows、diagnostics 和 local usage。

不在 Menubar v1 里做导入、登录、alias 编辑、复杂 usage 图表、年度统计、session browser 或自动选号策略。这些留给 CLI 或后续明确需求。

## Information Architecture

Popover 顺序固定：

1. Header：产品名、当前刷新状态、Refresh、Settings。
2. Provider selector：Overview 与每个 provider。
3. Overview：provider 数、account/profile 数、stale/error 数、最低 quota、provider 摘要。
4. Provider page：active target、refresh state、account/profile 列表、quota/status、Switch 操作。
5. Local usage：today tokens、top client、top model、coverage；它是辅助信息，不是 provider quota。
6. Footer：Manage in CLI、Quit。

Current implementation 已经接近这个结构：`DashboardView` 有 header、provider selector、overview/provider page、usage summary 和 footer；`StatusItemController` 使用 `NSStatusItem`、`NSPopover` 和 background timer。

## Layout

Popover 默认宽度 392pt。高度可以在 560-640pt 内调整，账号多时滚动，不撑大到接近普通窗口。

布局规则：

- 外边距 16pt。
- 卡片内边距 12pt。
- 列表行最小高度 44pt，保证可点击。
- 只给重复项目、状态组和 modal-like 区域使用 card。
- 页面 section 不再嵌套 card；避免 card-in-card。
- 右侧操作区固定宽度，避免 `Switch`、`Refreshing`、`Delete` 等文字导致行跳动。

## Typography

使用系统字体，不引入自定义字体。

- Product title：`title3.semibold`，只用于 Header。
- Section title：`headline`。
- Row primary label：`body` 或 `callout.semibold`。
- Metadata、diagnostic、时间：`caption` + secondary color。
- 数字摘要可以 semibold，但不使用 hero-scale type。

不使用负 letter spacing，不按 viewport 缩放字体。

## Color

颜色优先走系统动态色：

- 主文本：`Color.primary` / `NSColor.labelColor`。
- 次级文本：`Color.secondary` / `NSColor.secondaryLabelColor`。
- 背景：`NSColor.windowBackgroundColor`。
- 表面：`NSColor.controlBackgroundColor`。
- 分隔：`NSColor.separatorColor`。
- 强调色：`Color.accentColor` / `NSColor.controlAccentColor`。

状态色只表达状态，不做装饰：

- healthy/success：systemGreen。
- stale/warning：systemOrange 或 systemYellow。
- failed/danger：systemRed。
- unavailable/unknown：secondary gray。

不要做大面积渐变、紫蓝主调、装饰光斑或品牌色块。Menubar 是本地工具，应该像系统工具面板。

## Components

### Status Item

使用 SF Symbol + 短 title。title 不能展示 email、token、raw account id 或 auth payload。

优先级：

1. urgent quota/status。
2. stale/error。
3. provider pool health。
4. fallback `OpenMux`。

Icon-only 模式仍必须有 tooltip/accessibility description。

### Header

Header 左侧是 OpenMux 和状态副标题，右侧是 icon buttons：

- Refresh：`arrow.clockwise`。
- Settings：`gearshape`。

按钮使用 borderless/icon style，并设置 help text。不要把 Settings 做成大卡片。

### Provider Selector

使用 segmented control 语义。当前实现用自定义 capsule button 可以接受，但视觉上应贴近 macOS segmented control：低装饰、清晰 active、可键盘聚焦。

Provider 数超过 4 个时，selector 改为横向滚动或 menu；不要无限压缩文字。

### Target Row

Account/profile row 必须包含：

- display label。
- secondary label。
- active marker。
- status/quota summary。
- action button。

Switch 成功前不移动 active marker。失败时保留原 active，并显示可恢复错误。

### Usage Summary

Usage summary 只能叫 local parsed usage。不得把 local token usage 表达为账单、剩余额度或 provider quota。

无数据时显示 empty/partial/stale，不影响账号切换。

## State And Refresh

当前后端和前端不是实时推送模型，而是 intent-driven refresh + low-frequency timer：

- 打开 popover 时，`StatusItemController.togglePopover()` 会触发 `store.refresh(kind: "interactive")`。
- background timer 默认 300 秒，来自 `dev.openmux.menubar.backgroundRefreshCadence`。
- 用户可在 Settings 选择 5/15/30 分钟。
- `AppStore` 使用 generation 丢弃旧响应，避免过期请求覆盖新状态。
- `BackendClient` 在 detached task 调 Rust FFI，避免阻塞 main actor。
- Rust FFI 只暴露 `dashboard`、`accounts`、`refresh`、`switch`、`remove` envelope。

因此 Menubar 当前支持“打开时刷新、手动刷新、定时后台刷新、mutation 后以后端 dashboard 覆盖 UI”，不支持 websocket/file watcher 级实时更新。这个选择是合理的：本地账号切换和 quota 查询不需要毫秒级实时性，后台请求也要避免 provider 429 和电量浪费。

## Motion

动画必须遵守 Apple HIG 的 motion 原则：帮助用户理解状态变化，而不是制造装饰。Menubar 是短停留工具，动效要轻、快、可中断，并尊重系统 Reduce Motion。

全局规则：

- 默认使用系统控件自带 animation 和 transition。
- 只为状态变化、层级切换、操作反馈添加动效。
- 动画时长控制在 120-220ms；popover 内大面积内容切换最多 250ms。
- 使用 ease-out 或系统 spring；不要使用弹跳、过冲、循环装饰动画。
- 用户再次点击、切换 provider、关闭 popover 或新请求返回时，旧动画必须可中断。
- 开启 Reduce Motion 时，取消位移/缩放，只保留必要 opacity 或直接状态切换。

具体交互：

1. Popover open/close 使用 `NSPopover` 原生行为，不自定义窗口飞入、缩放或背景遮罩。
2. Provider selector 切换时，active indicator 做 120-160ms 的位置/opacity 过渡；页面内容使用 crossfade，不做横向滑屏。
3. Refresh 点击后，按钮进入 disabled/loading 状态；refresh icon 可以短暂旋转，但只在请求进行中显示，Reduce Motion 下改为 `ProgressView` 或静态状态。
4. Switch 点击后，目标 row 显示 inline progress，action 区宽度保持稳定；active marker 只在后端确认 dashboard 返回后用 160ms fade/scale 更新。
5. Error/stale banner 出现使用 opacity + 轻微 y offset；消失只 fade out，不推动布局做夸张位移。
6. Account/profile row hover 使用系统 hover/selection feedback 或 120ms background color fade。
7. 数字变化使用 crossfade，不做滚轮数字、粒子、脉冲或持续闪烁。
8. Delete/remove 确认态只能用 inline reveal 或 popover/menu confirmation，不使用 destructive shake。

SwiftUI 实现约束：

- 给明确状态值绑定 animation，例如 selected provider、refreshing id、switching id、banner visibility。
- 不对整个 root view 使用无差别 `.animation(...)`，避免异步 dashboard 返回时全页面乱动。
- 对列表使用 stable identity：`account_key` / provider target key；不要因为排序或刷新重建整列表。
- 使用 `withAnimation` 包住用户触发的 UI 状态变化；后端数据覆盖时只动画受影响的局部。
- `Environment(\.accessibilityReduceMotion)` 为 true 时，所有 custom transition 退化为 `.opacity` 或 `.identity`。

## Accessibility

最低要求：

- 所有 icon-only button 设置 `.help()` 和 accessibility label。
- 状态不能只靠颜色表达；必须有文字或 symbol。
- 列表行支持 VoiceOver 可读 label，例如 provider、target、active/status、quota。
- 支持 light/dark mode 和 Increase Contrast。
- 文字不能截断关键信息；长 alias/base URL 使用 line limit + tooltip。
- destructive action 必须二次确认；删除不作为默认高频操作展示。

## Current Gaps

基于当前代码，建议后续最小改动：

1. 把 custom provider selector 视觉再贴近 macOS segmented control，减少 capsule 装饰感。
2. 给 icon-only controls 补齐 explicit accessibility labels；`.help()` 不等于完整 VoiceOver 语义。
3. 统一 card radius 到 8pt 以内；当前 header icon 10pt 可降到 8pt。
4. 确认 target row 操作区固定宽度，避免 switching/deleting 状态导致布局跳动。
5. 将 delete/remove 从主路径弱化或移到 CLI/二级确认区，避免 Menubar v1 承担高风险 CRUD。
6. 为 provider 切换、refresh、switch、banner 和 row hover 添加局部动效，并接入 Reduce Motion。

## Do And Do Not

Do:

- 使用 AppKit 管 status item/popover lifecycle。
- 使用 SwiftUI 管内容状态和列表。
- 使用系统字体、系统颜色、SF Symbols。
- 用短文案表达可恢复错误。
- 显示 last-good 数据并标记 stale。
- 让动画解释操作反馈和状态变化。

Do not:

- 不复制 TokenBar/其他项目的数据层或视觉资产。
- 不展示 raw auth、token、email 或完整 provider endpoint。
- 不做实时网络轮询。
- 不把 Menubar 做成完整 usage dashboard。
- 不为了“品牌感”覆盖 macOS 原生控件语义。
- 不做装饰性循环动画、夸张弹跳、整页滑动或忽略 Reduce Motion。
