## Context

OpenMux 的共享后端不是 Menubar backend，而是 `omx-app` 承载的 control-plane。当前代码已经部分如此：Menubar FFI 调 `dashboard_view`，CLI 的 `list/status` 也开始消费 `dashboard_view`。问题在于聚合口径仍散在 Swift、CLI 和 control-plane 内部 helper 中。

本设计把本次变更的目标限定为：重新划清领域职责，并把 Overview/aggregate 所需业务口径收敛到 control-plane。Presentation surfaces 只展示，core 只提供事实。

## Layer Responsibility Matrix

| 层 | MUST | MUST NOT | 本变更判断 |
| --- | --- | --- | --- |
| `omx-core` | 领域模型、最小事实、持久化查询、安全基础操作、不含 product policy 的纯 helper | 依赖 `omx-app` DTO、接收 `MenubarAccount`、Overview 语义、UI headline、quota 风险阈值、最佳备选、display tone、预格式化金额字符串 | 保留 `UsageSnapshot` / `UsageLimit` / `UsageSummaryQuery` / `TargetCatalog`；不新增 Menubar/Overview 专属模型 |
| provider plugin | provider-specific 路径、auth、refresh、switch、profile patch、quota response 解析 | 跨 provider 聚合、产品总览、surface DTO、业务推荐 | 继续输出 `AccountStatus` / `ConfigProfile` / refreshed usage facts |
| `omx-app` control-plane | dashboard/provider view、quota health、usage headline、action eligibility、operation result、safe diagnostics、surface-agnostic projection | provider 私有文件读写、auth payload 解析、AppKit/terminal layout、把展示文案当成唯一语义字段 | 本变更新增共享聚合 projection |
| transport | FFI / future HTTP / machine JSON envelope、schema gate、encode/decode | status、quota、usage、refresh eligibility 判断 | `omx-menubar-ffi` 不改业务；schema 变更必须走版本/fixture/contract 测试 |
| frontend | loading/pending/stale UI state、layout、component、terminal/table rendering、platform interaction | quota risk、provider health、business headline、action eligibility、用 message 文本反推 provider | 删除 Swift/CLI 重复聚合，只保留展示 |

依赖方向固定为 `omx-core <- provider plugin <- omx-app <- transport <- frontend` 的消费链。`omx-core` 可以被 `omx-app` 使用，但不得引用 `omx-app` 类型；任何声明放在 core 的函数，签名都必须只吃 core/domain 类型。

## Data Ownership

### Core Facts

`omx-core` 是事实层。它可以暴露这些最小单元：

- account/profile facts: `AccountStatus`、`ConfigProfile`、`AccountRef`、`TargetCatalog`
- quota facts: `UsageSnapshot`、`UsageLimit`、`UsageResetCredits`、`UsageDiagnostic`
- usage facts: `UsageEvent`、`UsageSummary`、`UsageSummaryQuery`、`StateStore::usage_summaries_by`
- operation safety primitives: atomic write、snapshot hash、target resolution、plugin trait methods

`core` 可以有纯 helper，但 helper 必须满足三个条件：

- 输入输出是领域事实，不是 surface DTO。
- 不包含产品策略，例如 low/exhausted 阈值、best alternative、Overview 告警优先级。
- 输出保持机器可消费的事实值，例如 cost 使用数值/decimal-compatible 表达；`"$1.23"` 这类预格式化字符串只能出现在 renderer 或明确的 display projection。

### Control-plane Projections

`omx-app` 负责把 facts 组合成共享业务 projection：

- `QuotaHealthRollup`: account_count、reporting_count、avg_remaining、health counts、worst target、soonest reset、reset credit total
- `TargetRecommendation`: best alternative target、推荐原因、可执行 action（control-plane policy，依赖 action eligibility）
- `UsageHeadline`: period、total tokens、estimated cost、cost status、top model、breakdown
- `ProviderAggregateView`: provider target counts、active target、quota health、status/tone、diagnostics
- `DashboardAggregateView`: global quota health、provider aggregates、usage headline、aggregate diagnostics

这些 projection 是 CLI、Menubar、future Desktop 的共享输入。不同 presentation surface 可以有不同 layout，但不能改变字段语义。

### Frontend Rendering

Presentation surface 只做：

- Swift state: loading、refreshing、switching、failed with last-good、selection
- Swift component/layout/token/icon
- CLI table columns、JSON field ordering、terminal colors
- future Desktop navigation and interaction

Presentation surface 不做：

- 不按 raw quota 自算 red/yellow/green
- 不按 provider/account list 自算 provider health
- 不自定义 best alternative
- 不把 missing cost 显示成 `$0.00`
- 不从 auth、SQLite、usage logs 或 provider endpoint 读业务事实

## Decisions

### 1. 全量重命名为 surface-agnostic control-plane 命名，不保留 Menubar 别名

借本次重构一次性把共享类型从 `Menubar*` 重命名为 surface-agnostic control-plane 命名，**不保留旧名、不加别名、不做兼容垫片**。CLI 和 Menubar 同步改用新名。

```rust
pub struct DashboardReport { ... }       // was MenubarDashboardReport
pub struct UsageSummaryView { ... }      // was MenubarUsageSummary
pub struct QuotaHealthRollup { ... }
pub struct UsageHeadline { ... }
pub struct ProviderAggregateView { ... }
pub struct DashboardAggregateView { ... }
```

新挂载字段一律中性，例如 `quota_health`、`usage_headline`、`provider_aggregates`。这是一次性破坏式重命名，通过 schema version bump + 全量更新 fixtures/contract 测试兜住，而不是长期维护双名。

### 2. Quota 折叠分两层：事实折叠与产品判断分离

