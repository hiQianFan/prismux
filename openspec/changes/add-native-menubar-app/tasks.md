## 1. 基线、范围与目录脚手架

- [ ] 1.1 为现有 account list/use、quota refresh、usage query 和 JSON output 建立或补齐 integration baseline，确保提取 application service 不改变 CLI 行为。
- [ ] 1.2 在根 workspace 新增 `crates/omx-app` 与 `crates/omx-menubar-ffi`，保持 `apps/omx-menubar` 不属于 Cargo members。
- [ ] 1.3 创建 `apps/omx-menubar` Swift Package/App 目录、Sources/Tests/Resources 骨架和 macOS 14 arm64 build target；不提交 `.xcodeproj`、storyboard 或 Interface Builder 产物。
- [ ] 1.4 记录 TokenBar 参考 commit 和 v1 不复制源码/资源规则；未来若复制文件再新增 reuse manifest/NOTICE 任务。
- [ ] 1.5 记录 v1 明确范围：Codex 必选，Claude/Gemini 按 plugin readiness gate；Agents/Hourly/3D/live trace/动画和自动切换保持关闭。
- [ ] 1.6 记录本地 Swift 开发环境要求：完整 Xcode.app 作为 CLI toolchain/SDK，开发入口只使用 SwiftPM、scripts 和终端命令。

## 2. OpenMux Application Service 层

- [ ] 2.1 在 `omx-core` 定义 Menubar 所需的 account、quota window、freshness、coverage、diagnostic、today usage 和 cost status DTO，不引用 Swift/TokenBar/tokscale 类型。
- [ ] 2.2 在 `omx-app` 定义 `dashboard`、`list_accounts`、`switch_account`、`refresh` request/report 与普通函数入口；不为单一实现提前引入 trait。
- [ ] 2.3 从 `omx-cli/src/app.rs` 提取账号枚举与 active 状态组装到 `omx-app`，保持 CLI renderer/参数层不变并通过 baseline tests。
- [ ] 2.4 提取 provider plugin 选择、selector/local ID 解析和安全 switch orchestration，确保 CLI 与 Menubar 共用实现。
- [ ] 2.5 提取 interactive/background refresh 编排，接入 refresh attempts、provider floor、TTL、429/timeout/network cooldown 和 single-flight。
- [ ] 2.6 组装 dashboard report，将 active account、quota snapshot/current diagnostic、refreshed time 与 today usage/freshness/coverage 合并但保持不同口径。
- [ ] 2.7 为 application service 增加 unavailable plugin、removed target、no active account、empty state、stale snapshot 和 partial coverage 单元测试。
- [ ] 2.8 增加并发测试，验证 refresh 与 switch 不产生并行 auth replacement，失败时 active account 不变。

## 3. Usage 聚合与数据质量

- [ ] 3.1 扩展 StateStore/query service，按用户本地自然日聚合 total tokens，并测试时区、DST、跨日和空数据边界。
- [ ] 3.2 增加 top client 与 top model 的确定性排序和 tie-break，保证 group rows 与 totals 一致且 unknown model 不丢 token。
- [ ] 3.3 计算 freshness 与 requested/available/missing source coverage，区分 empty、partial、scan failed 和 stale。
- [ ] 3.4 汇总 estimated/provider-reported/missing cost coverage，partial/missing 时禁止生成无歧义 `$0.00` total。
- [ ] 3.5 增加 attribution 回归测试：切换 active account 后，无 evidence 的历史 event 仍为 unknown，不进入具名账号 usage。
- [ ] 3.6 验证 dashboard query 只读取 SQLite 聚合结果，不在 UI query 路径实时全量扫描 provider logs。

## 4. Rust C ABI 与 Contract

