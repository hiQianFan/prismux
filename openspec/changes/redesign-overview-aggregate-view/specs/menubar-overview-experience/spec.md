## ADDED Requirements

### Requirement: Overview tab 以 providers 列表为主体而非 provider 导航列表

OpenMux Menubar 的 Overview tab SHALL 以跨 provider 的 providers 列表作为主体内容，SHALL NOT 在列表之上展示全局聚合大数字块（如全局平均剩余），SHALL NOT 以"每个 provider 一行、点击跳转"的纯导航列表样式作为主内容。逐 provider 的 tab 导航 SHALL 由 tab bar 承担。

#### Scenario: 打开 Overview tab

- **WHEN** 用户选中 Overview tab
- **THEN** 页面 SHALL 依次展示 providers 列表、（存在时的）聚合告警、用量图表
- **AND** 页面 SHALL NOT 展示全局平均剩余等聚合大数字块

### Requirement: provider 行展示聚合均值、红绿灯计数与当前路由

每个 provider 行 SHALL 展示该 provider 的平均剩余额度、healthy/low/exhausted 计数、当前 active 身份，SHALL NOT 以单账号最低额度作为主指标。无任何上报账号时 SHALL 显示空占位而非 0%。

#### Scenario: 展示 provider 池子健康

- **WHEN** Overview 渲染某 provider 行
- **THEN** 主数字 SHALL 为该 provider `QuotaHealthRollup.facts.avg_remaining_percent_x100`
- **AND** SHALL 展示 healthy/low/exhausted 计数
- **AND** SHALL NOT 在客户端计算或硬编码额度阈值

#### Scenario: 无 quota 上报

- **WHEN** 某 provider 全部账号无 quota 上报
- **THEN** 该行平均剩余 SHALL 显示为占位（如 "—"）
- **AND** SHALL NOT 显示 0%

#### Scenario: 均值高但有账号耗尽

- **WHEN** 某 provider 平均剩余较高但存在 low/exhausted 账号
- **THEN** 该行 SHALL 通过红绿灯计数暴露该危险
- **AND** SHALL NOT 仅凭高均值让危险不可见

#### Scenario: 展示当前路由

- **WHEN** provider 行渲染
- **THEN** SHALL 展示该 provider 当前 active 账号身份（如 "现在用 #2"）
- **AND** active 身份 SHALL 仅表达路由状态，SHALL NOT 携带该账号的用量数字
- **AND** 受隐私设置影响的标签 SHALL 沿用既有脱敏规则
- **AND** 无 active 时 SHALL 显示占位

#### Scenario: 低额度附 reset 倒计时

- **WHEN** 某 provider 的 low/exhausted 计数大于 0
- **THEN** 该行 SHALL 展示最近 reset 倒计时（来自 `facts.soonest_reset_at_unix`）
- **WHEN** 某 provider 全部账号 healthy
- **THEN** 该行 SHALL NOT 展示 reset 倒计时

### Requirement: provider 行展示该 provider 本期 token 与带成色花费

每个 provider 行 SHALL 展示所选 period 内该 provider 的 token 总量与估算花费。花费 SHALL 标注 `CostStatus` 成色，SHALL 在 missing 时省略花费而非显示 0 金额。token/花费 period SHALL 与同页 UsageCard 同源。

#### Scenario: 花费可信

- **WHEN** 某 provider 的 cost_status 为 ProviderReported
- **THEN** 花费 SHALL 直接展示（不加估算标记）

#### Scenario: 花费为估算

- **WHEN** cost_status 为 Estimated 或 Mixed
- **THEN** 花费 SHALL 带估算标记（如 `~` 或 `est.`）

#### Scenario: 花费缺失

- **WHEN** cost_status 为 Missing
- **THEN** 该行 SHALL 只显示 token
- **AND** SHALL NOT 显示 `$0.00`

#### Scenario: period 切换

- **WHEN** 用户切换 Today/7d/30d
- **THEN** provider 行的 token/花费 SHALL 反映所选 period
- **AND** SHALL 与同 period 的图表来自同一份数据

### Requirement: provider 本期用量 headline 由 control-plane 挂载在 provider aggregate

control-plane SHALL 在 `ProviderAggregateView` 上提供该 provider 所选 period 的 usage headline（token 总量、估算花费、cost status）。Presentation surface SHALL 直接读取该字段渲染 provider 行的 token/花费，SHALL NOT 在前端按 provider 名重新 join 或折叠 usage。

#### Scenario: provider 行读取已挂载的 headline

- **WHEN** Overview 渲染某 provider 行的 token/花费
- **THEN** 它 SHALL 读取该 `ProviderAggregateView` 上的 usage headline 字段
- **AND** SHALL NOT 在 Swift 端按 provider 重新聚合 usage

### Requirement: Overview 聚合跨 provider 告警

Overview SHALL 将各 provider 的诊断与 dashboard 级诊断聚合为单一列表，并 SHALL 仅在存在告警时展示该区块。

#### Scenario: 有告警

- **WHEN** 任意 provider 或 dashboard 级存在诊断
- **THEN** Overview SHALL 在"需要处理"区块列出这些诊断

#### Scenario: 无告警

- **WHEN** 没有任何诊断
- **THEN** Overview SHALL NOT 显示空的告警区块

### Requirement: 单 provider 页 Overview 围绕切换决策组织

单 provider 页的 Overview 区块 SHALL 展示当前 active、容量分解、平均剩余、最佳备选与 reset 逃生口，SHALL NOT 展示与 Accounts/Diagnostics 卡片重复的裸计数（如 Targets/Alerts）。

#### Scenario: 渲染 provider Overview

- **WHEN** 用户进入某 provider tab
- **THEN** Overview SHALL 突出当前 active 账号
- **AND** SHALL 展示 healthy/low/exhausted 容量分解与平均剩余
- **AND** SHALL 展示可立即切换的最佳备选及其 reset 时间

#### Scenario: reset credit 可用

- **WHEN** 该 provider 账号持有 reset credit
- **THEN** Overview SHALL 展示 reset credit 合计
- **AND** 该值 SHALL 与 headroom 指标分开呈现

#### Scenario: 移除冗余计数

- **WHEN** Overview 渲染
- **THEN** SHALL NOT 展示 Targets 或 Alerts 裸计数（信息已在 Accounts 卡头与 Diagnostics）

### Requirement: 聚合数据来自后端原子

Overview 与 provider Overview 的全部聚合 SHALL 消费后端 `ProviderAggregateView`、`QuotaHealthRollup` 与 period-aware `UsageHeadline` 数据，SHALL NOT 在客户端重新实现均值、计数、阈值、最佳备选或全局聚合逻辑。

#### Scenario: 前端不复算聚合

- **WHEN** 任一 Overview 区块渲染聚合数字
- **THEN** 它 SHALL 读取后端提供的字段
- **AND** SHALL NOT 在 Swift 端重算这些聚合
- **AND** SHALL NOT 在前端构造全局平均等跨 provider 聚合数字
