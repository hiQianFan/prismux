## Context

OpenMux 当前是 Rust workspace：`omx-core` 保存领域类型、SQLite state 和 plugin trait，provider crates 实现账号导入/切换/quota，`omx-usage-tokscale` 负责本地 usage 解析，`omx-cli` 同时承担命令映射、部分应用编排和展示。已有 `refine-usage-cli-and-menubar-blueprint` 决定 Menubar 与 CLI 共享 OpenMux 数据引擎，并要求完整 Menubar 通过独立 change 交付。

对 TokenBar commit `813ff11aeba40bd48a25333f0b1aa1df7bfffc6a` 的调研显示：其原生 `NSStatusItem`/SwiftUI shell 和部分 layout 组件有参考价值，但 53 个 Swift 文件中多数依赖 TokenBar 自有 report model；Rust FFI、vendored tokscale、quota fetcher、cache 和 dashboard state 与 OpenMux 重叠，且部分 private/internal endpoint 和 auth 写入方式不符合本项目安全规则。因此本设计不 fork 完整 TokenBar，而创建 OpenMux 自有 App；v1 只把 TokenBar 作为设计参考，不复制其源码或资源。

约束：

- 第一版只面向 macOS 14+ Apple Silicon。
- Swift 开发采用 CLI-only：SwiftPM 管理源码、构建和测试，不提交 `.xcodeproj`、storyboard 或 Interface Builder 产物；开发者可安装完整 Xcode.app 作为 toolchain，但不要求打开 Xcode。
- CLI、Menubar App 和 release tag 使用同一仓库/Cargo workspace version。
- 认证材料、SQLite schema 和 provider endpoint 不能暴露给 Swift。
- CLI 与 Menubar 必须得到一致的 account/quota/usage 口径。
- Menubar 不能阻塞主线程执行 Rust I/O 或网络请求。
- 用户现有 worktree 和 CLI 行为必须可独立演进；Menubar 不应成为运行 CLI 的前置条件。

## Goals / Non-Goals

**Goals:**

- 提供可发布的原生 macOS 菜单栏 App，用最小界面完成查看状态和安全切换账号。
- 建立可由 CLI、Menubar 和未来本地入口复用的 Rust application service 层。
- 使用稳定、版本化、可测试的 C ABI/JSON envelope 隔离 Swift 和 Rust。
- 保持 SQLite 为 usage/quota/history 唯一事实来源，并在失败时保留 last-known data。
- 将 TokenBar 限制为设计参考；若未来复制源码或资源，再按 MIT attribution 和 reuse manifest 审查。
- 将目录、构建、测试和发布责任固定下来，使实现可以分阶段交付。

**Non-Goals:**

- 不 fork 或持续同步完整 TokenBar。
- v1 不复制 TokenBar 源码或资源；未来若复制，必须先新增 reuse manifest/NOTICE 审查任务。
- 不复用 TokenBar 的 Rust FFI、scanner、pricing、quota fetcher、cache、dashboard model 或资源动画。
- 不实现 Agents、Hourly、3D graph、live trace、年度 analytics 或完整 session browser。
- 不在 v1 实现 Sparkle 自动更新、appcast、notarization 自动化或 Homebrew cask 自动 bump。
- 不自动选择或切换“最佳账号”。
- 不根据当前 active account 反推历史 token event 归属。
- 不实现 Windows/Linux 桌面端、daemon、local socket、云同步或团队服务。

## Decisions

### 1. 新建 App，而不是 fork TokenBar

仓库采用以下目标目录：

```text
apps/
  omx-menubar/
    Package.swift
    Sources/
      OmxMenubarApp/
        App/                 # main、AppDelegate、composition root
        Backend/             # C ABI facade、DTO、error mapping
        Features/
          Dashboard/         # 当前账号、quota、usage overview
          Accounts/          # 账号列表与 switch flow
          Settings/          # refresh/tray settings
        Shell/               # NSStatusItem、popover、window lifecycle
        SharedUI/            # cards、format、layout、colors
        Resources/
    Tests/
      OmxMenubarAppTests/
    scripts/                 # 本地 build/bundle；不含 appcast/notarize 自动化

crates/
  omx-app/                   # application services/use cases；无 Swift/C ABI
  omx-menubar-ffi/           # staticlib、C header、JSON envelope
    include/omx_menubar.h
    src/{lib,envelope,runtime}.rs

crates/omx-core/             # domain/state/query primitives
crates/omx-plugin-*/         # provider-specific behavior
crates/omx-usage-tokscale/   # usage ingestion adapter
crates/omx-cli/              # CLI 参数与 renderer
```

`apps/` 表示独立发布产品；Rust crates 继续留在 workspace。Swift App 不作为 Cargo member，也不提交 Xcode project 文件；本地和 CI 都通过 `swift build --package-path apps/omx-menubar` 驱动。`omx-app` 把 scan/query/report/switch orchestration 从 CLI presentation 中提取出来；`omx-menubar-ffi` 只是 composition root 和语言桥，不承载 SQL 或 provider 逻辑。

