## 1. Phase 0 Spike 与依赖固定

- [x] 1.0 复核 `tokscale` upstream 仓库、commit、license、crate 结构、公开 API 和 adapter 限制，并记录到 `design.md`。
- [x] 1.1 Vendor 固定版本 `tokscale-core` 到 `vendor/tokscale-core`，记录 upstream commit、license 和本地修改策略。
- [x] 1.2 新建最小 spike，使用 Codex、Claude、Gemini fixtures 跑 tokscale-core 解析，并记录输出字段。
- [ ] 1.3 用 ccusage JSON 输出对照同一批 fixtures，比较 token、cache、reasoning、model、timestamp 和 cost 口径。
- [x] 1.4 固化第一版默认 client 集合为 `codex`、`claude`、`gemini`，确认其他 tokscale clients 不默认扫描。
- [x] 1.5 记录 dependency size、build time、主要 transitive dependencies 和 license 风险。

## 2. Core Usage Domain

- [x] 2.1 在 `omx-core` 增加 `UsageEvent`、`UsageSummary`、`UsageTokenBreakdown`、`UsageDataQuality::Parsed`、`CostStatus`、`UsageEventSource`。
- [x] 2.2 将命名模型明确为 `client`、`model_provider`、`model`、`usage_source`，避免使用含糊的 provider 主键。
- [x] 2.3 实现 normalized total 计算：input + output + cache read + cache write + reasoning + extra。
- [x] 2.4 支持 `provider_total_tokens` 单独保存，不覆盖 normalized total。
- [x] 2.5 增加 `UsageBackend`、`UsageScanOptions`、`UsageScanBudget`、`UsageScanReport`、`UsageScanDiagnostic` 抽象。
- [x] 2.6 实现 client/time-window/model/model-provider/project/session 维度的 summary 聚合函数。
- [x] 2.6a 实现第一版 client + since/until time-window summary 聚合函数。
- [ ] 2.7 增加 core 单元测试覆盖 token 汇总、provider total 差异、cost missing、cache 5m/1h、空数据和无 account/profile 归因。

## 3. Source Fingerprint 与 Event Identity

- [x] 3.1 实现 `UsageSourceFingerprint`，包含 canonical path、size、modified timestamp、sample hashes、content/prefix hash、related sidecar fingerprints。
- [ ] 3.2 支持 JSONL、SQLite DB/WAL、Claude sidecar metadata 等 source 的 fingerprint 生成。
- [x] 3.3 实现 parser schema version 和 backend version 对 watermark 失效的判断。
- [x] 3.4 实现分层 event hash：request/message id、JSONL offset + line hash、SQLite record id、fallback token tuple。
- [ ] 3.5 增加测试覆盖日志重写、日志轮转、sidecar 变化、parser schema version 变化、hash 冲突但 payload 不一致。
- [x] 3.5a 增加测试覆盖日志内容重写、sidecar 变化、parser schema version 变化、hash 冲突但 payload 不一致。

## 4. SQLite Usage Store

