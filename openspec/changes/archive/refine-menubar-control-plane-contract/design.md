## Architecture

Menubar 和 CLI 都是 presentation shell。业务事实和展示口径收敛到 `omx-app`：

```text
CLI UI ─┐
        ├─> omx-app DashboardView / OperationResult
Menubar ┘        │
                 ├─> omx-core / StateStore
                 └─> provider plugins
```

`omx-menubar-ffi` 只负责 JSON transport、schema gate、panic capture、error envelope 和 memory free，不承载 provider grouping、action eligibility 或 usage/quota 解释。

## Data Model

### Provider Group

`tool_provider` 是 OpenMux plugin ID，例如 `codex`、`claude`、`gemini`。Menubar provider grouping 只使用 `tool_provider`。

每个 provider group 包含：

- `tool_provider`
- `display_name`
- `status`
- `active_target_key`
- `active_target_kind`: `account | profile`
- `accounts[]`
- `profiles[]`（当 provider 支持 profile target）
- `diagnostics[]`
- `refresh_state`

一个 provider group 全局只能有一个 active target。account 和 profile 不是两个可同时生效的槽位，而是同一个 target 命名空间下的两类候选项。

### Account Target

Swift identity 使用 `account_key = tool_provider + "/" + local_id`，不能只用 `local_id`。

Account DTO 包含 presentation-ready 字段：

- `display_label`
- `secondary_label`
- `active`
- `status_level`
- `status_text`
- `quota_summary`
- `primary_window`
- `last_updated_unix`
- `actions.can_switch`
- `actions.disabled_reason`

### Usage

Local usage summary 保持独立：

- `today total tokens`
- `top client`
- `top model`
- `coverage`
- `freshness`

不得把 local token usage 当 provider quota，也不得把 `model_provider` 当 Menubar provider group。

## Operations

### dashboard.get

返回完整 dashboard view。首屏、popover reopen、mutation 后覆盖状态都使用同一个 shape。

### provider.refresh

输入：

- `tool_provider`
- `kind`: `interactive | background | startup`

返回：

- `operation.status`: `success | skipped | failed`
- `operation.message`
- `operation.diagnostics[]`
- `dashboard`

refresh skipped 要说明是 `fresh_enough`、`backoff` 还是 unsupported。

### account.switch

输入：

- `tool_provider`
- `target_kind`: `account | profile`
- `local_id`

返回：

- `operation.status`
- `changed`
- `active_before`
- `active_after`
- `message`
- `dashboard`

Swift 在 backend success 前不得移动 active 标记。

当 `target_kind = account` 时，backend-confirmed dashboard 中同 provider 的 profile target 必须为 inactive；当 `target_kind = profile` 时，同 provider 的 account target 必须为 inactive。

## Menubar UX

Popover 从上到下：

1. Header：OpenMux、聚合健康信号、last updated。
2. Top selector：`Overview | Codex | Claude | Gemini`。
3. Overview page：聚合 usage/quota/health、紧急 limits、provider summaries。
4. Provider page：单 provider 的 header、refresh state、diagnostics、account cards。
5. Local usage summary：today tokens/top client/top model/coverage。
6. Footer：Refresh All、Manage in CLI、Quit。

Provider 明细不堆在一个页面里。Overview 只做摘要和告警入口；用户点进 provider tab 后再操作该 provider 的账号。

Overview 内容顺序：

1. 全账号池聚合：provider 数、account 数、stale/error 数、最低可用 quota。
2. Provider 摘要：每个 provider 当前 active target（account 或 profile）、账号数量、最差状态、last refresh。
3. Token usage 聚合图表：按 provider/client 展示 local parsed usage，不作为 quota。
4. Urgent limits：只列需要注意的 5h/session、7d/weekly 或 missing auth。

单 provider 页面内容顺序：

1. Provider overview：active target、target kind、last refresh、最差 quota/status。
2. Accounts：左侧账号信息，右侧 5h/session 与 7d/weekly activity rings、百分比、reset time、Switch。
3. Profiles：同样作为可切换 target；profile 文案必须说明 backend/profile，不伪装成 account；与 Accounts 共享同一个 active target。
4. Limits：按 5h/session、7d/weekly 展示 percent left 与 reset time。
5. Local usage：该 provider 的 token usage，放在 account/profile 控制之后。

Tray title 不展示 email。默认优先展示：

1. usage rate 或短窗口消耗信号；
2. urgent quota/status；
3. provider pool health；
4. stale/error；
5. fallback `OpenMux`。

## Migration

优先 additive 更新现有 schema，避免一次性破坏 Swift contract tests。若 shape 变化过大，保留 v1 op 兼容层并新增 v2 op 名称。

## Risks

- CLI 全量改吃新 view model 可能扩大 diff；先让 shared DTO 覆盖 Menubar，再逐步迁移 CLI 展示字段。
- 多 provider 尚未完整落地；contract 必须支持 provider group，但实现可先用 Codex 验证。
