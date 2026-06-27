## 0. Phase 1 Guardrails

- [ ] 0.1 固定 Phase 1 范围：先做 control-plane 地基、FFI 边界、Swift 状态/组件骨架、最小 runtime correctness 和现有 UX 不回归。
- [ ] 0.2 明确 Phase 1 不做：WebKit/browser cookie、Sparkle、WidgetKit、HTTP server、独立分发 artifact、大量 provider registry、完整 managed account migration。
- [ ] 0.3 建立 no-regression contract：当前 `status/list/save/use/switch`、Menubar 打开、账号列表、激活、刷新、失败回退必须继续可用。
- [ ] 0.4 记录当前 CLI JSON/machine output 和 Menubar FFI response，作为 Phase 1 迁移前后的对照夹具。
- [ ] 0.5 明确低优先级设计只保留接口预留，不阻塞 Phase 1 验收：`serve`、future widget/desktop、installer matrix、full support report、account-scoped credential refresh。
- [ ] 0.6 每个 Phase 1 PR 必须说明是否改变用户可见行为；若改变，必须给出迁移/兼容策略。

## 1. Control Plane Contract

- [ ] 1.1 [Phase 1] 盘点现有 `omx-core`、`omx-app`、`omx-cli`、`omx-menubar-ffi`、Swift Menubar 的真实调用链，标出业务解释重复点。
- [ ] 1.2 [Phase 1] 定义首批 control-plane public API：`dashboard_view`、`provider_view`、`refresh_provider`、`activate_target`、`compatibility_view`。
- [ ] 1.3 [Phase 1] 定义首批 control-plane DTO：dashboard、provider view、target view、action state、quota/status、freshness、diagnostics、operation result、compatibility。
- [ ] 1.4 定义 domain model、application model、frontend-safe view model 的转换边界。
- [ ] 1.5 [Phase 1] 明确 stable JSON schema、schema version、state version、minimum frontend/backend compatibility 和 additive migration 规则。
- [ ] 1.6 [Phase 1] 增加 Rust serialization fixtures，覆盖 ready、stale、refresh skipped、refresh failed、switch success、switch failed、missing target、unsupported schema。
- [ ] 1.7 [Phase 1] 定义 `refresh_all`、`remove_target`、`settings_view`、`update_settings`、`support_report` contract，但不要求完成所有行为实现。
- [ ] 1.8 更新 `docs/ARCHITECTURE.md`，把 `omx-core -> omx-control-plane -> frontends` 作为主架构目标，并标注 Phase 1/后续阶段边界。

## 2. Rust Control Plane Implementation

- [ ] 2.1 [Phase 1] 将 `crates/omx-app` 模块边界整理为 control-plane application service，先不强制重命名 crate。
- [ ] 2.2 [Phase 1] 拆分 `api`、`query`、`mutation`、`runtime`、`mapper`、`diagnostics`、`compatibility` 模块，并保持 `lib.rs` 只做清晰 re-export。
- [ ] 2.3 [Phase 1] 将 provider plugin 输出映射为 provider/target/action view model，Swift 和 CLI 不再拼 action eligibility、quota health、provider status。
- [ ] 2.4 [Phase 1] 将 `activate_target`、`refresh_provider` mutation 统一返回 operation result + backend-confirmed view；`remove_target` 可先完成 contract。
- [ ] 2.5 [Phase 1] 将 safe diagnostics、freshness、last-known/stale 状态集中在 control-plane 输出。
- [ ] 2.6 [Phase 1] 保持 `omx-menubar-ffi` 只负责 envelope、schema gate、panic capture、JSON transport 和 memory free。
- [ ] 2.7 [Phase 1] 增加 persisted safe snapshot，支持 backend unavailable 时展示 non-sensitive stale view。
- [ ] 2.8 [Phase 1] 实现 request generation guard 和最小 single-flight/coalescing，防止旧 refresh 响应覆盖新状态。
- [ ] 2.9 [Phase 1] 为 control-plane 增加单元测试和 FFI contract tests，验证敏感字段不会出现在输出中。

## 3. Provider Runtime And Settings

- [ ] 3.1 [Phase 1] 定义 provider runtime lifecycle：detect、enabled、available、refresh eligible、in-flight、cancelled、timed out、backoff、last success、last failure。
- [ ] 3.2 [Phase 1] 实现 provider refresh coordinator 的最小闭环：single-flight/coalescing、generation guard、request replacement。
- [ ] 3.3 [Phase 2] 完善 waiter cancellation、timeout/backoff 策略和 failure gate。
- [ ] 3.4 [Phase 2] 定义 source strategy：priority、foreground/background eligibility、timeout、fallback reason、source label、confidence。
- [ ] 3.5 [Phase 1] 定义 provider maturity flags：detected-only、account-switchable、profile-switchable、quota-readable、account-scoped-refreshable、local-usage-readable、menubar-ready。
- [ ] 3.6 [Phase 1] 定义 shared settings/config schema：provider enablement/order、refresh cadence、display preference；只迁移当前 UX 需要的字段。
- [ ] 3.7 [Phase 2] 扩展 source preference、feature flags、debug/recovery switches。
- [ ] 3.8 [Phase 1] 定义 structured diagnostics 和 redaction policy，增加 redaction tests 覆盖 token、Cookie、Authorization、API key、email、raw auth marker。

## 4. Swift Menubar Architecture