- [x] 4.1 在 `StateStore` migration 中新增 `usage_events` 和 `scan_watermarks` 表及索引。
- [x] 4.2 配置 SQLite `busy_timeout`，评估并启用 WAL mode。
- [x] 4.3 实现 `insert_usage_events_idempotent`，通过 `event_hash` 避免重复写入，并检测 hash 冲突 payload 不一致。
- [x] 4.4 实现 `update_scan_watermark` 和 `scan_watermark` 查询，并保证 event ingest 与 watermark 更新同事务。
- [x] 4.4a 实现 `update_scan_watermark` 和 `scan_watermark` 查询。
- [x] 4.5 实现 `usage_summary` 查询，支持 client、since、until、model、model_provider、project/session 后续扩展。
- [x] 4.5a 实现第一版 `usage_summary` 查询，支持 client、since、until。
- [ ] 4.6 增加 SQLite 测试，覆盖重复 ingest 不重复计数、扫描失败不清空历史、busy timeout、watermark 原子更新。
- [x] 4.6a 增加 SQLite 测试，覆盖重复 ingest 不重复计数、hash 冲突拒绝、时间过滤、watermark round-trip。
- [x] 4.6b 增加 SQLite 测试，覆盖同批 ingest 发生 event hash 冲突时 event 与 watermark 同事务回滚，且既有历史 usage 不被清空。
- [x] 4.6c 增加 SQLite busy timeout 回归测试，覆盖短暂写锁竞争下 usage event insert 会等待锁释放并成功写入。
- [x] 4.6d 增加真实 `omx` 二进制 integration，覆盖已有 usage history 入库后触发 unsupported-client scan diagnostic，再用 `--no-scan` 查询确认历史 summary 保留。
- [ ] 4.7 验证 SQLite 不写入 raw prompt、raw response、raw auth payload、token、API key 或完整原始日志行。
- [x] 4.7a 增加 core SQLite 隐私回归测试，使用包含 raw prompt、raw response、access token、API key 的 source 文件生成 fingerprint，确认 SQLite 主库/WAL 不写入源文件原文。

## 5. tokscale Adapter

- [x] 5.1 新增 `crates/omx-usage-tokscale`，隔离 vendored `tokscale-core` 类型和 API。
- [x] 5.2 实现 Codex、Claude、Gemini source discovery 到 tokscale scan options 的映射，并禁止默认扫描其他 clients。
- [x] 5.3 将 tokscale parsed messages/reports 转换为 OpenMux `UsageEvent`，填充 client、model_provider、model、session、project、token breakdown。
- [ ] 5.4 转换 cache read/write、reasoning、provider total 和 fallback model 信息，避免重复计数。
- [x] 5.4a 转换 cache read/write、reasoning 和基础 model/provider 信息。
- [ ] 5.5 将 tokscale/tokscale-core cost 映射为 `CostStatus::Estimated` 或 `Missing`；如果 source 自带 cost，映射为 `ProviderReported`。
- [x] 5.5a 在 `parse_local_clients` adapter 中将 cost 映射为 `CostStatus::Missing`，避免伪造 `$0.00`。
- [ ] 5.6 为 missing source、unsupported client、parse error、budget exceeded 输出安全 diagnostics。
- [x] 5.6a 为 unsupported client 输出安全 diagnostic，并在默认路径禁止扫描非 `codex`、`claude`、`gemini` clients。
- [x] 5.6b 为 tokscale adapter 的 `timeout_ms` 预算增加严格等待限制和 `budget_exceeded` 安全 diagnostic；超预算时不返回 events，避免 CLI 入库超预算结果。
- [x] 5.6c 为 tokscale adapter 的 `max_source_files` 和 `max_source_bytes` 增加 source discovery preflight；超预算时返回安全 `budget_exceeded` diagnostic 且不启动 parse。
- [ ] 5.7 增加 fixture-based adapter 测试，覆盖缺 timestamp、缺 model、重复 event、cache-only event、reasoning-only event、跨天 event、unknown model。
- [x] 5.7a 增加 Codex、Claude、Gemini 最小 fixture-based adapter 测试。
- [x] 5.7b 增加 Gemini fixture-based adapter 边界测试，覆盖 cache-only event、reasoning-only event 和 unknown model label 保留。
- [x] 5.7c 增加 Gemini fixture-based adapter 边界测试，覆盖跨天 event window 过滤和重复 source message 生成相同 OpenMux event hash 以支持 SQLite 幂等去重。

## 6. CLI Usage Command

