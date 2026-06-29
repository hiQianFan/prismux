## Context

这次重设计建立在 UX/PM 联合评估的结论上：

1. **Overview = 总览，不是导航**。导航由 `ProviderTabBar`（Overview tab + 每 provider 一个 icon tab）承担，Overview 内容不该再放"逐 provider 点击跳转"的列表样式。
2. **砍掉全局聚合**。`avg(各 provider)` 是没人照着行动的数，会把危险平均没。Overview 主体直接是 **providers 列表**，每行即一个 provider 的总览，不在其上再加全局大数字块。
3. **每行带入"总视角"**。每行 = 该 provider 的池子健康（avg% + 红绿灯计数）+ 当前 active 身份 + 该 provider 本期 token/花费总量。容量用 average（回答"池子整体多满"），红绿灯计数回答"有没有要出事的"——avg 单独出现是漂亮的谎言，必须配计数。
4. **金额诚实展示**。token 是骨架（永远在），金额是注解（带 `CostStatus` 成色，reported 强调、estimated 加 `~`、missing 隐藏不占位）。

信息排序原则：按**时效性 + 行动性**——会出事的（容量红绿灯）在前，路由状态紧跟，回顾性的（token/花费）居后，要动手的（告警）单列，分析性的（趋势图）垫底。

数据全部来自 `compose-overview-aggregate-primitives` 的 control-plane projection：`QuotaHealthRollup`（挂在每个 `ProviderAggregateView`）与 period 化的 `UsageHeadline`。本提案不复算任何聚合。

## Goals / Non-Goals

**Goals:**

- 定义 Overview tab 的区块构成（providers 列表 → 告警 → UsageCard）、排序与各区块数据来源（全部指向后端原子）。
- 定义 provider 行的字段构成与降级行为。
- 定义单 provider 页 Overview 区块的新构成。
- 定义金额/红绿灯的展示规则与降级行为。
- 复用现有视觉语言（Card、ProviderStyle、ProviderIcon、UsageCard、DiagnosticView、脱敏逻辑）。

**Non-Goals:**

- 后端聚合算法、阈值、cost 折叠（在另一提案）。
- UsageCard 图表内部、tab bar 导航、账号操作入口。
- provider 过多时的折叠/虚拟列表（YAGNI，纵向滚动即可）。
- per-account 用量拆分（源数据无账号归属）。

## Decisions

### 1. Overview tab 区块构成与排序

砍掉独立的全局容量 Hero。自上而下只剩三段（告警块空则消失，实际常态两段）：

```
┌─ Providers ───────────────────────────────┐
│ ◗ Codex     72% avg ▕▔▔▔▔▔▏   3 · 1 low   │  ← 行 1：身份 · avg% · tone 条 · 红绿灯计数
│   → 现在用 #2 Account                       │  ← 行 2：当前 active 身份（路由状态）
│   1.2M tokens · ~$4.10 est.                 │  ← 行 3：该 provider 本期 token + 花费
│ ──────────────────────────────────────────│
│ ◖ Claude    18% avg ▕▔▏       2 · 1 low   │
│   → 现在用 #1 · resets 2h                   │  ← low/exhausted：active 行尾附 reset 倒计时
│   840k tokens · $6.20                       │
│ ──────────────────────────────────────────│
│ ○ Gemini    95% avg ▕▔▔▔▔▔▏   1 healthy   │
│   → 现在用 #3                               │
│   320k tokens · ~$1                         │
├─ 需要处理（有才显示）──────────────────────┤
│ ⚠ Claude · auth 过期 / 刷新失败 …           │  ← 跨 provider diagnostics 聚合
├─ Token Usage（保持不动）──────────────────┤
│ 2.4M tokens · 30d        Today│7d│30d       │
│ [▁▂▅▇▆▃▁ stacked chart] + 分段图例          │
└─────────────────────────────────────────────┘
```

砍掉现有的全局 pool/capacity 聚合块（不再有"平均剩余 62%"这类全局大数字）。`OverviewProviderRow` 改为承载上述三行结构（或新建 `ProviderSummaryRow` 替换它）。

### 2. provider 行的展示规则

一个 provider 一行（最多三行高），按行动性排列字段：

- **avg%**：`providerAggregate.quotaHealth.facts.avgRemainingPercentX100` → "72% avg"。`reportingCount == 0` 时显示 "—"，不显示 0%。
- **tone 条**：横向 headroom 条，宽度 = avg%，颜色用后端 `quotaHealth.statusTone`（low=橙、exhausted=红、healthy=绿）。前端不持有 `15/40` 阈值。
- **红绿灯计数**：`healthyCount / lowCount / exhaustedCount` → "3 · 1 low"（零值段省略；全 healthy 时简化为 "3 healthy"）。这是 avg 的防骗补丁——avg 高但有 low/exhausted 时此处暴露。
- **当前 active**：`providerAggregate.activeTarget` → "现在用 #2 Account"。无 active 显示 "—"。受 `hidePersonalIdentifiers` 影响，沿用现有脱敏逻辑。active 只表达路由身份，不带用量。
- **reset 倒计时**：仅当该 provider 非全 healthy（low/exhausted 计数 > 0）时，在 active 行尾附 `· resets {相对时间}`，来自 `quotaHealth.facts.soonestResetAtUnix`。全 healthy 省略。
- **token + 花费**：该 provider 本期 token 总量 + 花费，来自 `providerAggregate.usageHeadline`（见决策 5）。period 跟随 UsageCard 的 toggle。金额按 `cost_status` 降级（见决策 3）。