事实折叠是中性 helper，放 `omx-app`，签名只吃 core/domain 类型（`&[AccountStatus]` 或等价 facts 切片），**永不接收 app DTO**。

```rust
// omx-app: 中性事实折叠，签名只吃 core facts。
pub struct QuotaFactsRollup {
    pub account_count: u32,
    pub reporting_count: u32,
    pub avg_remaining_percent_x100: Option<u32>,
    pub min_remaining_percent_x100: Option<u32>,
    pub max_remaining_percent_x100: Option<u32>,
    pub soonest_reset_at_unix: Option<i64>,
    pub reset_credit_total: u32,
}

fn quota_facts_rollup(accounts: &[AccountStatus]) -> QuotaFactsRollup;
```

产品判断必须在 control-plane：

- healthy / low / exhausted 分类
- provider status/tone
- worst target 展示优先级
- best alternative
- reset escape hatch 文案与 action

这样 core 不需要知道 Overview，也不会把产品阈值固化成领域事实。

禁止形态：

- `core::quota_rollup(accounts: &[MenubarAccount])`
- 一个 `quota_rollup` 同时返回 avg/min/max facts、health buckets、status text、best alternative
- `core` 里的 `best_alternative` 调用或复制 `can_activate`

### 3. 全局平均必须折叠原始 reporting 账号

`avg(avg(A), avg(B))` 不等于 `avg(A ∪ B)`。全局 quota average 必须从全部 reporting accounts 一次性折叠，不能对 provider 平均再平均。

可结合字段（count、reset credit total、soonest reset）可以从子聚合合并，但实现上优先对同一批原始 facts 调同一个 helper，避免误用。

### 4. Usage 查询事实来自 StateStore，headline 来自 control-plane

`StateStore::usage_summaries_by` 继续作为最小 usage 查询能力。control-plane 负责调用它生成：

- hourly bucket projection
- model/provider series projection
- selected period headline
- cost status aggregation

hourly projection 可以增加 `estimated_cost_usd` 与 `cost_status`，但 scan/ingest/schema 不在本变更中重做。

hourly atom 中的 cost 字段不得是展示字符串。可接受形态是 `estimated_cost_usd: Option<f64>`、minor unit integer，或后续统一的 decimal wrapper；展示格式化属于 frontend/renderer。

### 5. Period folding 由 control-plane 统一提供

为保证 CLI、Menubar、future Desktop 同口径，period headline 不应由每个 frontend 自己折叠。Frontend 可以在本地做纯视觉 rollup（例如把 hourly bars 合并成 day bars），但 headline、cost、top model 应来自 control-plane projection。

### 6. Provider grouping 是 control-plane 原子

`group_targets_by_provider(accounts, profiles)` 放在 `omx-app`。它服务于 provider aggregate、active count、quota health、diagnostics scope。Presentation surface 不再按 provider 重复 filter 来决定业务状态；仅可为局部渲染选择已分组的 rows。

### 7. 本次必须迁走的错误逻辑

从 Swift 迁走：

- `DashboardView.lowestQuota`
- `DashboardView.lowestQuotaSummary`
- `OverviewProviderRow.quotaColor`
- provider health / alert count 推断
- 用 raw accounts/profiles filter 得出业务聚合

从 CLI 迁走：

- `menubar_overall_availability`
- `menubar_window_availability`
- `usage_groups` / `usage_total` 中与 period headline、cost total、top model 重复的业务折叠
- 与 provider quota health 重复的 status 推断
- 与 Menubar 不同源的 usage headline 聚合

从 control-plane/app DTO 中重新分类：

- `status_text("OK"/"Alert")` 不能作为唯一语义；必须先有 machine semantic status/tone，display text 只能是派生 display 字段。
- `provider_display_label` 如果保留，必须标注为 display projection；provider identity/lookup 不能依赖它。
- diagnostics 不得通过 `message.contains(provider)` 关联 provider；诊断产生时必须携带 `provider_id`、`target_id` 或明确的 scope。

保留在 frontend：

- 表格列选择
- 卡片布局
- icon/color token 映射到 semantic tone
- pending/loading state

## Composition Map

```text
provider plugin ─┐
                 ├─ core facts ───────────────┐
StateStore ──────┘                             │
                                               v
                                  omx-app control-plane
                         facts -> shared aggregate projection
                                               │
                     ┌─────────────────────────┼─────────────────────────┐
                     v                         v                         v
                    CLI                     Menubar                future Desktop
              terminal renderer        native renderer             GUI renderer
```

## Risks / Trade-offs

- **一次性破坏式重命名**：`Menubar*` → 中性名是全量改动，会同时触及 Rust DTO、FFI op、Swift 解码和 fixtures。通过 schema version bump + 全量更新 contract 测试一次兜住，换来长期没有双名负担。这是有意承担的一次性成本，不是持续兼容负债。
- **control-plane 会变重**：这是正确重量。产品策略需要一个地方承载；比散在 CLI/Swift 更便宜。
- **frontend 仍可能需要局部 filter 渲染 rows**：允许，但只能用于选择已知数据，不得由此重新计算业务健康或 action eligibility。

## Migration

不做兼容双轨。一次性切换：

1. 在 `omx-app` 落地中性 projection 与重命名后的 DTO，删除旧 `Menubar*` 名。
2. 同步改 FFI op/类型名、CLI 和 Menubar 的消费点，删除前端/CLI 的重复聚合 helper。
3. bump `CONTROL_PLANE_SCHEMA_VERSION`，全量重写 fixtures，更新 CLI/Menubar contract 测试断言新版本与同口径聚合值。
4. 一次提交内保持可编译、可测试；不引入"旧名仍可用"的过渡期。

## Open Questions

- 无（trend/环比已移出本次 scope，作为独立业务功能另行评估）。