- [x] 6.1 增加 `omx usage [client]` 命令和 `--since`、`--until`、`--json`、`--no-scan` 参数。
- [x] 6.2 默认使用本地时区的 `today` window，并在 JSON 输出中包含 timezone。
- [x] 6.3 默认执行有预算 best-effort scan，再从 SQLite 查询 client summary。
- [x] 6.4 实现 provider/client summary 表格输出，展示 input、output、cache read/write、reasoning、normalized total、optional provider total、cost/status、quality。
- [x] 6.5 实现 `omx usage --json` versioned schema，包含 `schema_version`、`generated_at_unix`、`timezone`、`window`、`quality`、`clients`。
- [ ] 6.6 增加 CLI 测试，覆盖全 client、单 client、日期范围、today 本地时区、无数据、no-scan、scan budget diagnostic 和 JSON 输出。
- [x] 6.6a 增加 CLI 单元测试覆盖 usage help、日期范围解析、missing cost 展示和 JSON 空数据 schema，并用临时 `OMUX_STATE_ROOT` smoke test 验证 `omx usage --no-scan --json` 空数据输出。
- [x] 6.6b 增加真实 `omx` 二进制 integration 测试，使用临时 `OMUX_STATE_ROOT` 覆盖 `usage --no-scan --json`、单 client、日期范围、空数据输出和 unsupported client 安全 diagnostic。
- [x] 6.6c 增加真实 `omx` 二进制 Codex、Claude、Gemini fixture scan integration，覆盖 scan → ingest → SQLite summary → JSON 输出，并修正 tokscale 毫秒 timestamp 到 OpenMux Unix seconds 的归一化。
- [x] 6.6d 增加真实 `omx` 二进制重复扫描 integration，确认同一 source 第二次扫描 `inserted_events=0` 且 summary 不重复计数。
- [x] 6.6e 增加真实 `omx` 二进制 scan diagnostic integration，确认 usage scan diagnostic 不会清空既有 SQLite usage history。

## 7. 安全、隐私与降级

- [ ] 7.1 确保 usage scan 错误不影响 `omx list`、`omx use`、`omx current` 等 account/profile 切换命令。
- [x] 7.1a 增加真实 `omx` 二进制 integration，覆盖 usage unsupported-client diagnostic 后 `omx use codex`、`omx current codex`、`omx list codex` 仍可正常使用同一临时 `OMUX_STATE_ROOT` 和 `CODEX_HOME`。
- [ ] 7.2 对 diagnostics 做脱敏，禁止包含 raw prompt、raw response、raw log line、auth payload、access token、refresh token、API key。
- [x] 7.2a 在 `omx usage` 表格和 JSON 输出边界对 usage scan diagnostics 做敏感标记脱敏，覆盖 raw prompt/response/log line、auth payload、access/refresh token、API key、Bearer 和 `sk-`。
- [ ] 7.3 当本地日志不存在或 client unsupported 时返回 empty/unavailable summary，不合成 fake usage。
- [x] 7.3a 增加真实 `omx usage --json` integration 覆盖：支持的 client 没有本地日志时返回 empty summary，不合成 fake usage；unsupported client 返回 empty summary + 安全 diagnostic。
- [ ] 7.4 文档和 CLI 文案明确 `parsed` 不是账单级 exact usage，cost 是 optional/estimated。
- [x] 7.4a 在 `omx usage` 表格 hint 和 JSON `notes` 中明确 parsed local usage 不是 provider billing/exact quota accounting，cost 可能 missing/estimated。
- [ ] 7.5 增加隐私回归测试，使用包含 prompt/response/API key 字样的 fixture，确认 SQLite、stdout、stderr 和 diagnostics 都不泄漏原文。
- [x] 7.5a 增加 `omx usage --json` diagnostic 隐私回归测试，确认包含 raw prompt、raw response、access token、API key 的 diagnostic message 不进入 JSON 输出。
- [x] 7.5b 增加真实 `omx usage --json` scan 隐私回归测试，使用包含 raw prompt、raw response、API key 字样的 fixture，确认 stdout/stderr 和 SQLite 文件不泄漏原文。

## 8. 验证

- [x] 8.1 运行 `cargo fmt --all`。
- [x] 8.2 运行 `cargo test`。
- [x] 8.3 运行 `cargo clippy --all-targets --all-features`。
- [ ] 8.4 用临时 `OMUX_STATE_ROOT`、`CODEX_HOME`、`CLAUDE_HOME`、`GEMINI_CLI_HOME` 手动验证 `omx usage` 只扫描预期 client source，且不打印敏感文件内容。
