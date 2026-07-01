## Context

OpenMux 当前是 Rust workspace：`omx-core` 保存领域类型、SQLite state 和 plugin trait，provider crates 实现账号导入/切换/quota，`omx-usage-tokscale` 负责本地 usage 解析，`omx-cli` 承担命令映射、部分应用编排和展示。Menubar v1 的目标不是复刻 TokenBar，而是给 OpenMux 账号池提供一个常驻控制面板。

对 TokenBar commit `813ff11aeba40bd48a25333f0b1aa1df7bfffc6a` 的调研显示：其 `NSStatusItem`/SwiftUI shell 和布局思路可参考，但数据层与 OpenMux 重叠且边界不符合本项目安全规则。因此 v1 创建 OpenMux 自有 App，不复制 TokenBar 源码、资源、FFI、scanner、pricing、quota fetcher、cache 或 dashboard model。

约束：

- 第一版只面向 macOS 14+ Apple Silicon。
- Swift 开发采用 CLI-only：SwiftPM、scripts 和终端命令；不提交 `.xcodeproj`、storyboard 或 Interface Builder 产物。
- OpenMux 只有一个产品版本号：CLI、Rust crates、Menubar App、GitHub release tag 和文档中的版本均来自 Cargo workspace version。
- Menubar 是账号控制面板：查看 active account、账号池状态、quota/status，并显式切换账号。
- usage 是附属摘要：只展示 today total tokens、top client/model、freshness/coverage，不做 analytics dashboard。
- 认证材料、SQLite schema、provider endpoint 和 usage logs 不能暴露给 Swift。
- CLI 与 Menubar 必须复用同一 Rust application service，避免第二套切换、quota 或 usage 口径。
- Menubar 不能阻塞主线程执行 Rust I/O 或网络请求。

## Goals / Non-Goals

**Goals:**

- 提供可运行的原生 macOS 菜单栏 App，用最小界面完成账号查看、状态刷新和安全切换。
- 建立 CLI 与 Menubar 可复用的 Rust application service 层，先覆盖 account list、active 状态、switch、quota refresh 和最小 dashboard。
- 使用稳定、版本化、可测试的 C ABI/JSON envelope 隔离 Swift 和 Rust。
- 在失败时保留 last-known account/quota data，并清楚表达 stale/error。
- 将 TokenBar 限制为设计参考。
- 用分阶段 vertical slice 交付，先得到可用账号面板，再补 usage 摘要和发布流程。

**Non-Goals:**

- 不 fork 或持续同步完整 TokenBar。
- 不复制 TokenBar 源码或资源；未来若复制，必须先新增 reuse manifest/NOTICE 审查任务。
- 不复用 TokenBar 的 Rust FFI、scanner、pricing、quota fetcher、cache、dashboard model 或资源动画。
- 不实现 Agents、Hourly、3D graph、live trace、年度 analytics、session browser 或完整 usage dashboard。
- 不在 v1 实现 account 级历史 token 归因、`usage --group-by account` 或按 usage/quota 自动选择账号。
- 不在 v1 从 Menubar 导入账号、删除账号、编辑 alias 或执行登录流程；这些低频/高风险动作继续通过 CLI。
- 不在 v1 实现 Sparkle 自动更新、appcast、notarization 自动化或 Homebrew cask 自动 bump。
- 不实现 Windows/Linux 桌面端、daemon、local socket、云同步或团队服务。

## Decisions

### 1. 新建 OpenMux Menubar App，不 fork TokenBar

目标目录：

```text
apps/
  omx-menubar/
    Package.swift
    Sources/
      OmxMenubarApp/
        App/                 # main、AppDelegate、composition root
        Backend/             # C ABI facade、DTO、error mapping
        Features/
          Accounts/          # active account、账号池、switch flow
          Dashboard/         # quota/status 与最小 usage 摘要
          Settings/          # tray 与 refresh 设置
        Shell/               # NSStatusItem、popover、window lifecycle
        SharedUI/            # format、layout、colors
        Resources/
    Tests/
      OmxMenubarAppTests/
    scripts/

crates/
  omx-app/                   # application services/use cases；无 Swift/C ABI
  omx-menubar-ffi/           # staticlib、C header、JSON envelope
    include/omx_menubar.h
    src/{lib,envelope,runtime}.rs
```

