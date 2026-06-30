---
version: "alpha"
name: "OpenMux Menubar"
description: "macOS menu bar control plane for switching AI provider accounts and watching quota."
colors:
  # Provider brand identity. Official tones, lightly desaturated so the three
  # sit together calmly. Used by tab icons, accents, and usage charts.
  provider-codex: "#10A37F"
  provider-claude: "#CC785C"
  provider-gemini: "#4285F4"
  provider-overview: "#8B5CF6"
  # Shell. The popover carries a faint indigo brand tint in dark mode; light
  # mode falls back to the system window background.
  shell-dark: "#2E2957"
  shell-light: "windowBackgroundColor"
  # Surfaces are translucent white over the tinted shell, not opaque controls.
  surface-dark: "rgba(255,255,255,0.08)"
  surface-light: "rgba(255,255,255,0.86)"
  stroke-dark: "rgba(255,255,255,0.12)"
  stroke-light: "rgba(0,0,0,0.08)"
  # Text rides system dynamic colors; dark mode lifts white slightly off pure.
  text-primary: "labelColor"
  text-secondary: "secondaryLabelColor"
  text-primary-dark: "rgba(255,255,255,0.95)"
  text-secondary-dark: "rgba(255,255,255,0.68)"
  accent: "controlAccentColor"
  # Status / quota health. Battery convention: comfortable → low → critical.
  status-healthy: "systemGreen"
  status-warning: "systemYellow"
  status-attention: "systemOrange"
  status-critical: "systemRed"
  status-muted: "secondaryLabelColor"
typography:
  title:
    fontFamily: "system"
    fontSize: "title3"
    fontWeight: 700
  section:
    fontFamily: "system"
    fontSize: "headline"
    fontWeight: 600
  body:
    fontFamily: "system"
    fontSize: "body"
    fontWeight: 400
  metric:
    fontFamily: "system"
    fontSize: "caption2"
    fontWeight: 600
    fontFeature: "tabular-nums"
  label:
    fontFamily: "system"
    fontSize: "caption"
    fontWeight: 400
rounded:
  control: "6px"
  row: "8px"
  card: "10px"
spacing:
  xs: 4
  sm: 8
  md: 12
  lg: 16
layout:
  popoverWidth: 392
  popoverHeight: 640
  rowMinHeight: 44
  tabSize: "38x30"
  iconButtonSize: 28
components:
  status-item:
    icon: "arrow.triangle.2.circlepath"
    textColor: "{colors.text-primary}"
  popover:
    width: "{layout.popoverWidth}"
    height: "{layout.popoverHeight}"
    backgroundColor: "{colors.shell-dark}"
  card:
    backgroundColor: "{colors.surface-dark}"
    rounded: "{rounded.card}"
    padding: "{spacing.md}"
  tab:
    rounded: "{rounded.row}"
    size: "{layout.tabSize}"
  quota-bar:
    height: "5px"
    rounded: "{rounded.card}"
  command-button:
    rounded: "{rounded.control}"
    textColor: "{colors.accent}"
    typography: "{typography.metric}"
  icon-button:
    size: "{layout.iconButtonSize}"
    rounded: "{rounded.control}"
---

# OpenMux Menubar Design System

## Overview

OpenMux Menubar 是一个 macOS 原生菜单栏控制面板，用来管理多个 AI provider（Codex、Claude、Gemini）的账号池与 profile，并随手查看 quota 健康度。它不是营销页，也不是完整 analytics dashboard。交互模型是“短暂打开、快速确认、立即关闭”。

第一屏服务三类高频任务：

1. 看清每个 provider 当前 active 的 account/profile，以及池子整体是否健康。
2. 判断哪条 quota window（5h / 7d）接近耗尽，哪个账号 stale 或报错。
3. 安全切换、新增、导入或重置账号，并在后端确认后看到结果。

文件采用 Google `DESIGN.md` 结构：顶部 YAML 是机器可读 design tokens，正文解释如何应用。OpenMux 不引入 Google 工具链，格式只用于给后续 agent 和实现代码稳定的 UI 约束。

外观立场介于“系统工具面板”与“轻品牌产品”之间：深色模式下 popover 带一层很淡的靛紫品牌底色，provider 用各自官方品牌色做强调，其余一律走 Apple HIG——系统字体、系统动态色、SF Symbols、原生 popover 行为、动态深浅色、Reduce Motion。除非系统控件无法表达状态，否则不自造主题系统。

### Sources

- Google Labs Code `DESIGN.md` spec: https://github.com/google-labs-code/design.md
- Apple Human Interface Guidelines: https://developer.apple.com/design/human-interface-guidelines/
- AppKit `NSStatusItem` / `NSPopover`: https://developer.apple.com/documentation/appkit/nsstatusitem
- SF Symbols: https://developer.apple.com/sf-symbols/

## Colors

颜色分三类：品牌、表面、状态。

