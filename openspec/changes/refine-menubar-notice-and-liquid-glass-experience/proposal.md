## Why

当前 Menubar popover 把大量提示堆在首屏，视觉噪音大，且不符合 Apple 对"菜单栏 extra 应轻量、可扫读"的期望：

- 每次操作（成功/无变化/失败）都会创建 `OperationNotice`，在 `DashboardView` header 下方渲染一个彩色 `StatusBanner`。这个 banner 从不被清除（`operationNotice` 被赋值后没有任何地方置 nil），等于常驻脏状态。
- `OperationStatus::Skipped`（尤其 `fresh_enough`）被映射成 "Operation skipped" + warning 颜色 + 大面积 banner；对用户这不是失败、也不需要行动，却占据首屏焦点。后端 `mutation.rs` 里 skip 的 message 是 `format!("Refresh skipped: {reason}.")`，snake_case 的 `fresh_enough` 因此直接漏给用户。
- Overview 的 "Needs attention" 卡片和 provider 页的 "Diagnostics" 卡片用彩色 `StatusBanner`（`DiagnosticView`）展示 provider/dashboard 级诊断，色块文字丑且喧宾夺主。
- 账号行现有的 inline 诊断（`account.diagnostic`）直接把 snake_case `code` 拼进文案、单行截断，同样不美观。
- `StatusItemController.togglePopover()` 每次打开 popover 都无条件 `store.refresh(kind: "interactive")`，让刷新按钮频繁进入 loading，即使只是想看一眼。

Apple《The menu bar》明确："Display a menu — not a popover ... unless the app functionality you want to expose is too complex for a menu"。Prismux 因为多账号 + 额度 + 操作已经复杂到用 popover，说明我们已在复杂度上限，应做减法、保持轻量，而不是继续堆信息或加导航深度。

## What Changes

- 移除所有 operation-notice banner（成功/无变化/失败一律不再弹顶部提示）。操作失败改为静默——依赖行状态与账号卡上的诊断，而非顶部彩条。
- 移除 Overview 的 "Needs attention" 卡片与 provider 页的 "Diagnostics" 卡片。
- 将 target 级诊断（带 `target_id`，如 auth 失效、`refresh_failed`）归到对应账号/配置卡片上；provider/dashboard 级（无 `target_id`）诊断从 popover 移除，仍保留在 support report 中，全局信号由 header stale 指示与 footer CLI 状态覆盖。
- target 级诊断落到对应账号卡：折叠态 identity + 5h/7d 额度条**保持不变**，有诊断时在额度条下方加一条**常驻诊断行**（severity 图标 + 人话 message），不做展开/折叠、不放操作按钮。（早期"点击展开、展开区放完整额度窗口 + Retry/Sign in 按钮"的方案已否决：会重复 usage、与既有 overflow/`+` 菜单重复、无独有内容支撑展开。）
- 诊断/情况的颜色采用两档 + 形状双编码：可恢复 = 橙色 `exclamationmark.triangle.fill`，需处理（认证失效）= 红色 `exclamationmark.octagon.fill`；文字中性、无填充背景。遵循《Color》"avoid relying solely on color ... use glyph shapes" 与 "apply color sparingly"。
- 将 popover open 与 foreground refresh 解耦：打开 Menubar 默认展示 last-good，不再让刷新按钮无条件进入 loading；只有显式点 Refresh、无数据首屏、或明显 stale 时才刷新。
- 删除失去消费者的组件文件 `StatusBanner.swift`、`DiagnosticView.swift`，以及已存在的死代码 `providerAttentionCount`/`isProviderAttention`。
- 更新 Menubar 视觉策略：遵照 Apple HIG，macOS 26/27 优先使用系统材料和原生控件，不手搓 Liquid Glass；重功能归 Settings 窗口，不在 transient popover 里做导航深度。
- 更新 `DESIGN.md` 与 Menubar UX checklist：反馈分级、diagnostics-on-card、菜单栏 extra 轻量策略、原生材料优先、footer/header freshness、open-popover 刷新策略。

## Capabilities

### New Capabilities

- `menubar-system-feedback-experience`: 约束 Menubar 中操作反馈与诊断的展示——哪些完全不展示、哪些以常驻诊断行落到账号卡片、颜色与形状分级、provider/dashboard 级诊断的去向，以及打开 popover 的刷新策略与原生材料策略。

### Modified Capabilities

无。

## Impact

- Swift Menubar：
  - `AppStore`：删除 `operationNotice` 属性、`request()` 中的赋值、`OperationNotice` 结构体。
  - `DashboardView`：删除顶部 banner、"Needs attention" 卡、provider "Diagnostics" 卡、`aggregatedDiagnostics`、死代码 `providerAttentionCount`/`isProviderAttention`。
  - `TargetRows`：新增 severity→(颜色, 图标) 映射与一条常驻诊断行 `TargetDiagnosticLine`（无展开/折叠、无操作按钮）。
  - `StatusItemController.togglePopover()`：打开 popover 不再无条件 interactive refresh。
  - 删除文件：`Components/Shared/StatusBanner.swift`、`Components/Shared/DiagnosticView.swift`。
- Rust control-plane：**不改动**。`OperationResult`/`skipped_reason`/`Diagnostic` 语义与 DTO 保持不变；本次仅在 Swift 展示层做减法与重排。
- 文档：`DESIGN.md`、`docs/menubar-ux-checklist.md`（及 `docs/menubar-v1.md` 相关描述）。
- 打包：不引入新依赖、不新增视觉框架。
