## ADDED Requirements

### Requirement: 按 provider 聚合 token usage

OpenMux SHALL 支持按 provider 汇总本机 AI coding tools 的 token usage，并展示 input、output、cache read、cache write、reasoning、total tokens 和 estimated cost。

#### Scenario: 展示全 provider usage 总览

- **WHEN** 用户运行 `omx usage`
- **THEN** OpenMux SHALL 展示每个可识别 provider 的 token usage summary
- **AND** summary SHALL 至少包含 provider、input tokens、output tokens、total tokens 和 data quality
- **AND** OpenMux SHALL NOT require account/profile 归因才能展示 provider summary。

#### Scenario: 展示单 provider usage

- **WHEN** 用户运行 `omx usage codex`
- **THEN** OpenMux SHALL 只展示 Codex provider 的 token usage summary
- **AND** OpenMux SHALL allow model、session 或 project/workspace 维度的后续明细扩展。

### Requirement: 第一版不按 account/profile 归因

OpenMux SHALL NOT 在 provider usage statistics 第一版中把 token usage 归因到具体 OpenMux account 或 profile。

#### Scenario: Usage event 无 account 归属

- **WHEN** usage backend 解析到一个 Codex usage event
- **AND** 该 event 没有可靠 account/profile identity
- **THEN** OpenMux SHALL ingest 该 event 并保留 provider 维度
- **AND** OpenMux SHALL NOT 合成 account/profile usage 归属。

#### Scenario: Provider summary 不依赖 active account

- **WHEN** 用户切换过多个 Codex account
- **AND** 用户运行 `omx usage codex`
- **THEN** OpenMux SHALL 展示 Codex provider 总消耗
- **AND** OpenMux SHALL NOT 把结果拆分为 Codex account `#1`、`#2` 或 profile usage。

### Requirement: Usage backend 必须通过 OpenMux 内部模型输出

OpenMux SHALL 使用内部 `UsageEvent` 和 `UsageSummary` 模型承载第三方 usage backend 的输出，CLI 和 menubar SHALL NOT 直接依赖第三方 backend 的原始 schema。

#### Scenario: tokscale-core event 转换

- **WHEN** `tokscale-core` adapter 解析到 provider、model、session、timestamp 和 token counts
- **THEN** adapter SHALL 转换为 OpenMux `UsageEvent`
- **AND** OpenMux SHALL persist OpenMux `UsageEvent`
- **AND** CLI SHALL render OpenMux `UsageSummary` instead of tokscale-native report。

#### Scenario: 更换 backend 不改变 CLI JSON schema

- **WHEN** OpenMux 后续从 `tokscale-core` backend 切换到另一个 backend
- **THEN** `omx usage --json` SHALL continue to emit OpenMux-owned fields
- **AND** it SHALL NOT expose backend-specific field names as required public API。

### Requirement: 标记 usage 数据质量

OpenMux SHALL 为 usage summary 和 usage event 标记数据质量，至少支持 `exact`、`parsed` 和 `inferred`。

#### Scenario: 本地日志解析标记为 parsed

- **WHEN** usage event 来自本地 session/log/db/cache 解析
- **THEN** OpenMux SHALL mark the event quality as `parsed`
- **AND** provider summary SHALL indicate that the source is parsed rather than exact billing data。

#### Scenario: Proxy 数据质量预留为 exact

- **WHEN** 未来 usage event 来自 OpenMux-controlled proxy 或 provider response usage 字段
- **THEN** OpenMux SHALL be able to mark the event quality as `exact`
- **AND** this capability SHALL NOT require changing stored parsed events。

### Requirement: Usage event 持久化到 SQLite

OpenMux SHALL 将去重后的 provider usage event 持久化到 SQLite `usage_events`，并通过 `scan_watermarks` 记录增量扫描水位。

#### Scenario: 写入 parsed usage event

- **WHEN** usage backend 返回一个 Claude usage event
- **THEN** OpenMux SHALL write the event into SQLite `usage_events`
- **AND** the row SHALL include provider、timestamp、token counts、backend、quality 和 event hash
- **AND** the row SHALL NOT include raw prompt、raw response、raw auth payload、access token、refresh token 或 API key。

#### Scenario: 更新扫描水位

- **WHEN** OpenMux 成功扫描一个 provider source
- **THEN** OpenMux SHALL update `scan_watermarks` for that source
- **AND** subsequent scans SHALL be able to skip already ingested offsets or fingerprints。

### Requirement: Usage ingest 必须幂等

OpenMux SHALL avoid double-counting usage events when the same local source is scanned multiple times.

#### Scenario: 重复扫描同一 source

- **GIVEN** SQLite 已经存在 event hash `abc`
- **WHEN** usage backend 再次返回相同 event hash `abc`
- **THEN** OpenMux SHALL NOT insert a duplicate usage event
- **AND** provider summary totals SHALL remain unchanged。

#### Scenario: 扫描失败不删除历史数据

- **GIVEN** SQLite 已经存在 Codex usage events
- **WHEN** Codex local log scan fails
- **THEN** OpenMux SHALL keep existing usage events
- **AND** `omx usage codex` SHALL report scan diagnostics without clearing historical summary。

### Requirement: Usage CLI 支持时间窗口和 JSON 输出

OpenMux SHALL expose provider usage statistics through CLI commands with time-window filters and JSON output.

#### Scenario: 查询今日 usage

- **WHEN** 用户运行 `omx usage --since today`
- **THEN** OpenMux SHALL aggregate usage events whose occurred time is within today
- **AND** output SHALL be grouped by provider by default。

#### Scenario: 查询日期范围 usage

- **WHEN** 用户运行 `omx usage --since 2026-06-01 --until 2026-06-19`
- **THEN** OpenMux SHALL aggregate usage events within the requested range
- **AND** it SHALL reject invalid date ranges with a safe diagnostic。

#### Scenario: JSON 输出

- **WHEN** 用户运行 `omx usage --json`
- **THEN** OpenMux SHALL emit machine-readable JSON using OpenMux-owned usage summary schema
- **AND** the JSON SHALL include data quality and backend source metadata。

### Requirement: Usage scan 失败必须安全降级

OpenMux SHALL handle missing local logs, unsupported providers, parse errors and backend failures without breaking account/profile switching commands.

#### Scenario: Provider local logs 不存在

- **WHEN** 用户运行 `omx usage gemini`
- **AND** Gemini local usage source does not exist
- **THEN** OpenMux SHALL display an unavailable or empty usage summary with diagnostics
- **AND** OpenMux SHALL NOT create fake token usage。

#### Scenario: Backend parse error

- **WHEN** usage backend cannot parse a local source
- **THEN** OpenMux SHALL record a safe diagnostic containing provider、backend 和 error code
- **AND** diagnostic SHALL NOT include raw prompt、raw response、token 或 API key。

### Requirement: Menubar 读取 SQLite 聚合结果

OpenMux SHALL define provider usage statistics so future menubar UI reads SQLite summary data rather than scanning provider logs directly in the UI process.

#### Scenario: Menubar 读取 provider summary

- **WHEN** future menubar opens its usage panel
- **THEN** it SHALL read provider usage summary from OpenMux state
- **AND** it SHALL NOT directly parse Codex、Claude 或 Gemini raw session logs from Swift UI code。

#### Scenario: Background worker ingests usage

- **WHEN** future menubar background worker scans local usage sources
- **THEN** it SHALL write events through the same SQLite ingest path as CLI
- **AND** CLI and menubar SHALL observe consistent provider summary results。
