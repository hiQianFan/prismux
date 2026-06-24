## Why

OpenMux 已具备本地 account/profile 切换、quota snapshot 和 usage event 基础，但用户仍需通过 CLI 才能判断当前账号、剩余额度和是否需要切换。既有 Menubar 蓝图与 TokenBar 调研表明，完整 fork 会引入第二套扫描、pricing、quota 和聚合实现；现在需要创建 OpenMux 自有的原生 macOS App，只参考 TokenBar 的通用 UI 思路，并让 CLI 与 Menubar 共享同一 Rust 领域层和 SQLite 事实来源。

## What Changes

- 新增独立的 `omx-menubar` macOS 14+ App，以 `NSStatusItem` + SwiftUI/AppKit popover 提供无 Dock 图标的菜单栏入口。
- Swift App 采用 CLI-only 开发流程：SwiftPM 管理源码与依赖，不提交 `.xcodeproj`/storyboard，不依赖打开 Xcode；完整 Xcode.app 只作为 macOS SDK、`xcodebuild`、codesign/notarization toolchain。
- 新增最小 Menubar 信息架构：当前 active account、账号列表、quota/reset、freshness/status、today usage total 与 top client/model。
- 支持在 Menubar 内执行显式账号切换和手动刷新；切换继续经过 OpenMux plugin/service，沿用备份、atomic write、私有权限和安全错误语义。
- 新增 OpenMux-owned Menubar application contract，通过 Rust staticlib/C ABI 与 versioned JSON envelope 向 Swift 暴露 dashboard、accounts、switch 和 refresh；Swift 不直接读取 SQLite、auth 文件或 provider endpoint。
- 将后台 refresh 与交互 refresh 分离，遵守 provider floor、TTL、失败退避和 last-known-data 降级语义。
- 新建独立 macOS 构建、测试和本地 bundle 边界，不把 Swift App 混入 CLI crate；v1 版本号统一使用仓库/Cargo workspace version。
- TokenBar 仅作为设计参考；v1 不复制其源码或资源，不 fork 完整仓库，不引入其 scanner、pricing、quota fetcher、FFI、dashboard model、live trace、Agents、3D graph 或动画资源。
- v1 不启用 Sparkle 自动更新、appcast、notarization 自动化或 Homebrew cask 自动 bump；先通过本地 bundle/GitHub Releases 手动分发验证。
- 第一版不提供自动最佳账号选择、account 级历史 token 归因、跨平台桌面端、完整 analytics dashboard 或云同步。

## Capabilities

### New Capabilities

- `native-menubar-shell`: 定义 macOS 菜单栏生命周期、popover、最小导航、加载/空/stale/error 状态和本地设置行为。
- `menubar-account-switching`: 定义账号列表、active 状态、显式安全切换、并发保护和切换结果反馈。
- `menubar-usage-overview`: 定义 quota、reset、today usage、top client/model、freshness、coverage 和 last-known-data 展示。
- `menubar-backend-contract`: 定义 Swift 与 OpenMux Rust application service 的 versioned C ABI/JSON contract、单一数据引擎和隐私边界。
- `menubar-distribution`: 定义独立 App 构建、测试、版本号、CLI-only Swift 开发、发布和第三方源码归属要求。

### Modified Capabilities

无。当前仓库尚无已归档的 Menubar capability；本 change 落实 `refine-usage-cli-and-menubar-blueprint` 中要求的独立 implementation change。

## Impact

- 新增 `apps/omx-menubar/`：Swift Package app shell、Swift DTO、最小 AppStore、views、resources 和 Swift tests。
- 新增 `crates/omx-app/`：供 CLI 与 Menubar 复用的 application services、report 组装和 account action 编排，避免把用例逻辑继续堆入 CLI 或 FFI。
- 新增 `crates/omx-menubar-ffi/`：`staticlib` C ABI、versioned envelope、panic/error 隔离和 contract tests。
- 扩展 `crates/omx-core/`：OpenMux-owned Menubar domain DTO、freshness/coverage 类型和所需聚合查询。
- 复用 `crates/omx-plugin-*` 与 `crates/omx-usage-tokscale/`，不在 Swift 或 FFI crate 中复制 provider 行为。
- 调整 workspace、CI、最小 bundle scripts 和文档；Rust CLI 的既有命令与 JSON 不产生 breaking change。
- macOS App 成为新的发布产物；v1 固定最低系统版本、arm64、本地 ad-hoc signing 和统一仓库版本号。Developer ID、notarization、Sparkle/更新机制留到公开分发阶段另行决策。
