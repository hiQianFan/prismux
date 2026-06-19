## Context

OpenMux 当前已经把账号/profile 切换、稳定 `local_id`、SQLite 本地状态库、quota snapshot 和 refresh attempt 放到了核心设计里。下一步需要补齐“实际 token 消耗”的账本，但用户已经明确第一阶段只需要 provider 维度，不需要 account/profile 维度。

调研到的市面实现可以按第一性原理分为两类：

- 统一出口统计：LiteLLM、Helicone、OpenLIT、Langfuse、OpenInference/OpenTelemetry instrumentation。它们在 proxy、SDK instrumentation 或 trace pipeline 上看到请求和响应，因此能从 provider response usage 字段获得接近账单级的 `exact` token/cost。前提是请求必须经过它们控制的出口。
- 本地 artifact 解析：ccusage、tokscale、TokenBar、toki、tokenusage。它们扫描 Claude Code、Codex、Gemini、Cursor 等工具本地留下的 JSONL/session/db/cache，把其中的 usage/token_count/model/session/project 字段解析成聚合报表。它们不控制请求出口，准确度取决于本地日志是否完整和格式是否稳定。

OpenMux 管理的是用户本机分散的官方 account/profile/config，不会在第一版强制所有 Codex/Claude/Gemini 调用经过 proxy。因此 provider usage 的第一版应使用本地 artifact 解析，并把数据质量标记为 `parsed`。

## Goals / Non-Goals

**Goals:**

- 按 provider 统计本机 AI coding tools 的 token 和 estimated cost。
- 支持 Codex、Claude、Gemini 作为第一批 provider，并允许 `tokscale-core` 支持的其他 provider 渐进启用。
- 通过 OpenMux 自己的 usage domain 隔离第三方 backend，避免 `tokscale-core` 或 `ccusage` schema 泄漏到 core/CLI/menubar。
- 使用 SQLite 保存去重后的 usage event 和扫描水位，支持 CLI 与未来 menubar 共享同一数据源。
- 明确数据质量：第一版是 `parsed`，不是 provider 官方账单，也不是 account/profile 精确归因。
- 保持安全边界：不保存 raw prompt、raw response、raw auth payload、access token、refresh token、API key 或完整原始日志。

**Non-Goals:**

- 不实现统一 proxy、API gateway、MITM、网络拦截或官方 CLI 请求改写。
- 不按 OpenMux account/profile 统计 token，不做 active timeline 归因。
- 不调用私有 provider usage endpoint 来补齐 token 消耗。
- 不把 LiteLLM、Helicone、Langfuse 这类服务端平台嵌入 OpenMux。
- 不在第一版实现云同步、团队报表、leaderboard 或账单级 reconciliation。

## Decisions

### 1. Usage backend 采用 adapter 架构

OpenMux core 定义稳定模型，第三方项目只作为 backend：

```rust
pub struct UsageEvent {
    pub provider: ProviderId,
    pub client: Option<String>,
    pub model: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub project_path: Option<PathBuf>,
    pub occurred_at_unix: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: Option<f64>,
    pub source: UsageEventSource,
    pub quality: UsageDataQuality,
}

pub enum UsageDataQuality {
    Exact,
    Parsed,
    Inferred,
}
```

`UsageBackend` 只负责 scan/ingest，返回 OpenMux `UsageEvent`：

```rust
pub trait UsageBackend {
    fn scan(&self, options: UsageScanOptions) -> Result<UsageScanReport>;
}
```

替代方案是直接在 CLI 调 `tokscale-core` 并展示它的 report。这个做法短期快，但会把第三方 grouping、pricing、client naming、错误语义带进 OpenMux 产品层；以后切换 backend 或加 proxy exact usage 会很困难。

### 2. 第一版 backend 选择 `tokscale-core`，`ccusage` 作为参考/fallback

`tokscale-core` 和 OpenMux 的 Rust 技术栈更接近，也已经被 TokenBar 以 vendored core + Swift FFI 的方式验证过。它支持多 provider 本地解析，适合后续 macOS native menubar。

`ccusage` 成熟度和用户验证更强，但主栈是 Node/TypeScript。第一版不应把它放进 OpenMux 的核心运行路径；可以保留为开发期交叉验证和可选 fallback：

- 开发/测试：同一 fixture 与 ccusage JSON 输出对比，验证 token/cost 聚合方向。
- 用户环境：如果 tokscale adapter 不支持某 provider，后续可通过可选 `ccusage --json` adapter 接入，但必须标记 backend source。

### 3. Provider 维度是正式产品口径

第一版 summary 的主键是 provider：

```text
provider + time_window
provider + model + time_window
provider + project/workspace + time_window
provider + session + time_window
```

`local_id` 在 `usage_events` 中允许为空。即使某些日志可以推断 active account，也不在第一版写入 account/profile attribution，避免用户误以为这是官方账单。

### 4. SQLite 保存 event ledger，不保存原始内容

在既有 `omx-state.sqlite` 中新增：

