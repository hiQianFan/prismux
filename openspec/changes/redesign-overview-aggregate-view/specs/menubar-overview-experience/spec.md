## ADDED Requirements

### Requirement: Overview tab 展示跨 provider 总览而非 provider 导航列表

OpenMux Menubar 的 Overview tab SHALL 展示跨 provider 的聚合总览，SHALL NOT 以"每个 provider 一行、点击跳转"的列表作为主内容。逐 provider 的导航 SHALL 由 tab bar 承担。

#### Scenario: 打开 Overview tab

- **WHEN** 用户选中 Overview tab
- **THEN** 页面 SHALL 展示容量总览、用量总览与（存在时的）聚合告警
- **AND** 页面 SHALL NOT 展示重复 tab bar 职责的 provider 跳转列表

### Requirement: 容量总览展示聚合均值与红绿灯而非单账号最低值

容量 Hero SHALL 展示后端给出的平均剩余额度与红绿灯计数，SHALL NOT 以单账号最低额度作为主指标。无任何上报账号时 SHALL 显示空占位而非 0%。

#### Scenario: 展示池子健康

- **WHEN** Overview 渲染容量总览
- **THEN** 主数字 SHALL 为后端 `avg_remaining`
- **AND** SHALL 展示 healthy/low/exhausted 计数
- **AND** SHALL NOT 在客户端计算或硬编码额度阈值

#### Scenario: 无 quota 上报

- **WHEN** 全部账号无 quota 上报
- **THEN** 平均剩余 SHALL 显示为占位（如 "—"）
- **AND** SHALL NOT 显示 0%

#### Scenario: 均值高但有账号耗尽

- **WHEN** 平均剩余较高但存在 exhausted 账号
- **THEN** Overview SHALL 通过红绿灯计数或最危险点名暴露该危险
- **AND** SHALL NOT 仅凭高均值让危险不可见

### Requirement: 用量总览展示 token 与带成色金额

用量 Hero SHALL 展示所选 period 的 token 总量与估算金额。金额 SHALL 标注 `CostStatus` 成色，SHALL 在 missing 时省略金额而非显示 0 金额。headline SHALL 与同 period 的 UsageCard 同源。

#### Scenario: 金额可信

- **WHEN** cost_status 为 ProviderReported
- **THEN** 金额 SHALL 直接展示（不加估算标记）

#### Scenario: 金额为估算

- **WHEN** cost_status 为 Estimated 或 Mixed
- **THEN** 金额 SHALL 带估算标记（如 `~` 或 `est.`）

#### Scenario: 金额缺失

- **WHEN** cost_status 为 Missing
- **THEN** Hero SHALL 只显示 token
- **AND** SHALL NOT 显示 `$0.00`

#### Scenario: period 切换

- **WHEN** 用户切换 Today/7d/30d
- **THEN** 用量 Hero 数字 SHALL 反映所选 period
- **AND** SHALL 与同 period 的图表来自同一份数据

### Requirement: Overview 展示当前路由快照

Overview SHALL 展示每个 provider 当前 active 的账号，作为系统当前路由状态的快照。无 active 时 SHALL 显示占位。

#### Scenario: 展示路由

- **WHEN** Overview 渲染路由快照
- **THEN** 每个 provider SHALL 显示其当前 active 账号标签
- **AND** 受隐私设置影响的标签 SHALL 沿用既有脱敏规则

### Requirement: Overview 聚合跨 provider 告警

Overview SHALL 将各 provider 的诊断聚合为单一列表，并 SHALL 仅在存在告警时展示该区块。

#### Scenario: 有告警

- **WHEN** 任意 provider 存在诊断
- **THEN** Overview SHALL 在"需要处理"区块列出这些诊断

#### Scenario: 无告警

- **WHEN** 没有任何 provider 诊断
- **THEN** Overview SHALL NOT 显示空的告警区块

### Requirement: 单 provider 页 Overview 围绕切换决策组织

单 provider 页的 Overview 区块 SHALL 展示当前 active、容量分解、平均剩余、最佳备选与 reset 逃生口，SHALL NOT 展示与 Accounts/Diagnostics 卡片重复的裸计数（如 Targets/Alerts）。

#### Scenario: 渲染 provider Overview

- **WHEN** 用户进入某 provider tab
- **THEN** Overview SHALL 突出当前 active 账号
- **AND** SHALL 展示 ready/exhausted 容量分解与平均剩余
- **AND** SHALL 展示可立即切换的最佳备选及其 reset 时间

#### Scenario: reset credit 可用

- **WHEN** 该 provider 账号持有 reset credit
- **THEN** Overview SHALL 展示 reset credit 合计
- **AND** 该值 SHALL 与 headroom 指标分开呈现

#### Scenario: 移除冗余计数

- **WHEN** Overview 渲染
- **THEN** SHALL NOT 展示 Targets 或 Alerts 裸计数（信息已在 Accounts 卡头与 Diagnostics）

### Requirement: 聚合数据来自后端原子

Overview 与 provider Overview 的全部聚合 SHALL 消费后端 `MenubarQuotaRollup` 与 period-aware usage 数据，SHALL NOT 在客户端重新实现均值、计数、阈值、最佳备选或环比逻辑。

#### Scenario: 前端不复算聚合

- **WHEN** 任一 Overview 区块渲染聚合数字
- **THEN** 它 SHALL 读取后端提供的字段
- **AND** SHALL NOT 在 Swift 端重算这些聚合
