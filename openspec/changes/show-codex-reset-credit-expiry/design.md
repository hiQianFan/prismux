## Context

`add-codex-reset-credit-controls` 已经把 Codex reset credit count 接进 `UsageSnapshot.reset_credits.available_count`，并在 Menubar account card 上展示 reset credit 数量。该提案当时明确把“credit 过期时间”留作后续 additive 扩展。

实测 Codex backend 有两个相关 endpoint：

- `GET /backend-api/wham/usage`：当前 OpenMux 已使用。它返回 quota windows、`rate_limit_reset_credits.available_count`，以及 window 的 `reset_at/reset_after_seconds`。
- `GET /backend-api/wham/rate-limit-reset-credits`：当前 OpenMux 未使用。它返回 `available_count`、`total_earned_count` 和 `credits[]`；当账号有 2 次 reset 时，`credits[]` 中每个元素包含 `status = "available"`、`reset_type = "codex_rate_limits"`、`granted_at`、`expires_at`、`redeemed_at` 等字段。

官方公开文档未发现这两个 `wham` endpoint 的稳定 contract。它们都应被视为私有/未文档化 endpoint。基于风险控制，本设计不把 detail endpoint 变成 quota refresh 的唯一数据源。

## Goals / Non-Goals

**Goals:**

- 在账号 quota snapshot 中保存可用 reset credit 的 `expires_at`。
- Menubar hover 到 account card 的 reset credit 标注时，展示最多两条过期时间，让用户知道这两次 reset 什么时候不用会过期。
- 保持 count-only 兼容：没有 detail 或 detail 请求失败时，UI 仍可展示现有 `N resets`/`N credits`。
- 不泄露 token、raw auth payload、Authorization header 或 raw provider response。

**Non-Goals:**

- 不把 `/wham/rate-limit-reset-credits` 替换为唯一 quota refresh endpoint。
- 不展示 redeemed / expired 历史 credit。
- 不改变 consume reset credit 的入口、确认流程或 idempotency 逻辑。
- 不把 reset credit expiry 混同为 5h/7d usage window 的 `reset_at`。
- 不为 Claude/Gemini 增加同名字段。

## Decisions

### 1. 接口选择：保留 `/wham/usage`，新增 detail 补充 expiry

主 refresh 继续调用 `/wham/usage`，原因：

- 当前代码已经围绕它解析 quota windows、`available_count`、缓存和 stale fallback。
- 它同时返回 5h/7d window 的 `reset_at`，是账号是否 limited/exhausted 的主数据源。
- detail endpoint 只包含 reset credit 明细，不包含完整 usage window，因此无法替代现有 quota refresh。

新增 `fetch_codex_reset_credit_details(auth)` 调用 `GET /wham/rate-limit-reset-credits`，只用于补充 `credits[].expires_at`。推荐流程：

```text
usage_from_snapshot(account)
  -> fetch_codex_usage(auth)
  -> parse_codex_usage_snapshot(payload, now)
  -> if usage.reset_credits.available_count > 0:
       fetch_codex_reset_credit_details(auth)
       merge available credit expiries into usage.reset_credits
  -> save_quota_snapshot(...)
```

detail 请求失败时不让整个 quota refresh 失败。最多记录一个非阻断 diagnostic，或直接保留 count-only snapshot。这样私有 endpoint schema 变更不会让 Menubar 失去 quota 主功能。

替代方案：完全改用 `/wham/rate-limit-reset-credits` 读取 count。暂不采用，因为它无法提供 usage windows，且官方未文档化；用它覆盖 `/wham/usage` 的 `available_count` 反而扩大风险。

### 2. 数据模型：在 `UsageResetCredits` 下增加 per-credit expiry

保持 reset credit 作为 quota 的附属 metadata：

```rust
pub struct UsageResetCredits {
    pub available_count: u32,
    pub credits: Vec<UsageResetCredit>,
}

pub struct UsageResetCredit {
    pub status: Option<String>,          // 只透传非敏感状态；UI 主要消费 available
    pub reset_type: Option<String>,      // 预期 codex_rate_limits
    pub granted_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
}
```

