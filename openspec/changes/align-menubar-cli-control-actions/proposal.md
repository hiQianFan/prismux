## Why

Menubar 已经承担账号控制面板职责，但 last-good dashboard 可能被显示成新数据，usage period 口径也可能与用户选择不一致。

同时 CLI 缺少 Menubar 已有的单账号 refresh 与 Codex reset credit 兜底入口，用户在远程环境、脚本化场景或 Menubar 不可用时无法完成同等操作。

## What Changes

- Menubar dashboard fallback 必须保留 last-good 数据，但通过 additive envelope 字段明确标记为 stale / served from snapshot，header、tray title 和状态展示不得把旧数据伪装成新数据。
- Menubar 的 `Today / 7d / 30d` usage period 必须传入后端 dashboard query，使 Overview headline、provider headline 和 usage chart 使用同一周期口径；period 变化时允许重新加载 dashboard。
- CLI `omx refresh <platform>` 增加可选 selector，支持刷新单个 account：`omx refresh codex 2`、`omx refresh codex work`。
- CLI 新增 Codex-only reset credit 消费入口：`omx reset-credit codex <selector>`，复用已有 `PlatformPlugin::consume_reset_credit` 能力，并要求显式 account selector。
- 不增加自动最佳账号选择、并发 refresh、完整 dashboard TUI 或历史 analytics。
- 不新增外部依赖。

## Capabilities

### New Capabilities

- `menubar-dashboard-freshness`: 定义 Menubar last-good fallback 的 stale 标记，以及 usage period 与 dashboard query 的一致口径。
- `cli-control-actions`: 定义 CLI 单账号 refresh 与 Codex reset credit 入口。

### Modified Capabilities

无。当前仓库没有已归档的 `openspec/specs/*` capability；本变更以新增 capability 描述当前未归档控制面能力的一致性要求。

## Impact

- `crates/omx-app`: dashboard query / operation report 继续作为共享 view model 来源。
- `crates/omx-menubar-ffi`: dashboard fallback response 需要携带明确 stale/snapshot 来源信号。
- `apps/omx-menubar`: dashboard request 需要传递 usage period；AppStore 需要按后端 stale 信号更新 state。
- `crates/omx-cli`: `refresh` 命令增加可选 selector；新增 `reset-credit` 命令。
- 测试影响：补充 FFI fallback stale、Menubar request encoding、CLI 单账号 refresh、CLI reset credit 的最小回归测试。
