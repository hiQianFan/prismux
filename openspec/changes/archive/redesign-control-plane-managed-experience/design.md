## Context

OpenMux 当前 Rust 核心已经具备账号 snapshot、profile import、SQLite state、usage summary、atomic replacement、backup、rollback 和 safe diagnostics。问题在产品架构层：`omx-cli`、`omx-app`、`omx-menubar-ffi`、Swift Menubar 的边界还停留在 v1 能用状态，Menubar UI 需要自己拼很多业务语义，导致状态弱、视觉不一致、操作体验不够无缝。

CodexBar 的可借鉴点不是 provider 数量，而是产品后端与前端体验的分层：core 能力集中、app store 编排状态、Menubar 是一等体验、CLI 独立复用核心能力、账号刷新可按账号隔离。OpenMux 应该保留 Rust 安全模型，同时把 `omx-app` 升级为 `omx-control-plane`，让所有 frontend 消费同一套产品级 view model。

CodexBar 还暴露了几个 OpenMux 必须提前固化的长期约束：provider refresh 需要 coalescing/generation/backoff，settings 需要统一来源，账号 reconciliation 不能只靠 email，source strategy 必须显式，诊断必须结构化和脱敏，CLI/Menubar 独立分发必须有 schema compatibility gate。

本设计采用“学习 CodexBar 的模式，不复制 CodexBar 的重量”的原则。CodexBar 已经证明 Menubar 产品需要稳定 refresh runtime、shared config、account reconciliation、last-good cache 和 redaction；OpenMux 应把这些变成 Rust control-plane 与 Swift frontend 的 contract，而不是把 CodexBar 的大型 Swift `UsageStore`、WebKit extras、Sparkle、WidgetKit 或 provider registry 直接搬过来。

## Goals / Non-Goals

**Goals:**

- 建立长期三层架构：`omx-core`、`omx-control-plane`、frontends。
- 让 CLI、Menubar、future desktop/widget/http 共享同一业务事实、状态语义、错误口径和 operation result。
- 支持 provider 持续扩展：新增 provider 只接入 core/control-plane contract，不为某个 frontend 写孤立特例。
- 将 Menubar 作为独立原生前端建设，具备稳定 state machine、组件体系、视觉 token 和无缝操作体验。
- 建立 managed/account-scoped refresh 架构方向：账号可隔离刷新，系统 active 只在用户显式激活时改变。
- 建立 provider runtime 生命周期：source strategy、refresh coalescing、timeout、cancellation、backoff、last-good cache、generation guard。
- 建立 shared settings/config：provider enablement、source mode、refresh cadence、display preference、debug/recovery switches 和 feature flags 使用统一语义。
- 建立结构化 diagnostics/logging：用户文案、recovery action、debug detail、support report 和 redaction policy 分层。
- 保持安全边界：auth、SQLite、usage logs、provider endpoint 和敏感诊断只能由 Rust 后端处理。
- 为未来 CLI/Menubar 独立分发奠定 contract 基础。

**Non-Goals:**

- 本 change 不要求一次性实现几十个 provider。
- 本 change 不引入 WebKit scraping、Sparkle、WidgetKit 或自动更新作为第一阶段必需项。
- 本 change 不要求废弃现有 CLI；CLI 仍是高级管理、脚本和诊断入口。
- 本 change 不允许为了 Menubar 体验把 token/auth payload 暴露给 Swift。

## Decisions

### Decision 1: `omx-app` 演进为 `omx-control-plane`

`omx-app` SHALL 从 Menubar 专用 application helper 演进为产品级 application service。它对外输出：

- dashboard view model
- provider page view model
- target/action view model
- operation result
- action eligibility
- stale/freshness/error diagnostics
- CLI/Menubar 共享文案和状态等级

备选方案是让 CLI 和 Menubar 各自消费 `omx-core`。这会造成业务解释重复，长期每加一个 provider 都要同步多个 frontend。control-plane 是更稳的地基。

