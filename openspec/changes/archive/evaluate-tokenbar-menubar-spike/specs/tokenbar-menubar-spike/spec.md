## ADDED Requirements

### Requirement: TokenBar 评估必须先以隔离 spike 完成
OpenMux SHALL 在采用 TokenBar fork 前完成限定 spike。Spike SHALL 固定 upstream commit，记录 license/NOTICE、第三方资源、发布边界和本地差异，并且 SHALL NOT 进入 OpenMux 默认 workspace/build。

#### Scenario: Spike fork 创建
- **WHEN** 团队创建 TokenBar spike fork
- **THEN** fork SHALL 位于隔离目录、独立分支或设计产物路径
- **AND** OpenMux 默认构建 SHALL NOT 引用该 fork

### Requirement: Spike 必须验证 OpenMux usage contract 接入
Spike SHALL 用 `omx usage --json --no-scan` 或 mock OpenMux `UsageReport` JSON 替换一个 Overview 数据入口。Spike MUST NOT 通过 TokenBar 自有 scanner、pricing、cache 或 aggregator 计算 usage totals。

#### Scenario: Overview 加载 OpenMux 数据
- **WHEN** Overview screen 展示 usage total
- **THEN** total SHALL 来自 OpenMux JSON/mock contract
- **AND** TokenBar 自有 parser/aggregator SHALL NOT 参与 totals 计算

### Requirement: Spike 必须输出复用与删除清单
Spike SHALL 列出可复用的 macOS shell、SwiftUI view 或交互模式，并列出必须删除或禁用的 TokenBar 数据引擎代码。

#### Scenario: Spike 完成 coupling audit
- **WHEN** spike 评估 Overview 相关代码
- **THEN** 输出 SHALL 区分 UI/shell 代码与 scanner、pricing、quota、cache、aggregation 代码
- **AND** 不可清晰隔离的数据层耦合 SHALL 计入 fork 风险

### Requirement: Spike 必须给出 go/no-go 结论
Spike SHALL 基于 license、Overview 接入成本、数据层删除成本、fork delta、upstream sync 成本和发布边界输出 go/no-go 结论。

#### Scenario: TokenBar 数据层无法低成本替换
- **WHEN** 接入 OpenMux contract 需要重写大部分 view model、FFI 或状态管理
- **THEN** spike SHALL 建议 no-go
- **AND** 后续 SHALL 仅保存 UX mapping 或创建更小原生 shell 方案

#### Scenario: TokenBar fork 满足采用门槛
- **WHEN** Overview 可接入 OpenMux contract 且无需保留 TokenBar 数据引擎
- **THEN** spike SHALL 建议 go
- **AND** 后续完整 Menubar 实现 SHALL 通过独立 OpenSpec change 定义范围与验收
