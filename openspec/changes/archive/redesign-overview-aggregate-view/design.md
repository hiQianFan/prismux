## Context

这次重设计建立在多轮 UX 评估的收敛结论上：

1. **Overview = 全平台账号总览**。用量（token/金额）跨所有 provider 汇总成一条；每个 provider 的健康与库存各自成卡。导航由 `ProviderTabBar` 承担。
2. **明确视觉层级**。menubar 的天职是"我还能不能继续干活"——quota 健康（彩色 5h/7d 条）是焦点，用量汇总是安静的事实带，图表垫底。砍掉不能触发决策的数字。
3. **组件复用、两种组合**。同一组组件在 Overview 喂全平台/全列表，在单 provider 页喂该 provider 单份。

数据全部来自 `compose-overview-aggregate-primitives` 的 control-plane projection。本提案不复算任何聚合。

## Goals / Non-Goals

**Goals:**

- 定义 Overview tab 的两块构成（用量汇总条 → provider 健康卡列表 → UsageCard）与视觉层级。
- 定义用量汇总条字段（total token + in/out + cost）与降级行为。
- 定义 provider 卡构成（account/profile 数 + active + 5h/7d 平均条）。
- 定义 5h/7d 条复用账号卡 `QuotaLine` 的封装方式。
- 定义单 provider 页用相同组件渲染单份数据。

**Non-Goals:**

- 后端聚合算法、阈值、cost 折叠（在另一提案，本提案在其内补字段）。
- UsageCard 图表内部、tab bar 导航、账号操作入口。
- provider 过多时的折叠/虚拟列表。
- per-account 用量拆分、cache/reasoning 细分 token。

## Decisions

### 1. Overview tab 区块构成与视觉层级

两个视觉块 + 图表，自上而下：

```
┌─ 用量汇总条（全平台）──────────────────────┐
│ 2.4M tokens            ~$11.40 est.        │  ← 焦点：量 | 钱（双 hero）
│ ↓ 1.6M in   ↑ 0.8M out                     │  ← 注解：in/out 是 total 的拆解，缩进从属
├─ provider 健康卡列表 ──────────────────────┤
│ ◗ Codex          3 accts · 1 prof   → #2   │  ← 身份 + 库存 + active
│   5h  ▕▔▔▔▔|▔▔▔:░░▏           72%          │  ← 焦点：5h/7d 平均条（QuotaLine 复用）
│   7d  ▕▔▔▔▔|▔▔:░░░▏           58%          │
│ ───────────────────────────────────────── │
│ ◖ Claude         2 accts            → #1   │
│   5h  ▕|▔:░░░░░░░░▏           18%  ← 红     │
│   7d  ▕▔▔|:░░░░░░▏            30%  ← 黄     │
├─ Token Usage（保持不动）──────────────────┤
│ 2.4M tokens · 30d        Today│7d│30d       │
│ [stacked chart] + 分段图例                  │
└─────────────────────────────────────────────┘
```

视觉层级（squint test 下的优先级）：provider 的彩色 5h/7d 条 = 焦点；用量汇总条 = 安静事实带（数字 semibold，但无彩色）；图表 = 细节。

### 2. 用量汇总条（UsageStatsStrip）

全平台汇总，内部三档层级：

- **总 token**（hero）：`aggregate.usageHeadline.totalTokens`，大号 semibold。
- **金额**（hero，与 token 并列）：`aggregate.usageHeadline` 的 cost，按 `cost_status` 降级。量和钱是用户最关心的两个总览数，并列双焦点。
- **in / out**（二级注解，缩进从属 total）：`↓ {in} in   ↑ {out} out`。↓ = 输入（喂给模型），↑ = 输出（模型生成），与下载/上传同构。颜色中性（secondary），箭头表方向不表状态。

金额降级：

| cost_status | 展示 |
|---|---|
| ProviderReported | `$11.40` |
| Estimated | `~$11.40 est.` |
| Mixed | `~$11.40` |
| Missing | 省略金额位，只剩单焦点 `2.4M tokens` |

**不展示**全局 account/profile 汇总总数——该数不能触发决策，是噪音；数量落回各 provider 卡。也不展示 cache/reasoning 细分（analytics 级）。

period 跟随 `store.usagePeriod`，与下方 UsageCard 同源。

### 3. provider 健康卡（ProviderSummaryCard）

每个 provider 一张卡，是该平台的完整缩影：