Control-plane 内部应分三层模型，避免自己变成新的大杂烩：

```text
provider domain records
  -> application operation/state model
  -> frontend-safe view model
```

Rust 输出语义，不输出 Swift 布局；Swift 输出布局，不重新解释业务事实。

Recommended Rust module split:

```text
crates/omx-app/src/
  lib.rs                  // public re-exports only
  api.rs                  // provider-agnostic public functions
  compatibility.rs        // schema/state/frontend/backend compatibility
  query/
    dashboard.rs          // dashboard_view
    provider.rs           // provider_view
    settings.rs           // settings_view
  mutation/
    activate.rs           // activate_target
    refresh.rs            // refresh_provider / refresh_all
    remove.rs             // remove_target
    settings.rs           // update_settings
  runtime/
    coordinator.rs        // single-flight/generation/backoff
    source_strategy.rs    // source priority/eligibility/confidence
    cache.rs              // last-good safe snapshots
  mapper/
    provider.rs           // domain -> application provider state
    target.rs             // account/profile -> target state
    view.rs               // application state -> frontend-safe view
  diagnostics/
    model.rs              // structured diagnostics
    redaction.rs          // shared redactor
    support_report.rs     // safe support bundle
```

Minimum public API shape:

```rust
pub fn dashboard_view(query: DashboardQuery) -> Result<DashboardView>;
pub fn provider_view(query: ProviderQuery) -> Result<ProviderView>;
pub fn refresh_provider(command: RefreshProviderCommand) -> Result<OperationEnvelope<ProviderView>>;
pub fn refresh_all(command: RefreshAllCommand) -> Result<OperationEnvelope<DashboardView>>;
pub fn activate_target(command: ActivateTargetCommand) -> Result<OperationEnvelope<ProviderView>>;
pub fn remove_target(command: RemoveTargetCommand) -> Result<OperationEnvelope<ProviderView>>;
pub fn settings_view() -> Result<SettingsView>;
pub fn update_settings(command: UpdateSettingsCommand) -> Result<OperationEnvelope<SettingsView>>;
pub fn compatibility_view(client: ClientDescriptor) -> CompatibilityResult;
pub fn support_report(command: SupportReportCommand) -> Result<SupportReport>;
```

These functions are intentionally provider-agnostic. Codex/Claude/Gemini specifics belong in plugins or provider adapters.

### Decision 1.1: CodexBar 参考原则 SHALL be explicit

OpenMux SHALL explicitly copy CodexBar's proven design patterns only where they strengthen OpenMux's core product experience:

- `CodexBarCore` 的 fetch/parse/provider 集中思路 → OpenMux 使用 `omx-core + omx-control-plane` 集中 provider/domain/application 语义。
- `UsageStore` 的状态编排职责 → OpenMux 拆为 Rust control-plane runtime state + Swift Menubar UI state，避免 Swift 承担 provider 业务逻辑。
- `ProviderRefreshCoordinator` 的 generation/coalescing/cancellation → OpenMux provider runtime coordinator 必须具备 request identity、single-flight、replacement、timeout/backoff。
- `CodexAccountReconciliationSnapshot` 的 live system/managed/profile-home 区分 → OpenMux 明确 `system_active_target`、`selected_ui_target`、`refresh_scope_target`、`observed_target`。
- `SettingsStore` 的 shared config 经验 → OpenMux 区分 shared provider config 与 frontend-local UI preferences。
- `LogRedactor` 的统一脱敏 → OpenMux diagnostics/logging/support report 必须统一 redaction。
- CodexBar App/CLI 分发模式 → OpenMux CLI/Menubar 可独立分发，但必须通过 schema compatibility gate 共享状态和语义。

OpenMux SHALL NOT copy CodexBar features that are outside current product strategy without a separate product decision:

