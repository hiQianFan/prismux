## Why

OpenMux 的核心价值是本地管理、查看和切换 AI coding tool accounts。当前这些能力只能通过 CLI 完成；用户在日常使用时缺少一个常驻、低摩擦的入口来确认当前 active account、查看账号池状态、刷新 quota 并显式切换账号。Menubar v1 应该优先成为账号控制面板，而不是 usage analytics 产品。

TokenBar 调研表明，完整 fork 会引入第二套 scanning、pricing、quota 和 aggregation。OpenMux Menubar 只参考其菜单栏交互形态，不复制其数据引擎；usage 仅作为账号决策的最小附属摘要。

## What Changes

- 新增独立的 `omx-menubar` macOS 14+ App，以 `NSStatusItem` + SwiftUI/AppKit popover 提供无 Dock 图标的菜单栏入口。
- Swift App 采用 CLI-only 开发流程：SwiftPM 管理源码与依赖，不提交 `.xcodeproj`/storyboard，不依赖打开 Xcode；完整 Xcode.app 只作为 macOS SDK、`xcodebuild`、codesign/notarization toolchain。
- 新增账号优先的信息架构：当前 active account、账号池、每账号状态、quota/reset、freshness/status、显式 switch 和手动 refresh。
- usage 只保留最小辅助信息：today total tokens、top client/model 和 freshness/coverage；不提供图表、历史 drill-down、account usage attribution 或完整 analytics。
- 支持在 Menubar 内执行显式账号切换和手动刷新；切换继续经过 OpenMux plugin/service，沿用备份、atomic write、私有权限和安全错误语义。导入、删除、改 alias 等高风险/低频管理动作 v1 继续交给 CLI。
- 新增 OpenMux-owned Menubar application contract，通过 Rust staticlib/C ABI 与 versioned JSON envelope 向 Swift 暴露 dashboard、accounts、switch 和 refresh；Swift 不直接读取 SQLite、auth 文件、provider endpoint 或 usage logs。
- 将后台 refresh 与交互 refresh 分离，遵守 provider floor、TTL、失败退避和 last-known-data 降级语义。
- 新建独立 macOS 构建、测试和本地 bundle 边界，不把 Swift App 混入 CLI crate；OpenMux 项目只维护一个产品版本号，CLI、Rust crates、Menubar App 和 release tag 统一使用仓库/Cargo workspace version。
- TokenBar 仅作为设计参考；v1 不复制其源码或资源，不 fork 完整仓库，不引入其 scanner、pricing、quota fetcher、FFI、dashboard model、live trace、Agents、3D graph 或动画资源。
- v1 不启用 Sparkle 自动更新、appcast、notarization 自动化或 Homebrew cask 自动 bump；先通过本地 bundle/GitHub Releases 手动分发验证。
- 第一版不提供自动最佳账号选择、account 级历史 token 归因、跨平台桌面端、完整 analytics dashboard 或云同步。

## Capabilities

### New Capabilities

- `native-menubar-shell`: 定义 macOS 菜单栏生命周期、popover、最小导航、加载/空/stale/error 状态和本地设置行为。
- `menubar-account-switching`: 定义账号池查看、active 状态、显式安全切换、并发保护和切换结果反馈。
- `menubar-usage-overview`: 定义账号决策所需的 quota、reset、最小 today usage、freshness、coverage 和 last-known-data 展示。
- `menubar-backend-contract`: 定义 Swift 与 OpenMux Rust application service 的 versioned C ABI/JSON contract、单一数据引擎和隐私边界。
- `menubar-distribution`: 定义独立 App 构建、测试、版本号、CLI-only Swift 开发、发布和第三方源码归属要求。

### Modified Capabilities

无。当前仓库尚无已归档的 Menubar capability；本 change 落实 `refine-usage-cli-and-menubar-blueprint` 中要求的独立 implementation change。

## Impact

- 新增 `apps/omx-menubar/`：Swift Package app shell、Swift DTO、最小 AppStore、views、resources 和 Swift tests。
- 新增 `crates/omx-app/`：供 CLI 与 Menubar 复用的 application services、report 组装和 account action 编排，避免把用例逻辑继续堆入 CLI 或 FFI。
- 新增 `crates/omx-menubar-ffi/`：`staticlib` C ABI、versioned envelope、panic/error 隔离和 contract tests。
- 扩展 `crates/omx-core/`：OpenMux-owned Menubar domain DTO、账号状态、freshness/coverage 类型和最小 usage 聚合查询。
- 复用 `crates/omx-plugin-*` 与 `crates/omx-usage-tokscale/`，不在 Swift 或 FFI crate 中复制 provider 行为。
- 调整 workspace、CI、最小 bundle scripts 和文档；Rust CLI 的既有命令与 JSON 不产生 breaking change。
- macOS App 成为新的发布产物；v1 固定最低系统版本、arm64、本地 ad-hoc signing，并使用与 CLI 相同的仓库/Cargo workspace version。Developer ID、notarization、Sparkle/更新机制留到公开分发阶段另行决策。