**Provider 品牌色**是这套系统唯一的彩色装饰来源，用各自官方色：Codex/ChatGPT 绿 `#10A37F`，Claude/Anthropic 陶土橙 `#CC785C`，Gemini Google 蓝 `#4285F4`，Overview 聚合视图用紫 `#8B5CF6`。它们出现在 tab 图标、active marker、usage chart accent，让一个 provider 在任何位置看起来一致（`ProviderStyle` 是单一来源）。

**表面**不是 opaque 控件色，而是叠在 shell 上的半透明白：深色 `white 8%`、浅色 `white 86%`，配 1pt 描边（深 `white 12%` / 浅 `black 8%`）。深色 shell 是淡靛紫 `#2E2957`，浅色 shell 用系统 `windowBackgroundColor`。这层淡紫是有意的品牌信号，不是装饰光斑。

**文本**走系统动态色（`Color.primary` / `.secondary`），深色模式把白色压到 95% / 68% 避免纯白刺眼。

**状态色只表达状态，不做装饰**，遵循 Apple 电量/磁盘约定 comfortable → low → critical：healthy `systemGreen`、warning `systemYellow`、attention `systemOrange`、critical `systemRed`、unknown/muted secondary gray。状态绝不能只靠颜色，必须配文字或 symbol。

不做大面积渐变、品牌色块或装饰光斑——淡紫 shell 是唯一允许的大面积着色。

### Design Tokens

- 品牌：`provider-codex` `provider-claude` `provider-gemini` `provider-overview`
- 表面：`shell-dark` `shell-light` `surface-dark` `surface-light` `stroke-dark` `stroke-light`
- 文本：`text-primary` `text-secondary`（+ `-dark` 变体）、`accent`
- 状态：`status-healthy` `status-warning` `status-attention` `status-critical` `status-muted`

## Typography

只用系统字体，不引入自定义字体，不做负字距，不按 viewport 缩放。层级保持克制——这是密集小面板，不是落地页。

- `title`：`title3.bold`，仅用于 Header 的 “OpenMux”。
- `section`：`headline`，card 标题。
- `body`：行主标签、正文。
- `label`：`caption` + secondary，metadata、时间、empty/diagnostic 文案。
- `metric`：`caption2.semibold` + **tabular nums**，所有数字——百分比、token 量、reset 时间。等宽数字保证 quota 列纵向对齐、刷新时不抖动。

数字摘要可以 semibold，但不使用 hero-scale type。

## Layout

Popover 固定窄宽 392pt、高 640pt，纵向滚动；不开独立 dashboard 窗口。（`StatusItemController` 初始 contentSize 略小，由 SwiftUI 撑到目标尺寸。）

垂直结构固定：

1. **Header**：产品名、刷新状态副标题、Refresh、Settings。
2. **Tab bar**：Overview + 每个 provider，icon-only。
3. **Carousel**：横向分页，每页独立滚动；当前页随选中 tab 平移。
4. **Footer**：Manage in CLI、Quit。

页面内容（卡片间）间距 12pt，外边距 16pt，card 内边距 12pt、内部元素 10pt。列表行最小高度 44pt 保证可点。右侧操作区固定宽度，避免 `Switch` / `Refreshing` / `Delete` 文案导致行跳动。

Quota 列用共享列宽（`QuotaLayout`）跨 account card 与 Overview provider card 对齐——同一视觉组件两处像素一致。

### Design Tokens

- `layout.popoverWidth` 392 · `layout.popoverHeight` 640
- `layout.rowMinHeight` 44 · `layout.tabSize` 38×30 · `layout.iconButtonSize` 28
- spacing scale：`xs` 4 / `sm` 8 / `md` 12 / `lg` 16

## Elevation & Depth

基本是平面设计，层级靠**半透明表面 + 细描边**而非阴影。Card 是叠在 shell 上的一层 translucent white 加 1pt hairline border，没有 drop shadow。深度只有两层：shell（底）与 card（上）。

唯一的运动深度来自 popover 自身——`NSPopover` 原生从 status item 弹出。内部不堆叠浮层、不做 card-in-card：页面 section 直接是 card，card 里不再嵌 card。

Active/选中态用品牌色低透明度填充表达（tab pill `tint 16%`，active row `accent 10%`），不是抬高阴影。

## Shapes

统一柔和圆角，三档：

- `control` 6pt：command button、icon button 等小控件。
- `row` 8pt：列表行高亮、tab pill、quota bar 端点。
- `card` 10pt：card 与 modal-like 面板。

圆角不超过 10pt——这是工具面板不是 widget，不用大圆角或胶囊化整块区域。Quota bar 用 capsule 端点是例外（细条 5pt 高，capsule 更干净）。Header 的 provider 图标块用 10pt 圆角方形。

### Design Tokens

`rounded.control` 6px · `rounded.row` 8px · `rounded.card` 10px

## Components

### Status Item

SF Symbol `arrow.triangle.2.circlepath` + 短 tray title（来自 `store.trayTitle`）。title 绝不展示 email、token、raw account id 或 auth payload。Settings 提供 `icon_only` 模式隐藏文字；icon-only 时仍保留 tooltip “OpenMux account switcher”。点击 toggle popover 并触发一次 interactive refresh。