- WebKit scraping 和 browser cookie import。
- Sparkle 自动更新。
- WidgetKit。
- 大型 Swift provider registry。
- 让 Swift 前端直接持有 provider credential 逻辑。
- 为未来 provider 预置复杂 UI 特例。

这个决策是后续实现的约束：当“快速复制 CodexBar 代码”与“保持 OpenMux control-plane 边界”冲突时，后者优先。

### Decision 1.2: Phase 1 先打地基，不追求大而全

Phase 1 SHALL 优先完成当前产品体验和底层架构最相关的地基，不把长期能力伪装成第一阶段必做项。第一阶段的核心判断标准是：是否让现有 CLI/Menubar 的账号管理体验更稳定、更一致、更可测试；是否减少 Swift 前端业务推断；是否为后续 managed account、独立分发和多 provider 扩展留下正确接口。

Phase 1 固定选择：

- 保留 `crates/omx-app` crate name，先完成内部 control-plane 模块化；代码层重命名为 `omx-control-plane` 后置，避免无价值 churn。
- Menubar backend 仍采用 embedded Rust/FFI 路径；不在 Phase 1 引入 helper daemon、HTTP server 或 installed CLI runtime 依赖。
- Managed `CODEX_HOME` 不作为 Phase 1 实现目标；Phase 1 只把 `system_active_target`、`selected_ui_target`、`refresh_scope_target`、`observed_target` 放进 model/API，避免后续账号隔离体验被当前 snapshot 模型锁死。
- Shared settings 先定义 schema 和边界，只迁移影响当前体验的 provider enablement、refresh cadence、display preference；不一次性改完所有配置。
- 首批 public API 只要求 `dashboard_view`、`provider_view`、`refresh_provider`、`activate_target`、`compatibility_view` 可实现、可测试、可供 CLI/Menubar 消费。
- `refresh_all`、`remove_target`、`settings_view`、`update_settings`、`support_report` 可以先完成 contract 和 DTO，不作为 Phase 1 行为验收的阻塞项。
- Swift Phase 1 是结构化迁移和组件地基，不是重新设计整套视觉；先让当前 dashboard、provider list、account row、status/error/action 的状态归属正确。
- 前端组件必须从第一阶段开始使用 typed props/callbacks；不得先拆文件但继续让子组件读全局 store、拼业务状态或直接调 backend。
- 当前用户可见能力不得倒退：现有账号列表、激活、刷新、错误展示、backend unavailable fallback、CLI machine output 必须保持可用。

这些选择让长期设计保留方向，但第一阶段验收聚焦最能改善用户体验和可维护性的部分。

### Decision 1.3: Phase 1 与 Phase 3 的职责边界 SHALL be explicit

组件拆分（建文件、定 props/tokens）与 monolith 替换（让 live view 真正消费组件、移除前端业务推断）是**两件不同的工作**，必须分给不同阶段，否则会出现「文件已建但 live UI 仍是 monolith」的半成品状态——这正是首次 Phase 1 实施暴露的问题。

边界固定如下：

**Phase 1 拥有（组件地基 + 纵向验证）：**

- 建立 `Backend/State/Design/Components/Features/Shell` 目录与 typed props/callbacks 定义。
- 建立 design tokens 与 primitive/shared/target 组件文件。
- **至少一个纵向切片落地**：把 `TargetRow`（及其 props）真正接入当前账号列表的一处渲染，证明组件契约可用、props 与真实数据匹配，避免组件骨架建好却无人消费、契约与需求脱节。
- Rust 侧 view model 映射（provider/target/action/status/quota health）已可用，CLI 已消费这些字段。

**Phase 3 拥有（Menubar UX rebuild）：**

- 用 shell/screen/section/shared/primitive 组件**整体替换** `DashboardView` monolith，使其行数显著下降并全面引用新组件。
- **移除 Swift 前端业务推断**：`status` 字符串比较、`quotaColor()`、alert 计数、active marker 等全部改为消费 control-plane view model 字段（对应原 4.9）。
- 页面切换状态、provider page、settings menu、视觉一致性、accessibility/reduced-motion、snapshot/smoke 覆盖。

