# 任务

> 依赖：本变更消费 `compose-overview-aggregate-primitives` 的后端原子（`ProviderAggregateView`、`QuotaHealthRollup`、period-aware `UsageHeadline`、聚合 diagnostics）。需在该后端变更对接后实现。

## 1. 后端字段补充

- [x] 1.1 `ProviderAggregateView` 新增 `usage_headline: UsageHeadline`，由 control-plane 用该 provider 的 period usage 折叠产生（period 随 `DashboardQuery.usage_period`）。
- [x] 1.2 bump `CONTROL_PLANE_SCHEMA_VERSION`，更新 fixtures 与 contract 测试。

## 2. Swift DTO 对接

- [x] 2.1 `DTO.swift`：`ProviderAggregateView` 补 `usageHeadline` 解码。
- [x] 2.2 确认 `QuotaHealthRollup`（facts.avg/min、healthy/low/exhausted 计数、worstTarget、bestAlternative、soonestResetAtUnix、resetCreditTotal）与 `UsageHeadline`（period、totalTokens、estimatedCostUsd、costStatus）解码齐全。

## 3. provider 行（ProviderSummaryRow）

- [x] 3.1 新增 `ProviderSummaryRow`：三行结构——身份 + avg%（None→"—"）+ tone 条 + 红绿灯计数（零段省略）/ 当前 active 身份（+ low 时 reset 倒计时）/ token + 花费（按 cost_status 降级）。
- [x] 3.2 颜色/分类用后端 `quotaHealth.statusTone`，移除前端阈值。
- [x] 3.3 token/花费 period 绑定 `store.usagePeriod`，与 UsageCard 同源；金额缺失省略不显示 `$0`。
- [x] 3.4 active 身份受脱敏设置影响；点击行跳转对应 provider tab。

## 4. Overview 组装与清理

- [x] 4.1 `DashboardView.overview(_:)` 改为：providers 列表（`ProviderSummaryRow`）→ 聚合告警 → UsageCard。
- [x] 4.2 聚合 `ProviderAggregateView.diagnostics` 与 `DashboardAggregateView.diagnostics`（concat，按 provider_id 去重），仅非空时渲染"需要处理"区块，复用 `DiagnosticView`。
- [x] 4.3 删除全局 capacity hero / `lowestQuota` / 平均 / 红绿灯 / `15/40` 阈值 / 相关 helper。
- [x] 4.4 `OverviewProviderRow` 被替换；若无其他引用则删除。

## 5. 单 provider 页 Overview 重构

- [x] 5.1 新增 `ProviderOverviewCard`：当前 active（突出）、容量分解（healthy/low/exhausted）、平均剩余 + tone 条、最佳备选（+reset，可切）、reset credit（>0 才显示）。
- [x] 5.2 替换 `providerOverview`，移除 Targets/Alerts 裸 MetricCell。
- [x] 5.3 provider Overview 主数字用 average，与全局口径一致。

## 6. 可访问性与视觉

- [x] 6.1 红绿灯/金额成色均配文字，不单靠颜色或符号。
- [x] 6.2 provider 行合并 `accessibilityElement` 为可读语句。
- [x] 6.3 空字段省略策略落实（reportingCount 0 显 "—"、全 healthy 无 reset 行、missing 金额省略、无告警块）。
- [x] 6.4 沿用 DESIGN.md：系统色、radius ≤ 8pt、不嵌 card。

## 7. 验证

- [x] 7.1 Swift 编译 + 现有 menubar 测试通过。
- [x] 7.2 contract fixtures 渲染验证（含 missing cost、无 quota、全 healthy、有告警、avg 高但有 low 等边界）。
- [x] 7.3 `openspec validate redesign-overview-aggregate-view` 通过。
- [ ] 7.4 视觉回归：UsageCard 行为不变；Overview 在 0/1/多 provider、深浅色下正常；provider 多时纵向滚动正常。（需人工跑起 GUI 目检；编译 + 契约测试已过）
