## Context

OpenMux 现在已有 `usage_events`、`scan_watermarks`、`UsageEvent`、`UsageSummary`、`omx usage` 和 menubar usage 查询基础。问题不在是否能显示 token，而在数据层职责不清：当前扫描依赖 tokscale 的 report API，OpenMux 拿不到足够稳定的 source path、offset、parser state，也无法把 usage 查询优化成产品需要的小时级账本。

调研事实：

- tokscale/TokenBar 的 Codex 统计口径合理：读取本地 Codex JSONL `event_msg/token_count`，优先使用 `last_token_usage`，用 `total_token_usage` 辅助 dedup、fork、reset、replay。
- CodexBar 证明了性能关键不是新公式，而是 per-file cache：`path + mtime + size + parsedBytes + parser state`，文件未变跳过，append 时从 offset 继续。
- tokenuse 证明了产品数据应进入本地 archive，而不是每次从原始日志生成报表；它也把 usage calls 和 limit snapshots 分开。
- codex-usage-tracker 证明了 Codex 专项增量索引需要 `parsed_until_byte`、`parsed_until_line`、`parser_state_json`。
- 这些项目都不适合作为 OpenMux 的长期底层依赖：它们是 app 或 report engine，不是 OpenMux account switching 产品的数据层。

因此本设计选择：OpenMux 自己实现 usage parser/data layer，tokscale 只作为迁移期对照，不进入目标运行路径。

## Goals / Non-Goals

**Goals:**

- 生产路径取代 tokscale，第一阶段实现 OpenMux 自有 Codex parser。
- 以 `UsageEvent` 作为事实账本最小单元，以 `usage_hourly_rollups` 作为产品查询最小单元。
- 通过 `source_state` 支持 Codex JSONL append 增量解析、重写检测、schema 失效和幂等重放。
- 正确处理 Codex `last_token_usage`、`total_token_usage`、forked session、replayed token_count、total reset、缺 model、缺 timestamp。
- 保持 quota snapshots 与 token usage 分离。
- 让 CLI/menubar 查询不依赖扫描原始日志；refresh 负责增量 ingest，查询读 SQLite。
- 用 tokscale/CodexBar/tokenuse/codex-usage-tracker fixtures 和本地真实日志做回归对照。

**Non-Goals:**

- 不实现统一 proxy、网络拦截或远程 quota 倒推 token。
- 不在第一阶段实现 Claude/Gemini 自有 parser；Codex 稳定后按同一 contract 接入。
- 不实现实时 filesystem watcher；refresh 时扫描即可。
- 不做分钟级 rollup；小时是当前产品最小桶。
- 不按账号硬猜历史归属；没有可靠 active-auth 时间线时 `account_id` 保持 `NULL`。
- 不持久化 raw prompt、raw response、auth payload、token、API key 或完整原始 JSONL 行。

## Decisions

### 1. 目标运行路径不依赖 tokscale

目标路径：

```text
Codex JSONL
  -> omx native codex parser
  -> usage_events
  -> usage_hourly_rollups
  -> CLI / menubar
```

tokscale 只保留两个用途：

- 迁移期对照：同一 fixture 下比较 token totals、model、timestamp、session。
- 临时 fallback：只在 native parser 未覆盖的测试阶段可手动使用，不作为最终默认路径。

替代方案：继续 patch tokscale。拒绝原因：它的核心模型是 report generator，OpenMux 需要 source-level checkpoint、account-aware ledger 和 product rollup，长期 patch 会把维护心智放在外部 schema 上。

需要同步清理的历史内容：