判定「组件拆分完成」以 **live 引用 + monolith 缩小** 为准，而非「新增文件数」。Phase 1 不要求 monolith 被整体替换，但要求纵向切片证明骨架可用；Phase 3 才负责把 monolith 收掉。Rust 模块拆分（`api.rs` → `query/mutation/runtime/mapper`）属于无 UX 耦合的地基整理，仍归 Phase 1 收尾，与本边界无关。

### Decision 2: `omx-core` 只保留领域与安全操作

`omx-core` SHALL 保留领域对象、StateStore、plugin trait、安全文件操作、usage schema 和错误类型。它不应知道 Menubar 布局、CLI 表格或 Swift 组件。这样 core 可测试、可复用，也不会被某个 frontend 的展示字段污染。

### Decision 3: Frontend 只负责交互和呈现

`omx-cli`、`omx-menubar` 和 future frontend SHALL 消费 control-plane view model。Frontend MAY 有自己的状态机，例如 Swift Menubar 的 loading、stale、switching、refreshing、last-good、selection state，但 MUST NOT 重新解释 provider health、quota、action eligibility 或 auth safety。

### Decision 4: Menubar 需要自己的 Swift 架构，而不是单文件 Dashboard

Swift Menubar SHALL 拆分为 store/state machine、DTO/client、design tokens、基础组件、feature views 和 shell controller。`DashboardView` 这种集中承载布局、业务判断和样式的形态不可持续。更优结构：

```text
OmxMenubarCore
  Backend/
  State/
  Design/
  Components/
  Features/Dashboard/
  Shell/
```

这不是为了抽象而抽象，而是让 UI 一致性和交互状态可测试。

Recommended Swift structure:

```text
apps/omx-menubar/Sources/OmxMenubarCore/
  Backend/
    BackendClient.swift
    BackendEnvelope.swift
    CompatibilityClient.swift
    DTO/
      DashboardDTO.swift
      ProviderDTO.swift
      OperationDTO.swift
      SettingsDTO.swift

  State/
    MenubarStore.swift
    MenubarState.swift
    SelectionState.swift
    OperationState.swift
    SettingsState.swift

  Design/
    Tokens.swift
    ColorRoles.swift
    Typography.swift
    Motion.swift

  Components/
    Primitive/
      IconButton.swift
      CommandButton.swift
      MeterRing.swift
      Badge.swift
    Shared/
      StatusBanner.swift
      EmptyStateView.swift
      DiagnosticView.swift
      SectionHeader.swift
      ProviderBadge.swift
    Target/
      TargetRow.swift
      TargetIdentityView.swift
      TargetQuotaView.swift
      TargetActionView.swift

  Features/
    Dashboard/
      DashboardScreen.swift
      DashboardHeader.swift
      ProviderSelector.swift
      OverviewPage.swift
      ProviderPage.swift
    Usage/
      LocalUsageSummaryView.swift
    Settings/
      MenubarSettingsMenu.swift

  Shell/
    StatusItemController.swift
    PopoverController.swift
    BackgroundRefreshController.swift
```

Parent/child state ownership:

- `MenubarStore` owns backend data, last-good snapshot, compatibility, selected provider/target, active operations, and shared settings view.
- Screen components own composition only.
- Section components own expand/collapse and scroll-local state only.
- Row/shared components own hover/focus/pressed/animation/inline-confirmation only.
- No child component owns account active state, quota health, provider availability, or action eligibility.

Component API example:

```swift
struct TargetRowProps: Equatable {
    let id: String
    let title: String
    let subtitle: String
    let status: StatusViewModel
    let quota: TargetQuotaViewModel?
    let actions: TargetActionViewModel
    let operation: TargetOperationState?
}

struct TargetRowCallbacks {
    let onActivate: (String) -> Void
    let onRemove: (String) -> Void
    let onRevealDiagnostics: (String) -> Void
}
```

