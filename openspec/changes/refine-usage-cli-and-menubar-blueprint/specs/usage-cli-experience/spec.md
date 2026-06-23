## ADDED Requirements

### Requirement: 默认 usage 输出必须是紧凑决策摘要
OpenMux SHALL 使无额外展示参数的 `omx usage` 在单个常规终端屏幕内展示查询窗口、total tokens、按 client 的紧凑汇总、主要消耗来源和数据新鲜度。默认输出 SHALL NOT 展示完整 input/output/cache/reasoning/provider total/event count 宽表。

#### Scenario: 用户查看今日 usage
- **WHEN** 用户运行 `omx usage`
- **THEN** OpenMux SHALL 使用本地时区的 today window 输出 total 与按 client 的紧凑摘要
- **AND** OpenMux SHALL 在存在 scan/coverage 异常时显示简洁警告

### Requirement: Usage CLI 必须支持常用时间预设和精确范围
OpenMux SHALL 支持 `today`、`7d`、`30d`、`all` 时间预设，并继续支持 `--since`、`--until` 精确日期或时间边界。`7d` 与 `30d` SHALL 使用用户本地时区的自然日边界，并包含当前日。

#### Scenario: 用户查询最近七个本地自然日
- **WHEN** 用户运行 `omx usage --period 7d`
- **THEN** OpenMux SHALL 查询包含今天在内的最近七个本地自然日
- **AND** 输出 SHALL 标明实际 since/until 边界和 timezone

#### Scenario: 用户同时提供冲突窗口参数
- **WHEN** 用户同时提供 `--period` 与 `--since` 或 `--until`
- **THEN** OpenMux SHALL 返回清晰的参数冲突错误
- **AND** OpenMux SHALL NOT 猜测用户期望的窗口

### Requirement: Usage CLI 必须按显式 lens 分组
OpenMux SHALL 支持按 `client`、`day`、`model` 分组 usage；只有底层数据与查询行为稳定时才 SHALL 开放 `project` 或 `session` 分组。缺失维度值的 token SHALL 归入 `unknown`，不得被丢弃。

#### Scenario: 用户查看模型用量
- **WHEN** 用户运行 `omx usage --group-by model --period 7d`
- **THEN** OpenMux SHALL 按 model 聚合 token 和可用 cost
- **AND** 未知 model SHALL 作为 `unknown` row 计入 total

#### Scenario: 用户查看每日用量
- **WHEN** 用户运行 `omx usage --group-by day --period 30d`
- **THEN** OpenMux SHALL 按用户本地日期输出 daily rows
- **AND** daily rows 之和 SHALL 与同一 report 的 total 一致

### Requirement: 详细 token accounting 必须渐进披露
OpenMux SHALL 通过 `--details` 展示 input、output、cache read、cache write、reasoning、provider total、event count、cost status 和 data quality。未指定 `--details` 时默认摘要 SHALL 隐藏这些 accounting 列，但不得从 JSON contract 删除数据。

#### Scenario: 用户请求完整 token breakdown
- **WHEN** 用户运行 `omx usage --details`
- **THEN** OpenMux SHALL 展示每个当前分组 row 的完整 token breakdown
- **AND** missing cost SHALL 显示为 missing 而不是 `$0.00`

### Requirement: Consumption、cost 与 quota 必须保持独立口径
OpenMux SHALL 明确区分本地 parsed token consumption、estimated/provider-reported cost 和 subscription quota。OpenMux MUST NOT 根据 token consumption 推断 subscription quota remaining。

#### Scenario: 同时存在 token usage 和 quota snapshot
- **WHEN** OpenMux 同时拥有本地 parsed usage 与 provider quota snapshot
- **THEN** CLI SHALL 使用不同标签和数据来源展示两者
- **AND** CLI SHALL NOT 将 token total 转换为 quota remaining percentage

### Requirement: Usage report 必须暴露 freshness 与 coverage
OpenMux SHALL 区分无 usage、source 缺失、scan 失败、partial coverage 和完整扫描。默认 human output SHALL 在正常时压缩这些信息，在异常时显示安全警告；JSON SHALL 始终暴露结构化 freshness、coverage 和 diagnostics。

#### Scenario: 扫描失败但存在历史数据
- **WHEN** 当前 scan 失败且 SQLite 中存在查询窗口内的历史 usage
- **THEN** OpenMux SHALL 保留并展示历史 summary
- **AND** OpenMux SHALL 标明最后成功更新时间和当前 scan failure

#### Scenario: 查询窗口内没有事件且扫描成功
- **WHEN** scan 成功且查询窗口内没有 usage event
- **THEN** OpenMux SHALL 返回 empty usage 而不是 unavailable
- **AND** JSON SHALL 将 coverage 与 empty result 分开表达

### Requirement: Human output 与 JSON 必须共享同一 report
OpenMux SHALL 使用同一个 OpenMux-owned `UsageReport` 生成 human output 与 versioned JSON。相同 query 下 table totals、group totals、window 和 accounting status SHALL 与 JSON 一致。

#### Scenario: 对比 table 与 JSON
- **WHEN** 用户对相同 state 和 query 分别运行 human output 与 `--json`
- **THEN** 两种输出的 window、totals 和 groups SHALL 数值一致

### Requirement: Account/profile attribution 必须证据驱动
OpenMux MUST NOT 将扫描时的 current account/profile 归因到历史 usage event。只有存在可验证 attribution evidence 时，OpenMux 才 SHALL 将 event 计入具名 account/profile；否则 SHALL 保持 `unknown`。

#### Scenario: 历史 event 缺少账号证据
- **WHEN** parser 读取到 token event 但无法证明其使用的 OpenMux account/profile
- **THEN** OpenMux SHALL 将 attribution 标记为 `unknown`
- **AND** OpenMux SHALL NOT 使用当前 active account/profile 填充归属

### Requirement: Usage CLI 不得演化为强制交互式 TUI
`omx usage` SHALL 保持可脚本化的一次性命令，不得要求进入全屏 TUI 才能访问核心 summary、group-by、details 或 JSON 能力。

#### Scenario: 非交互环境查询 usage
- **WHEN** 用户在 CI、管道或无 TTY 环境运行 `omx usage --json --no-scan`
- **THEN** OpenMux SHALL 输出完整机器可读 report
- **AND** OpenMux SHALL NOT 初始化全屏终端 UI
