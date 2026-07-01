## 1. Native Codex Parser

- [ ] 1.1 新建 OpenMux native Codex parser 模块，直接读取 `CODEX_HOME` / `~/.codex/sessions/**/*.jsonl`。
- [ ] 1.2 解析 `session_meta`，提取 `session_id`、fork parent、fork timestamp。
- [ ] 1.3 解析 `turn_context` 和 `token_count.info.model/model_name`，维护当前 model。
- [ ] 1.4 解析 `event_msg/token_count`，按 `last_token_usage` 优先、`total_token_usage` 辅助的规则生成 `UsageEvent`。
- [ ] 1.5 实现 Codex parser state，覆盖 previous totals、raw totals baseline、divergent totals、fork inherited baseline、current turn id。
- [ ] 1.6 对缺 timestamp、缺 usage、total 回退且无 last、无法解析行输出安全 diagnostic，不写 raw line。

## 2. Source Checkpoint

- [ ] 2.1 扩展 usage source checkpoint schema，保存 `source_path`、`size_bytes`、`mtime_ns`、`parsed_until_byte`、`parsed_until_line`、`parser_state_json`、`parser_schema_version`。
- [ ] 2.2 实现 source fingerprint/prefix 检查，区分 unchanged、append、rewrite、schema bump。
- [ ] 2.3 实现 append 增量解析，从 `parsed_until_byte` 带 parser state 继续。
- [ ] 2.4 实现 source rewrite 重建：删除该 source 旧 events，重建受影响 rollup，再更新 checkpoint。
- [ ] 2.5 增加测试覆盖 unchanged skip、append increment、rewrite rebuild、schema bump rebuild、source missing 保留历史。

## 3. Event Ledger

- [ ] 3.1 补齐 `usage_events` 的 source identity：`source_path`、`source_offset`、`source_record_hash`、`parser_schema_version`。
- [ ] 3.2 增加 nullable `account_id` 字段或等价扩展，未知归因保持 `NULL`。
- [ ] 3.3 实现 native parser event hash：优先 source offset/hash，其次 session/turn/token tuple fallback。
- [ ] 3.4 保持 event ingest 幂等，hash 冲突 payload 不一致时拒绝覆盖并记录 diagnostic。
- [ ] 3.5 增加隐私测试，确认 SQLite/stdout/stderr/diagnostics 不包含 raw prompt、response、token、API key 或完整 JSONL 行。

## 4. Hourly Rollups

- [ ] 4.1 新增 `usage_hourly_rollups` 表和索引。
- [ ] 4.2 实现 event ingest 同事务更新受影响 hourly bucket。
- [ ] 4.3 实现 source rebuild 时重建受影响 hourly bucket。
- [ ] 4.4 将 today/7d/30d/all usage summary 查询迁移到 hourly rollups。
- [ ] 4.5 增加测试覆盖 today 24 小时、7d/30d 折叠、all 汇总、事务回滚后 rollup 不漂移。

## 5. CLI 与 Menubar 切换

- [ ] 5.1 将 `omx usage codex` 默认扫描切到 native Codex parser。
- [ ] 5.2 将 menubar refresh 切到 native Codex parser + rollup 查询。
- [ ] 5.3 保留 `omx-usage-tokscale` 为 dev/test 对照路径，不作为默认 runtime 依赖。
- [ ] 5.4 增加 fixture 对照测试，比较 native parser 与 tokscale/CodexBar 统计差异并记录可解释差异。
- [ ] 5.5 用真实临时 `CODEX_HOME` 验证重复 refresh 不全量扫描、不重复计数。

## 6. Remove Tokscale Default Path

- [ ] 6.1 从默认 `omx usage` 生产路径移除 tokscale adapter 调用。
- [ ] 6.2 评估是否将 `vendor/tokscale-core` 和 `crates/omx-usage-tokscale` 标记为 dev-only，或在迁移完成后删除。
- [ ] 6.3 更新文档，明确 OpenMux 使用自有 usage ledger/parser，tokscale 仅为历史迁移对照。
- [ ] 6.4 从 `crates/omx-cli`、`crates/omx-menubar-ffi` 和 workspace 默认成员中清理 tokscale runtime 依赖。
- [ ] 6.5 更新 `docs/ARCHITECTURE.md`，删除 `tokscale-core -> omx-usage-tokscale` 作为目标链路的描述。
- [ ] 6.6 标记或归档旧 OpenSpec 中依赖 tokscale 的 usage 设计，保留仍有效的数据层结论。
- [ ] 6.7 运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`。
