## Context

OpenMux Menubar 当前 account card 已经有固定结构：

```text
identity line + right actions
diagnostic line
quota windows
```

Codex backend usage payload 可以返回 `rate_limit_reset_credits.available_count`，官方 Codex 也有 consume reset credit 的请求。该能力和 quota 同源，且是账号级、高风险、低频动作；因此不应该成为主按钮，也不应该进入 provider overview 的聚合操作。

定位前提:**本 app 本质是账号切换器**。用户有多个号,限额卡住时优先切号;credit 珍贵稀少(会过期),reset 是低频慎重动作。所以**限额卡住不主动怂恿 reset**——卡片对 credit 只做安静标注,reset 入口低调待在 `⋯` 菜单。

用户确认的展示方向：

- credit 数量安静标注在账号副标题行尾,仅 `available_count > 0` 时展示,带 hover 说明它不是余额/token;
- `Reset usage limit` 进账号 `⋯` 菜单,与 `Delete` 平级,**无 credit 或无活动限额时灰掉禁用**(非隐藏);
- 二次确认弹窗说明"重置 eligible 限额 + 消费 1 credit"。

## Goals / Non-Goals

**Goals:**

- 读取 Codex reset credit 数量并安静展示在对应 account card。
- 让用户能从账号 `⋯` 菜单消费 1 个 reset credit(灰态门控保证仅在可 reset 时可点)。
- reset 前二次确认，reset 后刷新该账号 quota。
- 所有 token、auth、raw response 继续留在 Rust/backend 边界内。
- DTO additive 演进，旧 Swift decode 不因缺字段失败。

**Non-Goals:**

- 不提供强制清零 weekly usage 的能力。
- 不做 provider-level 或 overview-level Reset。
- 不做自动 reset。
- 限额卡住时不主动浮现 reset 提示/横幅(reset 是低频菜单动作,不抢切号的戏)。
- 本期不展示 credit 过期时间(后续 additive 扩展)。
- 不给 Claude/Gemini 硬套 Codex reset credit 模型。
- 不把 reset credit 当 token usage、billing balance 或 workspace credit balance 展示。

## Decisions

### 1. Reset credit 挂在 quota DTO 下

模型：

```rust
UsageSnapshot {
    limits: Vec<UsageLimit>,
    reset_credits: Option<UsageResetCredits>,   // 新增可选字段，additive
}

UsageResetCredits {
    available_count: u32,                       // 来自 rate_limit_reset_credits.available_count
}

MenubarQuota {
    windows: Vec<MenubarQuotaWindow>,
    reset_credits: Option<MenubarResetCredits>,
}

MenubarResetCredits {
    available_count: u32,
}
```

解析来源(已对照官方 Codex 调用):usage payload 的 `rate_limit_reset_credits.available_count`,与现有 `parse_codex_usage_snapshot`(`plugin.rs:2222`)同一个 GET `/wham/usage` 响应。`available_count` 可能是 int 或十进制 string,解析两者皆取,缺失/非法时整个 `reset_credits` 保持 `None`(不写 0)。

理由：credit 来自同一个 remote usage payload，展示位置也依附 account quota。独立建 `AccountCreditState` 会让状态源变多，但没有收益。

替代方案：把 credit 做成 `UsageLimitKind::CreditBalance`。暂不采用，因为 reset credit 是动作次数，不是剩余额度窗口；混进 window list 会让 UI 把它当进度条展示。

**本期不建模 credit 过期时间**:官方 reset credit 带有效期(社区脚本会显示 expiry countdown),但本期只读 `available_count`。`UsageResetCredits` 留作可扩展结构,后续要展示倒计时再 additive 加 `expires_at_unix: Option<i64>`。这是显式取舍,不是遗漏。

### 2. credit 标注放进现有 identity 副标题行

定位:本 app 本质是**账号切换器**。用户有多个号,限额卡住时优先**切号**,reset 是低频、慎重、稀缺(credit 少且会过期)的动作。所以卡片对 credit 只做一件事——**安静标个数量**;reset 入口低调待在 `⋯` 菜单。**限额卡住不触发任何 reset 提示**,卡住就切号。

