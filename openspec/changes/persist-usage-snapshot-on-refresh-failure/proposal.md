## Why

当前 `omx list <platform>` 每次都会尝试刷新账号额度；一旦网络超时、官方接口异常、认证快照不完整或响应结构变化，列表会退化成 unknown，用户无法区分“真的没有额度数据”和“本次刷新失败”。额度展示需要保留上一次成功结果，同时明确标注该结果的刷新时间和本次失败状态，避免把过时数据伪装成实时数据。

主流项目的设计共识是：配额/限流信息可以保留最近成功样本，但必须暴露时间戳、错误状态和重试边界。GitHub REST API 通过 rate-limit response headers 暴露 remaining/reset/used，并建议优先使用请求返回头而不是额外轮询；Stripe 对 429 和 timeout 强调指数退避、避免重试风暴；Prometheus/Grafana 在监控语义中都把 stale/no data/error 明确建模，而不是静默覆盖为正常值。

后续 OpenMux 明确会支持 menubar。menubar 需要长期运行、快速打开、后台刷新、历史趋势和 usage 聚合，单纯的 per-account JSON snapshot 会难以表达 refresh attempt、扫描水位、token usage event 和跨进程读取。参考 TokenBar/ClaudeBar 这类本地优先 menubar 工具后，本变更需要把短期 CLI 兜底和长期本地状态库放到同一条演进路径里：认证 payload 继续留在私有文件或未来 Keychain；账号/profile 索引、额度快照、刷新尝试、移除状态和 token usage event 进入 SQLite。

## What Changes

- Codex usage 刷新成功后，将结构化 `UsageSnapshot` 写入 SQLite `quota_snapshots`，并记录本次 `refresh_attempts`。
- 当下一次 `list` 刷新失败时，从 SQLite 读取上一次成功 quota snapshot，继续展示旧的额度和 reset time。
- 降级展示时把 `UsageSnapshot.source` 标为 `StoredSnapshot`，并将 diagnostics 替换为本次失败原因，例如 `timeout`、`network`、`http_429`、`auth` 或 `schema`。
- CLI 账号明细新增 `Refresh` 列，显示额度数据最后一次成功刷新的时间；没有成功刷新记录时显示 `-`。
- 没有可用历史 snapshot 时，保持现有 unknown/error 展示，不制造假额度。
- 设计并预留稳定账号身份：持久层使用 OpenMux 生成的 `local_id`，数字编号只作为 CLI selector/display number，不作为 usage/cache 的长期主键。
- 设计本地 SQLite 状态库：保存非敏感 `accounts` 索引、`quota_snapshots`、`refresh_attempts`、未来 `usage_events` 和 `scan_watermarks`；auth snapshot、token 和 raw provider payload 不进入 SQLite。
- 统一数据口径：`accounts`/`profiles` 管 OpenMux 管理对象，`quota_snapshots` 管 provider 返回的额度视图，`refresh_attempts` 管每一次刷新请求，`usage_events` 管后续 `tokscale-core` 解析出的本地 token 消耗。
- 为未来 menubar 定义刷新语义：区分 `interactive` refresh 和 `background` refresh，后台刷新必须有 provider floor、TTL、失败退避和 no-activity 降频策略。
- 为后续接入 `tokscale-core` 预留 usage event 模型：TokenBar 类本地日志解析结果可落到 `usage_events`，用于按账号、provider、project、session、model 聚合 token/cost。
- 新增 remove/archive 语义：账号或 profile 被移除时删除 OpenMux 管理的 secret/config snapshot，停止参与 `list/use/refresh`，但在 SQLite 中保留 archived 记录和历史 quota snapshot、refresh attempt、usage event 的可追溯归属；真正删除所有历史以后再单独设计 purge。
- 不新增自动 doctor 修复、私有 endpoint 调用，也不在本变更中实现第三方 API key 的余额/额度查询。

## Capabilities

### New Capabilities

- `usage-refresh-fallback`: 账号额度刷新失败时复用 SQLite 中最后一次成功 quota snapshot，并在展示层标注刷新时间与当前失败诊断。
- `local-usage-state-store`: 为账号 usage、refresh 记录、menubar 和 token usage history 设计 SQLite 本地状态库边界。
- `managed-target-removal`: 为账号和 profile 定义 remove/archive 语义，避免过时账号继续参与切换和刷新。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-plugin-codex`: usage refresh 成功/失败时写入 SQLite 状态库；失败时从 SQLite 读取最近成功 quota snapshot 进行降级展示。
- `crates/omx-cli`: 账号明细表增加 `Refresh` 列，继续通过 `Status` 展示当前诊断。
- `crates/omx-core`: 复用既有 `UsageSnapshot.refreshed_at_unix`、`UsageSource::StoredSnapshot` 和 `UsageDiagnostic`，并补充稳定账号 ID、refresh kind、refresh attempt、usage event 的领域模型。
- `crates/omx-storage` 或 `omx-core::storage`: 增加本地 SQLite 状态库封装，避免 plugin/CLI 直接拼 SQL。
- `crates/omx-plugin-codex` / `crates/omx-plugin-claude`: 后续 remove/archive 账号或 profile 时更新 SQLite active/archived 状态，并处理对应 auth/profile 文件。
- 后续 `omx-menubar`: 读取同一状态库，使用 refresh attempts 和 quota snapshots 展示 stale/error/fresh 状态。
- 测试：增加 Codex 降级读取 SQLite quota snapshot、记录 refresh attempt、账号/profile remove 后不再参与 list/use/refresh 的回归测试，更新 CLI 表头测试；不保留旧 JSON snapshot 迁移路径。