备选方案：把所有编排放进 `omx-core`。未选择，因为 `omx-core` 当前定位是共享类型、存储和 plugin trait，继续加入 UI 用例会扩大核心职责。备选方案：直接在 FFI 调用各 plugin。未选择，因为会让 CLI 与 Menubar 再次形成两套应用流程。

### 2. 使用 Rust staticlib/C ABI，不使用 CLI 子进程或 daemon

第一版只导出一个业务入口和一个 free 函数：

```c
char *omx_menubar_call(const char *request_json);
void  omx_menubar_free(char *value);
```

所有调用都是阻塞函数，Swift 必须在非 MainActor executor 上调用。每个返回值使用统一 envelope：

```json
{
  "schema_version": 1,
  "op": "dashboard",
  "payload": {},
  "request_id": "optional-client-id"
}
```

每个返回值使用统一 envelope：

```json
{
  "schema_version": 1,
  "ok": true,
  "data": {},
  "error": null
}
```

错误对象只包含稳定 `code`、安全 message、`retryable` 和可选 operation ID。Rust entry point 使用 `catch_unwind`，panic 不跨 C ABI；Swift 在每条路径恰好调用一次 `omx_menubar_free`。

CLI JSON 仅适合 spike，会增加进程启动、PATH、取消和 stdout 契约问题。local socket/daemon 在第一版没有足够收益，却会引入生命周期、权限和升级协议，因此不采用。

### 3. `omx-app` 提供四类普通函数，不提供 UI view model

`omx-app` 第一版提供普通 Rust 函数，不为单一实现提前引入 service trait：

- `dashboard(query) -> MenubarDashboardReport`
- `list_accounts(query) -> AccountListReport`
- `switch_account(command) -> SwitchAccountReport`
- `refresh(command) -> RefreshReport`

Report 使用 OpenMux 领域语义：account local ID、alias、active、plan、quota windows、last successful refresh、current diagnostic、today usage totals、top client/model、freshness、coverage、cost status。它不包含 Swift layout、颜色、图表坐标或 TokenBar payload。

CLI 可以逐步复用相同 service，但本 change 不要求一次性重写全部 CLI command。Menubar 所需路径必须先从 `omx-cli/src/app.rs` 中提取，避免复制。

### 4. Swift 使用单一 AppStore/generation 管理并发与 last-good state

Swift `BackendClient` 封装 C ABI；单一 `@MainActor @Observable AppStore` 管理 dashboard、accounts、refresh 和 switch 状态。实际调用通过受控 background task 执行，并为每个请求记录 generation：旧请求在账号切换或 filter 改变后返回时不得覆盖新状态。

状态先压缩为：`loading`、`ready(report, stale?)`、`failed(lastGood?)`。`refreshing`、`partial`、`empty` 作为 report 字段或 view 派生状态处理，不建立独立状态机。refresh 失败时保留最近成功 report；切换期间禁用重复 switch，并在 backend 确认成功后才更新 active 标记。

### 5. 第一版信息架构服务“判断并切换”，不复制 analytics dashboard

菜单栏 title/icon 默认展示 active account 的最紧迫 quota signal；无 quota 时显示 OpenMux 图标和安全占位。Popover 使用单页渐进披露：

1. Header：active alias、provider、last refresh、手动刷新。
2. Quota：主要窗口 remaining/used、reset time、stale/error。
3. Accounts：可切换账号列表、active 标记、每账号紧凑 quota/status。
4. Today usage：total tokens、top client/model、cost status、coverage。
5. Footer：Settings、Open CLI Help、Quit。

第一版不提供 TokenBar 六 lens。未来 daily/model 图表只能通过新增 OpenMux query contract 扩展，不能让 Swift 直接 SQL。

### 6. 后台调度复用 OpenMux refresh 语义

打开 popover、点击刷新和切换前检查属于 `interactive`；常驻 timer 属于 `background`。`omx-app` 根据 refresh attempts、provider floor、TTL、429/timeout/network cooldown 和近期 activity 决定是否真正访问 provider。Swift timer 只是提出 refresh 请求，不能绕过 backend 调度。

本地 usage 扫描按 bounded scan + watermark 执行；第一版可以低频 polling，不要求 file watcher。UI query 只读取已聚合结果，扫描和网络不能发生在主线程。

### 7. 安全切换完全委托 provider plugin

Swift 只提交 platform 和稳定 local ID，不提交文件路径或 auth payload。`omx-app` 重新解析目标、确认仍存在且可用，并调用 plugin 的现有安全切换流程。切换前备份、atomic replacement、权限保护和 active registry 更新必须保持一个后端事务语义；失败时不得在 UI 中提前标记新 active account。

自动切换、基于 quota 的推荐切换和并发批量切换不进入第一版。

### 8. TokenBar 只作为设计参考

v1 不复制 TokenBar 源码或资源，不建立 upstream merge/cherry-pick 关系。实现可以参考其 `NSStatusItem` 生命周期、popover chrome 和基础 layout 思路，但用 OpenMux 自有 Swift 文件重新实现。