落点(对照 `TargetRows.swift`):标注塞进现有 `TargetIdentity` 的副标题行(`metaText`,即 `Pro · 06-28 11:20`)**尾部**,不新增行、不改卡片高度——和现有把 plan+refreshed 折成一行"避免错位 meta 行"的做法一致。

```text
●  #1 work                              [Use] ⋯
   Pro · 06-28 11:20    ↺ 2 credits
   5h   100%                    2026-06-28 16:20
   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   7d    42%                    2026-07-02 09:00
   ━━━━━━━━━━━━━━━
```

标注样式(贴合现有 token):

| 属性 | 值 | 依据 |
|---|---|---|
| 字号 | `.caption2` | 与 subtitle / diagnostic 同级 |
| 颜色 | `.secondary` | 纯信息,不抢眼 |
| 图标 | `arrow.counterclockwise.circle` | 语义 = reset |
| 文案 | `N credits`(单数 `1 credit`) | 短,行尾空间紧 |
| 显示条件 | `available_count > 0` 才显示;`0`/缺失不占位 | |
| hover/help | `Codex reset credits: each can be consumed once to reset eligible usage limits for this account. Not a token or billing balance.` | 防混淆 |

窄宽度降级:若 `Pro · 06-28 11:20` 已挤满,credit 单独成行(像 diagnostic 那样),默认走同行。

理由：主任务是识别和切换账号；credit 是附属提示,不该比限额/账号身份更显眼。

### 3. reset 菜单项(灰态门控)+ 确认弹窗

reset 项加进现有 `overflowMenu`(目前只有 Delete),置于 Delete **之上**,中间一条 `Divider`:

```text
        ⋯ ┌──────────────────────────┐
          │ ↺  Reset usage limit      │  ← 仅 credit>0 且有活动限额 才启用
          ├──────────────────────────┤
          │ 🗑  Delete                 │
          └──────────────────────────┘
```

**启用条件 = `available_count > 0` 且 本地有活动限额(任一 window limited/exhausted)。** 这是"方向 B":前置门控保证只有真能 reset 时菜单项才可点,从源头杜绝 `nothing_to_reset` 的无效消费惊吓。

灰态(**禁用而非隐藏**,让用户知道有此功能、只是当前不可用),按原因给 help 文案:

| 状态 | 菜单项 | help / disabledReason |
|---|---|---|
| credit>0 且有活动限额 | 启用 | `Consume 1 reset credit to reset eligible usage limits` |
| `available_count == 0` 或缺失 | 灰掉 | `No reset credits available` |
| credit>0 但无活动限额 | 灰掉 | `No active limit to reset` |
| reset in-flight | 灰掉(Reset/Delete/Use 同账号全禁) | overflow 图标转 `hourglass`(对照现有 deleting 态) |

确认弹窗(复用现有 `DeleteConfirmPopover` 同款 popover 形态,`arrowEdge: .trailing`):

```text
Reset eligible usage limits?
Consumes 1 reset credit for #1 work.

                          Cancel   Reset
```

文案走"方向 B":既说**重置范围**(eligible usage limits,账号级、可能 >1 窗口),又说**代价**(1 credit)。因为前置门控已保证有活动限额,这里不会再出现"点完没扣"的矛盾。

理由：reset 消费不可本地回滚;二次确认 + 灰态门控比把按钮放卡片主区更稳。reset 永远在菜单、不随限额状态变显眼,符合"非必要不 reset"的低频定位。

### 4. Consume operation:分层、封装与调用链

consume 必须沿用现有账号操作的分层(和 `use_target` / `remove_target` 同形),不能从 omx-app 直接发 HTTP。完整调用链:

```text
Swift BackendRequest.consumeResetCredit
  → FFI dispatch op "consume_reset_credit"        (omx-menubar-ffi/src/lib.rs)
    → omx_app::consume_reset_credit(...)          (omx-app/src/mutation.rs)
      → PlatformPlugin::consume_reset_credit(...)  (omx-core/src/plugin.rs, trait 新增方法)
        → Codex plugin HTTP POST                   (omx-plugin-codex/src/plugin.rs)
```

**(a) Plugin trait 新增方法(omx-core)**。照抄 `remove_target` 的"默认 unsupported"模式,使 Claude/Gemini plugin 无需改动即可编译:

```rust
// omx-core/src/plugin.rs
pub enum ResetCreditOutcome {
    Reset { windows_reset: u32 },
    NothingToReset,
    NoCredit,
    AlreadyRedeemed,
}

fn consume_reset_credit(
    &self,
    _selector: &str,
    _idempotency_key: &str,
) -> Result<ResetCreditOutcome> {
    Err(OpenMuxError::Message(format!(
        "{} does not support reset credits",
        self.name()
    )))
}
```

注意 `ResetCreditOutcome` 是 core 层类型,只表达"业务成功结果"。网络/auth/HTTP/schema 失败走 `Result::Err`,**不在 enum 里**——`failed` 不是 provider 返回值,是 omx-app 把 `Err` 归类后合成的 `MenubarOperationStatus::Failed`。

**(b) Codex plugin HTTP 请求(omx-plugin-codex)**。复用 Decision 6 抽出的鉴权 helper:

```text
POST https://chatgpt.com/backend-api/wham/rate-limit-reset-credits/consume
headers: Authorization / ChatGPT-Account-Id / User-Agent: codex-cli (+ X-OpenAI-Fedramp 见 Decision 6)
body:    { "redeem_request_id": "<idempotency_key>" }
```

响应字段是 **`code`**(不是 "outcome"),成功时附 `windows_reset`:

| 响应 `code` | 映射到 `ResetCreditOutcome` |
|---|---|
| `reset` | `Reset { windows_reset }`(读 `windows_reset`,缺失记 0) |
| `nothing_to_reset` | `NothingToReset`(未扣 credit) |
| `no_credit` | `NoCredit` |
| `already_redeemed` | `AlreadyRedeemed` |
| 其它/无法解析 | `Err(schema)` → 上层 `Failed` |

**(c) omx-app 编排(mutation.rs)**。新增 `consume_reset_credit`,结构对照 `menubar_remove`:取 `OPERATION_LOCK` → `find_plugin` → 调 `plugin.consume_reset_credit` → 成功后刷新该账号 quota(Decision 5)→ 组 report。`Err` 路径走 `sanitize_diagnostic`,产出 `Failed` operation + 脱敏 diagnostic。

```rust
// omx-app/src/dto.rs
MenubarConsumeResetCreditCommand {
    provider: String,
    local_id: String,
    idempotency_key: String,                 // 非空,Swift 生成 UUID
    #[serde(default)] target_kind: Option<MenubarTargetKind>,
}

MenubarConsumeResetCreditReport {
    control_plane_schema_version: u32,
    state_schema_version: u32,
    generated_at_unix: u64,
    provider: String,
    requested_local_id: String,
    operation: MenubarOperationResult,       // status: Success/Failed
    outcome: Option<MenubarResetCreditOutcome>, // 业务结果,Failed 时为 None
    dashboard: MenubarDashboardReport,
    accounts: MenubarAccountsReport,
}

// snake_case enum,与现有 DTO 风格一致
MenubarResetCreditOutcome {
    Reset { windows_reset: u32 },
    NothingToReset,
    NoCredit,
    AlreadyRedeemed,
}
```

**(d) FFI(omx-menubar-ffi)**。`dispatch` 增加 `"consume_reset_credit"` arm,照 `"remove"` 写:`payload::<MenubarConsumeResetCreditCommand>` → `consume_reset_credit(...)` → `json_value`。panic-safe、脱敏沿用现有 `sanitize`。

