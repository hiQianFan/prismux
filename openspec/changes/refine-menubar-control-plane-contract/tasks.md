## 1. Contract 定义

- [ ] 1.1 定义 `DashboardView`、`OverviewView`、`ProviderPageView`、`AccountTargetView`、`ActionState`、`OperationResult` DTO。
- [ ] 1.2 明确字段命名：`tool_provider`、`model_provider`、`account_key`、`target_kind`、`local_usage`、`account_quota`。
- [x] 1.3 为 switch/refresh 增加 `operation.status`、`changed`、`active_before`、`active_after`、`message`、`diagnostics`。
- [ ] 1.4 增加 Rust serialization fixture，覆盖 success、skipped、failure、stale last-good。
- [x] 1.5 明确 provider 全局单 active target：account/profile 共享一个 active slot，mutation 后由 backend dashboard 确认互斥状态。

## 2. Rust Application Service

- [ ] 2.1 在 `omx-app` 生成 Overview 聚合 view 和单 provider page view。
- [ ] 2.2 后端生成 `display_label`、`secondary_label`、`status_level`、`status_text`、`actions`，Swift 不再拼业务语义。
- [x] 2.3 switch mutation 后返回 backend-confirmed operation result + full dashboard。
- [x] 2.4 refresh mutation 区分 refreshed、fresh-enough skipped、backoff skipped、failed stale。
- [ ] 2.5 保留安全边界：Swift 不读 auth/SQLite/log/provider endpoint。
- [x] 2.6 provider target catalog 输出 accounts/profiles，并保证同 provider 只有一个 active target。

## 3. FFI Contract

- [x] 3.1 调整 `omx-menubar-ffi` envelope，支持 dashboard view + operation result。
- [x] 3.2 对 bad JSON、unsupported schema、application error、panic 仍返回脱敏 error envelope。
- [x] 3.3 更新 golden fixtures 和 Swift contract tests。

## 4. Swift Menubar

- [x] 4.1 Swift DTO 使用 `account_key` 作为 identity，不再用裸 `localId`。
- [x] 4.2 移除 `activeProvider ?? "codex"` fallback，active provider 来自 backend view。
- [x] 4.3 popover 按 `Overview + provider selector` 渲染：Overview 摘要页和单 provider 账号页。
- [x] 4.4 switch/refresh 显示局部 pending、success、failure、stale，不吞错误。
- [x] 4.5 tray title 改为 usage rate、quota urgency 或 provider health 聚合信号，不默认展示 email。
- [x] 4.6 provider 页账号/profile row 使用左右布局：左侧 identity/state，右侧 5h/session 与 7d/weekly ring + percent/reset text。
- [x] 4.7 移除 provider 内二级 tab；Accounts、Profiles、Limits、Local Usage 在同一滚动页按任务优先级排列。

## 5. CLI 口径统一

- [ ] 5.1 让 CLI overview/platform list 逐步消费 `omx-app` 的 shared view fields。
- [ ] 5.2 增加 CLI 与 Menubar 同 state root 的一致性测试。

## 6. 验收

- [ ] 6.1 临时 `OMUX_STATE_ROOT` / `CODEX_HOME` 下，Menubar switch 后 `omx current codex` 与 UI active 一致。
- [ ] 6.2 无账号、quota unknown、refresh failed、switch target missing 都有明确 UI 状态。
- [x] 6.3 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`、Swift build/test、OpenSpec validation 通过。