- `Cargo.toml` workspace 中的默认 `crates/omx-usage-tokscale` 与 `vendor/tokscale-core` 成员关系。
- `crates/omx-cli` 和 `crates/omx-menubar-ffi` 对 `TokscaleUsageBackend` 的直接调用。
- `docs/ARCHITECTURE.md` 中 `tokscale-core -> omx-usage-tokscale -> UsageEvent` 的目标链路描述。
- `openspec/changes/provider-usage-statistics` 中把 tokscale 作为第一版采集/解析层的内容；该 change 完成历史价值后应归档或被本 change supersede。
- `openspec/changes/refine-usage-cli-experience` 中“继续复用 tokscale core”的体验设计假设；保留 CLI UX 结论，移除 tokscale runtime 假设。
- `openspec/changes/persist-usage-snapshot-on-refresh-failure` 中“为后续 tokscale-core 预留”的 roadmap 文字；保留 `usage_events` / `scan_watermarks` 数据口径，替换为 native parser。

### 2. Codex parser 先做够用，不做通用 parser 框架

第一阶段只新增 Codex parser contract：

```text
discover_sources(codex_home) -> Vec<UsageSource>
parse_source(source, checkpoint) -> ParsedUsageBatch
```

`ParsedUsageBatch` 包含：

- `events: Vec<UsageEvent>`
- `next_checkpoint`
- `diagnostics`
- `parsed_until_byte`
- `parser_state_json`

不为未来 Claude/Gemini 先造 trait 层级。等第二个 native parser 出现，再抽共同接口。

### 3. Codex token 口径

事实单元是 turn 级 `event_msg/token_count`，不是 session。实测一个真实 session 文件含 13 个 `token_count` 事件，各带独立 timestamp、可跨小时甚至跨天；`last_token_usage` 是本 turn 增量，`total_token_usage` 是会话内单调累计。因此每个 `token_count` 最多生成一个 `UsageEvent`，按其自身 `occurred_at` 落桶，session_id 只作维度不作聚合粒度。

计数规则：

- 有 `last_token_usage` 时优先使用它作为本次增量。
- `total_token_usage` 不直接作为输出 token tuple；它用于 dedup、monotonic check、total-only fallback、fork inherited baseline 和 reset 判断。
- 如果只有 `total_token_usage`，用同 source/parser state 中的前一个 raw total 计算 delta。
- 如果 total 回退且 `last_token_usage` 存在，使用 last。
- 如果 total 回退且没有 last，跳过该事件并记录安全 diagnostic。
- stale/replay 快照：若某事件带 `last_token_usage` 增量，但 `total_token_usage` 未越过上一个已采纳 total（乱序、replay、stale），则不得再次计入该 last，保留原单调 total，记 diagnostic。移植 tokscale `looks_like_stale_regression` 状态机。
- `cached_input_tokens`/`cache_read_input_tokens` 计入 `cache_read`，并从 input 拆出：`cache_read = min(cached, input)`、`input = input - cache_read`；OpenMux token total 仍保留原始 breakdown。
- `reasoning_output_tokens` 计入 `reasoning`，不吞进 `output`，cost 估算阶段再决定费率。

会话内对账不变式（写成测试）：`sum(采纳的 last 增量) == 该 session 最后一个已采纳 total`，逐 token bucket 成立。

> 证据：对本机真实 session 天真求和 `sum(last).input = 645057`，而最后 `total.input = 586502`，差值 58555 恰好等于最后一个 turn 的 last.input —— 末尾是一个 stale replay 快照。天真对 last 求和会整整多算一个 turn（input 约 +10%）。这说明 last/total 混合口径与 stale 检测是必需，不是可选优化；对账不变式能一眼抓出此类重复计数与漏算。

### 4. Source checkpoint 是性能根

新增或收敛 `source_state`：

```text
source_id
client
source_path
source_kind
size_bytes
mtime_ns
fingerprint_json
parsed_until_byte
parsed_until_line
parser_state_json
parser_schema_version
last_scanned_at_unix
last_scan_status
diagnostic_code
```

扫描规则：

- 文件未变：跳过。
- 文件 size 增大且 prefix/fingerprint 匹配：从 `parsed_until_byte` 继续，带入 `parser_state_json`。
- 文件变小、mtime 异常、prefix 不匹配、parser schema 变化：重扫该 source，并先删除该 source 已入库事件及受影响 rollup 后重建。
- source 删除：不删除历史 usage，只标记 source missing。历史账本保留。

### 5. usage_events 是事实账本

