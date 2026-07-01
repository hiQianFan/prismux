## ADDED Requirements

### Requirement: 按 client 聚合 parsed token usage

OpenMux SHALL 支持按本地 AI coding client 汇总 parsed token usage，第一版默认 client 集合 SHALL be `codex`、`claude`、`gemini`。

#### Scenario: 展示全 client usage 总览

- **WHEN** 用户运行 `omx usage`
- **THEN** OpenMux SHALL 展示 Codex、Claude 和 Gemini 中可识别 client 的 usage summary
- **AND** summary SHALL 至少包含 client、input tokens、output tokens、normalized total tokens 和 data quality
- **AND** OpenMux SHALL NOT require account/profile 归因才能展示 client summary。

#### Scenario: 展示单 client usage

- **WHEN** 用户运行 `omx usage codex`
- **THEN** OpenMux SHALL 只展示 Codex client 的 token usage summary
- **AND** OpenMux SHALL NOT scan non-Codex clients for this command unless a future explicit opt-in option is provided。

#### Scenario: 默认不扫描全部 tokscale clients

- **WHEN** 用户运行 `omx usage`
- **THEN** OpenMux SHALL NOT scan clients outside `codex`、`claude`、`gemini` by default
- **AND** OpenMux SHALL NOT read Cursor、OpenCode、Cline、Roo、Kilo 或其他 tokscale-supported client directories by default。

### Requirement: 区分 client、model provider 和 usage source

OpenMux SHALL model usage identity with separate `client`、`model_provider` and `usage_source` concepts.

#### Scenario: Codex 使用 OpenAI model provider

- **WHEN** OpenMux ingests a Codex usage event whose model provider is OpenAI
- **THEN** the event SHALL store `client = "codex"`
- **AND** the event MAY store `model_provider = "openai"`
- **AND** the client summary SHALL group the event under Codex rather than OpenAI。

#### Scenario: Usage source is not displayed as provider

- **WHEN** OpenMux scans a local JSONL or SQLite source
- **THEN** the source SHALL be represented as source metadata
- **AND** it SHALL NOT replace the event client identity。

### Requirement: 第一版不按 account/profile 归因

OpenMux SHALL NOT 在 provider usage statistics 第一版中把 token usage 归因到具体 OpenMux account 或 profile。

#### Scenario: Usage event 无 account 归属

- **WHEN** usage backend 解析到一个 Codex usage event
- **AND** 该 event 没有可靠 account/profile identity
- **THEN** OpenMux SHALL ingest 该 event 并保留 client 维度
- **AND** OpenMux SHALL NOT 合成 account/profile usage 归属。

#### Scenario: Summary 不依赖 active account

- **WHEN** 用户切换过多个 Codex account
- **AND** 用户运行 `omx usage codex`
- **THEN** OpenMux SHALL 展示 Codex client 总消耗
- **AND** OpenMux SHALL NOT 把结果拆分为 Codex account `#1`、`#2` 或 profile usage。

### Requirement: Usage backend 必须通过 OpenMux 内部模型输出

OpenMux SHALL 使用内部 `UsageEvent` 和 `UsageSummary` 模型承载第三方 usage backend 的输出，CLI 和 menubar SHALL NOT 直接依赖第三方 backend 的原始 schema。

#### Scenario: tokscale-core event 转换

- **WHEN** `tokscale-core` adapter 解析到 client、model provider、model、session、timestamp 和 token counts
- **THEN** adapter SHALL 转换为 OpenMux `UsageEvent`
- **AND** OpenMux SHALL persist OpenMux `UsageEvent`
- **AND** CLI SHALL render OpenMux `UsageSummary` instead of tokscale-native report。

#### Scenario: 更换 backend 不改变 CLI JSON schema

- **WHEN** OpenMux 后续从 `tokscale-core` backend 切换到另一个 backend
- **THEN** `omx usage --json` SHALL continue to emit OpenMux-owned fields
- **AND** it SHALL NOT expose backend-specific field names as required public API。

### Requirement: 标记 parsed usage 数据质量

OpenMux SHALL mark locally parsed usage events and summaries with data quality `parsed`.

#### Scenario: 本地日志解析标记为 parsed

- **WHEN** usage event 来自本地 session/log/db/cache 解析
- **THEN** OpenMux SHALL mark the event quality as `parsed`
- **AND** client summary SHALL indicate that the source is parsed rather than exact billing data。

### Requirement: 定义 token total 和 cache 口径