**(e) Swift(omx-menubar)**。`BackendRequest` 增加 `case consumeResetCredit(provider:targetKind:localId:idempotencyKey:)`,encode 出 `idempotency_key`(Swift 侧 `UUID().uuidString`);`BackendData` 增加 outcome 解码。

理由:consume 是账号级 mutating 操作,和 remove/switch 同类,必须共用 trait + OPERATION_LOCK + dashboard 刷新这套既有骨架,而不是另起炉灶。

并发:`OPERATION_LOCK`(`mutation.rs:15`)已让所有 menubar 操作全局串行,后端层面**不存在多账号并发 reset**。idempotency key 的真正作用是**同一次 UI attempt 网络失败重试时不重复扣**(POST 已送达但响应丢失的场景);Swift 在一次 attempt 内复用同一 UUID,换 attempt 才换 key。

### 5. Consume 后刷新账号 quota

consume 返回业务 outcome 后，后端应尽量刷新该账号 usage/quota，并返回 dashboard。刷新失败时仍返回 consume outcome，同时让账号卡片沿用 last-good quota + 当前 diagnostic。

理由：用户触发 reset 后最关心卡片上的 `5h/7d` 和 credit 数是否变化；让前端再拼第二个 refresh 会引入竞态。

### 6. 抽出 Codex 鉴权请求 helper

现有 `fetch_codex_usage`(`plugin.rs:502`)把 curl-config 构建(`Authorization` / `ChatGPT-Account-Id` / `User-Agent`)+ 执行 + HTTP 状态解析内联在一处,只发 GET。consume 要发同样鉴权头的 POST,所以把这段抽成:

```rust
fn codex_backend_request(
    &self,
    auth: &CodexUsageAuth,
    method: &str,                 // "GET" | "POST"
    url: &str,
    body: Option<&[u8]>,
) -> std::result::Result<serde_json::Value, UsageDiagnostic>
```

usage 走 `("GET", ".../wham/usage", None)`,consume 走 `("POST", ".../consume", Some(body))`(curl `--request POST --data @file` 或 config 内 `data`)。

**fedramp 头**:官方对 fedramp 账号会带 `X-OpenAI-Fedramp: true`。现有 `CodexUsageAuth` 只存 `access_token` + `account_id`,需在 `parse_codex_usage_auth`(`plugin.rs:2173`)从 id_token/access_token 的 `https://api.openai.com/auth.chatgpt_account_is_fedramp` claim 解出该 flag,helper 按需追加这个 header。usage 和 consume 共用,顺带修了现有 usage 在 fedramp 账号下可能缺头的问题。

理由:一处构建鉴权头,GET/POST 共用,避免 consume 复制粘贴一份 curl-config 逻辑导致两边漂移。

## Risks / Trade-offs

- 私有 endpoint 结构变化 → 解析失败时保持 credit 字段缺失，UI 不展示 reset，diagnostic 脱敏。
- 用户误以为 credit 是余额 → 标注 hover 必须明确“consume once to reset eligible usage limits”，不写成 `credits` 裸词。
- 用户误触 reset → Reset 不在卡片主按钮区，必须通过菜单和确认弹窗。
- Consume 成功但刷新失败 → operation result 展示 reset outcome，quota 区标 stale/error，不伪造新 quota。
- 重复提交同一 attempt → 后端 `OPERATION_LOCK` 已全局串行化操作;同一 attempt 的网络重试复用同一 idempotency key,POST 重达时后端按 `already_redeemed` 返回,不重复扣 credit。
- `nothing_to_reset` 未扣 credit → 确认弹窗不能写死"将消费 1 个 credit";结果提示必须明说没有消费 credit(见 spec 对应 scenario)。

## Migration Plan

1. 后端 additive 增加 reset credit 字段；旧数据缺字段时等价于 unknown。
2. Swift DTO 字段设为 optional；缺失时不展示 credit 标注和 Reset 菜单项。
3. 增加 consume operation 后再接 UI 菜单；未实现 consume 时前端不展示 Reset。
4. 若 endpoint 不可用或回滚，移除/隐藏 Reset operation 即可，不影响已有 quota 展示。
