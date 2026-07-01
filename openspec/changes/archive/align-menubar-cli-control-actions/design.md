## Context

Menubar 当前是 presentation shell：Swift 通过 FFI 发送 `dashboard`、`refresh`、`switch`、`remove`、`consume_reset_credit` 请求，Rust `omx-menubar-ffi` 做 JSON transport，`omx-app` 生成 dashboard view model 和 operation result。

后端已经会在 dashboard 查询失败时读取 `control-plane/dashboard.last-good.json`，但 response envelope 没有告诉 Swift 这是 snapshot fallback。Swift 收到 dashboard 后统一写入 `.ready(report, stale: false)`，因此旧数据可能显示为 fresh。

`DashboardQuery` 已支持 `usage_period`，但 Swift `Payload.dashboard` 当前只传 provider。Menubar 的 period toggle 会改变图表折叠口径，却不会要求后端 headline 使用同一周期。

CLI 已有 `refresh <platform>`，但只能整个平台刷新；Menubar 后端已有单账号 refresh。Codex reset credit 后端与 Menubar 入口已存在，CLI 缺少兜底命令。

## Goals / Non-Goals

**Goals:**

- last-good dashboard fallback 在 Menubar 中明确显示 stale。
- Menubar usage period 选择传入后端 dashboard query。
- `omx refresh <platform> [selector]` 支持单账号 refresh。
- `omx reset-credit codex <selector>` 支持消费 1 个 Codex reset credit。

**Non-Goals:**

- 不做自动最佳账号选择。
- 不做并发 refresh。
- 不做 CLI dashboard TUI。
- 不把 reset credit 抽象成 provider 通用命令族。
- 不新增外部依赖。

## Decisions

### Decision 1: FFI envelope 用 additive 字段表达 fallback

在 `ResponseEnvelope` 增加可选字段：

- `data_stale: bool`
- `served_from_snapshot: bool`

正常 dashboard 返回 `false` 或省略；dashboard 查询失败但成功读取 last-good snapshot 时返回 `true`。Swift `BackendEnvelope` 解码该字段，`AppStore.request` 在 dashboard 存在时用 `envelope.dataStale || envelope.servedFromSnapshot` 设置 `.ready(report, stale: true)`。

替代方案：把 stale 写入 dashboard body。暂不采用，因为 fallback 来源属于 transport/result 元数据，不是 provider dashboard 事实；放 envelope 对所有 dashboard response 更直接。

### Decision 2: Menubar period 直接传给 DashboardQuery

Swift `UsagePeriod` 按 Rust `UsagePeriod` 当前 serde 名称编码到 `usage_period`：`Today`、`SevenDays`、`ThirtyDays`。`Payload.dashboard` 和会返回 dashboard 的 mutation payload 都携带当前 period。period toggle 变化后触发一次 dashboard reload。

替代方案：继续只在 Swift 端折叠 30d buckets。暂不采用，因为 headline 和 chart 会出现周期口径不一致。

### Decision 3: `refresh` 复用现有 CLI 命令

`Refresh` command 增加可选 `selector`。无 selector 时保持现有 `plugin.refresh_accounts()` 行为；有 selector 时复用 `resolve_target`，只允许 `TargetKind::Account`，然后调用 `plugin.refresh_account(&target_id)`。

替代方案：新增 `refresh-account` 命令。暂不采用，因为 `use/remove` 已使用 `platform + selector` 心智，给 `refresh` 增加可选 selector 更短。

### Decision 4: reset credit 是 Codex-only CLI 命令

新增 `omx reset-credit codex <selector> [--yes]`。命令解析 account selector 后调用 `plugin.consume_reset_credit(&target_id, idempotency_key)`，成功后刷新该账号并展示 outcome 与 quota 摘要。

TTY 且未传 `--yes` 时要求确认；非 TTY 且未传 `--yes` 时返回错误。其他 provider 走已有 trait 默认 unsupported。

替代方案：命名为 `reset` 或做 provider 通用 `credits` 子命令。暂不采用，`reset` 容易误解为重置账号，通用 credits 目前只有一个实现。

## Risks / Trade-offs

- FFI envelope 字段需要 Swift/Rust 双端同步 -> additive optional 字段，旧数据仍可解码。
- Period toggle 触发 dashboard reload 会增加本地扫描/SQLite 查询次数 -> 只在用户切换 period 时触发，不加自动轮询。
- CLI reset credit 是 destructive-ish 远端动作 -> TTY 确认、非 TTY 要 `--yes`。
- 单账号 refresh 不适用于 profile -> resolver 命中 profile 时返回清晰错误。
