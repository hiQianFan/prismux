## Why

OpenMux 需要判断是否复用 TokenBar 的 macOS Menubar shell，但完整 fork 可能带来第二套 scanning、pricing、quota 和 aggregation。先做限定 spike，给后续 Menubar implementation change 一个 go/no-go 依据。

## What Changes

- 新增 TokenBar Menubar spike：固定 upstream commit，审查 license/NOTICE/第三方资源和发布边界。
- 在隔离目录或分支中验证一个 Overview 数据入口能否改接 OpenMux `omx usage --json --no-scan` 或 mock `UsageReport` contract。
- 梳理 TokenBar SwiftUI views、`tokscale-core`、quota、cache、scanner、aggregator 和 app state 的耦合点。
- 明确可复用 UI/shell 与必须删除的数据引擎代码，禁止 spike 进入 OpenMux 默认 workspace/build。
- 输出 go/no-go、fork delta、维护成本和后续 Menubar implementation change 建议。

## Capabilities

### New Capabilities

- `tokenbar-menubar-spike`: 定义 TokenBar 二次开发评估、OpenMux usage contract 接入、重复数据引擎排除和 go/no-go 输出。

### Modified Capabilities

- 无。

## Impact

- `openspec/changes/evaluate-tokenbar-menubar-spike`: 新增独立 spike 提案、设计、任务和 capability spec。
- `design-artifacts/` 或独立临时目录：可保存 spike 记录、耦合图、截图和 fork delta，不进入默认构建。
- `crates/omx-cli` / `crates/omx-core`: 不在本提案中实现新功能；只消费既有或本地 mock 的 versioned usage JSON。
- `add-native-menubar-app`: 不被本 spike 修改；如果 spike go，再由独立 implementation change 决定是否调整完整 Menubar 方案。