不复制 `tb_core_ffi`、`TBCore.swift`、`DashboardModel`、quota/trace/Agents/3D graph、scanner/pricing/cache 或动画。若未来确实需要复制某个文件，必须先补充 reuse manifest、NOTICE 和 dependency audit，并证明该文件不依赖 TokenBar 数据模型或品牌资源。

### 9. 构建和发布保持独立但可由根 CI 编排

Rust CI 继续运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`。macOS job 额外构建 staticlib、运行 Swift tests、bundle app，并验证 C header/JSON fixtures。v1 本地开发允许 ad-hoc signing，公开发布先走 GitHub Releases 手动下载；Developer ID、notarization、Sparkle、appcast 和 Homebrew cask 自动 bump 留到独立发布增强任务。

根目录只维护两个 Menubar 脚本：

- `scripts/build-menubar.sh`：构建 `omx-menubar-ffi` release staticlib 并运行 `swift build --package-path apps/omx-menubar`。
- `scripts/bundle-menubar.sh`：读取 Cargo workspace version，构建 release Swift executable，组装 `.app`，写入 `Info.plist`，设置 `LSUIElement=true`、`LSMinimumSystemVersion=14.0`、`CFBundleShortVersionString=<workspace version>`，并执行 ad-hoc codesign。

第一版不要求 Mac App Store sandbox，因为需要读取 provider 本地状态并替换 CLI auth；发布前必须记录所需文件/Keychain 权限和用户可见说明。

## Risks / Trade-offs

- [Swift 与 Rust JSON contract 漂移] → 固定 `schema_version`、additive-first、共享 golden fixtures，并在 Rust/Swift 两侧运行 contract tests。
- [提取 `omx-app` 扰动现有 CLI] → 先为现有 CLI 建 integration baseline，再逐个提取用例；CLI renderer 和参数保持不变。
- [C ABI 内存或 panic 导致进程崩溃] → 单一 free API、RAII Swift wrapper、`catch_unwind`、空指针/重复调用/坏 JSON tests。
- [后台刷新形成 provider 请求风暴] → 所有调度决策在 Rust，使用 provider floor、TTL、backoff、single-flight；Swift 不自行重试网络。
- [账号切换竞态破坏 active auth] → backend 重新校验 local ID、single-flight switch、备份与 atomic write，UI 仅在成功 report 后更新。
- [参考 TokenBar 时带入隐式数据依赖] → v1 不复制源码；若未来复制，必须先加 reuse manifest + dependency audit。
- [macOS 发布链路延误] → 第一个 vertical slice 就建立 build/bundle/ad-hoc sign smoke，不把可运行 `.app` 留到最后。
- [Swift CLI-only 开发踩到 Xcode 隐式依赖] → 以 SwiftPM 和 scripts 为唯一开发入口；完整 Xcode 只作为 SDK/toolchain，CI 用同一路径验证。
- [Apple Silicon 限制用户范围] → 第一版明确 arm64；稳定后再以独立任务评估 universal binary。
- [App 常驻带来资源消耗] → idle 状态不扫描，低频 query，记录 cold/warm latency、CPU wakeup 和 memory 基线。

## Migration Plan

1. 固定 Rust/CLI 现状测试，并建立 `omx-app` application service crate。
2. 定义 Menubar DTO、C ABI header、Rust golden fixtures 和最小 Swift decoder test。
3. 建立空的 `apps/omx-menubar`，完成 status item → popover → mock dashboard vertical slice。
4. 接入真实 dashboard/accounts 查询，再实现 refresh 与安全 switch action。
5. 参考 TokenBar 的 shell/layout 思路，用 OpenMux 自有 Swift 文件替换 mock presentation。
6. 完成 stale/partial/error、并发、隐私和临时 state root 集成测试。
7. 建立 build/bundle/ad-hoc sign 流程，先发布内部或 prerelease build。
8. 运行完整 Rust/Swift/手工验证后再启用公开分发。

回滚策略：Menubar 是独立产物，可以停止分发或关闭更新 feed，而不回滚 SQLite schema、CLI 或账号数据。若 C ABI 出现严重问题，可保留 App shell 并暂时切回 mock/只读 unavailable 状态；不得用 CLI subprocess 作为静默长期 fallback。`omx-app` 的提取必须保持 CLI tests 通过，可逐用例回退到原编排函数。

## Open Questions

- 第一版 bundle ID、应用显示名和 Developer ID team 由谁持有。
- tray 默认显示 active alias、quota percent 还是仅图标；建议首次运行默认 alias + 最紧迫 quota，允许设置关闭文字。
- background refresh 默认周期和不同 provider floor 的最终数值，需要以现有 quota refresh 行为和请求限制测试后固定。
- Claude profile 和未来 Gemini account 在第一版是否与 Codex 同时进入切换 UI，取决于各 plugin 的 list/use/report 契约完成度。