OpenMux SHALL store normalized token breakdowns with a stable total formula and preserve provider-reported total separately when available.

#### Scenario: 计算 normalized total

- **WHEN** OpenMux ingests an event with input、output、cache read、cache write、reasoning 和 extra tokens
- **THEN** `normalized_total_tokens` SHALL equal input + output + cache read + cache write + reasoning + extra
- **AND** this formula SHALL NOT be replaced by provider-reported total。

#### Scenario: 保留 provider total

- **WHEN** a source event includes a provider-reported total token value
- **THEN** OpenMux SHALL store it as `provider_total_tokens`
- **AND** OpenMux SHALL still compute `normalized_total_tokens` from normalized token fields。

#### Scenario: Cache write duration breakdown

- **WHEN** a source event distinguishes 5m and 1h cache creation tokens
- **THEN** OpenMux SHALL preserve those values when available
- **AND** OpenMux SHALL include them in cache write total without double-counting。

### Requirement: Cost 是可选二级指标

OpenMux SHALL treat cost as optional estimated metadata and SHALL NOT display missing pricing as zero cost.

#### Scenario: Pricing missing

- **WHEN** OpenMux cannot calculate or read cost for a usage event with nonzero tokens
- **THEN** the event SHALL use `cost_status = "missing"`
- **AND** CLI SHALL display cost as `-` rather than `$0.00`。

#### Scenario: Provider reported cost

- **WHEN** a usage event contains provider-reported cost
- **THEN** OpenMux SHALL store that cost
- **AND** the event SHALL use `cost_status = "provider_reported"`。

#### Scenario: Estimated cost

- **WHEN** OpenMux calculates cost from pricing tables
- **THEN** the event SHALL use `cost_status = "estimated"`
- **AND** CLI and JSON SHALL identify the cost as estimated。

### Requirement: Usage event 持久化到 SQLite

OpenMux SHALL 将去重后的 usage event 持久化到 SQLite `usage_events`，并通过 `scan_watermarks` 记录增量扫描水位和 source fingerprint。

#### Scenario: 写入 parsed usage event

- **WHEN** usage backend 返回一个 Claude usage event
- **THEN** OpenMux SHALL write the event into SQLite `usage_events`
- **AND** the row SHALL include client、timestamp、token counts、backend、backend version、parser schema version、quality 和 event hash
- **AND** the row SHALL NOT include raw prompt、raw response、raw auth payload、access token、refresh token、API key 或完整原始日志行。

#### Scenario: 更新扫描水位

- **WHEN** OpenMux 成功扫描一个 usage source
- **THEN** OpenMux SHALL update `scan_watermarks` for that source
- **AND** the watermark SHALL include source fingerprint, parser schema version and scan status。

### Requirement: Source fingerprint 必须能识别重写和 sidecar 变化

OpenMux SHALL fingerprint usage sources with enough metadata to detect rewrites, rotations and related sidecar changes.

#### Scenario: JSONL 文件重写

- **WHEN** a JSONL source keeps the same path but changes earlier content
- **THEN** OpenMux SHALL detect a changed source fingerprint
- **AND** OpenMux SHALL NOT rely only on the previous byte offset。

#### Scenario: Sidecar 文件变化

- **WHEN** a usage source has related sidecar metadata such as `.meta.json` or SQLite `-wal`
- **THEN** changes to the sidecar SHALL invalidate or update the source fingerprint
- **AND** OpenMux SHALL rescan the source as needed。

#### Scenario: Parser schema version changes

- **WHEN** the adapter parser schema version changes
- **THEN** OpenMux SHALL treat existing watermarks for that parser as stale
- **AND** it SHALL rescan affected sources without double-counting existing events。

### Requirement: Usage ingest 必须幂等

OpenMux SHALL avoid double-counting usage events when the same local source is scanned multiple times.

#### Scenario: 重复扫描同一 source

- **GIVEN** SQLite 已经存在 event hash `abc`
- **WHEN** usage backend 再次返回相同 event hash `abc`
- **THEN** OpenMux SHALL NOT insert a duplicate usage event
- **AND** client summary totals SHALL remain unchanged。

#### Scenario: Hash 冲突但 payload 不一致

- **GIVEN** SQLite 已经存在 event hash `abc`
- **WHEN** usage backend returns event hash `abc` with different token payload
- **THEN** OpenMux SHALL reject the new conflicting event
- **AND** OpenMux SHALL record a safe diagnostic without overwriting historical data。

#### Scenario: 扫描失败不删除历史数据

