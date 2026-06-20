## Context

OpenMux 当前已经具备账号/profile 切换、稳定 `local_id`、SQLite 本地状态库、quota snapshot 和 refresh attempt 的基础。usage 统计要补的是“本机实际发生过多少 token 消耗”，不是 provider quota，也不是账号池额度。

调研结论：

- LiteLLM、Helicone、OpenLIT、Langfuse 和 OpenTelemetry GenAI instrumentation 适合统一出口或 SDK hook 场景，能把 provider response 中的 usage 字段作为 `exact` usage。OpenMux 第一版不控制 Codex/Claude/Gemini 官方 CLI 的请求出口，所以不能采用这条路线作为默认。
- ccusage、tokscale、TokenBar、toki、tokenusage 采用本地 artifact 解析。成熟实现都不是一个简单 parser，而是包含 client registry、source discovery、token 口径转换、pricing、去重、缓存、水位和输出 schema。
- tokscale-core 已经有 `clients`、`scanner`、`message_cache`、`provider_identity`、`pricing`、`aggregator` 等模块；TokenBar 采用 vendored `tokscale-core` + 自己的 FFI/API envelope。这个路线符合 OpenMux 的 Rust/native menubar 目标。
- ccusage 已迁到 Rust core，并按 adapter 拆分 Claude/Codex/Gemini 等 agent；它的 cost mode、missing pricing、JSON output 和 fixtures 适合作为验证参考。

因此本变更采用 `parsed usage backend`：读取本地 session/log/db/cache，转换为 OpenMux-owned `UsageEvent`，持久化到 SQLite，再按 client/time window 聚合展示。

## Goals / Non-Goals

**Goals:**

- 第一版默认统计 `codex`、`claude`、`gemini` 三个 client 的本地 token usage。
- 明确区分 `client`、`model_provider` 和 `usage_source`，避免把 Codex/Claude/Gemini 与 OpenAI/Anthropic/Google 混用。
- token-first：input、output、cache read、cache write、reasoning、normalized total 是主指标。
- cost 是可选二级指标，必须带 `CostStatus`，缺 pricing 时显示 missing 而不是 `$0.00`。
- 通过 OpenMux 自己的 usage domain 隔离 third-party backend。
- 通过 source fingerprint、parser schema version、event hash 和 SQLite unique constraint 保证重复扫描不重复计数。
- 支持本地时区的 date window，SQLite 内部保存 UTC unix seconds。
- 为未来 menubar 提供只读 summary 查询和后台 worker ingest 路径。

**Non-Goals:**

- 不实现统一 proxy、API gateway、MITM、网络拦截或官方 CLI 请求改写。
- 不按 OpenMux account/profile 归因，不做 active timeline inference。
- 不默认扫描 tokscale 支持的全部 clients；除 Codex/Claude/Gemini 外的 client 必须未来显式 opt-in。
- 不调用私有 provider usage endpoint 来补齐 token 消耗。
- 不把 LiteLLM、Helicone、Langfuse 这类服务端平台嵌入 OpenMux。
- 不在第一版实现 retention/prune、云同步、团队报表、leaderboard 或账单级 reconciliation。

## Decisions

### 1. 命名模型采用 client-first

OpenMux usage 领域模型使用以下命名：

```text
client: 本地 coding tool，例如 codex / claude / gemini
model_provider: 模型服务商或路由商，例如 openai / anthropic / google / openrouter
model: 模型 ID，例如 gpt-5 / claude-sonnet-4 / gemini-2.5-pro
usage_source: 本地 JSONL / session DB / cache 文件
```

第一版 `omx usage` 默认按 client 汇总；如果事件中有 model_provider，则作为明细和 JSON 字段保留。

### 2. tokscale-core 使用 vendor + adapter

`tokscale-core` 不作为 OpenMux public API，也不直接暴露到 CLI/menubar。接入方式：

```text
vendor/tokscale-core
  -> crates/omx-usage-tokscale
    -> omx-core UsageEvent
      -> StateStore usage_events
        -> CLI / menubar summary
```

选择 vendor 的理由：

- TokenBar 已验证该路径可行。
- tokscale-core 没有稳定 SDK 承诺，vendor 能固定行为。
- 后续升级可以通过 adapter 和 fixtures 控制风险。

2026-06-19 Phase 0 复核结论：

