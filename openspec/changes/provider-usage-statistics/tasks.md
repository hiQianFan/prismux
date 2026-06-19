## 1. 设计与依赖收敛

- [ ] 1.1 确定 `tokscale-core` 接入方式：vendor path、git dependency 或 crates.io dependency，并记录版本固定策略。
- [ ] 1.2 确定第一版默认启用 provider 集合：Codex、Claude、Gemini，其他 tokscale 支持项默认关闭或实验启用。
- [ ] 1.3 确定 `estimated_cost_usd` pricing 来源和缺失策略，确保 cost 可为空且 token 是主指标。

## 2. Core Usage Domain

- [ ] 2.1 在 `omx-core` 增加 `UsageEvent`、`UsageSummary`、`UsageTokenBreakdown`、`UsageDataQuality`、`UsageEventSource`。
- [ ] 2.2 增加 `UsageBackend`、`UsageScanOptions`、`UsageScanReport`、`UsageScanDiagnostic` 抽象。
- [ ] 2.3 实现 provider/time-window/model/project/session 维度的 summary 聚合函数。
- [ ] 2.4 增加 core 单元测试覆盖 token 汇总、cost 汇总、空 cost、混合 data quality 和无 account/profile 归因。

## 3. SQLite Usage Store

- [ ] 3.1 在 `StateStore` migration 中新增 `usage_events` 和 `scan_watermarks` 表及索引。
- [ ] 3.2 实现 `insert_usage_events_idempotent`，通过 `event_hash` 避免重复写入。
- [ ] 3.3 实现 `update_scan_watermark` 和 `scan_watermark` 查询。
- [ ] 3.4 实现 `usage_summary` 查询，支持 provider、since、until、model、project/session 的后续扩展。
- [ ] 3.5 增加 SQLite 测试，覆盖重复 ingest 不重复计数、扫描失败不清空历史、watermark 更新。
- [ ] 3.6 验证 SQLite 不写入 raw prompt、raw response、raw auth payload、token 或 API key。

## 4. tokscale Adapter

- [ ] 4.1 新增 `crates/omx-usage-tokscale` 或等价 module，隔离 `tokscale-core` 类型和 API。
- [ ] 4.2 实现 Codex、Claude、Gemini 的本地 source discovery 到 tokscale scan options 的映射。
- [ ] 4.3 将 tokscale parsed messages/reports 转换为 OpenMux `UsageEvent`。
- [ ] 4.4 为 source path、offset/fingerprint、session/request id 和 token tuple 生成稳定 `event_hash`。
- [ ] 4.5 为 missing source、unsupported provider、parse error 输出安全 diagnostics。
- [ ] 4.6 增加 fixture-based adapter 测试，覆盖 Codex、Claude、Gemini 最小样本。

## 5. CLI Usage Command

- [ ] 5.1 增加 `omx usage [provider]` 命令和 `--since`、`--until`、`--json`、`--no-scan` 参数。
- [ ] 5.2 默认执行 best-effort scan，再从 SQLite 查询 provider summary。
- [ ] 5.3 实现 provider summary 表格输出，展示 input、output、cache read/write、reasoning、total、estimated cost、source quality。
- [ ] 5.4 实现 `omx usage --json`，输出 OpenMux-owned schema，不暴露 tokscale 原始结构。
- [ ] 5.5 增加 CLI 测试，覆盖全 provider、单 provider、日期范围、无数据、scan diagnostic 和 JSON 输出。

## 6. 安全与降级

- [ ] 6.1 确保 usage scan 错误不影响 `omx list`、`omx use`、`omx current` 等 account/profile 切换命令。
- [ ] 6.2 对 diagnostics 做脱敏，禁止包含 raw prompt、raw response、auth payload、access token、refresh token、API key。
- [ ] 6.3 当本地日志不存在或 provider unsupported 时返回 empty/unavailable summary，不合成 fake usage。
- [ ] 6.4 文档和 CLI 文案明确 `parsed` 不是账单级 exact usage。

## 7. 验证

- [ ] 7.1 运行 `cargo fmt --all`。
- [ ] 7.2 运行 `cargo test`。
- [ ] 7.3 运行 `cargo clippy --all-targets --all-features`。
- [ ] 7.4 用临时 `OMUX_STATE_ROOT` 和 `CODEX_HOME` 手动验证 `omx usage` 不读取或打印敏感文件内容。
