# 提案：重设计 Overview 为「用量汇总 + provider 健康卡」总览视图

## 背景

当前 menubar 的 Overview tab 与单 provider 页的 Overview 区块都没有真正承担"总览"职责，且信息等权平铺、缺乏视觉层级：

- **Overview tab** 现在是一张 "Providers" 卡片，每个 provider 一行（`OverviewProviderRow`），显示图标、当前 active target、账号/profile 数、**最低**额度% 和状态点。既重复 `ProviderTabBar` 的导航职责，又把单 provider 明细当总览。
- **单 provider 页的 Overview**（`providerOverview`）是四个裸数字 MetricCell（Tokens / Targets / Lowest / Alerts）+ 一行 active，信息冗余、信息量低。

经多轮 UX 评估，收敛出三条原则：

1. **Overview = 全平台账号的总览**。用量（token/金额）跨所有 provider 汇总；每个 provider 的健康与库存各自成卡。导航交给 tab bar。
2. **明确视觉层级**。菜单栏的天职是"我还能不能继续干活"——quota 健康是视觉焦点（彩色 5h/7d 条），用量汇总是安静的事实带，图表垫底。砍掉不能触发决策的数字（healthy/low/exhausted 计数、全局账号总数）。
3. **组件复用、两种组合**。同一组组件（用量汇总条、provider 健康卡、5h/7d quota 条）在 Overview 喂全平台/全列表数据，在单 provider 页喂该 provider 单份数据。总览是 N 张卡，provider 页是 1 张卡。

本提案是 `compose-overview-aggregate-primitives`（control-plane 聚合 projection）的消费方：UI 只渲染 control-plane 拼好的聚合数据，不在前端自算聚合。

## 目标

- 把 Overview tab 重排为两个视觉块：**用量汇总条**（全平台 token + 金额）→ **provider 健康卡列表**，下接用户满意的 `UsageCard`。
- 用量汇总条展示全平台 **总 token（焦点）+ in/out 拆解（带 ↓↑ 箭头）+ 估算金额（焦点，带成色）**。
- provider 卡展示该 provider 的 **account/profile 数量 + 当前 active + 5h/7d 平均剩余额度条**（复刻账号卡的 `QuotaLine`：阈值刻度竖线 + health 着色）。
- 砍掉视觉噪音：全局 healthy/low/exhausted 计数、全局 account/profile 汇总总数。
- 用量汇总条与 provider 卡封装成可复用组件，单 provider 页用相同组件渲染该 provider 单份数据（Codex 页 = Codex 的用量条 + Codex 的健康卡）。
- 全部聚合口径来自后端原子，前端不持有阈值或均值逻辑。

## 非目标

- 不改后端聚合算法本身的范围（属于 `compose-overview-aggregate-primitives`，本提案在其内补字段）。
- 不改 `UsageCard` 的图表实现、分段维度或交互。
- 不动 `ProviderTabBar` 的导航职责或样式。
- 不新增账号切换/删除/reset 的入口。
- 不引入新的设置项或窗口。
- 不为"provider 过多"做折叠/虚拟列表（现实 ~5-8 个，纵向滚动即可）。
- 不在 Overview 展示 per-account 用量（用量源数据不携带 active 账号归属）。
- 不展示 cache/reasoning 等细分 token（analytics 级，留给 provider 页或 CLI）。

## 用户价值

- 顶部一眼看清全平台烧了多少 token、花了多少钱（量 + 钱双焦点），in/out 拆解一并带出。
- provider 卡用彩色 5h/7d 条把"哪个平台快耗尽"做成视觉焦点，扫一眼即知危险，颜色 + 刻度替代文字计数。
- 总览与单 provider 页视觉与逻辑一致（同组件），"进去和外面长一样"，零学习成本。
- 单 provider 页直接给出该平台的用量 + 健康缩影，对齐用户在该页的核心判断。