### Header

左侧 provider 图标块（`switch.2`，stale 时换警告 symbol）+ “OpenMux” + 刷新状态副标题（`Updated 3:42 PM` / stale 时橙色 `Stale …`）。右侧两个 icon button：Refresh（`arrow.clockwise`，进行中换 `ProgressView` 并 disable）、Settings（`gearshape`）。icon button 用 borderless pressable chrome，必须有 `.help()` 和 accessibility label。

### Tab Bar

Icon-only：Overview tab（紫）+ 每个 provider tab（品牌色）。选中 tab 用品牌色 16% 填充的 pill 标记，未选中 secondary gray。选 tab 平移 carousel（240ms smooth），Reduce Motion 下直接切换。每个 tab 必须有 `.help()`、accessibility label 和 `.isSelected` trait。Provider 超过 4 个时改横向滚动，不无限压缩。

### Carousel & Page

横向分页容器，按选中 index 平移（280ms smooth，方向由 index delta 自然得出）。每页独立 `ScrollView`。Overview 页：usage 摘要条 + Providers card（每行一个 provider 摘要，可点进）+ Needs attention（聚合 diagnostics）+ Token Usage card。Provider 页：usage 摘要条 + Accounts card + Profiles card + Token Usage card + Diagnostics card。

### Card

Translucent 表面 + hairline 描边 + 10pt 圆角，可选 `headline` 标题。只给重复项目、状态组、modal-like 区域使用；不嵌套。

### Target Row（Account / Profile）

每行包含：display label、secondary label、active marker、quota/status 摘要、操作区。Active row 用 accent 10% 背景。**Switch 成功前不移动 active marker**；失败保留原 active 并显示可恢复错误。操作区固定宽度。Account row 额外支持 refresh、reset usage limit（Codex）、delete；delete 与 reset 走 inline 二次确认，不是默认高频按钮。列表用 stable identity（`localId` / target key），刷新或排序不重建整列。

### Quota Bar

共享 quota 视觉：一行文字（label · percent … 可选 reset 时间）压在一条 5pt 高全宽细条上，条上有 50% / 20% 阈值刻度线（中性参考线，不是警报）。条色编码**剩余健康度**：>50% 绿、≤50% 黄、≤20% 红；window 由 label 命名（5h / 7d），所以颜色专门用来预警。Account card 与 Overview provider card 都走这个组件，保持像素一致。

### Usage Card

只表达 local parsed usage——绝不表述为账单、剩余额度或 provider quota。带 day / 7d / 30d period 选择器和 model 拆分。无数据显示 empty/partial/stale，不影响账号切换。

### Command / Icon Button

`CompactCommandButton`：accent 色、`metric` 字体、6pt 圆角、按下 0.96 缩放 + 18% 背景反馈、disabled 0.45 透明。Sign in / Use existing login / Import / Manage in CLI / Quit 都用它。加载态把 leading icon 换成小号 `ProgressView`。

### Status Banner & Diagnostics

操作结果用 inline banner（severity 着色 + 标题 + 短消息），出现 opacity + 轻微 y offset，消失只 fade。Diagnostics 用文字 + symbol，不靠纯色。

### Motion

遵守 Apple HIG：动效解释状态变化，不做装饰。时长 120–280ms，ease-out / 系统 spring，无弹跳过冲循环。只给明确状态值绑定动画（selected provider、refreshing/switching/deleting id、banner、active marker），不对 root view 用无差别 `.animation`。所有 custom transition 在 `accessibilityReduceMotion` 为真时退化为 `.opacity` 或 `.identity`。Popover 用 `NSPopover` 原生开合，不自造窗口动画。

## Do's and Don'ts

Do:

- 用 AppKit 管 status item / popover lifecycle，用 SwiftUI 管内容与列表。
- 用系统字体、系统动态色、SF Symbols；数字一律 tabular nums。
- provider 视觉走 `ProviderStyle` 单一来源（品牌色 + 图标）。
- quota 用全宽细条 + 阈值刻度，颜色编码剩余健康度。
- 显示 last-good 数据并标 stale；可恢复错误用短文案 + inline banner。
- 局部动画 + 接入 Reduce Motion；操作区固定宽度。
- 高风险操作（delete / reset）走 inline 二次确认。

Don't:

- 不展示 raw auth / token / email / 完整 provider endpoint。
- 不做实时网络轮询；刷新是 intent-driven + 低频后台 timer（默认 300s，可选 5/15/30 分钟）。
- 不把 Menubar 做成完整 usage dashboard，不开独立大窗口。
- 不嵌套 card，不堆浮层，不做 drop shadow 深度。
- 不做大面积渐变 / 品牌色块 / 装饰光斑（淡紫 shell 是唯一例外）。
- 不让状态只靠颜色表达。
- 不做装饰性循环动画、夸张弹跳、整页横滑或忽略 Reduce Motion。