`omx-app` 提取 CLI 中已有的账号枚举、active 状态、switch orchestration、quota refresh 和最小 usage report 组装。Swift App 不直接读 SQLite、不扫描文件、不调用 provider endpoint。

### 2. UX 参考：ClashBar 的控制台链路，TokenBar 的状态密度

ClashBar 值得借鉴的是“菜单栏一开就能完成日常控制”的产品结构：核心状态在顶部，主要操作在当前面板完成，排障和系统设置作为低频入口收到底部或次级页面。OpenMux 对应的主任务不是代理接管，而是确认 active account、判断账号是否可用、切换到另一个账号并刷新状态。

TokenBar 值得借鉴的是紧凑状态卡、数值摘要和 stale/error 的可视层级；不借鉴其 analytics lens、动画、图表和 usage-first 信息架构。OpenMux 的 first screen 必须让账号池先被看见，usage 只能辅助“要不要切换”的判断。

视觉和交互约束：

- 窗口是工具面板，不是 dashboard：宽度目标 360-420pt，高度随账号数滚动，首屏优先露出 active account 和前几个账号。
- 信息密度偏高但不拥挤：少用大标题和营销式空白，采用 macOS 原生字号、分隔线、hover/selection、secondary label。
- 状态颜色只表达健康度：success/ warning/ danger/ stale，不用装饰性渐变或强品牌色块。
- 操作按钮少而明确：Refresh、Switch、Open CLI Help、Settings、Quit；账号 CRUD 不出现在 v1。
- 文案像本地工具：短句、动词开头、错误可恢复；避免把 usage 说成账单或额度。

### 3. v1 用户链路

主链路：确认当前账号并切换。

1. 用户看到菜单栏显示 `work 42%` 或 icon-only stale indicator。
2. 点击 status item，popover 打开并立即显示 last-good account/quota。
3. Header 告诉用户当前 active account、provider、last refresh。
4. Active status 告诉用户 quota 是否充足、是否 stale、是否有可恢复错误。
5. Accounts 列表按 active first、healthy next、stale/unavailable last 排序；每行显示编号/alias/status。
6. 用户点击目标账号的 `Switch`。
7. UI 禁用重复 switch，显示 switching 状态，但 active 标记不乐观迁移。
8. backend 成功后返回权威 active account；UI 更新 active 行并触发受调度约束的 refresh。
9. 若失败，UI 保留原 active，显示安全错误和 Retry/List refresh。

辅助链路：查看用量背景。

1. 用户在账号状态下方看到 today tokens、top client/model 和 coverage。
2. usage empty/partial/stale 只影响该摘要，不影响账号切换。
3. 用户需要详细分析时，点击 `Open CLI Help` 查看 `omx usage`，不在 Menubar 展开分析页。

恢复链路：状态不可信时继续可用。

1. 首次加载失败且无 last-good：展示 unavailable、Retry、Open CLI Help。
2. refresh 失败但有 last-good：继续显示旧账号池和 quota，标记 stale/error。
3. target removed：switch 失败后刷新账号列表，解释目标已不存在。

### 4. v1 页面以账号池为中心

菜单栏 title/icon 默认显示 active alias 和最紧迫 quota/status；用户可切换为 icon-only。Popover 单页完成主要任务：

1. Header：active alias、provider、account label/plan、last refresh、手动 Refresh。
2. Active account status：quota remaining/used/reset、stale/error、credential/status diagnostic。
3. Accounts：所有可切换账号，显示 alias/number、provider、plan、active 标记、紧凑 quota/status 和 Switch 操作。
4. Usage summary：today total tokens、top client、top model、coverage/freshness。没有数据时显示 empty，不影响账号切换。
5. Footer：Open CLI Help、Settings、Quit。

导入、删除、alias 编辑和登录不放进 v1 Menubar。Footer 可以提供 CLI help 入口，说明这些动作使用 `omx login`、`omx import`、`omx alias`、`omx remove`。

### 5. Native UI 技术选择：AppKit shell + SwiftUI content

Apple UI 技术栈按平台分工：