- [ ] 4.1 在 `omx-menubar-ffi/include/omx_menubar.h` 定义 `omx_menubar_call(request_json)` 和 `omx_menubar_free(value)` 接口及内存所有权文档。
- [ ] 4.2 实现 `schema_version = 1` 的 success/error envelope，稳定 error code、retryable 和 operation ID，采用 additive-first 字段规则。
- [ ] 4.3 建立 FFI runtime/composition root，连接 `omx-app`、provider plugins、StateStore 和 usage adapter，并遵守 `OMUX_STATE_ROOT`、`CODEX_HOME` 等 override。
- [ ] 4.4 为每个 entry point 增加 `catch_unwind`、null/bad UTF-8/bad JSON 处理，确保 panic 不跨 C ABI。
- [ ] 4.5 实现 Rust 字符串 allocation/free，并测试 success、application error、decode error 和 panic 路径无泄漏/double-free。
- [ ] 4.6 为单一 entry point 的 dashboard/accounts/switch/refresh `op` 建立脱敏 golden JSON fixtures，不包含 raw auth、token、API key、raw log、email 默认值或未脱敏路径。
- [ ] 4.7 增加 ABI contract tests，验证 unsupported schema、unknown optional field、removed target、stale/partial/error envelope。
- [ ] 4.8 生成 release staticlib/header，并验证 Swift Package 能在干净 arm64 macOS 环境链接。

## 5. 原生 App Shell 与 Backend Client

- [ ] 5.1 实现 accessory app lifecycle、单一 `NSStatusItem`、单一 popover hosting controller 和 clean teardown。
- [ ] 5.2 实现 Swift `BackendClient`/RAII response wrapper，所有成功、decode failure 和 cancellation 路径恰好调用一次 Rust free。
- [ ] 5.3 定义 Swift DTO 与 decoder，使用 Rust golden fixtures 做双端 contract tests并忽略同 major version 的未知 optional 字段。
- [ ] 5.4 实现单一 `@MainActor AppStore` 和 background FFI executor，禁止 main actor 执行阻塞调用。
- [ ] 5.5 增加 request generation/cancellation guard，验证旧 dashboard/refresh response 不覆盖较新 switch 或筛选状态。
- [ ] 5.6 实现 `loading`、`ready(report, stale?)`、`failed(lastGood?)` 三态模型及 last-good report 保留；refreshing/partial/empty 作为 report 字段或 view 派生状态。
- [ ] 5.7 实现 OpenMux 自有 UserDefaults namespace 下的 tray mode 和 background cadence；不加入 update preference。

## 6. Menubar MVP 界面

- [ ] 6.1 实现 header：OpenMux brand、active alias/provider、last refresh、refresh progress 和手动 Refresh。
- [ ] 6.2 实现 quota section：主要窗口 remaining/used、reset、refreshed time、stale/current diagnostic 和无 quota 状态。
- [ ] 6.3 实现 accounts section：active 标记、alias/plan、紧凑 quota/status 和明确 switch action。
- [ ] 6.4 实现 today usage section：total tokens、top client/model、cost status、freshness 和 partial/missing coverage。
- [ ] 6.5 实现 footer：Settings、CLI/help 入口和 Quit，并保证键盘 focus/VoiceOver label/基本 Dynamic Type 可用。
- [ ] 6.6 实现 tray icon/title：active alias 与最紧迫 quota signal；无数据、stale 和 icon-only mode 有稳定安全 fallback。
- [ ] 6.7 建立最小手工 UI checklist 覆盖窄宽度、长 alias、多 quota window、空数据、partial、stale 和 error；SwiftUI preview/snapshot 不作为 v1 必选。

## 7. 安全账号切换与刷新交互

- [ ] 7.1 实现用户显式 switch flow，提交 platform + stable local ID，不向 Swift 暴露 auth path/payload。
- [ ] 7.2 switch 期间禁用重复操作并显示进行态；仅在权威 success report 后更新 active 标记。
- [ ] 7.3 处理 target removed、backup failed、atomic replacement failed、permission denied 和 registry commit failed，保留原 active UI。
- [ ] 7.4 switch 成功后触发受 provider floor/TTL 管理的 dashboard/quota refresh，不由 Swift 直接访问 provider。
- [ ] 7.5 将 popover open/manual refresh/switch preflight 映射为 interactive refresh，将 timer 映射为 background refresh。
- [ ] 7.6 验证 Swift 高频 timer、重复打开 popover和网络错误不会绕过 backend cooldown 或形成重试风暴。

