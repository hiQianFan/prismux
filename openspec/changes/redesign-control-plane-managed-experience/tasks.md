## 0. Phase 1 Guardrails

- [x] 0.1 固定 Phase 1 范围：先做 control-plane 地基、FFI 边界、Swift 状态/组件骨架、最小 runtime correctness 和现有 UX 不回归。
- [x] 0.2 明确 Phase 1 不做：WebKit/browser cookie、Sparkle、WidgetKit、HTTP server、独立分发 artifact、大量 provider registry、完整 managed account migration。
- [x] 0.3 建立 no-regression contract：当前 `status/list/save/use/switch`、Menubar 打开、账号列表、激活、刷新、失败回退必须继续可用。
- [x] 0.4 记录当前 CLI JSON/machine output 和 Menubar FFI response，作为 Phase 1 迁移前后的对照夹具。
- [x] 0.5 明确低优先级设计只保留接口预留，不阻塞 Phase 1 验收：`serve`、future widget/desktop、installer matrix、full support report、account-scoped credential refresh。
- [x] 0.6 每个 Phase 1 PR 必须说明是否改变用户可见行为；若改变，必须给出迁移/兼容策略。

## 1. Control Plane Contract

- [x] 1.1 [Phase 1] 盘点现有 `omx-core`、`omx-app`、`omx-cli`、`omx-menubar-ffi`、Swift Menubar 的真实调用链，标出业务解释重复点。
- [x] 1.2 [Phase 1] 定义首批 control-plane public API：`dashboard_view`、`provider_view`、`refresh_provider`、`activate_target`、`compatibility_view`。
- [x] 1.3 [Phase 1] 定义首批 control-plane DTO：dashboard、provider view、target view、action state、quota/status、freshness、diagnostics、operation result、compatibility。
- [x] 1.4 定义 domain model、application model、frontend-safe view model 的转换边界。
- [x] 1.5 [Phase 1] 明确 stable JSON schema、schema version、state version、minimum frontend/backend compatibility 和 additive migration 规则。
- [x] 1.6 [Phase 1] 增加 Rust serialization fixtures，覆盖 ready、stale、refresh skipped、refresh failed、switch success、switch failed、missing target、unsupported schema。
- [x] 1.7 [Phase 1] 定义 `refresh_all`、`remove_target`、`settings_view`、`update_settings`、`support_report` contract，但不要求完成所有行为实现。
- [x] 1.8 更新 `docs/ARCHITECTURE.md`，把 `omx-core -> omx-control-plane -> frontends` 作为主架构目标，并标注 Phase 1/后续阶段边界。

## 2. Rust Control Plane Implementation

- [x] 2.1 [Phase 1] 将 `crates/omx-app` 模块边界整理为 control-plane application service，先不强制重命名 crate。
- [x] 2.2 [Phase 1] 拆分 `api`、`query`、`mutation`、`runtime`、`mapper`、`diagnostics`、`compatibility` 模块，并保持 `lib.rs` 只做清晰 re-export。
- [x] 2.3 [Phase 1] 将 provider plugin 输出映射为 provider/target/action view model，CLI 不再拼 action eligibility、quota health、provider status。
- [x] 2.4 [Phase 1] 将 `activate_target`、`refresh_provider` mutation 统一返回 operation result + backend-confirmed view；`remove_target` 可先完成 contract。
- [x] 2.5 [Phase 1] 将 safe diagnostics、freshness、last-known/stale 状态集中在 control-plane 输出。
- [x] 2.6 [Phase 1] 保持 `omx-menubar-ffi` 只负责 envelope、schema gate、panic capture、JSON transport 和 memory free。
- [x] 2.7 [Phase 1] 增加 persisted safe snapshot，支持 backend unavailable 时展示 non-sensitive stale view。
- [x] 2.8 [Phase 1] 实现 request generation guard 和最小 single-flight/coalescing，防止旧 refresh 响应覆盖新状态。
- [x] 2.9 [Phase 1] 为 control-plane 增加单元测试和 FFI contract tests，验证敏感字段不会出现在输出中。

## 3. Provider Runtime And Settings

- [x] 3.1 [Phase 1] 定义 provider runtime lifecycle：detect、enabled、available、refresh eligible、in-flight、cancelled、timed out、backoff、last success、last failure。
- [x] 3.2 [Phase 1] 实现 provider refresh coordinator 的最小闭环：single-flight/coalescing、generation guard、request replacement。
- [x] 3.3 [Phase 2] 完善 waiter cancellation、timeout/backoff 策略和 failure gate。
- [x] 3.4 [Phase 2] 定义 source strategy：priority、foreground/background eligibility、timeout、fallback reason、source label、confidence。
- [x] 3.5 [Phase 1] 定义 provider maturity flags：detected-only、account-switchable、profile-switchable、quota-readable、account-scoped-refreshable、local-usage-readable、menubar-ready。
- [x] 3.6 [Phase 1] 定义 shared settings/config schema：provider enablement/order、refresh cadence、display preference；只迁移当前 UX 需要的字段。
- [x] 3.7 [Phase 2] 扩展 source preference、feature flags、debug/recovery switches。
- [x] 3.8 [Phase 1] 定义 structured diagnostics 和 redaction policy，增加 redaction tests 覆盖 token、Cookie、Authorization、API key、email、raw auth marker。