- [ ] 4.1 [Phase 1] 重构 Swift 目录为 Backend、State、Design、Components、Features、Shell 边界。
- [ ] 4.2 [Phase 1] 设计 Menubar state machine：loading、ready、stale last-good、failed、refreshing、switching、backend unavailable、upgrade required；deleting 可随 `remove_target` 后续落地。
- [ ] 4.3 [Phase 1] 建立最小 design tokens：spacing、typography、status colors、row/button/quota indicator 规则，先服务当前 dashboard 一致性。
- [ ] 4.4 [Phase 1] 定义 component props/callbacks：`TargetRowProps`、`StatusBannerProps`、`QuotaMeterProps`、`ProviderSelectorProps`、`EmptyStateProps`。
- [ ] 4.5 [Phase 1] 拆分 primitive components：IconButton、CommandButton、MeterRing、Badge、ProgressInline。
- [ ] 4.6 [Phase 1] 拆分 shared components：StatusBanner、EmptyStateView、DiagnosticView、SectionHeader、ProviderBadge。
- [ ] 4.7 [Phase 1] 拆分 target components：TargetRow、TargetIdentityView、TargetQuotaView、TargetActionView。
- [ ] 4.8 [Phase 1] 拆分 feature screens：DashboardScreen、DashboardHeader、ProviderSelector、OverviewPage、ProviderPage；`LocalUsageSummaryView`、`MenubarSettingsMenu` 可在 Phase 2/3 完善。
- [ ] 4.9 [Phase 1] 移除 Swift 业务推断：action eligibility、quota health、provider status、diagnostic 文案全部消费 control-plane 字段。
- [ ] 4.10 [Phase 1] 增加 Swift contract/smoke tests，覆盖首次打开、无账号、stale、失败、切换中、missing backend、权限缺失。
- [ ] 4.11 [Phase 1] 实现 response generation/request identity guard，旧响应不得覆盖新状态。
- [ ] 4.12 [Phase 3] 增加完整 snapshot、accessibility/focus/reduced-motion 检查，确保组件一致性不只停留在视觉层。

## 5. Managed Account Experience

- [ ] 5.1 [Phase 1 model only] 在 DTO/model 中预留 `system_active_target`、`selected_ui_target`、`refresh_scope_target`、`observed_target`，但不实现 managed runtime migration。
- [ ] 5.2 [Phase 4] 为 Codex 设计 account-scoped runtime scope，明确 managed `CODEX_HOME` 与现有 auth snapshot 的关系。
- [ ] 5.3 [Phase 4] 实现从现有 snapshot account 到 managed runtime scope 的 lazy migration 和验证。
- [ ] 5.4 [Phase 4] 实现 Codex inactive account quota/status refresh，不改变系统 active `auth.json`。
- [ ] 5.5 [Phase 4] 实现 credential refresh persistence：inactive managed token refresh 只更新对应 managed scope。
- [ ] 5.6 [Phase 4] 实现 explicit activation/promote 流程，复用 provider subject、hash、backup、atomic write、rollback 安全语义。
- [ ] 5.7 [Phase 4] 支持 same-email different-workspace 的身份区分和展示 metadata。
- [ ] 5.8 [Phase 4] 为 missing/unreadable/expired managed runtime 增加可恢复状态和用户可执行 action。
- [ ] 5.9 [Phase 4] 增加隔离测试：刷新 inactive account 不改变 active auth；switch 失败不污染系统 active home；token refresh 不写错账号。

## 6. CLI Alignment

- [ ] 6.1 [Phase 1] 让 `omx status` 和 `omx list` 逐步消费 control-plane provider/target/status 字段。
- [ ] 6.2 [Phase 1] 保持 CLI JSON schema 稳定；如需新增字段，使用 additive 方式。
- [ ] 6.3 [Phase 1] 增加 CLI 与 Menubar 同 state root 的一致性测试。
- [ ] 6.4 [Phase 2] 明确 CLI 作为高级管理入口：login、import、alias、diagnose、recovery 的 Menubar handoff 文案与命令示例。
- [ ] 6.5 [Phase 5] 如引入 `serve`/HTTP API，复用 control-plane contract、loopback-only 默认和 request timeout/last-good cache。

## 7. Modular Distribution

- [ ] 7.1 [Phase 5] 设计 CLI-only、Menubar-only、full bundle 三种未来 artifact 的能力矩阵。
- [ ] 7.2 [Phase 5] 明确 Menubar 如何获得 backend/control-plane runtime：embedded staticlib、helper binary 或 installed CLI。
- [ ] 7.3 [Phase 1] 增加 schema compatibility gate 的基础字段：control-plane schema、state schema、minimum backend/frontend version。
- [ ] 7.4 [Phase 5] 为缺失 optional module 增加 unavailable view 和安装指导，不隐藏能力或崩溃。
- [ ] 7.5 [Phase 5] 更新安装/发布文档，声明每个 artifact 包含的能力、平台和依赖。

## 8. Validation

- [ ] 8.1 [Proposal] 运行 `openspec status --change redesign-control-plane-managed-experience` 确认 artifacts complete。
- [ ] 8.2 [Implementation] 运行 `cargo fmt --all`。
- [ ] 8.3 [Implementation] 运行 `cargo test`。
- [ ] 8.4 [Implementation] 运行 `cargo clippy --all-targets --all-features`。
- [ ] 8.5 [Implementation] 运行 Swift Menubar build/contract tests。