- AppKit 是 macOS 原生 UI 框架，覆盖 `NSApplication`、`NSWindow`、`NSStatusItem`、`NSPopover`、menu、Dock、activation policy 和大量系统集成。
- SwiftUI 是跨 Apple 平台的声明式 UI 层，适合状态驱动内容视图，但在复杂 macOS menu bar shell 上仍需要 AppKit 承接生命周期和窗口行为。
- UIKit 主要服务 iOS/iPadOS/tvOS，不是 macOS menu bar app 的原生基础。
- Mac Catalyst 适合把 UIKit/iPad app 带到 macOS，不适合从零实现轻量原生菜单栏工具。

v1 采用混合方案：

- AppKit 负责 application lifecycle、`NSStatusItem`、`NSPopover`、activation policy、global event handling、popover positioning、menu bar title/icon 和 teardown。
- SwiftUI 负责 popover content、账号列表、状态卡、设置页和大部分可测试 presentation。
- `NSHostingController` 作为边界，把 SwiftUI content 放入 AppKit popover。

不采用纯 SwiftUI：`MenuBarExtra` 更适合简单 menu/action，不适合需要稳定 popover lifecycle、精细 sizing、last-good state、后台 FFI executor、复杂账号列表和安全 switch 状态的控制面板。

不采用纯 AppKit：账号列表、状态视图和空/错误/加载态用 SwiftUI 更少代码、更容易预览和测试；AppKit 全量实现会把 UI 复杂度转移到 imperative view/controller。

不采用 UIKit/Catalyst：OpenMux 没有 iPad app 可迁移，且 v1 依赖 macOS-only menu bar 行为和本地文件/Keychain 权限边界；Catalyst 会增加平台适配层而不减少核心风险。

### 6. Rust staticlib/C ABI，单入口 versioned envelope

第一版只导出：

```c
char *omx_menubar_call(const char *request_json);
void  omx_menubar_free(char *value);
```

request:

```json
{
  "schema_version": 1,
  "op": "dashboard",
  "payload": {},
  "request_id": "optional-client-id"
}
```

response:

```json
{
  "schema_version": 1,
  "ok": true,
  "data": {},
  "error": null
}
```

支持的 op：

- `dashboard`：active account、账号池摘要、quota/status、最小 today usage。
- `accounts`：账号池详情，用于刷新列表或 switch 后一致性检查。
- `switch`：platform + stable local ID 的显式切换。
- `refresh`：interactive/background refresh 请求，由 Rust 决定是否实际访问 provider。

CLI JSON 子进程只适合 spike；production 不采用，避免 PATH、取消、stdout contract 和进程启动开销。

### 7. `omx-app` 提供普通函数，不提前抽象 service trait

第一版函数：

- `menubar_dashboard(query) -> MenubarDashboardReport`
- `menubar_accounts(query) -> MenubarAccountsReport`
- `menubar_switch(command) -> MenubarSwitchReport`
- `menubar_refresh(command) -> MenubarRefreshReport`

Report 使用 OpenMux 领域语义：provider、local ID、display number、alias、active、account label、plan、auth type、quota windows、last successful refresh、diagnostic、today usage totals、freshness 和 coverage。它不包含 Swift layout、颜色、图表坐标或 TokenBar payload。

不为单一实现引入 trait；需要测试时使用临时 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 provider homes。

### 8. 安全切换完全委托 provider plugin

Swift 只提交 provider/platform 和稳定 local ID。`omx-app` 重新解析目标、确认仍存在且可用，并调用 plugin 现有安全切换流程。备份、atomic replacement、权限保护和 active registry 更新保持后端事务语义；失败时 UI 不得提前标记目标 active。

同一 runtime 内 switch single-flight。refresh 与 switch 的协调在 Rust 侧完成，防止并行 auth replacement 或旧 refresh 覆盖新 active 状态。

### 9. usage 只做最小附属摘要

Menubar 使用现有 OpenMux usage query，只读 SQLite 聚合结果。v1 只展示：

- today total tokens；
- top client；
- top model；
- freshness/coverage；
- cost status 可选，只有 coverage 明确时展示。

不展示 daily/model 图表，不提供 drill-down，不新增 account group-by，不把当前 active account 反推到历史 usage event。usage 缺失、扫描失败或 partial coverage 不能影响账号查看和切换。

### 10. refresh 调度由 Rust 决定

打开 popover、点击 Refresh 和切换后的刷新属于 `interactive`；常驻 timer 属于 `background`。Swift 只提交 refresh intent。`omx-app` 根据 provider floor、TTL、refresh attempts、429/timeout/network cooldown 和近期 activity 决定是否真正访问 provider。