`TargetRow` receives props/callbacks; it never imports provider-specific DTOs and never calls backend directly.

### Decision 5: 账号体验采用 managed/account-scoped 方向

长期账号模型 SHALL 支持每个 account 使用隔离 runtime scope，例如 managed `CODEX_HOME`。Account-scoped refresh 使用该 scope 查询 quota/status，不改变系统 active home。用户显式 switch 时，再通过后端安全 promote/activate 到系统 active home。

当前 snapshot replacement 模型对 CLI 有价值，应作为安全 activate 路径继续保留。managed scope 是 Menubar 无缝体验的补强，不是绕过现有安全模型。

必须显式区分四类 target：

- `system_active_target`：真实系统 home 当前激活的账号/profile。
- `selected_ui_target`：用户当前在 frontend 中查看或选中的对象。
- `refresh_scope_target`：本次 quota/status refresh 使用的账号 runtime scope。
- `observed_target`：最近一次从系统或 managed scope 观测到的账号状态。

Codex 先采用 lazy migration：现有 snapshot account 在需要 account-scoped refresh 时生成 managed `CODEX_HOME`，验证后再用于 refresh；原 snapshot 保留到新 scope 可用且 activation 路径可回滚。

### Decision 6: Operation 必须后端确认，不做乐观 active 切换

所有 switch/remove/refresh/login/import 这类 mutation SHALL 返回 operation result 和 backend-confirmed view。Frontend 在后端成功前只能展示 pending，不能移动 active marker。失败时必须保留 last-good 状态并显示脱敏原因。

所有后台 refresh/mutation 必须带 request identity 或 generation。旧响应不得覆盖新状态；account-scoped refresh 返回时还必须验证 refresh scope 仍匹配对应 view target。

### Decision 7: Provider 扩展走 contract，不走 frontend 特例

新增 provider 需要先实现 core plugin 能力，再由 control-plane 生成统一 provider/target/usage/status view。Frontend 可以按 provider branding 或小组件做轻量定制，但不能把 provider 特有安全逻辑放进 Swift 或 CLI presentation。

Provider runtime 必须声明 maturity/capability，不允许半成品 provider 污染主体验：

- `detected-only`
- `account-switchable`
- `profile-switchable`
- `quota-readable`
- `account-scoped-refreshable`
- `local-usage-readable`
- `menubar-ready`

不同 source strategy 必须显式声明 source priority、foreground/background eligibility、timeout、fallback、confidence 和 source label。Codex 的首个目标应优先学习 CodexBar 的 OAuth/CLI RPC 分层，但私有 endpoint/WebKit extras 仍需单独产品决策。

Provider adapter split:

```text
provider plugin
  domain detection/auth/profile/switch/usage primitives
provider adapter
  maps plugin capabilities into control-plane runtime/source strategy
control-plane mapper
  maps provider-neutral application state into frontend-safe view model
```

No Swift component may know how a provider stores credentials, refreshes tokens, or builds runtime scope.

### Decision 8: CLI 与 Menubar 可独立分发，但共享状态和 contract

长期发布产物可以拆成 `omx` CLI、`OpenMux.app` Menubar、future server/widget。每个产物可单独安装/启用，但必须使用同一 state root、schema migration、control-plane contract 和 safe diagnostics。

独立分发必须有 compatibility gate：

- control-plane schema version
- state schema version
- minimum backend version
- minimum frontend version
- supported provider capability matrix

不兼容时只允许 read-only safe snapshot 或 upgrade-required 状态，不允许执行 state-changing operation。

### Decision 9: Settings 和 diagnostics 是架构能力，不是 UI 小功能

Shared config/settings 承载 provider enablement、provider order、source mode、refresh cadence、feature flags、debug/recovery switches。Frontend-local preferences 只能保存纯 UI 选择，例如窗口位置、最近 tab、图标显示模式。