点击行跳转对应 provider tab（保留轻导航）。

### 3. token / 花费的金额规则

`{token} tokens · {金额}`，金额按 `cost_status` 降级：

| cost_status | 展示 |
|---|---|
| ProviderReported | `$4.10` |
| Estimated | `~$4.10 est.` |
| Mixed | `~$4.10` |
| Missing | 省略金额，只显示 token |

provider 行的 token/花费 period 跟随 `store.usagePeriod`（同一个 toggle），与下方 UsageCard 同口径同源。趋势/环比不在本次 scope。

### 4. 需要处理（聚合告警）

把各 `ProviderAggregateView.diagnostics` 与 `DashboardAggregateView.diagnostics` 聚合成一个列表，仅在非空时显示该区块，复用现有 `DiagnosticView`。让用户不用逐 tab 翻就知道哪里要动手。这是砍掉全局聚合后的两个保命信号之一（另一个是每行红绿灯计数）。

> 去重：provider diagnostics 按 `provider_id` filter 得到；`DashboardAggregateView.diagnostics` 来自账号级（无 provider_id）的诊断。两者直接 concat 不重复。

### 5. 后端字段补充：per-provider usage headline

provider 行要展示"该 provider 本期 token + 花费"。数据已存在于 `report.providerUsage[provider].usage`（含 `total_tokens` / `estimated_cost_usd` / `cost_status`），但挂载点不顺手。

**后端补充**：在 `ProviderAggregateView` 上新增 `usage_headline: UsageHeadline`，由 control-plane 用该 provider 的 period usage 折叠产生。这样 provider 行只读 `providerAggregate.usageHeadline`，不必再去 `providerUsage` 里按 provider 名 join。period 随 `DashboardQuery.usage_period` 变化，与图表同源。

> 这是唯一的后端字段新增。其余（avg/min、红绿灯计数、active、soonest reset）`QuotaHealthRollup` 已全部具备。

### 6. 单 provider 页 Overview 重构

替换现有四个裸数字，改为围绕"现在用谁 / 能切到谁 / 何时缓解"：

```
┌─ {Provider} Overview ─────────────────────┐
│ 现在用 #2 Account（突出）                  │  ← provider active
│ 3 accounts · 2 healthy · 1 exhausted       │  ← providerAggregate.quotaHealth 计数
│ 平均剩余 68% ▕▔▔▔▔▏                        │  ← quotaHealth.facts.avgRemainingPercentX100 + tone 条
│ 最佳备选 #3 · 95% · resets 1h  ↪可切        │  ← quotaHealth.bestAlternative（+ reset）
│ Reset credits：2 可用                      │  ← quotaHealth.facts.resetCreditTotal（>0 才显示）
└─────────────────────────────────────────────┘
```

主数字与全局口径一致（average），下方补 drill-down 才有的"具体哪个能切"。`Targets`/`Alerts` 裸计数移除（信息已在 Accounts 卡头 / Diagnostics）。

> 最佳备选只放在 provider 页：Overview 行已三行高，再加备选会过挤，且备选是"决定切到谁"的下钻动作，属于 provider 页职责。

### 7. 复用与新增的 Swift 组件

- 复用：`Card`、`ProviderStyle`、`ProviderIcon`、`UsageCard`、`DiagnosticView`、脱敏逻辑。
- 新增：`ProviderSummaryRow`（三行结构：身份+avg+tone条+计数 / active+reset / token+花费）、provider 页的 `ProviderOverviewCard`（替换 `providerOverview`）。
- `OverviewProviderRow` 被 `ProviderSummaryRow` 替换；若无其他引用则删除。
- 移除全局 capacity hero 相关 helper（`lowestQuota` / 平均 / 红绿灯 / 阈值）。

## 视觉与可访问性

- 沿用 DESIGN.md：原生 macOS 系统色、系统字体、Restrained、无装饰、radius ≤ 8pt、section 不嵌 card。
- 颜色沿用品牌色（ProviderStyle）与 tone（success/warning/danger），红绿灯不仅靠颜色，配文字（healthy/low/exhausted）满足色觉无障碍。
- 金额成色用文字（`est.`）而非仅靠 `~`。
- provider 行 `accessibilityElement` 合并为可读语句（如 "Codex, 平均剩余 72%, 3 个账号 1 个偏低, 现在用 2 号账号, 本期 1.2M tokens 约 4.1 美元"）。

## Risks / Trade-offs

- **删全局聚合后丢失"一眼扫危险"的能力**：通过每行红绿灯计数 + 聚合告警块两条信号兜底。只要这两个在，删 pool 是净赚。
- **provider 行信息密度上升（三行）**：通过"空字段省略"（全 healthy 无 reset 行、reportingCount 0 显 "—"、missing 金额省略）控制。三行仍比当前 5 段堆叠轻。
- **provider 过多**：现实 ~5-8 个，纵向滚动（popover 已支持 560-640pt 可滚）即可，不做特殊布局。真超出再加，是一句话功能不是当下债。
- **依赖后端原子先落地**：本提案数据全部来自 `compose-overview-aggregate-primitives`，需在其之后实现；新增的 `ProviderAggregateView.usage_headline` 也在该后端变更内落地。

## Migration

- 纯前端渲染变化 + 一个后端字段新增（`ProviderAggregateView.usage_headline`），无状态迁移。
- 删除前端聚合代码（`lowestQuota` / 平均 / 红绿灯 / 阈值 / 全局 capacity hero）在对接 rollup 时一并完成。

## Open Questions

- 无。（per-account 用量、provider 折叠、趋势/环比均已明确移出 scope。）
