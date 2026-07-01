## ADDED Requirements

### Requirement: Overview tab 以用量汇总条与 provider 健康卡为主体

OpenMux Menubar 的 Overview tab SHALL 自上而下展示全平台用量汇总条、provider 健康卡列表、用量图表。SHALL NOT 以"每个 provider 一行、点击跳转"的纯导航列表样式作为主内容。逐 provider 的 tab 导航 SHALL 由 tab bar 承担。

#### Scenario: 打开 Overview tab

- **WHEN** 用户选中 Overview tab
- **THEN** 页面 SHALL 依次展示用量汇总条、provider 健康卡列表、用量图表
- **AND** quota 健康条 SHALL 是视觉焦点

### Requirement: 用量汇总条展示全平台 token 与金额

用量汇总条 SHALL 展示跨所有 provider 汇总的总 token、输入/输出 token 拆解与估算金额。SHALL NOT 展示全平台 account/profile 汇总总数（该数量落在各 provider 卡）。SHALL NOT 展示 cache/reasoning 等细分 token。

#### Scenario: 展示总 token 与金额

- **WHEN** Overview 渲染用量汇总条
- **THEN** 它 SHALL 展示全平台总 token 与估算金额作为并列焦点
- **AND** 金额 SHALL 标注 `CostStatus` 成色

#### Scenario: 展示输入/输出拆解

- **WHEN** 用量汇总条渲染
- **THEN** 它 SHALL 展示输入 token 与输出 token，并以方向标记（如 ↓ 输入 / ↑ 输出）区分
- **AND** 输入/输出 SHALL 在视觉上从属于总 token（次级层级）

#### Scenario: 金额缺失

- **WHEN** cost_status 为 Missing
- **THEN** 汇总条 SHALL 省略金额，只展示 token
- **AND** SHALL NOT 显示 `$0.00`

#### Scenario: period 切换

- **WHEN** 用户切换 Today/7d/30d
- **THEN** 汇总条数字 SHALL 反映所选 period
- **AND** SHALL 与同 period 的图表来自同一份数据

#### Scenario: 不展示库存汇总

- **WHEN** 汇总条渲染
- **THEN** 它 SHALL NOT 展示全平台 account/profile 总数

### Requirement: provider 健康卡展示库存、active 与 5h/7d 平均额度条

每个 provider 健康卡 SHALL 展示该 provider 的 account/profile 数量、当前 active 身份、以及 5h/7d 窗口的平均剩余额度条。SHALL NOT 展示 healthy/low/exhausted 计数。

#### Scenario: 渲染 provider 卡

- **WHEN** Overview 渲染某 provider 卡
- **THEN** 它 SHALL 展示该 provider 的 account 与 profile 数量
- **AND** SHALL 展示当前 active 账号身份（受隐私设置脱敏）
- **AND** SHALL 展示 5h 与 7d 窗口的平均剩余额度条

#### Scenario: 额度条复用账号卡视觉

- **WHEN** provider 卡渲染 5h/7d 额度条
- **THEN** 它 SHALL 复用账号卡的额度条组件（阈值刻度 + health 着色）
- **AND** 条色 SHALL 由后端给出的剩余比例决定，前端 SHALL NOT 硬编码业务阈值之外的判断

#### Scenario: 平均额度来自后端按窗口类聚合

- **WHEN** provider 卡的 5h/7d 条取值
- **THEN** 它 SHALL 读取后端按窗口类（5h / 7d）聚合的平均剩余
- **AND** SHALL NOT 在前端对账号窗口重新求平均

#### Scenario: 不展示健康计数

- **WHEN** provider 卡渲染
- **THEN** 它 SHALL NOT 展示 healthy/low/exhausted 数字计数（健康由额度条颜色表达）

### Requirement: 单 provider 页复用相同的汇总条与健康卡组件

单 provider 页的 Overview 区块 SHALL 复用用量汇总条与 provider 健康卡组件，喂入该 provider 的单份数据。SHALL NOT 展示与 Accounts/Diagnostics 卡片重复的裸计数（如 Targets/Alerts）。

#### Scenario: 渲染 provider 页 Overview

- **WHEN** 用户进入某 provider tab
- **THEN** Overview SHALL 以该 provider 的用量数据渲染用量汇总条
- **AND** SHALL 以该 provider 的数据渲染同一种健康卡（库存 + active + 5h/7d 条）
- **AND** SHALL NOT 展示 Targets 或 Alerts 裸计数

### Requirement: 聚合数据来自后端原子

Overview 与 provider Overview 的全部聚合 SHALL 消费后端 `ProviderAggregateView`、`QuotaHealthRollup`（含按窗口类聚合的平均）与 period-aware `UsageHeadline`（含 input/output token）数据，SHALL NOT 在客户端重新实现均值、计数、阈值或全局聚合逻辑。

#### Scenario: 前端不复算聚合

- **WHEN** 任一 Overview 区块渲染聚合数字
- **THEN** 它 SHALL 读取后端提供的字段
- **AND** SHALL NOT 在 Swift 端重算这些聚合
- **AND** SHALL NOT 在前端构造全局 token 汇总或按窗口类平均