Diagnostics 使用结构化模型：

- `code`
- `severity`
- `user_message`
- `recovery_action`
- `debug_detail`
- `source`
- `redaction_status`

日志和 support report 必须经过 redactor，学习 CodexBar 的做法但按 OpenMux 安全规则更保守：token、Cookie、Authorization、raw auth、API key、raw provider response 默认不得出现在 frontend、stdout 或 support report。

## Risks / Trade-offs

- [Risk] 架构重构范围大，容易拖慢功能迭代 → Mitigation: 先落 control-plane contract 和 Swift state/component skeleton，再逐步迁移现有功能。
- [Risk] managed runtime home 与现有 snapshot pool 双模型复杂 → Mitigation: 明确职责，managed scope 用于 account-scoped refresh，activate 仍走现有安全 replacement/promote。
- [Risk] Presentation-ready view model 可能把 UI 细节塞进 Rust → Mitigation: Rust 输出业务语义和状态等级，不输出具体 Swift layout；视觉组件仍在 frontend。
- [Risk] CLI 脚本输出可能被 view model 影响 → Mitigation: control-plane 同时提供 machine-friendly stable JSON，CLI human output 只是其中一种 renderer。
- [Risk] 多 provider 统一 contract 过早泛化 → Mitigation: 以 Codex/Claude 当前真实需求建模，扩展点保持 additive，不提前实现未需要的 provider-specific 抽象。
- [Risk] Source strategy 复制 CodexBar 私有 endpoint 行为 → Mitigation: 只学习 source/fallback/timeout 结构；私有 endpoint/WebKit extras 需要独立产品决策后才能实现。
- [Risk] 独立分发导致 backend/frontend 版本错配 → Mitigation: schema compatibility gate 和 upgrade-required 状态先于分发拆分实现。
- [Risk] Shared settings 和 frontend-local settings 混淆 → Mitigation: provider 行为配置进入 shared config，纯视觉偏好留在 frontend-local storage。

## Migration Plan

Phase 1（地基 + 骨架 + 不回归）：

1. 保留 `crates/omx-app` crate name，按 control-plane 目录拆分 `api/query/mutation/runtime/mapper/diagnostics/compatibility`，让 public re-export 清晰（真实代码搬家，非仅声明模块）。
2. 定义并实现首批 DTO/API：dashboard、provider、target、action、operation、freshness、diagnostics、compatibility；用当前 Codex 数据填充，Claude/Gemini 只保留 additive 扩展位。
3. 将 `omx-menubar-ffi` 限缩为 transport：schema gate、JSON envelope、panic-safe error、memory free；所有业务状态从 control-plane 输出。
4. 实现最小 runtime correctness：generation guard、single-flight/coalescing、last-good safe snapshot、stale/freshness、safe diagnostics/redaction；timeout/backoff/source strategy 可先有模型和基础实现。
5. 引入 Swift Menubar 目录与状态骨架：`Backend/State/Design/Components/Features/Shell`，建立 design tokens 与 primitive/shared/target/screen 组件文件及 typed props/callbacks。
6. 纵向验证切片：把 `TargetRow`（及 props）真正接入当前账号列表的一处渲染，证明组件契约可用；不要求整体替换 monolith（替换属 Phase 3）。
7. 让 CLI 的 `status/list` 消费 control-plane 字段；保留现有命令行为和 machine output，避免破坏脚本。

Phase 2（runtime/settings 深化）：

8. 扩展 runtime/settings：完善 source strategy、timeout/backoff、shared settings、diagnostics support report 和更多 failure recovery。

Phase 3（Menubar UX rebuild，可与 Phase 2 安全并行）：

