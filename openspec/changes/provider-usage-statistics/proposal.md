## Why

OpenMux 后续产品形态会从账号切换扩展到 menubar 和本地 token 汇总；第一阶段用户只需要知道本机 Codex、Claude、Gemini 这类 AI coding client 的整体消耗，不需要也不应该先承诺 account/profile 粒度归因。把第一版收敛到 client/provider 级 `parsed` token usage，可以快速提供有用信息，同时避开不可靠归因和账单级准确性的误导。

调研后的结论是：成熟项目分为两条路线。LiteLLM、Helicone、OpenLIT、Langfuse、OpenTelemetry instrumentation 通过统一 proxy、SDK hook 或 trace pipeline 看到请求/响应，可获得接近账单级的 `exact` usage；ccusage、tokscale、TokenBar 则解析本地 session/log/db/cache，得到本地 coding tool 的 `parsed` usage。OpenMux 当前不控制官方 CLI 的请求出口，因此本变更只实现本地 artifact 解析路线，并明确标注数据质量为 `parsed`。

## What Changes

- 新增本地 token usage 统计能力，第一版默认只启用 `codex`、`claude`、`gemini` 三个 client。
- 明确命名边界：
  - `client`: 本地 AI coding tool，例如 `codex`、`claude`、`gemini`。
  - `model_provider`: 模型服务商或路由商，例如 `openai`、`anthropic`、`google`、`openrouter`。
  - `usage_source`: 本地日志、JSONL、SQLite DB、cache 等被扫描对象。
- 第一版产品主视图按 `client` 汇总，并保留 `model_provider`、model、session、project/workspace、time window 作为明细维度；不按 OpenMux account/profile 归因。
- 引入 usage backend 抽象：OpenMux core 定义稳定 `UsageEvent`、`UsageSummary`、`UsageBackend`、`UsageDataQuality` 和 JSON schema，第三方解析器只作为 adapter。
- 采用 TokenBar 式接入策略：vendor 固定版本 `tokscale-core`，通过 `omx-usage-tokscale` adapter 转成 OpenMux 内部 usage event；不让 CLI、menubar 或 core storage 直接依赖 tokscale 原始 schema。
- 保留 `ccusage` 作为开发期交叉验证参考和未来可选 fallback，不作为第一版核心运行路径。
- SQLite 新增 `usage_events`、`scan_watermarks` 和 source fingerprint 字段，支持幂等 ingest、增量扫描、日志重写/轮转识别和重复扫描保护。
- CLI 新增 `omx usage [client]`，支持 `--since`、`--until`、`--json`、`--no-scan`。默认执行有预算的 best-effort scan，再查询 SQLite 聚合结果。
- token 是主指标；cost 是可选二级指标。JSON 和表格必须标记 cost 状态，例如 `provider_reported`、`estimated`、`missing`。
- 明确隐私边界：parser 可以读取本地日志，但 OpenMux 不持久化、不打印、不在 diagnostics 中包含 raw prompt、raw response、raw auth payload、access token、refresh token、API key 或完整原始日志行。
- 为未来 menubar 定义读取边界：UI 只读 SQLite 聚合结果，后台 worker 负责 scan/ingest，CLI 和 menubar 共用同一路径。

## Capabilities

### New Capabilities

- `provider-usage-statistics`: 按本地 client/provider 统计 AI coding tools 的 parsed token usage，并通过 usage backend 适配第三方本地解析器。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-core`: 增加 usage 领域模型、summary 聚合、backend trait、data quality、cost status、source fingerprint 和 diagnostics。
- `crates/omx-core::state_store`: 增加 `usage_events`、`scan_watermarks` schema、幂等写入、聚合查询、busy timeout/WAL/短事务策略。
- 新增 `crates/omx-usage-tokscale` 或等价 adapter crate：隔离 vendored `tokscale-core` API、版本和类型。
- `vendor/tokscale-core`: 固定第三方解析核心版本；后续升级通过 adapter 和 fixture 回归测试验证。
- `crates/omx-cli`: 增加 `omx usage` 命令、表格输出和 versioned JSON 输出。
- 后续 `omx-menubar`: 读取 SQLite usage summary，不直接扫描本地日志。
- 测试：增加 fixture-based adapter 测试、ccusage/tokscale 对照测试、SQLite ingest 去重测试、source fingerprint/watermark 测试、时间窗口/时区测试、CLI JSON schema 测试、隐私 diagnostics 测试。
