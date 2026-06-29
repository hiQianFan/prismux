## Why

Codex 官方 backend 已经返回 `rate_limit_reset_credits`，并提供消费已有 reset credit 的接口；OpenMux 目前只展示 usage window，用户看不到自己是否有可用 reset credit，也无法在 Menubar 中安全触发 reset。

这个能力应该作为账号 quota 的附属控制呈现：让用户知道 credit 是什么、数量是多少，并把高风险的 consume 动作放进账号右侧操作菜单，而不是做成显眼主按钮。

## What Changes

- Codex usage refresh 解析 `rate_limit_reset_credits.available_count`，并通过现有 quota/account DTO 传给 Menubar。
- Account card 标题/副标题区域展示紧凑 credit 标注，例如 `1 reset credit` 或 `3 reset credits`；没有 credit 时不展示。
- Credit 标注必须有 hover/help 文案，说明这是可消费的 Codex rate-limit reset credit，不是 token usage、余额或 weekly quota。
- Account card 右侧 `⋯` 菜单新增 `Reset usage limit`，与 `Delete` 平级；只有存在可用 reset credit 时启用或展示。
- 点击 reset 前必须二次确认，说明会消费 1 个 reset credit，并展示该账号身份。
- 后端新增 consume reset credit operation，沿用现有账号操作分层(`PlatformPlugin` trait 方法 + `OPERATION_LOCK` + dashboard 刷新),调用 Codex backend 的 reset-credit consume endpoint，按响应 `code` 字段返回结构化 outcome 并刷新该账号 quota。
- Reset 成功(含 `windows_reset` 计数)、无可 reset 窗口、无 credit、重复 idempotency key、网络/认证失败都必须用安全 operation result 表达;`failed` 由 omx-app 对 `Err` 归类合成,不是 provider 返回值。
- 不提供强制清零 weekly usage 的能力；只消费服务端已授予的 reset credit。本期不展示 credit 过期时间(留作后续 additive 扩展)。

## Capabilities

### New Capabilities

- `codex-reset-credit-controls`: 定义 Codex reset credit 的读取、Menubar 展示、hover 说明、菜单入口、确认交互和 consume outcome。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-core`: `UsageSnapshot` 增加可选 reset credit metadata;`PlatformPlugin` trait 新增 `consume_reset_credit`(默认 unsupported)+ `ResetCreditOutcome` enum;保持 raw provider response 和 token 不入库。
- `crates/omx-plugin-codex`: 解析 `rate_limit_reset_credits`,把 `fetch_codex_usage` 鉴权逻辑抽成 GET/POST 共用 helper(含 fedramp 头),实现 consume reset credit 请求和 `code` → outcome 映射。
- `crates/omx-app`: Menubar quota DTO additive 增加 reset credit 字段;新增 `consume_reset_credit` 编排(mutation.rs)和 command/report/outcome DTO。
- `crates/omx-menubar-ffi`: `dispatch` 暴露 `consume_reset_credit` op，保持 schema additive、脱敏和 panic-safe。
- `apps/omx-menubar`: account card 展示 credit 标注，`⋯` 菜单增加 reset action、确认弹窗、pending/result 状态;`BackendRequest` 增加 consume case(生成 idempotency key)。
- 测试：覆盖 usage JSON 解析、`code` outcome 映射、HTTP/auth/schema 失败脱敏、DTO decoding、菜单可用性和无 credit 不展示标注。
