# 提案：重设计 Overview 为「provider 列表 + 用量图表」总览视图

## 背景

当前 menubar 的 Overview tab 与单 provider 页的 Overview 区块都没有真正承担"总览"职责：

- **Overview tab** 现在是一张 "Providers" 卡片，每个 provider 一行（`OverviewProviderRow`），显示图标、当前 active target、账号/profile 数、**最低**额度% 和状态点，点击跳转。这本质上在重复 `ProviderTabBar` 已经承担的导航职责，而每行展示的又是单 provider 明细——既不是总览，又和 provider tab 内容重叠。下方是用户满意的 `UsageCard`。
- **单 provider 页的 Overview**（`providerOverview`）是四个裸数字 MetricCell：Tokens / Targets / Lowest / Alerts，外加一行 active target。`Targets` 在 Accounts 卡头就能看到、`Alerts` 在 Diagnostics 里，信息冗余且信息量低。

经 UX/PM 评估，得到两条收敛结论：

1. **砍掉全局聚合（global pool avg）**。`avg(Codex 72%, Claude 18%) = 45%` 是个没人会照着行动的数——它恰好把危险（Claude 18%）平均没了。一个不能触发任何决策的全局数字就是噪音。**provider 列表本身就是 overview**，不需要在它上面再顶一个聚合大数字块。
2. **每行带入"总视角"**。Overview 的每行展示该 provider 的 **池子健康（avg% + 红绿灯计数）**、**当前路由到谁（active 身份）**、以及该 **provider 本期的 token 与花费总量**。active 只表达"现在用谁"（路由状态），用量/花费是 provider 级总量——符合"overview = 总览"。

按第一性原理，menubar 是"扫一眼就走"的工具。Overview 应回答那些横跨账号池才有意义的问题，且每个数字都要能触发决策。导航交给 tab bar，Overview 专注总览与分诊。

本提案是 `compose-overview-aggregate-primitives`（control-plane 聚合 projection）的消费方：UI 只渲染 control-plane 拼好的 `QuotaHealthRollup` 与 period 化 `UsageHeadline`，不在前端自算聚合。

## 目标

- 把 Overview tab 从"provider 导航列表"重定位为"跨 provider 总览 / 分诊屏"，主体即 **providers 列表**，下接用户满意的 `UsageCard`。
- 每个 provider 行展示：**平均剩余额度 + 红绿灯计数**（聚合，非单账号最低值）、**当前 active 身份**、**该 provider 本期 token + 花费（带成色）**、低额度时附 **reset 倒计时**。
- 砍掉独立的全局容量 Hero 大数字块；保命信号交给"每行红绿灯计数 + 聚合告警块"。
- 用量/花费 headline 与 `UsageCard` 同 period 同源；金额诚实标注 reported/estimated，缺失时不显示 `$0`。
- 把单 provider 页 Overview 从"四个裸数字"改为"现在用谁 / 能切到谁 / 什么时候缓解"，与全局口径统一、层层下钻。
- 全部聚合口径来自后端原子，前端不持有阈值或均值逻辑。

## 非目标

- 不改后端聚合算法本身（属于 `compose-overview-aggregate-primitives`）。
- 不改 `UsageCard` 的图表实现、分段维度或交互。
- 不动 `ProviderTabBar` 的导航职责或样式。
- 不新增账号切换/删除/reset 的入口（沿用现有 provider 页能力）。
- 不引入新的设置项或窗口。
- 不为"provider 过多"做折叠/虚拟列表——现实天花板 ~5-8 个，纵向滚动即可（YAGNI）。
- 不在 Overview 展示 per-account 用量——用量源数据（本地工具日志）不携带 active 账号归属，无法按账号拆分。

## 用户价值

- 打开 menubar 一眼看清每个 provider：还剩多满、有没有账号快耗尽、现在路由到谁、本期烧了多少 token 和钱、什么时候缓解——无需逐个 tab 翻。
- "avg + 红绿灯计数"既给整体健康判断，又不丢危险信号（avg 72% 但有 1 个账号 low → 进去看看），避免漂亮的均值掩盖危险。
- 单 provider 页直接给出"现在该让谁 active / 可以切到哪个备选 / 什么时候缓解"，对齐用户在该页的唯一核心决策。