9. 用 shell/screen/section/shared/primitive 组件**整体替换** `DashboardView` monolith，使其行数显著下降并全面引用新组件。
10. 移除 Swift 前端业务推断（`status` 比较、`quotaColor()`、alert 计数、active marker 等），全部改为消费 control-plane view model 字段。
11. 打磨页面切换状态、provider page、settings menu、视觉一致性、accessibility/reduced-motion、snapshot/smoke 覆盖。

Phase 4（managed account）：

12. 实现 Codex account-scoped refresh：managed `CODEX_HOME`、lazy migration、credential refresh persistence、safe activate/promote 和 rollback。

Phase 5（独立分发）：

13. 推进独立分发：CLI-only、Menubar-only、full bundle、installer matrix、compatibility gate 和发布文档。

Implementation phases:

- Phase 1: Foundation first. Land control-plane modules (real code split, not just declared), first API surface, versioned DTO fixtures, FFI transport boundary, generation/last-good/diagnostics basics, Swift state/design/component skeleton with typed props, **one vertical slice wired into the live account list**, and no-regression CLI/Menubar behavior. Phase 1 does NOT replace the `DashboardView` monolith.
- Phase 2: Runtime and settings depth. Expand refresh coordinator, source strategy, timeout/backoff, shared settings, support report, and redaction coverage.
- Phase 3: Menubar UX rebuild. Replace the monolithic `DashboardView` with shell/screen/section/shared/primitive components, **remove all Swift business inference in favor of control-plane fields**, and polish page switching, visual consistency, accessibility, and snapshot coverage. May run in parallel with Phase 2 — Phase 3 component wiring and a11y/snapshot work depend on the Phase 1 skeleton, not on Phase 2 source strategy.
- Phase 4: Managed account experience. Add Codex managed runtime scope, lazy migration, account-scoped refresh, credential refresh persistence, safe activation.
- Phase 5: Distribution and future surfaces. Move more CLI rendering to control-plane facts, add compatibility gates, document artifact split, and decide whether future HTTP/widget surfaces are worth implementing.

## Open Questions

- `omx-app` 是否在代码层重命名为 `omx-control-plane`，还是先保留 crate name 以减少 churn？
- Codex managed runtime home 是否复制完整 config，还是只保存 auth 并在 refresh 时合成最小环境？
- Claude/Gemini 的 account-scoped runtime 如何映射，是否需要 provider-specific isolation trait？
- Menubar 是否需要内置 login/import，还是第一阶段只提供强引导和 CLI command handoff？
- 独立分发第一目标是 embedded staticlib、helper binary 还是 installed CLI？
- OpenMux 是否需要 lightweight localhost serve/API；如需要，应复用 control-plane contract 而不是新建数据口径。
- DTO 类型命名是否从 `Menubar*` 改为 provider-neutral 命名：Phase 1 明确推迟。原因是 JSON 字段已 provider-neutral，Swift DTO 也已解码为 `DashboardReport`/`ProviderView`；Rust 类型重命名会连带 FFI fixtures、CLI、Swift 和测试大面积 churn，但不改变用户行为或 Phase 1 control-plane 边界。

## Phase 1 实施现状（2026-06-27 核对）

正确性地基已落地并通过 `cargo test`/`clippy`/`swift build`：generation guard（Rust + Swift 两端）、last-good snapshot、compatibility gate、统一 redaction、CLI 消费 `dashboard_view`。

2026-06-27 收尾决策：

- `provider_view` 保留为独立 API，但要求 `MenubarQuery.provider` 非空；`dashboard_view` 才表示全局 dashboard。当前返回类型仍复用 `MenubarDashboardReport` 以保持 additive JSON 兼容，后续可在不破坏字段的前提下引入更窄的 `ProviderView` envelope。
- `Menubar*` Rust 类型名不在 Phase 1 改名；后续若需要 provider-neutral Rust 类型名，应单独做机械迁移并保持 JSON 字段兼容。

判定「拆分完成」以代码搬家 + live 引用为准，而非新增文件——见 Decision 1.2 第 142 行的反模式约束。
