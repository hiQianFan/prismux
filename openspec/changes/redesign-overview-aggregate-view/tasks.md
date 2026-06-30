# 任务

> 依赖：本变更消费并扩展 `compose-overview-aggregate-primitives` 的后端原子。新增字段（in/out token、按窗口类平均）在本变更内随该后端 projection 一并落地。

## 1. 后端字段补充

- [x] 1.1 `UsageHeadline` 加 `input_tokens` / `output_tokens`，从 `UsageTokenBreakdown` 折叠填充（全局 headline 与 per-provider headline 都填）。
- [x] 1.2 `QuotaHealthRollup`（或 `ProviderAggregateView`）加按窗口类聚合的平均剩余：5h(short/session) 与 7d(weekly) 各一个 `avg_remaining_percent_x100`（None 表示该类无上报）。窗口分类沿用 frontend `quotaWindow` 的 id/label 文本判定。
- [x] 1.3 bump `CONTROL_PLANE_SCHEMA_VERSION`，重生成 fixtures（`OMX_UPDATE_FIXTURES=1`），更新 Rust contract 测试的版本断言。

## 2. Swift DTO 对接

- [x] 2.1 `DTO.swift`：`UsageHeadline` 补 `inputTokens` / `outputTokens` 解码。
- [x] 2.2 `DTO.swift`：provider aggregate 补按窗口类平均（5h/7d）解码。

## 3. 共享 QuotaLine

- [x] 3.1 把 `QuotaLine` 从 `TargetRows.swift` private 提升为共享组件（保持阈值刻度 + health 着色不变）。
- [x] 3.2 账号卡改用共享 `QuotaLine`，确认账号页视觉不变（回归）。

## 4. 用量汇总条（UsageStatsStrip）

- [x] 4.1 新增 `UsageStatsStrip`：total token + 金额（双焦点，金额按 cost_status 降级、missing 省略）+ in/out（缩进注解，↓/↑ 箭头，中性色）。
- [x] 4.2 输入 `UsageHeadline`，不取数；period 由调用方传入。

## 5. provider 健康卡（ProviderSummaryCard）

- [x] 5.1 新增 `ProviderSummaryCard`：身份 + `{n} accts · {m} prof` + `→ active`（脱敏）+ 5h/7d 共享 `QuotaLine` 两行。
- [x] 5.2 5h/7d 条喂后端按窗口类平均；无该窗口类数据时该行省略或占位。
- [x] 5.3 点击卡跳转对应 provider tab；`accessibilityElement` 合并为可读语句。
- [x] 5.4 移除 healthy/low/exhausted 计数。

## 6. Overview 与 provider 页组装、清理

- [x] 6.1 `DashboardView.overview(_:)` 改为：`UsageStatsStrip(全局 headline)` → provider 卡列表 → UsageCard。
- [x] 6.2 provider 页 Overview 改为：`UsageStatsStrip(该 provider headline)` → `ProviderSummaryCard(该 provider)`；移除四个裸 MetricCell。
- [x] 6.3 删除 `OverviewProviderRow`、全局 capacity hero、`lowestQuota`、平均/红绿灯/阈值 helper 及上一轮的 `ProviderSummaryRow`（被 Card 取代）/ `ProviderOverviewCard`（被复用组合取代）等死代码。

## 7. 可访问性与视觉

- [x] 7.1 5h/7d 条配文字标签（5h/7d + %），金额配 `est.`，in/out 配箭头，不单靠颜色。
- [x] 7.2 空字段省略策略落实（missing 金额省略、无该窗口类则该行省略、无 active 占位）。
- [x] 7.3 沿用 DESIGN.md：系统色、radius ≤ 8pt、不嵌 card。

## 8. 验证

- [x] 8.1 Rust：`cargo test -p omx-app -p omx-menubar-ffi` 通过；`cargo build -p omx-cli` 通过。
- [x] 8.2 Swift：`swift build` + `OmxMenubarContractTests` 通过（先 `xattr -cr .build`）。
- [x] 8.3 `openspec validate redesign-overview-aggregate-view` 通过。
- [ ] 8.4 视觉回归（人工跑 GUI）：账号卡 QuotaLine 不变；Overview 与 provider 页用同组件；0/1/多 provider、深浅色、missing cost 边界正常。