解析规则：

- 只把 `status == "available"` 且存在 `expires_at` 的 credit 用于 Menubar expiry hover。
- `expires_at`/`granted_at` 使用 RFC3339 解析为 Unix seconds，避免 Swift/Rust 时区口径分裂。
- `credits[]` 按 `expires_at_unix` 升序排序；缺失 `expires_at` 的 available credit 保留在模型中也可以，但 UI 不展示具体过期时间。
- `available_count` 仍以 `/wham/usage` 的 `rate_limit_reset_credits.available_count` 为主。detail 的 `available_count` 可用于测试/diagnostic，不用于覆盖主值，除非后续明确产品决策。

DTO additive 映射：

```rust
ResetCreditsView {
    available_count: u32,
    credits: Vec<ResetCreditView>,       // default empty
}

ResetCreditView {
    expires_at_unix: Option<i64>,
    granted_at_unix: Option<i64>,
    status: Option<String>,
    reset_type: Option<String>,
}
```

Swift `ResetCredits` 增加 optional/defaulted `credits: [ResetCredit]`，旧 fixture 缺字段时 decode 成空数组。

### 3. UI：hover reset credit 标注展示两次过期时间

hover 目标是 account card 上的 reset credit 标注，例如 `2 resets` 或 `2 credits`，不是 5h/7d quota window 的 reset time。展示内容建议：

```text
Codex reset credits
1. Expires 2026-07-18 08:27
2. Expires 2026-07-27 08:01
```

规则：

- 最多展示两条，因为 Codex 当前最多常见为两次，且用户核心诉求是“两次 reset 的过期 time”。
- 时间使用用户本地时区，格式与 Menubar 现有 `fullDateTimeLabel` 口径一致，避免显示 UTC 造成误判。
- 没有 expiry detail 时，hover 仍显示说明文案：`2 reset credits available. Expiry unavailable.`
- 如果 AppKit `.help` 在 transient `NSPopover` 中不稳定，则实现轻量 SwiftUI hover overlay 或 popover-local tooltip；不要依赖只能偶发显示的系统 tooltip。
- Account card 本体不新增显式文本行，避免压缩账号切换主信息。过期时间只出现在 hover/help。

### 4. 错误与安全边界

- detail endpoint 的 HTTP status、网络、timeout、JSON parse、schema 缺字段都不应让 `refresh_account` 返回失败。
- diagnostic 只能写安全摘要，例如 `reset credit expiry unavailable`，不得保存 raw provider body。
- auth 读取仍走现有 managed runtime scope 与 hash verification，不新增直接读取 Swift 侧 auth 的路径。
- SQLite quota snapshot 只保存结构化 expiry timestamp，不保存 raw response。

## Risks / Trade-offs

- 私有 endpoint schema 变化 → detail 解析失败时降级为 count-only，quota refresh 继续可用。
- 额外 GET 增加 refresh latency → 仅在 `available_count > 0` 时调用 detail；无 credit 账号不额外请求。
- `/wham/usage` count 与 detail `credits[]` 不一致 → UI 展示 `/wham/usage` count，并只把 detail expiry 作为补充；hover 可少于 count 条并标注 unavailable。
- 系统 tooltip 在 Menubar popover 中不可靠 → 实现 popover 内自绘 hover overlay，保证用户真的能看到 expiry。

## Migration Plan

1. 后端先 additive 扩展 core/app DTO 和 parser，旧 snapshot 缺 `credits` 时按空列表处理。
2. FFI fixture 添加 count-only 与带 expiry 两类样例，保持旧字段兼容。
3. Swift DTO 先兼容新字段，再接 hover 文案。
4. 若 detail endpoint 后续不可用，回滚 UI hover detail 展示即可；count 与 consume 逻辑不受影响。

## Open Questions

- 是否需要在 hover 中显示相对时间（例如 `in 18d`）？本提案先只要求本地绝对时间，避免倒计时刷新复杂度。
- 当 `credits[]` 中出现非 `codex_rate_limits` 类型时，是否过滤？本提案建议只展示 `status == available`，`reset_type` 先作为诊断字段保留。