## 8. TokenBar 参考与合规

- [ ] 8.1 固定 TokenBar 参考 commit，并记录仅参考 `NSStatusItem` lifecycle、popover chrome 和 layout 思路，不复制源码或资源。
- [ ] 8.2 使用 search audit 证明 Swift target 不引用 `TBCore`、TokenBar report DTO、TokenBar scanner/pricing/quota/cache、bundle ID、UserDefaults key 或品牌资源。
- [ ] 8.3 审计第三方 asset；没有明确 license/NOTICE 的资源不得进入 release bundle。
- [ ] 8.4 若未来确需复制 TokenBar 文件，先新增 reuse manifest、`ThirdPartyNotices/TokenBar.md` 和 dependency audit 任务。

## 9. 测试、性能与隐私

- [ ] 9.1 使用临时 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 provider homes 运行 dashboard/accounts/refresh/switch 跨层 integration tests，不访问用户真实状态。
- [ ] 9.2 增加真实文件操作回归：切换前备份、atomic replacement、私有权限、失败回滚和 active registry 一致性。
- [ ] 9.3 增加 Swift state/concurrency tests：slow response、out-of-order response、cancel、double click、popover reopen 和 last-good fallback。
- [ ] 9.4 记录 cold/warm dashboard latency、popover first-content latency、idle CPU wakeup、memory 和后台 refresh 网络次数基线。
- [ ] 9.5 增加 privacy regression，扫描 Rust envelope、Swift log、crash diagnostic 和 bundle fixture，禁止 raw auth/token/API key/raw provider response/raw log。
- [ ] 9.6 运行 sleep/wake、popover lifecycle、SQLite contention 和网络恢复手工 smoke test；24 小时 soak 不作为 v1 gate。

## 10. 构建、签名、更新与发布

- [ ] 10.1 建立 macOS CI job：构建 Rust staticlib、运行 Rust tests、Swift build/tests、link smoke 和 app bundle smoke。
- [ ] 10.2 实现 `scripts/build-menubar.sh`，构建 `omx-menubar-ffi` release staticlib 并运行 `swift build --package-path apps/omx-menubar`。
- [ ] 10.3 实现 `scripts/bundle-menubar.sh`，读取 Cargo workspace version，固定 bundle ID、version、minimum macOS、arm64 architecture、embedded library、resource 清单和 `LSUIElement=true`。
- [ ] 10.4 建立本地 ad-hoc signing 并验证 `codesign --verify`；Developer ID、hardened runtime、entitlements 和 notarization 留到公开分发增强任务。
- [ ] 10.5 v1 通过本地 bundle/GitHub Releases 手动分发；不实现 Sparkle、appcast、自动更新或 Homebrew cask 自动 bump。
- [ ] 10.6 审计最终 `.app` linked symbols/resources，证明未包含 TokenBar `tb_core_ffi`、vendored tokscale、独立 quota fetcher、动画或第二份 usage cache。
- [ ] 10.7 创建内部 prerelease，执行安装、首次启动、更新/卸载和 CLI 独立运行验证后再批准公开 release。

## 11. 文档与交付验收

- [ ] 11.1 更新 `docs/ARCHITECTURE.md`，记录 `omx-core -> omx-app -> omx-menubar-ffi -> Swift` 边界和目录责任。
- [ ] 11.2 更新 `docs/PRD.md` 和用户文档，说明 Menubar MVP、macOS/arm64 要求、parsed usage 与 quota 区别及非账单级准确性。
- [ ] 11.3 文档化 macOS 本地文件/Keychain 权限、签名/notarization、日志脱敏和故障排查，不要求用户分享 auth 文件。
- [ ] 11.4 完成 release NOTICE review；若 v1 未复制第三方源码/资源，则不新增 `ThirdPartyNotices/TokenBar.md`。
- [ ] 11.5 运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`、Swift format/build/test 和 OpenSpec validation。
- [ ] 11.6 按五个 capability specs 执行 acceptance review，确认不含自动切换、伪 account attribution、第二数据引擎或被排除 analytics 功能。
