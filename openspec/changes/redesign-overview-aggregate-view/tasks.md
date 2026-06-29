# 任务

> 依赖：本变更消费 `compose-overview-aggregate-primitives` 的后端原子（`MenubarQuotaRollup`、period-aware headline、hourly cost、聚合 diagnostics）。需在该后端变更对接后实现。

## 1. Swift DTO 对接

- [ ] 1.1 `DTO.swift` 新增 `QuotaRollup` / `QuotaRollupAccountRef` 解码类型，挂到 `ProviderView` 与 `DashboardReport`。
- [ ] 1.2 `UsageSummary` 补 `costStatus` / `estimatedCostUsd` 解码；`HourlyBucket` 补 cost 字段。
- [ ] 1.3 确认 headline 字段语义随 period 变化（period-aware），更新相关 mock/fixtures 解码。

## 2. Overview tab 容量 Hero

- [ ] 2.1 新增 `CapacityHeroView`：avg 剩余%（None→"—"）、红绿灯计数（零段省略）、最危险点名（全 healthy 省略）、最近 reset 倒计时。
- [ ] 2.2 颜色/分类用后端 tone，移除前端阈值。
- [ ] 2.3 最危险点名可点击跳转对应 provider tab。

## 3. Overview tab 用量 Hero

- [ ] 3.1 新增 `UsageHeroRow`：token + 金额（按 cost_status 降级）+ 环比（零省略、正橙负绿）。
- [ ] 3.2 period 绑定 `store.usagePeriod`，与下方 UsageCard 同源同口径。
- [ ] 3.3 金额缺失时省略，不显示 `$0`。

## 4. 路由快照与聚合告警

- [ ] 4.1 新增 `RoutingSnapshotView`：每 provider 当前 active 账号，受脱敏设置影响，可点击跳转。
- [ ] 4.2 聚合各 `ProviderView.diagnostics`，仅非空时渲染"需要处理"区块，复用 `DiagnosticView`。

## 5. Overview 组装与清理

- [ ] 5.1 `DashboardView.overview(_:)` 改为：容量 Hero → 用量 Hero → 路由快照 → 告警 → UsageCard。
- [ ] 5.2 移除 Overview 中的 "Providers" 列表用法；若 `OverviewProviderRow` 无其他引用则删除。
- [ ] 5.3 删除 `DashboardView` 中 `lowestQuota` / 平均 / 红绿灯 / `15/40` 阈值 / 相关 helper。

## 6. 单 provider 页 Overview 重构

- [ ] 6.1 新增 `ProviderOverviewCard`：当前 active（突出）、容量分解（ready/exhausted）、平均剩余、最佳备选（+reset，可切）、reset credit（>0 才显示）。
- [ ] 6.2 替换 `providerOverview`，移除 Targets/Alerts 裸 MetricCell。
- [ ] 6.3 provider Overview 主数字用 average，与全局口径一致。

## 7. 可访问性与视觉

- [ ] 7.1 红绿灯/环比/金额成色均配文字，不单靠颜色或符号。
- [ ] 7.2 各 hero 行合并 `accessibilityElement` 为可读语句。
- [ ] 7.3 空区块省略策略落实（无 worst 行、无告警块、零值段）。

## 8. 验证

- [ ] 8.1 Swift 编译 + 现有 menubar 测试通过。
- [ ] 8.2 contract fixtures 渲染验证（含 missing cost、无 quota、全 healthy、有告警等边界）。
- [ ] 8.3 `openspec validate redesign-overview-aggregate-view` 通过。
- [ ] 8.4 视觉回归：UsageCard 行为不变；Overview 在 0/1/多 provider、深浅色下正常。