```sql
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider TEXT NOT NULL,
  client TEXT,
  model TEXT,
  session_id TEXT,
  request_id TEXT,
  project_path TEXT,
  occurred_at_unix INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_tokens INTEGER NOT NULL DEFAULT 0,
  cache_write_tokens INTEGER NOT NULL DEFAULT 0,
  reasoning_tokens INTEGER NOT NULL DEFAULT 0,
  total_tokens INTEGER NOT NULL DEFAULT 0,
  estimated_cost_usd REAL,
  source_kind TEXT NOT NULL,
  source_path TEXT,
  source_offset INTEGER,
  source_fingerprint TEXT,
  backend TEXT NOT NULL,
  quality TEXT NOT NULL,
  event_hash TEXT NOT NULL UNIQUE,
  ingested_at_unix INTEGER NOT NULL
);

CREATE TABLE scan_watermarks (
  source_id TEXT PRIMARY KEY,
  provider TEXT NOT NULL,
  backend TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_path TEXT NOT NULL,
  last_offset INTEGER,
  last_fingerprint TEXT,
  last_scanned_at_unix INTEGER NOT NULL
);
```

索引：

```sql
CREATE INDEX idx_usage_provider_time
  ON usage_events(provider, occurred_at_unix DESC);
CREATE INDEX idx_usage_provider_model_time
  ON usage_events(provider, model, occurred_at_unix DESC);
CREATE INDEX idx_usage_provider_project_time
  ON usage_events(provider, project_path, occurred_at_unix DESC);
CREATE INDEX idx_usage_session
  ON usage_events(provider, session_id, occurred_at_unix DESC);
```

`event_hash` 基于 provider、backend、source path、source offset/fingerprint、session/request id、timestamp 和 token tuple 生成。目标是幂等 ingest，而不是密码学身份。

### 5. CLI 使用 scan-then-query 流程

`omx usage` 默认执行一次 best-effort local scan，然后从 SQLite 聚合结果：

```text
omx usage
omx usage codex
omx usage --since today
omx usage --since 2026-06-01 --until 2026-06-19
omx usage --json
```

表格第一版展示：

```text
Usage today
Provider  Input   Output  Cache R/W  Reasoning  Total   Cost     Source
Codex     12.3k   4.1k    8.2k/0     1.0k       25.6k   $0.18    parsed
Claude    31.0k   9.8k    2.1k/1.2k  -          44.1k   $0.62    parsed
Gemini    8.4k    2.0k    -          -          10.4k   $0.03    parsed
```

`--json` 输出使用 OpenMux schema，而不是 tokscale/ccusage 原始 JSON。

### 6. Menubar 只读聚合结果

未来 macOS menubar 不应该在 Swift UI 层直接扫描 Codex/Claude/Gemini 日志。后台 Rust worker 负责调用 usage backend、写 SQLite；Swift/AppKit 或 SwiftUI 只查询最近 summary。这样 UI 打开速度、跨进程锁、错误降级和数据一致性都可控。

## Risks / Trade-offs

- `tokscale-core` API 不是稳定官方 SDK → 通过 `omx-usage-tokscale` adapter 隔离，必要时 vendor 固定版本，并用 fixture 测试覆盖转换逻辑。
- 本地日志格式变化导致解析失败 → `UsageScanReport` 必须记录 provider/backend/source 级 diagnostics；失败不删除历史 event。
- 数据不是账单级准确 → CLI 和 menubar 明确显示 `Source: parsed`，文档说明只有未来 proxy/provider response 才是 `exact`。
- 重复扫描导致重复计数 → SQLite `event_hash UNIQUE`、watermark 和 source offset/fingerprint 三层去重。
- 大日志扫描影响体验 → 使用增量扫描、水位、时间窗口过滤；CLI 可提供 `--no-scan` 只读历史聚合，menubar 后台低频扫描。
- 第三方 pricing 与 provider 实际价格不一致 → cost 字段命名为 `estimated_cost_usd`，允许为空；保留 token 作为主指标。
- 多 backend 同时接入产生重复 → 默认同一 provider 只启用一个 primary backend；fallback backend 只在 primary 不可用时启用，或者写入时用 backend/source 去重策略隔离。

## Migration Plan

1. 在 `omx-core` 增加 usage event/summary/backend domain，不改变现有 quota refresh 行为。
2. 扩展 SQLite migration，新增 `usage_events` 和 `scan_watermarks`。
3. 增加 `omx-usage-tokscale` adapter，把 tokscale parsed messages 映射为 OpenMux `UsageEvent`。
4. 增加 `omx usage` CLI：先 scan，再 query provider summary。
5. 增加 JSON 输出和 fixture tests。
6. 后续 menubar 只消费 SQLite summary，不直接依赖 tokscale。

回滚策略：如果 backend 扫描不可用，保留 schema 和历史 usage events；`omx usage` 显示 unavailable diagnostics，不影响 account/profile switching。

## Open Questions

- `tokscale-core` 是直接作为 git/path dependency、crates.io dependency，还是像 TokenBar 一样 vendor 固定版本。
- 第一版是否只启用 Codex/Claude/Gemini，还是默认启用 tokscale-core 能发现的全部 supported clients。
- pricing 数据源优先级：使用 tokscale 内置 pricing、LiteLLM pricing 数据，还是 OpenMux 自己维护最小 pricing table。
- `omx usage` 默认时间窗口使用 today、last 24h，还是 month-to-date。
