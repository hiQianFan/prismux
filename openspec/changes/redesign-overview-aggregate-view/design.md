## Context

这次重设计建立在三轮思考结论上：

1. **Overview = 总览，不是导航**。导航由 `ProviderTabBar`（Overview tab + 每 provider 一个 icon tab）承担，Overview 内容不该再放"逐 provider 点击跳转"的列表。
2. **容量用 average 不用 lowest**。Overview 的每个聚合是"一组账号的总览"，放单账号极值在逻辑上错位。均值回答"池子整体多满"，状态点回答"要不要进去看"。想看最低值去 provider tab。
3. **金额诚实展示**。token 是骨架（永远在），金额是注解（带 `CostStatus` 成色，reported 强调、estimated 加 `~`、missing 隐藏不占位）。

信息排序原则：按**时效性 + 行动性**——会出事的（容量）在最前，要动手的（告警）紧跟，回顾性的（花了多少）居中，分析性的（趋势图）垫底。

数据全部来自 `compose-overview-aggregate-primitives` 的 control-plane projection：`QuotaHealthRollup`（挂在 `DashboardAggregateView` 全局 + 每个 `ProviderAggregateView`）与 period 化的 `UsageHeadline`（headline + hourly/series）。本提案不复算任何聚合。

## Goals / Non-Goals

**Goals:**

- 定义 Overview tab 的区块构成、排序与各区块的数据来源（全部指向后端原子）。
- 定义单 provider 页 Overview 区块的新构成。
- 定义金额/红绿灯的展示规则与降级行为。
- 复用现有视觉语言（Card、MetricCell、ProviderStyle、UsageCard）。

**Non-Goals:**

- 后端聚合算法、阈值、cost 折叠（在另一提案）。
- UsageCard 图表内部、tab bar 导航、账号操作入口。

## Decisions

### 1. Overview tab 区块构成与排序

自上而下：

```
┌─ 容量 Hero ───────────────────────────────┐
│ 池子健康：avg 剩余% + 红绿灯计数            │  ← dashboard.quota_rollup（折叠全部账号）
│ 最危险点名：⚠ Claude #2 · 8% · resets 2h   │  ← rollup.worst
│ 最近 reset 倒计时                          │  ← rollup.soonest_reset_at_unix
├─ 用量 Hero（一行）─────────────────────────┤
│ 2.4M tokens · ~$12.40 est.                 │  ← UsageHeadline(period)
├─ 路由快照 ────────────────────────────────┤
│ Codex→#1  Claude→#2  Gemini→#1             │  ← 每 provider active 账号（已分组）
├─ 需要处理（有才显示）──────────────────────┤
│ 聚合诊断：auth 过期 / 刷新失败 …            │  ← 跨 provider diagnostics 聚合
├─ UsageCard（保持不动）────────────────────┤
│ Today/7d/30d 图表 + 分段 + 图例             │
└───────────────────────────────────────────┘
```

砍掉现有的 "Providers" 导航列表（`OverviewProviderRow` 在 Overview 的用法）。

### 2. 容量 Hero 的展示规则

- 主数字：`avg_remaining_percent_x100` → "平均剩余 62%"。`reporting_count == 0` 时显示 "—"，不显示 0%。
- 红绿灯：`healthy_count / low_count / exhausted_count` → "6 healthy · 1 low · 1 exhausted"，零值的段省略。
- 最危险点名：`worst` 存在且非 healthy 时显示 "⚠ {provider} #{n} · {remaining}% · resets {相对时间}"；全 healthy 时此行省略。
- 颜色用后端分类（low=橙、exhausted=红），前端不持有 `15/40` 阈值。

### 3. 用量 Hero 的金额规则

一行：`{token} tokens · {金额}`，金额按 `cost_status` 降级：

| cost_status | 展示 |
|---|---|
| ProviderReported | `$12.40` |
| Estimated | `~$12.40 est.` |
| Mixed | `~$12.40` |
| Missing | 省略金额，只显示 token |

period 跟随 `UsageCard` 的 toggle（同一个 `store.usagePeriod`），headline 与图表同口径同源。趋势/环比不在本次 scope（属另行评估的业务功能）。

### 4. 路由快照

每 provider 一行紧凑展示当前 active 账号（`provider → active label`）。这是 OpenMux 独有的"系统当前路由配置"快照。无 active 时显示 "—"。点击该行跳到对应 provider tab（保留轻导航，但不是主列表）。受 `hidePersonalIdentifiers` 影响，沿用现有脱敏逻辑。

### 5. 需要处理（聚合告警）

把各 `MenubarProviderView.diagnostics` 聚合成一个列表，仅在非空时显示该区块，复用现有 `DiagnosticView`。让用户不用逐 tab 翻就知道哪里要动手。

### 6. 单 provider 页 Overview 重构

替换现有四个裸数字，改为围绕"现在用谁 / 能切到谁 / 何时缓解"：

```
┌─ {Provider} Overview ─────────────────────┐
│ 当前 active：#2 Account（突出）            │  ← provider active
│ 容量：3 accounts · 2 ready · 1 exhausted   │  ← provider_view.quota_rollup 计数
│ 平均剩余：68%                              │  ← rollup.avg_remaining
│ 最佳备选：#3 · 95% · resets 1h ↪可切       │  ← rollup.best_alternative
│ Reset credits：2 可用                      │  ← rollup.reset_credit_total（>0 才显示）
└───────────────────────────────────────────┘
```

主数字与全局口径一致（average），下方补 drill-down 才有的"具体哪个能切"。`Targets`/`Alerts` 裸计数移除（信息已在 Accounts 卡头 / Diagnostics）。

### 7. 复用与新增的 Swift 组件

- 复用：`Card`、`MetricCell`、`ProviderStyle`、`ProviderIcon`、`UsageCard`、`DiagnosticView`、脱敏逻辑。
- 新增：`CapacityHeroView`、`UsageHeroRow`、`RoutingSnapshotView`、provider 页的 `ProviderOverviewCard`（替换 `providerOverview`）。
- `OverviewProviderRow` 从 Overview 移除；若无其他引用则删除。

## 视觉与可访问性

- 颜色沿用品牌色（ProviderStyle）与 tone（success/warning/danger），红绿灯不仅靠颜色，配文字（healthy/low/exhausted）满足色觉无障碍。
- 金额成色用文字（`est.`）而非仅靠 `~`。
- 各 hero 行 `accessibilityElement` 合并为可读语句。

## Risks / Trade-offs

- **信息密度上升**：Overview 从两块变四块 + UsageCard。通过"空区块省略"（全 healthy 无 worst 行、无告警不显示该块、零值段省略）控制视觉负担。
- **路由快照与 tab bar 的轻微导航重叠**：可接受——tab bar 是"切视图"，快照是"读当前路由状态"，点击跳转是顺带。
- **依赖后端原子先落地**：本提案的数据全部来自 `compose-overview-aggregate-primitives`，需在其之后实现。

## Migration

- 纯前端渲染变化，无状态迁移。
- 依赖后端 DTO 新增字段（`MenubarQuotaRollup`、headline period 语义、hourly cost）；在后端提案合并后对接。
- 删除前端聚合代码（`lowestQuota`/平均/红绿灯/阈值）在对接 rollup 时一并完成。

## Open Questions

- 环比 "上期" 在 today period 下取"昨天同时段"还是"前 24h"？沿用后端 `trend` 原子的最终定义。
- 全局容量 Hero 的"最危险点名"是否需要可点击直达该账号所在 provider tab？倾向需要（与路由快照一致的轻导航）。
