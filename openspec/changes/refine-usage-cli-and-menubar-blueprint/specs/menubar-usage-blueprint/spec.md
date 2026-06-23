## ADDED Requirements

### Requirement: Menubar 必须消费 OpenMux-owned usage contract
未来 OpenMux Menubar SHALL 通过 versioned JSON、Rust FFI 或等价稳定本地接口消费 OpenMux `UsageQuery`/`UsageReport`，不得直接依赖第三方 parser schema 或 OpenMux SQLite 表结构。

#### Scenario: Menubar 加载 usage overview
- **WHEN** Menubar 请求 today usage overview
- **THEN** OpenMux backend SHALL 返回与 `omx usage` 相同 query 口径的 totals、groups、freshness 和 coverage
- **AND** Swift/UI layer SHALL NOT 直接执行 `usage_events` SQL

### Requirement: Menubar 与 CLI 必须共享唯一采集和聚合实现
Menubar SHALL NOT 重新实现 provider log scanning、event deduplication、pricing mapping 或 usage aggregation。后台 ingest 与 CLI SHALL 共享 OpenMux adapter、SQLite 和 query service。

#### Scenario: CLI 与 Menubar 查询相同窗口
- **WHEN** CLI 与 Menubar 对同一 state 查询相同 window 和 group-by
- **THEN** 两者 SHALL 得到一致 totals 和 groups
- **AND** Menubar SHALL NOT 通过独立 TokenBar/tokscale scan 产生第二份数据

### Requirement: TokenBar 二次开发必须先通过限定 spike
OpenMux SHALL 在采用 TokenBar fork 前完成限定 spike，验证 license/NOTICE、一个 Overview 数据入口替换、UI 与数据层耦合、启动性能、签名/更新影响及 upstream sync 成本。Spike SHALL 记录固定 upstream commit 和本地差异。

#### Scenario: TokenBar spike 满足采用门槛
- **WHEN** Overview 能通过 OpenMux contract 加载数据且无需保留 TokenBar 自有 scanner/aggregator
- **THEN** spike SHALL 输出继续 fork 的推荐、预计维护成本和 production integration 方案

#### Scenario: TokenBar 数据层无法低成本替换
- **WHEN** 接入 OpenMux contract 需要重写大部分 view model、FFI 或状态管理
- **THEN** spike SHALL 建议停止 fork
- **AND** 后续 SHALL 仅复用经许可的 UX/视觉思路或创建更小原生 shell

### Requirement: TokenBar 复用范围必须排除重复数据引擎
TokenBar fork 可以复用 macOS menu bar shell、SwiftUI views、图表与交互模式，但 MUST 删除或禁用其独立 session scanning、pricing/model mapping、usage cache/aggregation 和与 OpenMux 冲突的 account/quota 定义。

#### Scenario: Menubar 构建进入 production candidate
- **WHEN** TokenBar fork 被选为 production candidate
- **THEN** 默认构建 SHALL 只使用 OpenMux backend 提供 usage 数据
- **AND** TokenBar 自带 parser/aggregator SHALL NOT 参与 usage totals 计算

### Requirement: Menubar 复杂 lens 不得扩大 CLI 必选范围
Menubar 可以提供 models、daily、hourly、stats、agents、trend 或 graph lens，但这些可视化 SHALL NOT 自动成为 `omx usage` 的必选 TUI 功能。CLI 只需提供支撑这些 lens 的稳定结构化 query。

#### Scenario: Menubar 增加 hourly heatmap
- **WHEN** Menubar 增加 hourly heatmap
- **THEN** OpenMux SHALL 可以扩展 query contract 提供 hourly buckets
- **AND** CLI SHALL NOT 被要求实现交互式 heatmap

### Requirement: Menubar 必须保留数据质量和降级语义
Menubar SHALL 展示 last known usage、freshness、partial/missing coverage 和 cost status；refresh 或 scan 失败 MUST NOT 清空最近成功数据。

#### Scenario: 后台刷新失败
- **WHEN** Menubar 后台 scan/refresh 失败且存在最近成功 report
- **THEN** Menubar SHALL 继续展示最近成功数据
- **AND** Menubar SHALL 标明 stale 状态和安全诊断

### Requirement: Menubar 完整实现必须使用独立后续变更
本 capability SHALL 产出架构 spike、接口建议、采用/停止决策和分阶段蓝图。完整 Menubar 产品实现 MUST 在 spike 决策后通过独立 OpenSpec change 定义范围与验收。

#### Scenario: Spike 完成并建议继续 fork
- **WHEN** TokenBar spike 输出继续采用结论
- **THEN** 团队 SHALL 在实现完整 Menubar 前创建独立 change
- **AND** 该 change SHALL 固定 UI 范围、发布平台、签名、更新和测试要求