v1 可以低频 polling，不要求 file watcher。idle 状态不扫描 usage logs。

### 11. Swift state 保持小而明确

Swift `BackendClient` 封装 C ABI；单一 `@MainActor AppStore` 管理 dashboard、accounts、refresh 和 switch 状态。阻塞 FFI 调用必须在 background executor 上执行。状态压缩为：

- `loading`
- `ready(report, stale?)`
- `failed(lastGood?)`

每个请求带 generation；旧请求返回时若不匹配当前 generation，必须丢弃。

### 12. 构建和发布保持独立

Rust CI 继续运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`。macOS Menubar job 单独构建 staticlib、运行 Swift tests、bundle app，并验证 C header/JSON fixtures。

版本策略保持项目级单版本：`Cargo.toml` workspace version 是唯一人工维护的产品版本源。`omx --version`、Rust crate versions、Menubar `CFBundleShortVersionString`、bundle metadata、GitHub release tag 和用户文档必须引用同一个值。Swift 源码、Info.plist 模板或 bundle script 不得维护第二个独立 app version。

根目录只维护两个 Menubar 脚本：

- `scripts/build-menubar.sh`：构建 `omx-menubar-ffi` release staticlib 并运行 `swift build --package-path apps/omx-menubar`。
- `scripts/bundle-menubar.sh`：读取 Cargo workspace version，组装 `.app`，写入 `LSUIElement=true`、`LSMinimumSystemVersion=14.0` 和版本号，并执行 ad-hoc codesign。

Developer ID、notarization、Sparkle、appcast 和 Homebrew cask 自动 bump 留到独立发布增强任务。

## Risks / Trade-offs

- [Menubar 范围继续膨胀] → v1 只允许账号查看、refresh、switch 和最小 usage 摘要；账号 CRUD 与 analytics 延后。
- [Swift 与 Rust JSON contract 漂移] → 固定 `schema_version`、additive-first、共享 golden fixtures，并在 Rust/Swift 两侧运行 contract tests。
- [提取 `omx-app` 扰动现有 CLI] → 先为现有 CLI 建 baseline，再逐个提取账号路径；CLI renderer 和参数保持不变。
- [C ABI 内存或 panic 导致进程崩溃] → 单一 free API、RAII Swift wrapper、`catch_unwind`、空指针/坏 JSON tests。
- [账号切换竞态破坏 active auth] → backend 重新校验 local ID、single-flight switch、备份与 atomic write，UI 仅在成功 report 后更新。
- [后台 refresh 请求风暴] → 所有调度决策在 Rust，Swift 不自行重试网络。
- [参考 TokenBar 时带入隐式数据依赖] → v1 不复制源码；若未来复制，先加 reuse manifest + dependency audit。

## Migration Plan

1. 固定 CLI account list/use/quota/usage baseline。
2. 新建 `omx-app`，先提取 account list、active account 和 switch orchestration。
3. 新建 `omx-menubar-ffi`，完成 envelope、panic/error、fixtures 和 `dashboard/accounts/switch/refresh` op。
4. 新建 Swift App，完成 status item、popover、mock account dashboard。
5. 接入真实 accounts/dashboard 查询，完成只读账号面板。
6. 接入显式 switch，验证备份、atomic replacement、失败回滚和 UI single-flight。
7. 接入 interactive/background refresh 与 last-good/stale 降级。
8. 增加最小 today usage 摘要；确认 usage 缺失不影响账号能力。
9. 建立 build/bundle/ad-hoc sign 流程，做内部 prerelease。

回滚策略：Menubar 是独立产物，可以停止分发而不回滚 SQLite schema、CLI 或账号数据。`omx-app` 提取必须保持 CLI tests 通过，可逐用例回退到原编排函数。

## Open Questions

- 第一版 bundle ID、应用显示名和 Developer ID team 由谁持有。
- tray 默认显示 active alias + quota signal，还是 icon-only；建议默认 active alias + 最紧迫状态，并允许设置关闭文字。
- background refresh 默认周期和不同 provider floor 的最终数值，需要以现有 quota refresh 行为测试后固定。
- Claude profile/OAuth account 是否进入同一账号列表取决于 plugin readiness；Codex 是 v1 必选 provider，Gemini 不进入 v1。