- **身份**：图标 + provider 名。
- **库存**：`{n} accts · {m} prof`（account/profile 数量，来自 `providerAggregate.account_count / profile_count`）。数量是 per-provider 属性，落在这里语境完整。
- **当前 active**：`→ {active 身份}`，来自 `providerAggregate.activeTarget`，受脱敏设置影响，无 active 显占位。
- **5h/7d 平均条**：两行，复用账号卡的 `QuotaLine`（见决策 4）。值为该 provider 所有账号在该窗口类的**平均剩余**。

点击卡跳转对应 provider tab。

### 4. 5h/7d 条复用 QuotaLine

把账号卡里现有的 `QuotaLine`（`TargetRows.swift`，当前 private）提升为共享组件，Overview 的 provider 卡与账号卡渲染**同一套** 5h/7d 条：

- **阈值刻度竖线**：`warnThreshold`（0.50）/ `criticalThreshold` 各一条竖线，给"离麻烦多远"固定参照。
- **health 着色**：`remaining ≤ critical → 红`、`≤ warn → 黄`、否则绿。条色编码剩余健康，窗口身份由 "5h"/"7d" label 表达。

provider 卡喂进去的是**聚合后的平均窗口**（后端 per-window-class 平均），账号卡喂的是单账号窗口；同一组件，不同输入。"进去和外面长一样"靠共用组件天然满足。

> 取舍：平均会抹平单个快死的账号（池子 `[95,95,20]` 平均 70%，条显绿，20% 那个看不见）。这是有意接受的代价——想看单账号进 provider tab 的账号列表。

### 5. 单 provider 页 Overview 用相同组件

单 provider 页顶部 = `UsageStatsStrip(该 provider 的 usageHeadline)`，下面 = `ProviderSummaryCard(该 provider)`。Codex 页就只展示 Codex 的用量与健康。靠传单 provider 数据进同一组件实现，无特判。

移除现有四个裸 MetricCell（Targets/Alerts 信息已在 Accounts 卡头与 Diagnostics）。

### 6. 复用与新增的 Swift 组件

- 复用：`Card`、`ProviderStyle`、`ProviderIcon`、`UsageCard`、脱敏逻辑。
- 提升为共享：`QuotaLine`（从 `TargetRows.swift` private 提出）。
- 新增：`UsageStatsStrip`（total + cost 双焦点 + in/out 注解）、`ProviderSummaryCard`（身份 + 库存 + active + 5h/7d 条）。
- 移除：`OverviewProviderRow`、`DashboardView` 中 `lowestQuota` / 平均 / 红绿灯 / 阈值 / 全局 capacity hero 及相关死组件。

## 后端字段补充（在 compose-overview-aggregate-primitives 内）

1. `UsageHeadline` 加 `input_tokens` / `output_tokens`（来自 `UsageTokenBreakdown`），供用量汇总条的 in/out。
2. `QuotaHealthRollup`（或 `ProviderAggregateView`）加 **per-window-class 平均剩余**：按 5h(short/session) / 7d(weekly) 分类，对该 provider 所有账号在该窗口类求平均剩余。窗口分类沿用现有 frontend `quotaWindow` picker 的 id/label 文本判定（`5h|session|short` / `7d|week`）。
3. `ProviderAggregateView.usage_headline` 已具备（前一轮已加），保留。
4. account/profile count 已具备。

## 视觉与可访问性

- 沿用 DESIGN.md：原生 macOS 系统色、系统字体、Restrained、无装饰、radius ≤ 8pt、section 不嵌 card（用量汇总条与 provider 卡是同级 card，不互相嵌套）。
- 5h/7d 条颜色配 "5h"/"7d" 文字标签与百分比数字，不单靠颜色。
- 金额成色用文字（`est.`），in/out 用箭头 + 文字。
- provider 卡 `accessibilityElement` 合并为可读语句。

## Risks / Trade-offs

- **平均抹平单个快死账号**：接受；想看单账号进 provider tab。Overview 的彩色条仍会因平均偏低而变色，提供粗粒度预警。
- **删全局计数后"扫危险"靠条色**：5h/7d 彩色条比文字计数 squint test 更强，净赚。
- **per-window-class 平均是新后端 fold**：数据已在（每账号 windows 全在），是新增折叠，复用现有窗口分类逻辑。
- **provider 过多**：纵向滚动（popover 已支持 560-640pt），不做特殊布局。

## Migration

- 前端渲染重排 + 后端补字段（in/out token、per-window 平均），无状态迁移。
- bump `CONTROL_PLANE_SCHEMA_VERSION`，重生成 fixtures，更新 contract 测试。
- 删除前端聚合代码与死组件在对接时一并完成。

## Open Questions

- 无。
