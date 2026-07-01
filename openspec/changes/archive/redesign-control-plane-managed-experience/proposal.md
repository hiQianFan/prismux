## Why

OpenMux 当前已经有可靠的 Rust 账号切换核心，但整体产品形态仍像 `CLI + Menubar 壳`：Swift 前端状态弱、UI 口径不一致，CLI/Menubar/未来 API 缺少同一个产品级 control plane。现在需要先把长期架构地基落稳，让后续逐步增加 provider、独立分发 CLI/Menubar、扩展桌面或 HTTP API 时，不再重复解释账号、状态、错误和操作体验。

本 change 明确参考 CodexBar 的设计思路和工程哲学，但不照搬 CodexBar 的体量。OpenMux 要学习的是：core 能力集中、Menubar 是一等产品、账号状态需要 reconciliation、后台刷新需要 runtime 生命周期、CLI/Menubar 可分发但共享语义、诊断和日志必须脱敏；不学习的是把大量 provider 业务逻辑放进 Swift 前端、提前引入 WebKit scraping、Sparkle、WidgetKit 或几十个 provider registry。

## What Changes

- 将 OpenMux 的长期架构目标明确为三层：
  - `omx-core`：账号、profile、usage、状态、错误、安全操作和 provider plugin contract。
  - `omx-control-plane`：产品级 application service，输出 CLI、Menubar、future desktop/widget/http 共享的 view model、operation result、diagnostics 和 action contract。
  - `frontends`：`omx-cli`、`omx-menubar`、future desktop/widget/http，各自只负责交互与呈现，不重新解释业务事实。
- 将 `omx-app` 从 Menubar FFI helper 升级为 `omx-control-plane` 的实现载体，作为所有前端共享的产品后端。
- 重新定义 Swift Menubar 的角色：它是独立维护的原生前端，有自己的状态机、组件体系和交互细节，但不读取 auth、SQLite、usage logs 或 provider endpoint。
- 引入长期账号体验目标：每个账号可拥有隔离 runtime home 或等价 backend scope，用于 account-scoped refresh；系统 active home 只在用户显式激活账号时被安全替换或 promote。
- 保留 OpenMux 当前安全优势：provider subject 校验、snapshot hash、private permission、atomic write、backup、rollback、safe diagnostics 仍位于后端。
- 为后续 provider 扩展建立统一能力口径：新增 provider 必须接入 core/control-plane contract，而不是只给某一个 frontend 写特例。
- 将 CLI 与 Menubar 的关系定义为两个可独立分发、可单独安装、但共享 state 和 control-plane contract 的 frontend；独立分发是后续演进目标，不作为第一阶段阻塞项。
- 将 UI/UX 质量纳入架构约束：前端必须使用统一 view model、operation state、component/token 系统，避免每个页面独立拼状态和文案。
- 将 provider runtime 生命周期纳入 control-plane：source strategy、refresh coalescing、timeout、cancellation、backoff、last-good cache、request generation 和 failure gate 都必须有统一口径。
- 将 settings/config 边界纳入架构：provider enablement、source preference、refresh cadence、display preference、debug/recovery switches 必须进入共享配置模型，不散落在 Swift `UserDefaults`、CLI 参数和 Rust 默认值之间。
- 将诊断与本地日志纳入产品能力：用户文案、recovery action、debug detail、support report 和 redaction policy 必须分层，不能只返回一段 string。
- 将 CodexBar 的可验证模式转译为 OpenMux 架构约束：`CodexBarCore` 对应 OpenMux 的 `omx-core + omx-control-plane`，`UsageStore/ProviderRefreshCoordinator` 对应 OpenMux 的 provider runtime coordinator，`CodexAccountReconciliationSnapshot` 对应 OpenMux 的 active/selected/observed/refresh-scope target 模型，`LogRedactor` 对应 OpenMux 的 diagnostics/redaction policy，CodexBar CLI/Menubar 分发经验对应 OpenMux 的 compatibility gate。
- 将 control-plane API 和 Menubar component architecture 显式化：函数按 query、mutation、runtime、settings、diagnostics 拆分；Swift 组件按 shell、screen、section、shared component、primitive 分层，业务逻辑不得进入组件。

## Phase 1 Scope

本 change 的第一阶段目标不是一次性完成所有长期架构，而是先把 OpenMux 的低层地基打稳，并直接改善当前 CLI/Menubar 用户体验。Phase 1 必须保持现有账号保存、列表、切换、刷新和 Menubar 打开体验可用；内部重构不能让用户为了架构升级失去当前能力。

Phase 1 MUST land:

- `crates/omx-app` 先作为 control-plane 实现载体继续存在，不强制立即改 crate name；优先完成内部模块边界、public API、DTO、compatibility 和测试夹具。
- Control-plane 首批 API 聚焦 `dashboard_view`、`provider_view`、`refresh_provider`、`activate_target`、`compatibility_view`；其它 API 可以先设计 contract，不阻塞第一阶段验收。
- `omx-menubar-ffi` 收敛为 transport 层：JSON envelope、schema version、panic-safe error、memory free 和 compatibility gate；不得继续承载业务解释。
- Runtime 先落最小但正确的地基：request generation、single-flight/coalescing、last-good safe snapshot、freshness/stale metadata、safe diagnostics/redaction。
- Swift Menubar 先落项目结构和组件状态地基：`Backend`、`State`、`Design`、`Components`、`Features`、`Shell`；建立 design tokens、primitive/shared/target/screen 组件文件与 typed props/callbacks。
- 前端全局组件先落骨架：`StatusBanner`、`ProviderSelector`、`TargetRow`、`TargetIdentityView`、`TargetActionView`、`DiagnosticView`、基础 button/badge/meter token。
- **至少一个纵向切片接入 live view**：把 `TargetRow`（及 props）真正接入当前账号列表的一处渲染，证明组件契约可用、props 与真实数据匹配。整体替换 monolith 属 Phase 3，不在 Phase 1 验收范围。
- 父子组件状态边界必须先落：`MenubarStore` 拥有 backend data、last-good、selection、operation state；子组件只拿 typed props/callbacks，不直接调用 backend，不推断 provider 业务状态。
- 增加 serialization/contract/smoke tests，覆盖 ready、stale、refresh failed、switch success/failure、backend unavailable、unsupported schema 和敏感字段不泄漏。
- 保持 CLI 行为稳定：`status`、`list`、`save`、`use/switch` 等当前能力不能因为 control-plane 重构改变语义或破坏 machine output；CLI 消费 control-plane 给定的 status/attention 字段，不自行推断。

Phase 1 SHOULD NOT implement:

- 整体替换 `DashboardView` monolith、移除 Swift 端全部业务推断、视觉/页面切换/a11y 打磨——这些属 Phase 3（见 design.md Decision 1.3）；Phase 1 只建组件骨架并接入一个纵向切片。
- WebKit scraping、browser cookie import、私有 provider endpoint 或 CodexBar 的 WebKit extras。
- Sparkle、WidgetKit、自动更新、HTTP server、future widget/desktop API。
- CLI/Menubar 独立分发 artifact、installer matrix 或完整 release pipeline。
- 一次性引入大量 provider registry 或为未来 provider 写 Swift 特例。
- 完整 managed account lazy migration、account-scoped credential refresh persistence；第一阶段只保留模型和 API 预留，避免堵死后续实现。
- Menubar 内置完整 login/import 流程；第一阶段可提供稳定 CLI handoff 和清晰 recovery action。

Phase 1 的验收标准是：代码边界更清楚（Rust 模块真实拆分、FFI 收敛为 transport）、状态更一致、错误更可恢复、组件骨架可用且已有纵向切片验证，同时当前用户能继续完成最核心的账号管理工作。Swift monolith 的整体替换与前端推断清零交由 Phase 3，避免 Phase 1 过载与 P1/P3 职责重叠。

## Capabilities

### New Capabilities

- `control-plane-architecture`: 定义 `omx-core`、`omx-control-plane` 和 frontends 的职责边界、共享 view model、operation contract 和 provider extensibility。
- `managed-account-experience`: 定义账号隔离刷新、系统 active promotion、账号切换安全语义和无缝用户体验。
- `frontend-experience-boundary`: 定义 Swift Menubar、CLI 和未来 frontend 如何消费 control-plane，并保证 UI 状态、组件和错误体验一致。
- `modular-product-distribution`: 定义 CLI、Menubar 和 future frontend 独立分发/启用的长期约束，保证模块可剥离但状态和体验一致。
- `provider-runtime-and-settings`: 定义 provider runtime 生命周期、source strategy、refresh/cache/backoff、settings/config 和 diagnostics/logging 边界。
- `control-plane-api-surface`: 定义 control-plane 的 public API、模块拆分、函数职责、DTO 层级和 provider-agnostic execution boundary。
- `menubar-component-architecture`: 定义 Swift Menubar 的项目结构、父子组件、状态归属、组件粒度、设计 token 和一致性规则。

### Modified Capabilities

- 无。当前 repo 尚无已归档 base specs；本 change 先建立长期架构 capability。现有未归档 Menubar change 后续应与本 change 对齐或被其吸收。

## Impact

- `crates/omx-core`: 需要稳定 provider plugin、account/profile/usage/error、安全操作和 future account-scoped runtime 的领域模型。
- `crates/omx-app`: 将演进为 `omx-control-plane`，输出 presentation-ready view model、operation result、action eligibility、diagnostics、freshness 和 frontend-independent UX contract。
- `crates/omx-cli`: 从直接拼业务输出逐步迁移到消费 control-plane view model，同时保留脚本友好的 machine output。
- `crates/omx-menubar-ffi`: 只保留 transport/envelope/schema/panic-safe/free 责任，不承载业务解释。
- `apps/omx-menubar`: 重构为独立 Swift frontend，拥有稳定 state machine、组件系统、视觉 token 和 Menubar-native UX。
- Provider plugins: 新 provider 逐步增加时必须先满足 core/control-plane contract，再暴露给 CLI/Menubar。
- Shared config/settings: 需要统一 provider enablement、source mode、refresh cadence、display mode、diagnostic preferences 和 feature flags。
- Runtime/cache/logging: 需要建立 provider refresh coordinator、last-good cache、schema version compatibility、local debug log 和 redaction policy。
- Menubar component system: 需要将当前 `DashboardView` 拆分为 shell/screen/section/shared/primitive 层级，并用明确 props/callbacks 避免子组件持有业务状态。
- Docs/OpenSpec: 后续 Menubar、CLI、provider 扩展和分发方案都应引用本 change 的架构边界，避免口径分叉。
