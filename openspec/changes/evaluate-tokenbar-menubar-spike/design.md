## Context

`refine-usage-cli-experience` 负责让 `omx usage` 提供稳定的摘要、分组、details 和 versioned JSON。Menubar 需要复用这些 usage 语义，但不应把 TokenBar 的 scanner、pricing、quota 或 aggregation 带入 OpenMux。

已有 `add-native-menubar-app` 是完整原生 Menubar implementation proposal。本 change 只回答一个更小的问题：TokenBar 是否值得 fork/二次开发，还是只借鉴 UX。

## Goals / Non-Goals

**Goals:**

- 验证 TokenBar Overview 是否能低成本接入 OpenMux-owned usage contract。
- 识别必须删除或禁用的 TokenBar 数据引擎代码。
- 固定 license/NOTICE、第三方资源、发布链路和 upstream sync 风险。
- 输出可执行 go/no-go 结论。

**Non-Goals:**

- 不交付完整 Menubar App。
- 不把 TokenBar fork 加入 OpenMux workspace、CI 或默认 build。
- 不复制 TokenBar scanner、pricing、quota fetcher、usage cache 或 aggregation。
- 不新增 Rust FFI、local socket、Sparkle、notarization 或 Homebrew cask。

## Decisions

### 1. Spike 使用隔离 fork

TokenBar fork 只允许放在临时目录、独立分支或 `design-artifacts/` 记录中。OpenMux 默认 workspace/build 不引用该 fork，避免未决代码路径变成维护负担。

### 2. 数据入口只接 OpenMux contract

spike 可以使用 `omx usage --json --no-scan` 或 mock `UsageReport` JSON 替换一个 Overview 数据入口。Swift 不直接读取 OpenMux SQLite，也不运行 TokenBar/tokscale 独立 scan。

### 3. Go/no-go 看删除成本

继续 fork 的最低门槛：

- license/NOTICE 和第三方资源可合规再分发；
- 一个 Overview screen 能接入 OpenMux contract；
- TokenBar 数据层可被清晰删除或隔离；
- fork delta 和 upstream sync 成本可控。

停止线：如果替换数据层需要重写大部分 view model、FFI 或状态管理，或发布链路与 OpenMux 产品边界冲突，则停止 fork，只保存 UX mapping。

## Risks / Trade-offs

- [TokenBar UI 与数据层强耦合] → spike 只改一个 Overview，超过停止线就不继续。
- [fork 持续分叉] → 记录 upstream commit、fork delta 和预计同步成本。
- [CLI JSON 子进程有启动开销] → 只用于 spike；production 接口由后续 Menubar implementation change 决定。
- [合规不清] → 无明确 license/NOTICE 的代码或资源不得进入 release 路径。

## Output

- TokenBar upstream commit 与 license/NOTICE 记录。
- Overview 接入 OpenMux JSON/mock contract 的结果。
- 可复用 UI/shell 清单与必须删除的数据引擎清单。
- 启动/刷新延迟、错误降级和 stale data 行为记录。
- go/no-go 结论；go 时创建或更新独立 Menubar implementation change，no-go 时仅保存 UX mapping。
