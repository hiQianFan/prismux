## Why

当前 `omx usage` 已经能读取本地 Codex usage，但实现仍偏“即时扫描 + 第三方报表汇总”：历史 JSONL 变大时会慢，source identity 不够细，menubar 高频查询会反复付出解析成本。OpenMux 的产品目标是账号切换与本地用量管理，usage 数据层和 parser 口径需要由 OpenMux 自己掌握，而不是长期绑定 tokscale 这类第三方 report engine。

## What Changes

- 将 usage 数据层明确为 OpenMux 自有模型：`source_state`、`usage_events`、`usage_hourly_rollups`、`quota_snapshots` 分层负责不同问题。
- 保留 `usage_events` 作为事件级事实账本，最小事实单元是 provider 本地日志中的一次 token usage event，例如 Codex `event_msg/token_count`。
- 新增 source checkpoint 能力，记录 `source_path`、`size_bytes`、`mtime_ns`、`parsed_until_byte`、`parser_state_json`、`parser_schema_version`，让扫描从“按窗口重扫”变为“按 source 增量推进”。
- 新增小时级 rollup，作为产品查询层服务 today / 7d / 30d / all、menubar 图表、burn rate 和未来趋势视图。
- 将 tokscale 定位为迁移期对照工具和回归 oracle；目标架构中 `omx usage` 不依赖 tokscale 运行。
- 新增 OpenMux 自有 Codex usage parser，直接解析 `~/.codex/sessions/**/*.jsonl`，覆盖 `token_count`、`turn_context`、`session_meta`、fork/replay/reset、append 增量解析和安全 diagnostics。
- 后续 Claude/Gemini parser 只在 Codex parser 稳定后按同一 adapter contract 增量接入，不提前抽象全部 provider。
- quota 数据继续独立存储，作为额度窗口和 reset 信息，不参与 token 事件账本。
- 明确不做实时 watcher、分钟级 rollup、全 provider 大抽象或远程 quota 倒推 token。

## Capabilities

### New Capabilities

- `usage-ledger-rollups`: 管理本地 usage 的事件账本、source 增量扫描水位、小时级产品 rollup，以及 OpenMux 自有 parser 取代 tokscale 的运行边界。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-core`: 扩展 usage source checkpoint、event source identity、hourly rollup 类型和 SQLite 查询。
- `crates/omx-core::state_store`: 新增或调整 `source_state` / `scan_watermarks`、`usage_hourly_rollups`，并保证 usage event ingest 与 rollup 更新同事务。
- `crates/omx-usage-tokscale`: 降级为开发期对照/测试辅助；生产 usage scan 迁移到 OpenMux 自有 parser。
- 新增或重构 usage parser 模块：第一阶段只实现 Codex，避免为未来 provider 预先造大框架。
- `crates/omx-cli`: `omx usage` 查询从 usage events 聚合迁移到 hourly rollups，debug/drilldown 才查事件表。
- `crates/omx-menubar-ffi` / `crates/omx-app`: dashboard 使用 rollup 查询，refresh 只触发增量扫描。
- `vendor/tokscale-core`: 迁移完成后从默认构建路径移除；是否保留为 dev-only fixture 对照由实现阶段决定。
- 测试：增加 source append、source rewrite、parser schema 变化、rollup 幂等更新、today/7d/30d 查询、tokscale fallback/替换边界测试。
