# 提案：重设计 Overview 为跨 provider 总览视图

## 背景

当前 menubar 的 Overview tab 与单 provider 页的 Overview 区块都没有真正承担"总览"职责：

- **Overview tab** 现在是一张 "Providers" 卡片，每个 provider 一行（`OverviewProviderRow`），显示图标、当前 active target、账号/profile 数、**最低**额度% 和状态点，点击跳转。这本质上在重复 `ProviderTabBar` 已经承担的导航职责，而每行展示的又是单 provider 明细——既不是总览，又和 provider tab 内容重叠。下方是用户满意的 `UsageCard`。
- **单 provider 页的 Overview**（`providerOverview`）是四个裸数字 MetricCell：Tokens / Targets / Lowest / Alerts，外加一行 active target。`Targets` 在 Accounts 卡头就能看到、`Alerts` 在 Diagnostics 里，信息冗余且信息量低。

按第一性原理，menubar 是"扫一眼就走"的工具，Overview 应回答那些**横跨所有 provider 才有意义**的问题，而不是逐个 provider 罗列单点数据。导航交给 tab bar，Overview 专注总览。

本提案是 `compose-overview-aggregate-primitives`（control-plane 聚合 projection）的消费方：UI 只渲染 control-plane 拼好的 `QuotaHealthRollup` 与 period 化 `UsageHeadline`，不在前端自算聚合。

## 目标

- 把 Overview tab 从"provider 导航列表"重定位为"跨 provider 总览 / 分诊屏"，按时效性与行动性排序信息。
- 容量数据用 **平均剩余额度 + 红绿灯计数**（聚合），而非单账号最低值；想看最低自己进 provider tab。
- 用量 headline 展示 **token + 金额（带成色）**，金额诚实标注 reported/estimated，缺失时不显示 `$0`。（趋势/环比不在本次 scope。）
- 保留用户满意的 `UsageCard` 不动，只在其上方补一行 hero 结论。
- 把单 provider 页 Overview 从"四个裸数字"改为"现在用谁 / 能切到谁 / 什么时候缓解"，与全局口径统一、层层下钻。
- 全部聚合口径来自后端原子，前端不持有阈值或均值逻辑。

## 非目标

- 不改后端聚合算法本身（属于 `compose-overview-aggregate-primitives`）。
- 不改 `UsageCard` 的图表实现、分段维度或交互。
- 不动 `ProviderTabBar` 的导航职责或样式。
- 不新增账号切换/删除/reset 的入口（沿用现有 provider 页能力）。
- 不引入新的设置项或窗口。

## 用户价值

- 打开 menubar 一眼看清：整个账号池还剩多满、有没有东西快耗尽、最近什么时候缓解、最近花了多少钱、有没有需要处理的告警——无需逐个 tab 翻。
- "平均 + 状态点"既给整体健康判断，又不丢危险信号（均值高但点是红的 → 进去看看），符合"总览就是总览"。
- 单 provider 页直接给出"现在该让谁 active / 可以切到哪个备选 / 什么时候缓解"，对齐用户在该页的唯一核心决策。
