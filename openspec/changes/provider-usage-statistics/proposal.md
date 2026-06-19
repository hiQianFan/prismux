## Why

OpenMux 后续产品形态会从账号切换扩展到 menubar 和本地 token 汇总；第一阶段不需要按 account/profile 归因，先按 provider 统计 Codex、Claude、Gemini 等工具的 token/cost 使用量即可。这样可以避免把不可靠的账号归因做成核心承诺，同时让用户快速看到“本机各 AI coding provider 今天/本周/本月整体消耗了多少”。

市面上成熟项目大致分两类：LiteLLM、Helicone、OpenLIT 通过统一 proxy 或 OpenTelemetry instrumentation 统计经过自己出口的 API 调用；ccusage、tokscale、TokenBar 则解析本地 agent session/log/db 来汇总 coding tool 使用量。OpenMux 当前管理的是分散的官方账号和 profile，不控制所有请求出口，因此第一版 usage backend 应采用本地日志解析路线，并把数据质量明确标记为 `parsed`，而不是伪装成 provider 官方账单。

## What Changes

- 新增 provider 维度 token usage 统计能力，按 provider/client 汇总 input、output、cache read、cache write、reasoning、total tokens 和 estimated cost。
- 第一版统计维度限定为 provider、model、session、project/workspace、time window；不做 account/profile 级别用量归因。
- 引入 usage backend 抽象：OpenMux core 定义稳定 `UsageEvent`、`UsageSummary`、`UsageBackend` 和 `UsageDataQuality`，第三方解析器只作为 adapter。
- 第一阶段接入 `tokscale-core` 作为本地日志解析 backend，读取 Codex、Claude、Gemini 等 provider 的本地 session/log/db 数据，转成 OpenMux 内部 usage event。
- 保留 `ccusage` 作为 CLI fallback / 验证参考，不把 Node/TypeScript 栈引入 OpenMux 核心路径。
- 将 parsed usage event 写入 SQLite `usage_events`，并通过 `scan_watermarks` 支持增量扫描、去重和重复 ingest 保护。
- 新增 CLI usage 总览命令，展示 provider 级别 token/cost summary，例如 `omx usage`、`omx usage codex` 和 JSON 输出。
- 为未来 menubar 定义读取边界：UI 只读 SQLite 聚合结果，后台 worker 负责 refresh/scan/ingest。
- 明确数据质量语义：`exact` 代表统一 proxy/provider response，`parsed` 代表本地日志解析，`inferred` 代表 OpenMux 时间线推断；本变更只实现 `parsed`。
- 不新增统一 proxy，不拦截官方 CLI 网络请求，不调用私有 provider usage endpoint，不展示或存储 raw prompt、raw response、token、API key 或完整原始日志内容。

## Capabilities

### New Capabilities

- `provider-usage-statistics`: 按 provider 统计本地 AI coding tools 的 token/cost 使用量，并通过 usage backend 适配第三方本地解析器。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-core`: 增加 provider usage 领域模型、summary 聚合、backend trait、data quality 和 diagnostics。
- `crates/omx-storage` 或 `omx-core::storage`: 增加 `usage_events`、`scan_watermarks` schema、查询和去重写入封装。
- 新增 `crates/omx-usage-tokscale` 或等价 adapter crate：隔离 `tokscale-core` API、版本和类型，不把第三方 schema 泄漏到 OpenMux core。
- `crates/omx-cli`: 增加 provider usage 命令和表格/JSON 输出。
- 后续 `omx-menubar`: 读取 provider usage summary，不直接扫描本地日志。
- 测试：增加 fixture-based parser adapter 测试、SQLite ingest 去重测试、provider/time window 聚合测试、CLI 输出测试。
