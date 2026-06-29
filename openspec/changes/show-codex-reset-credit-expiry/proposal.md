## Why

Codex reset credit 会过期，但 OpenMux 目前只展示可用次数，不告诉用户这两次 reset 什么时候不用就失效。用户在 menubar 判断账号状态时，需要同时看到“有几次 reset”和“最晚何时使用”，否则容易把稀缺机会放到过期。

## What Changes

- 新增读取 Codex reset credit detail 的能力：从 `GET https://chatgpt.com/backend-api/wham/rate-limit-reset-credits` 读取 `credits[]`，解析每个可用 credit 的 `expires_at`。
- 保留现有 `/wham/usage` 作为 quota window 与 `available_count` 的主 refresh 路径，不用 detail endpoint 替换 usage endpoint。
- 将 reset credit 从“只有总数”扩展为“总数 + 可用 credit 的过期时间列表”，按账号 quota snapshot 进入 core、app DTO、FFI 和 Swift DTO。
- Menubar account card 的 reset credit 标注继续保持安静展示；用户 hover 到 account card item 的 reset credit 文案/区域时，展示最多两条过期时间。
- 当 detail endpoint 失败、无 `credits[]`、字段缺失或 schema 变化时，Menubar 仍展示现有 reset credit count，不阻断 quota refresh。
- 不新增消费 reset credit 的交互入口；继续沿用 `add-codex-reset-credit-controls` 已定义的 reset 菜单与确认流程。

## Capabilities

### New Capabilities

- `codex-reset-credit-expiry`: 定义 Codex reset credit detail 的读取、过期时间建模、失败降级和 Menubar hover 展示。

### Modified Capabilities

- 无。

## Impact

- `crates/omx-core`: 扩展 `UsageResetCredits`，新增可选 per-credit expiry metadata，保持 serde additive。
- `crates/omx-plugin-codex`: 新增 reset-credit detail GET 请求与 `credits[].expires_at` 解析；usage refresh 可组合 detail 结果，但 detail 失败只产生非阻断 diagnostic 或静默降级。
- `crates/omx-app`: quota DTO / mapper 增加 reset credit expiry 列表；聚合仍只统计 count，不把 expiry 混进 quota window reset time。
- `crates/omx-menubar-ffi`: schema v1 additive 输出新字段，旧 Swift decode 不受影响。
- `apps/omx-menubar`: Swift DTO 解码 expiry；account card reset credit hover/help 展示两次 reset credit 的过期时间。
- 测试：覆盖 detail payload 解析、字段缺失/失败降级、DTO decode、Menubar hover 文案生成和现有 count-only fixture 兼容。