- upstream: `https://github.com/junhoyeo/tokscale`
- 固定候选 commit: `cbbd0dffda93a3a4588fc08fd631ca10bba73ff1`
- license: MIT
- workspace crate: `crates/tokscale-core`
- 公开入口包含 `parse_local_clients(LocalParseOptions)`、`parse_local_unified_messages(...)`、`ClientId`、`UnifiedMessage`、`TokenBreakdown`。
- README 标注支持 Codex、Claude Code、Gemini CLI，并声明数据来自 `~/.codex/sessions/`、`~/.claude/projects/` / `~/.claude/transcripts/`、`$GEMINI_CLI_HOME/tmp/*/chats/*.json` 等本地 artifact。
- `tokscale-core` 能按 `clients` 参数限制扫描范围，但它的 `ParsedMessage` / `UnifiedMessage` 不稳定暴露 source path、offset、line hash；OpenMux 仍必须在 adapter 层生成自己的 `UsageEventSource`、`UsageSourceFingerprint` 和 `event_hash`。
- `tokscale-core` 自带 pricing、message cache、scanner settings、SQLite parser 等较多依赖；第一版不应把其类型泄漏到 `omx-core`，也不应默认启用 tokscale 支持的全部 clients。
- vendored `crates/tokscale-core` 源码约 1.8MB；首次构建 `omx-usage-tokscale` 在当前环境约 46s，主要新增依赖包括 `rayon`、`simd-json`、`walkdir`、`reqwest`、`tokio`、`dirs`、`rusqlite`、`zstd`、`sha2`、`fs2`。网络/pricing 依赖来自 tokscale pricing 模块，当前 adapter 第一版使用同步 `parse_local_clients`，cost 先映射为 `CostStatus::Missing`。
- OpenMux adapter 最小 fixture 已覆盖 Codex、Claude、Gemini 三个默认 client 的 token 字段映射：`input`、`output`、`cache_read`、`cache_write`、`reasoning`、`client`、`model_provider`、`model`、`session_id`、`timestamp`。

### 3. Core usage model 不存 third-party schema

建议模型：

```rust
pub struct UsageEvent {
    pub client: String,
    pub model_provider: Option<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub project_path: Option<PathBuf>,
    pub occurred_at_unix: i64,
    pub tokens: UsageTokenBreakdown,
    pub normalized_total_tokens: u64,
    pub provider_total_tokens: Option<u64>,
    pub estimated_cost_usd: Option<f64>,
    pub cost_status: CostStatus,
    pub source: UsageEventSource,
    pub quality: UsageDataQuality,
}

pub struct UsageTokenBreakdown {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_5m: Option<u64>,
    pub cache_write_1h: Option<u64>,
    pub reasoning: u64,
    pub extra: u64,
}

pub enum UsageDataQuality {
    Parsed,
}

pub enum CostStatus {
    ProviderReported,
    Estimated,
    Missing,
    Mixed,
}
```

`normalized_total_tokens` 固定为：

```text
input + output + cache_read + cache_write + reasoning + extra
```

`provider_total_tokens` 保留 provider 或 parser 给出的原始 total；它可能与 normalized total 不一致，不能覆盖 normalized total。

### 4. SQLite schema 采用 robust source fingerprint

`usage_events` 不保存 raw 内容，只保存可聚合 metadata：

```sql
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  client TEXT NOT NULL,
  model_provider TEXT,
  model TEXT,
  session_id TEXT,
  request_id TEXT,
  project_path TEXT,
  occurred_at_unix INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_tokens INTEGER NOT NULL DEFAULT 0,
  cache_write_tokens INTEGER NOT NULL DEFAULT 0,
  cache_write_5m_tokens INTEGER,
  cache_write_1h_tokens INTEGER,
  reasoning_tokens INTEGER NOT NULL DEFAULT 0,
  extra_tokens INTEGER NOT NULL DEFAULT 0,
  normalized_total_tokens INTEGER NOT NULL DEFAULT 0,
  provider_total_tokens INTEGER,
  estimated_cost_usd REAL,
  cost_status TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_path TEXT,
  source_fingerprint_json TEXT,
  source_offset INTEGER,
  source_record_id TEXT,
  source_record_hash TEXT,
  backend TEXT NOT NULL,
  backend_version TEXT NOT NULL,
  parser_schema_version INTEGER NOT NULL,
  quality TEXT NOT NULL,
  event_hash TEXT NOT NULL UNIQUE,
  ingested_at_unix INTEGER NOT NULL
);
```

`scan_watermarks` 保存 source 层水位：

```sql
CREATE TABLE scan_watermarks (
  source_id TEXT PRIMARY KEY,
  client TEXT NOT NULL,
  backend TEXT NOT NULL,
  backend_version TEXT NOT NULL,
  parser_schema_version INTEGER NOT NULL,
  source_kind TEXT NOT NULL,
  source_path TEXT NOT NULL,
  source_fingerprint_json TEXT NOT NULL,
  last_offset INTEGER,
  last_record_id TEXT,
  last_scanned_at_unix INTEGER NOT NULL,
  last_scan_status TEXT NOT NULL,
  diagnostic_code TEXT
);
```

`source_fingerprint_json` 至少应支持：

- canonical path
- file size
- modified timestamp
- sample hashes
- content hash 或 prefix hash
- related sidecar fingerprints，例如 `.meta.json`、SQLite `-wal`
- parser schema version