事件表保留事件级事实，不直接服务高频图表。

必须补齐：

- `source_path`
- `source_offset`
- `source_record_hash`
- `parser_schema_version`
- `event_hash`
- `account_id NULL`

`event_hash` 优先级：

1. `client + source_path + source_offset + source_record_hash + parser_schema_version`
2. `client + session_id + turn_id + occurred_at_unix + token tuple`
3. fallback：`client + source_path + occurred_at_unix + model + token tuple + source_record_hash`

hash 冲突且 payload 不一致时拒绝覆盖，记录 diagnostic。

### 6. usage_hourly_rollups 是产品查询层

新增表：

```text
bucket_start_unix
local_hour
timezone_offset_seconds
client
account_id nullable
model_provider nullable
model nullable
input_tokens
output_tokens
cache_read_tokens
cache_write_tokens
reasoning_tokens
extra_tokens
normalized_total_tokens
estimated_cost_usd nullable
cost_status
event_count
PRIMARY KEY(bucket_start_unix, client, account_id, model_provider, model)
```

refresh 事务：

```text
parse changed source
delete old events for changed source if full rescan
insert new usage_events idempotently
rebuild affected hourly buckets
update source_state
commit
```

查询规则：

- today：查当天 24 个 local hour bucket。
- 7d：查 168 个 bucket，必要时按 day prefix 折叠。
- 30d：查 720 个 bucket，必要时按 day prefix 折叠。
- all：从 rollup 汇总，不扫事件表。
- drilldown/debug：查 `usage_events`。

### 7. Quota snapshots 不参与 usage total

Codex token_count 里可能携带 rate limit snapshot，远程 `/wham/usage` 也能给 quota window。它们进入 `quota_snapshots`，不生成 `UsageEvent`，不参与 token total。

### 8. 删除 tokscale 的收敛标准

满足以下条件后，默认构建路径移除 `omx-usage-tokscale`：

- Codex native parser 在 fixture 和真实本地样本上与 tokscale/CodexBar totals 差异可解释。
- today/7d/30d 不再触发全量 JSONL scan。
- source append、rewrite、schema bump、duplicate scan 均有测试。
- CLI/menubar 查询全部从 SQLite rollup 返回。
- 隐私测试确认 SQLite/stdout/stderr 不泄漏 raw source 内容。

## Risks / Trade-offs

- [Risk] 自己维护 Codex parser 会跟随 Codex JSONL schema 变化。→ Mitigation: parser schema version、fixture corpus、真实样本 shape survey、遇到未知 shape 记录 diagnostic 而不是猜。
- [Risk] fork/replay/reset 逻辑比看起来复杂。→ Mitigation: 先移植 CodexBar/tokscale 已验证的最小状态机，只覆盖 token_count，不解析 prompts。
- [Risk] rollup 与 event 表可能不一致。→ Mitigation: event ingest、rollup rebuild、source_state 更新必须同事务；测试覆盖回滚。
- [Risk] account 归因诱人但证据不足。→ Mitigation: `account_id` nullable，只有未来建立 active-auth timeline 后才填。
- [Risk] 重扫 source 需要删除旧事件，可能影响历史。→ Mitigation: 只按 `source_path + parser_schema_version/client` 删除，并在同事务重建受影响 bucket。

## Migration Plan

1. 冻结当前 `omx-usage-tokscale`，只允许测试对照，不再修产品行为。
2. 新增 native Codex parser 和 source_state 字段。
3. Codex scan 生产只写 native events；fixture 测试可并行对照 tokscale/CodexBar。
4. 加 `usage_hourly_rollups`，CLI/menubar 查询切换到 rollup。
5. 从 CLI、menubar、架构文档和默认 workspace 中移除 tokscale runtime 路径。
6. 归档或 supersede 旧 OpenSpec 中依赖 tokscale 的 usage 设计，避免后续实现读到过时方向。

Rollback：如果 native parser 出现严重回归，可临时切回现有 tokscale adapter 查询路径；SQLite schema 只增不破坏旧事件表。