- **GIVEN** SQLite 已经存在 Codex usage events
- **WHEN** Codex local log scan fails
- **THEN** OpenMux SHALL keep existing usage events
- **AND** `omx usage codex` SHALL report scan diagnostics without clearing historical summary。

### Requirement: Usage CLI 支持有预算扫描、时间窗口和 JSON 输出

OpenMux SHALL expose client usage statistics through CLI commands with scan controls, local-time windows and versioned JSON output.

#### Scenario: 查询今日 usage

- **WHEN** 用户运行 `omx usage --since today`
- **THEN** OpenMux SHALL aggregate usage events whose occurred time is within today in the user's local timezone
- **AND** output SHALL be grouped by client by default。

#### Scenario: 查询日期范围 usage

- **WHEN** 用户运行 `omx usage --since 2026-06-01 --until 2026-06-19`
- **THEN** OpenMux SHALL aggregate usage events within the requested local-time date range
- **AND** it SHALL reject invalid date ranges with a safe diagnostic。

#### Scenario: No-scan 查询

- **WHEN** 用户运行 `omx usage --no-scan`
- **THEN** OpenMux SHALL skip backend scanning
- **AND** OpenMux SHALL read summary only from existing SQLite usage events。

#### Scenario: Scan budget exceeded

- **WHEN** backend scan exceeds configured timeout, max files or max bytes
- **THEN** OpenMux SHALL stop or skip remaining scan work
- **AND** OpenMux SHALL keep existing usage events
- **AND** OpenMux SHALL return a safe diagnostic。

#### Scenario: JSON 输出

- **WHEN** 用户运行 `omx usage --json`
- **THEN** OpenMux SHALL emit machine-readable JSON using OpenMux-owned usage summary schema
- **AND** the JSON SHALL include `schema_version`、`generated_at_unix`、`timezone`、`window`、`quality` and client summaries
- **AND** the JSON SHALL NOT expose tokscale-native or ccusage-native report fields as required public API。

### Requirement: Usage scan 失败必须安全降级

OpenMux SHALL handle missing local logs, unsupported clients, parse errors and backend failures without breaking account/profile switching commands.

#### Scenario: Client local logs 不存在

- **WHEN** 用户运行 `omx usage gemini`
- **AND** Gemini local usage source does not exist
- **THEN** OpenMux SHALL display an unavailable or empty usage summary with diagnostics
- **AND** OpenMux SHALL NOT create fake token usage。

#### Scenario: Backend parse error

- **WHEN** usage backend cannot parse a local source
- **THEN** OpenMux SHALL record a safe diagnostic containing client、source kind 和 error code
- **AND** diagnostic SHALL NOT include raw prompt、raw response、raw log line、token 或 API key。

#### Scenario: Scan error does not affect switching

- **WHEN** usage scanning fails
- **THEN** `omx list`、`omx use` and `omx current` SHALL continue to work according to their existing account/profile semantics。

### Requirement: SQLite 并发访问必须受控

OpenMux SHALL support CLI and future menubar background worker sharing the same SQLite state without corrupting usage data.

#### Scenario: Concurrent ingest uses idempotent writes

- **WHEN** two OpenMux processes ingest the same usage event
- **THEN** SQLite unique event hash constraint SHALL prevent duplicate rows
- **AND** both processes SHALL complete without corrupting existing usage events。

#### Scenario: Busy database

- **WHEN** SQLite is temporarily busy
- **THEN** OpenMux SHALL wait using a configured busy timeout
- **AND** if the timeout expires, OpenMux SHALL return a safe diagnostic without deleting usage events。

#### Scenario: Watermark and events update atomically

- **WHEN** OpenMux writes usage events and updates the related scan watermark
- **THEN** both operations SHALL occur in the same transaction
- **AND** partial updates SHALL NOT advance the watermark without writing corresponding events。

### Requirement: Menubar 读取 SQLite 聚合结果

OpenMux SHALL define usage statistics so future menubar UI reads SQLite summary data rather than scanning provider logs directly in the UI process.

#### Scenario: Menubar 读取 client summary

- **WHEN** future menubar opens its usage panel
- **THEN** it SHALL read client usage summary from OpenMux state
- **AND** it SHALL NOT directly parse Codex、Claude 或 Gemini raw session logs from Swift UI code。

#### Scenario: Background worker ingests usage

- **WHEN** future menubar background worker scans local usage sources
- **THEN** it SHALL write events through the same SQLite ingest path as CLI
- **AND** CLI and menubar SHALL observe consistent client summary results。