## 4. Swift Menubar Architecture

- [x] 4.1 [Phase 1] 重构 Swift 目录为 Backend、State、Design、Components、Features、Shell 边界。
- [x] 4.2 [Phase 1] 设计 Menubar state machine：loading、ready、stale last-good、failed、refreshing、switching、backend unavailable、upgrade required；deleting 可随 `remove_target` 后续落地。
- [x] 4.3 [Phase 1] 建立最小 design tokens：spacing、typography、status colors、row/button/quota indicator 规则，先服务当前 dashboard 一致性。
- [x] 4.4 [Phase 1] 定义 component props/callbacks：`TargetRowProps`、`StatusBannerProps`、`QuotaMeterProps`、`ProviderSelectorProps`、`EmptyStateProps`。
- [x] 4.5 [Phase 1] 拆分 primitive components：IconButton、CommandButton、MeterRing、Badge、ProgressInline。
- [x] 4.6 [Phase 1] 拆分 shared components：StatusBanner、EmptyStateView、DiagnosticView、SectionHeader、ProviderBadge。
- [x] 4.7 [Phase 1] 拆分 target components：TargetRow、TargetIdentityView、TargetQuotaView、TargetActionView。
- [x] 4.8 [Phase 1] 纵向验证切片：把 `TargetRow`（及 props）真正接入当前账号列表的一处 live 渲染，证明组件契约可用、props 与真实数据匹配。（不要求整体替换 monolith；见 Decision 1.3。）
- [x] 4.9 [Phase 1] 增加 Swift contract/smoke tests，覆盖首次打开、无账号、stale、失败、切换中、missing backend、权限缺失。
- [x] 4.10 [Phase 1] 实现 response generation/request identity guard，旧响应不得覆盖新状态。
- [x] 4.11 [Phase 3] 用 shell/screen/section/shared/primitive 组件整体替换 `DashboardView` monolith，使其行数显著下降并全面引用新组件；screen 文件不再是空壳（`DashboardScreen` 真正组合 sections，`OverviewPage`/`ProviderPage` 承载实际页面）。（live UI 由 `StatusItemController` 挂载 `DashboardScreen`，screen 接管 loading/failed/ready shell；`DashboardView.swift` 已降至约 1012 行并接入 `ProviderSelector`、`OverviewPage`、`ProviderPage`、`DiagnosticView`、`TargetRow`/`TargetQuotaView`。）
- [x] 4.12 [Phase 3] 移除 Swift 业务推断：action eligibility、quota health、provider status、diagnostic 文案全部消费 control-plane 字段。（provider status/attention 改为消费 control-plane `status_text`/`status_tone`；quota health 使用后端 quota view + shared `TargetQuotaView`；diagnostic 文案使用 `DiagnosticView` 和 `recovery_action`。）
- [x] 4.13 [Phase 3] 增加完整 snapshot、accessibility/focus/reduced-motion 检查，确保组件一致性不只停留在视觉层。

## 5. Managed Account Experience

- [x] 5.1 [Phase 1 model only] 在 DTO/model 中预留 `system_active_target`、`selected_ui_target`、`refresh_scope_target`、`observed_target`，但不实现 managed runtime migration。
- [x] 5.2 [Phase 4] 为 Codex 设计 account-scoped runtime scope，明确 managed `CODEX_HOME` 与现有 auth snapshot 的关系。
- [x] 5.3 [Phase 4] 实现从现有 snapshot account 到 managed runtime scope 的 lazy migration 和验证。
- [x] 5.4 [Phase 4] 实现 Codex inactive account quota/status refresh，不改变系统 active `auth.json`。
- [x] 5.5 [Phase 4] 实现 credential refresh persistence：inactive managed token refresh 只更新对应 managed scope。
- [x] 5.6 [Phase 4] 实现 explicit activation/promote 流程，复用 provider subject、hash、backup、atomic write、rollback 安全语义。
- [x] 5.7 [Phase 4] 支持 same-email different-workspace 的身份区分和展示 metadata。
- [x] 5.8 [Phase 4] 为 missing/unreadable/expired managed runtime 增加可恢复状态和用户可执行 action。
- [x] 5.9 [Phase 4] 增加隔离测试：刷新 inactive account 不改变 active auth；switch 失败不污染系统 active home；token refresh 不写错账号。

## 6. CLI Alignment

- [x] 6.1 [Phase 1] 让 `omx status` 和 `omx list` 逐步消费 control-plane provider/target/status 字段。
- [x] 6.2 [Phase 1] 保持 CLI JSON schema 稳定；如需新增字段，使用 additive 方式。
- [x] 6.3 [Phase 1] 增加 CLI 与 Menubar 同 state root 的一致性测试。
- [x] 6.4 [Phase 2] 明确 CLI 作为高级管理入口：login、import、alias、diagnose、recovery 的 Menubar handoff 文案与命令示例。
- [x] 6.5 [Phase 5] 如引入 `serve`/HTTP API，复用 control-plane contract、loopback-only 默认和 request timeout/last-good cache。（本轮未引入 `serve`；future surface 继续受 compatibility gate、request timeout 和 last-good cache 约束。）

