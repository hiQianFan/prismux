## Why

`add-native-menubar-app` 已经建立 Menubar 技术链路，但当前产品模型仍像账号列表页：基础数据可见，switch/refresh 的操作反馈弱，provider 分组和状态口径不清，tray title 还展示邮箱这类噪音。用户真正需要的是一个可信的账号控制台：快速判断当前状态、明确切换账号、知道操作成功或失败。

## What Changes

- 重新定义 Menubar 信息架构：header、`Overview + provider selector`、单 provider 页面、account cards、local usage summary、footer actions。
- Overview 首屏展示所有账号池的聚合情况、各 provider 当前选用账号/profile 的摘要，以及按 provider/client 聚合的 token usage 图表。
- 单 provider 页面先展示该 provider overview，再展示 account/profile 选择，最后展示该 provider 的 token usage；不增加 provider 内二级 tab。
- 每个 provider 全局只能有一个 active target：account 或 profile 二选一；切换 account 会替换当前 active profile，切换 profile 会替换当前 active account。
- 将 tray 收起态从 email/alias 改为聚合健康信号，例如 quota urgency、provider health、stale/error。
- 在 `omx-app` 定义统一 `DashboardView`、`ProviderPageView`、`OverviewView` 和 `OperationResult` contract，CLI 与 Menubar 共享展示口径。
- 明确区分 `tool_provider`、account/profile target、account quota、local usage 和 `model_provider`。
- 强化 switch/refresh 操作语义：pending、success、failure、skipped、stale、last-good 和脱敏 diagnostics。
- 修正 Swift Menubar：不保留 `codex` fallback、不用裸 `localId` 做唯一 id、不吞 ok-but-no-dashboard/decode failure。

## Capabilities

### New Capabilities

- `menubar-control-plane-contract`: 定义 Menubar 控制台信息架构、统一 dashboard contract、operation result、tray 聚合信号和 CLI/Menubar 口径一致性。

### Modified Capabilities

- `menubar-account-switching`: switch response 必须返回明确 operation result 和 backend-confirmed dashboard。
- `menubar-backend-contract`: FFI contract 从 ad-hoc dashboard/accounts/switch/refresh DTO 收敛为统一 view model + operation envelope。
- `native-menubar-shell`: SwiftUI popover 根据 provider groups 和 operation state 渲染，不自行推断业务状态。

## Impact

- `crates/omx-app`: 新增/调整 Menubar/CLI 共享 view model 和 operation DTO。
- `crates/omx-menubar-ffi`: 调整 schema v1 additive fields 或引入 v2-compatible envelope；保持脱敏和 panic-safe。
- `crates/omx-cli`: 逐步消费共享 view model 的展示字段，避免与 Menubar 口径分叉。
- `apps/omx-menubar`: 重排 UI 信息架构、状态表达、tray title、provider grouping 和 operation feedback。
- `docs/ARCHITECTURE.md` / `docs/menubar-v1.md`: 更新控制台边界和数据口径。