这个设计借鉴 tokscale message cache：不能只靠 offset，因为日志可能重写、轮转、sidecar 变化、SQLite WAL 更新。

### 5. Event hash 使用分层策略

`event_hash` 生成优先级：

1. 有 request/message id：`client + session_id + request_id/message_id`
2. JSONL append source：`source fingerprint + byte offset + line hash`
3. SQLite source：`db fingerprint + table primary key/source_record_id + updated_at`
4. fallback：`client + session_id + occurred_at_unix + model + token tuple + source line hash`

如果 hash 冲突且 payload 不一致，OpenMux 记录 diagnostic 并拒绝覆盖旧 event，避免静默篡改历史汇总。

### 6. CLI scan 有预算

`omx usage` 默认执行 best-effort scan，但必须受预算限制：

- 默认 client: `codex,claude,gemini`
- 默认 window: `today`，按本地时区计算
- scan timeout
- max source files
- max source bytes
- SQLite busy timeout
- `--no-scan` 只读历史聚合

扫描失败时不删除历史 event；summary 可以展示历史数据和 scan diagnostics。

### 7. JSON schema versioned

`omx usage --json` 输出 OpenMux-owned schema：

```json
{
  "schema_version": 1,
  "generated_at_unix": 1780000000,
  "timezone": "Asia/Shanghai",
  "window": {
    "since_unix": 1780000000,
    "until_unix": 1780086400,
    "label": "today"
  },
  "quality": "parsed",
  "clients": []
}
```

每个 client summary 包含 token breakdown、normalized total、optional provider total、cost/status、backend/source diagnostics。JSON 不暴露 tokscale 或 ccusage 的原始结构。

### 8. 隐私和 diagnostics

Parser 必然会读取本地日志，但 OpenMux 的安全边界是“不泄漏原始内容”：

- 不持久化 raw prompt、raw response、raw log line。
- 不打印 raw prompt、raw response、raw log line。
- diagnostics 只记录 client、source kind、source path hash 或脱敏 path、error code。
- parse error 不包含 serde 原始行上下文。
- debug mode 也默认脱敏；未来如需 raw debug 必须显式 opt-in，且不进入本变更。

### 9. 并发策略

CLI 与未来 menubar 后台 worker 会共享 SQLite：

- 打开连接时设置 `busy_timeout`。
- 优先启用 WAL mode，除非平台或测试环境不支持。
- scan 可以在内存中完成，ingest 使用短批量事务。
- `usage_events` 通过 `event_hash UNIQUE` 幂等。
- `scan_watermarks` 更新与 event ingest 在同一事务内完成。
- menubar UI 进程只读 summary，不直接写库。

## Risks / Trade-offs

- `tokscale-core` API 变动 → vendor 固定版本，adapter 封装，fixture 回归后再升级。
- 本地日志格式变化 → source fingerprint + parser schema version 触发重扫；失败时保留历史 event 并展示 diagnostics。
- 数据不是账单级准确 → CLI/JSON 明确 `quality = parsed`，future proxy 才能引入 `exact`。
- cost 不可靠 → token 为主指标，cost 可为空并带 `CostStatus`。
- 默认扫描过宽 → 第一版只启用 Codex/Claude/Gemini，其他 client 必须 future opt-in。
- 数据库增长 → 第一版不做 retention，但 schema 保留 `ingested_at_unix` 和 time indexes；后续单独设计 prune。
- 跨进程写入竞争 → busy timeout、WAL、短事务和 unique constraint 降低风险。

## Migration Plan

1. Phase 0 spike：vendor tokscale-core，使用 Codex/Claude/Gemini fixtures 跑 parser，并与 ccusage JSON 输出对照 token/cache/reasoning/model/timestamp 口径。
2. 在 `omx-core` 增加 usage event/summary/backend domain，不改变现有 quota refresh 行为。
3. 扩展 SQLite migration，新增 `usage_events`、`scan_watermarks`、indexes 和 busy timeout/WAL 配置。
4. 增加 `omx-usage-tokscale` adapter，把 tokscale parsed messages 映射为 OpenMux `UsageEvent`。
5. 增加 `omx usage` CLI：有预算 scan，再 query client summary。
6. 增加 versioned JSON 输出、fixture tests、隐私 diagnostics tests。
7. 后续 menubar 只消费 SQLite summary，不直接依赖 tokscale。

回滚策略：如果 backend 扫描不可用，保留 schema 和历史 usage events；`omx usage` 显示 unavailable diagnostics，不影响 account/profile switching。

## Open Questions

- 第一版 `omx usage` 默认 window 最终选 `today` 还是 `last 24h`；当前建议 `today`，因为更符合日常菜单栏查看心智。
- 是否在第一版提供实验开关启用 tokscale 其他 clients；当前建议不提供，等 Codex/Claude/Gemini 稳定后再加。
- 是否需要独立 `omx usage doctor` 展示每个 source 的 scan diagnostics；第一版可以先通过 JSON 暴露。