## 7. Modular Distribution

- [x] 7.1 [Phase 5] 设计 CLI-only、Menubar-only、full bundle 三种未来 artifact 的能力矩阵。
- [x] 7.2 [Phase 5] 明确 Menubar 如何获得 backend/control-plane runtime：embedded staticlib、helper binary 或 installed CLI。
- [x] 7.3 [Phase 1] 增加 schema compatibility gate 的基础字段：control-plane schema、state schema、minimum backend/frontend version。
- [x] 7.4 [Phase 5] 为缺失 optional module 增加 unavailable view 和安装指导，不隐藏能力或崩溃。
- [x] 7.5 [Phase 5] 更新安装/发布文档，声明每个 artifact 包含的能力、平台和依赖。

## 8. Validation

- [x] 8.1 [Proposal] 运行 `openspec status --change redesign-control-plane-managed-experience` 确认 artifacts complete。
- [x] 8.2 [Implementation] 运行 `cargo fmt --all`。
- [x] 8.3 [Implementation] 运行 `cargo test`。
- [x] 8.4 [Implementation] 运行 `cargo clippy --all-targets --all-features`。
- [x] 8.5 [Implementation] 运行 Swift Menubar build/contract tests。

## 9. Phase 1 收尾（交接给 Codex 实施）

第一阶段的正确性地基已落地并通过验证：generation guard（Rust `begin_refresh_request`/`RefreshAdmission` + Swift `AppStore` generation/requestId）、last-good snapshot、compatibility gate、统一 redaction、CLI 消费 `dashboard_view`，`cargo test`/`clippy`/`swift build` 全绿。组件骨架（primitive/shared/target/screen 文件 + props/tokens）也已建立。

剩余 Phase 1 收尾项聚焦「Rust 地基真实拆分 + 一个纵向切片 + CLI 推断清零 + 命名/API 决策」。Swift monolith 整体替换与前端推断清零已按 Decision 1.3 移至 **Phase 3（见 §4.11、§4.12）**，不属 Phase 1 收尾。完成定义（DoD）：判定标准是「代码搬家 + live 引用」，不是「新增文件」。

- [x] 9.1 真实拆分 `omx-app`（对应 2.2）：把 `api.rs` 的 DTO 迁到 `mapper`（或新 `dto` 模块）、query 函数迁到 `query`、mutation 迁到 `mutation`、refresh 协调器（`begin_refresh_request`/`record_refresh_result`/`release_refresh_request`/`RefreshAdmission`/`RefreshState` 及相关 static/const）迁到 `runtime`、映射 helper 迁到 `mapper`。
  - 验收：`api.rs` 显著缩小（仅保留薄 public 别名或删除）；`query.rs`/`mutation.rs`/`mapper.rs` 不再是空壳/纯 re-export；`lib.rs` 维持清晰 re-export。
  - 约束：`omx-menubar-ffi` 与 `omx-cli` 现有 `omx_app::` 导入符号必须保持可用（含 `reset_menubar_refresh_state_for_tests`、`compatibility::CONTROL_PLANE_SCHEMA_VERSION` 等）；`cargo test`/`clippy` 维持全绿。
- [x] 9.2 纵向切片 + CLI 推断清零（对应 4.8、2.3）：把 `TargetRow`（及 props）接入账号列表一处 live 渲染；CLI（`app.rs`）改为消费后端给定的 status/attention 字段，移除 `!matches!(Healthy)` 自行推断。
  - 验收：`TargetRow` 在 live view 被引用 ≥1 次并渲染真实数据；CLI 不再做 status 推断；`swift build` 与 CLI 行为/machine output 不回归。
  - 注：Swift monolith 整体替换与 `DashboardView`/`AppStore` 全量推断清零不在此项——属 Phase 3 §4.11/§4.12。
- [x] 9.3 DTO 命名决策（对应 1.3、Decision 8 Open Question）：决定 `Menubar*` → provider-neutral 命名（如 `DashboardView`/`ProviderView`/`TargetRow` DTO）是否纳入 Phase 1 收尾，或显式推迟并在 design.md 记录理由。
  - 若执行：同步更新 Rust 类型、`lib.rs` re-export、FFI fixtures、CLI 与 Swift 解码侧，保持 JSON 字段 additive 兼容。
- [x] 9.4 澄清 `provider_view` 与 `dashboard_view`（Decision Open Question）：二者当前实现完全相同（均调 `menubar_dashboard`），需明确 `provider_view` 的 provider-scoped 语义差异或合并为单一 API 并更新 contract。
- [x] 9.5 收尾后回填勾选：9.1 完成则勾回 2.2；9.2 完成则勾回 4.8、2.3；重新运行 8.2–8.5 验证。Swift §4.11/§4.12（monolith 替换、推断清零）在 Phase 3 完成时回填，不在 Phase 1 收尾。
