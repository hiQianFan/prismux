## 1. Baseline 与范围锁定

- [x] 1.1 为现有 `omx list`、`omx list codex`、`omx use codex <selector>`、quota refresh、`omx usage --json --no-scan` 建立 integration baseline。
- [x] 1.2 记录 v1 范围：Menubar 主功能是账号池查看、active 状态、quota/status、手动 refresh 和显式 switch；usage 仅为 today 摘要。
- [x] 1.3 记录 v1 非范围：TokenBar fork、账号登录/导入/删除/alias 编辑、自动最佳账号选择、account usage attribution、analytics dashboard、Sparkle/notarization 自动化。
- [x] 1.4 固定 provider gate：Codex 必选；Claude 仅在现有 plugin account list/switch/report 能力满足 contract 时进入；Gemini 不进入 v1。
- [x] 1.5 产出一张低保真 wireframe 或文字布局清单，覆盖菜单栏 title、active account、accounts list、usage summary、settings/footer。
- [x] 1.6 记录 UX inspiration：ClashBar 借鉴常驻控制台链路，TokenBar 借鉴紧凑状态卡和 stale/error 层级；不得复制源码、资源或数据模型。

## 2. OpenMux Application Service

- [x] 2.1 新增 `crates/omx-app`，提供普通函数而非提前抽象 trait。
- [x] 2.2 从 CLI 编排中提取 account list、active account、selector/local ID 解析和安全 switch orchestration，保持 CLI 参数与输出不变。
- [x] 2.3 定义 Menubar account DTO：provider、display number、local ID、alias、account label、plan、auth type、active、quota/status、diagnostic。
- [x] 2.4 实现 `menubar_accounts(query)`，返回账号池、唯一 active 标记、stale/unavailable 状态和安全 diagnostics。
- [x] 2.5 实现 `menubar_switch(command)`，只接受 provider + stable local ID，后端重新解析目标并调用 plugin 安全切换流程。
- [x] 2.6 实现 switch single-flight，验证并发 switch/refresh 不会产生并行 auth replacement。
- [x] 2.7 实现 `menubar_refresh(command)`，区分 interactive/background，并遵守 provider floor、TTL、backoff 和 last-known-data。
- [x] 2.8 实现 `menubar_dashboard(query)`，组合 active account、账号池摘要、quota/status 和最小 today usage。
- [x] 2.9 增加 unavailable plugin、removed target、no active account、stale quota、refresh failure、switch rollback 的单元/集成测试。

## 3. 最小 Usage 摘要

- [x] 3.1 复用现有 OpenMux usage query，按用户本地自然日返回 total tokens、top client、top model、freshness 和 coverage。
- [x] 3.2 确认 usage query 只读 SQLite 聚合结果，不在 dashboard 查询路径实时全量扫描 provider logs。
- [x] 3.3 增加回归测试：usage empty、scan failed、partial coverage 时，账号列表和 switch 仍可用。
- [x] 3.4 增加回归测试：缺少可验证 account/profile evidence 时，Menubar report 不输出 account usage attribution。
- [x] 3.5 cost 仅在 coverage 明确时展示；partial/missing 不得显示为无歧义 `$0.00`。

## 4. Rust C ABI 与 Contract

- [x] 4.1 新增 `crates/omx-menubar-ffi`，定义 `omx_menubar_call(request_json)` 和 `omx_menubar_free(value)`。
- [x] 4.2 实现 `schema_version = 1` 的 request/response envelope，支持 `dashboard`、`accounts`、`switch`、`refresh` op。
- [x] 4.3 实现 FFI runtime/composition root，连接 `omx-app`、provider plugins、StateStore 和 usage adapter，并遵守 `OMUX_STATE_ROOT`、`CODEX_HOME` 等 override。
- [x] 4.4 捕获 panic、null pointer、bad UTF-8、bad JSON、unknown op 和 unsupported schema，返回安全 error envelope。
- [x] 4.5 实现 Rust 字符串 allocation/free，并测试 success、decode error、application error 和 panic 路径无泄漏/double-free。
- [x] 4.6 建立脱敏 golden JSON fixtures，覆盖 accounts/dashboard/switch/refresh，不包含 raw auth、token、API key、raw log、email 默认值或未脱敏路径。
- [x] 4.7 增加 Rust contract tests，验证 additive optional field、removed target、stale quota、usage empty 和 switch failure。

## 5. Swift App Shell

- [x] 5.1 新增 `apps/omx-menubar` Swift Package/App，macOS 14 arm64，SwiftPM 驱动，不提交 `.xcodeproj`、storyboard 或 Interface Builder 产物。
- [x] 5.2 使用 AppKit 实现 accessory app lifecycle、单一 `NSStatusItem`、`NSPopover`、activation policy、popover positioning 和 clean teardown。
- [x] 5.3 使用 `NSHostingController` 承载 SwiftUI popover content；SwiftUI 负责账号列表、状态卡、settings 和空/错误/加载态。
- [x] 5.4 实现 Swift `BackendClient`/RAII response wrapper，所有成功、decode failure 和 cancellation 路径恰好调用一次 Rust free。
- [x] 5.5 定义 Swift DTO 与 decoder，使用 Rust golden fixtures 做 contract tests，并忽略同 major version 的未知 optional 字段。
- [x] 5.6 实现单一 `@MainActor AppStore`、background FFI executor、generation guard 和 last-good state。
- [x] 5.7 实现 `loading`、`ready(report, stale?)`、`failed(lastGood?)` 三态；refreshing/empty/partial 作为 report 字段或 view 派生状态。
- [x] 5.8 验证不采用纯 SwiftUI `MenuBarExtra` 作为 v1 shell，除非后续 spike 证明它能覆盖 popover lifecycle、sizing、generation 和 FFI 调度需求。
- [x] 5.9 验证不引入 UIKit 或 Mac Catalyst lifecycle；记录 AppKit 是 macOS menu bar shell 的基础，SwiftUI 仅承载 content views。

## 6. 账号控制面板 UI

- [x] 6.1 实现 tray icon/title：active alias + 最紧迫 quota/status；无数据、stale 和 icon-only mode 有稳定 fallback。
- [x] 6.2 实现 header：OpenMux brand、active alias/provider、account label/plan、last refresh、Refresh。
- [x] 6.3 实现 active account status：quota remaining/used/reset、refreshed time、stale/error 和 credential/status diagnostic。
- [x] 6.4 实现 accounts section：display number、alias/account label、provider、plan/auth type、active 标记、紧凑 quota/status 和 Switch 操作。
- [x] 6.5 switch 期间禁用重复操作；仅在后端 success report 后更新 active 标记，失败时保留原 active UI。
- [x] 6.6 实现 usage summary：today total tokens、top client、top model、freshness/coverage；empty/partial 不影响账号 section。
- [x] 6.7 实现 footer：Open CLI Help、Settings、Quit；CLI help 指向登录、导入、alias、删除等 v1 未内置动作。
- [x] 6.8 保证键盘 focus、VoiceOver label、基本 Dynamic Type、长 alias、窄宽度和空账号池状态可用。
- [x] 6.9 建立手工 UX checklist：首次打开、last-good stale、无账号、多个账号、长 alias、switch 成功、switch 失败、usage empty、窄高度滚动。

## 7. Refresh、并发与安全

- [x] 7.1 打开 popover、点击 Refresh 和 switch 后刷新映射为 interactive refresh；timer 映射为 background refresh。
- [x] 7.2 验证 Swift 高频 timer、重复打开 popover 和网络错误不会绕过 backend cooldown。
- [x] 7.3 使用临时 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 provider homes 运行 accounts/dashboard/refresh/switch 跨层 integration tests。
- [x] 7.4 增加真实文件操作回归：切换前备份、atomic replacement、私有权限、失败回滚和 active registry 一致性。
- [x] 7.5 增加 privacy regression，扫描 Rust envelope、Swift logs、fixtures 和 diagnostics，禁止 raw auth/token/API key/raw provider response/raw log。

## 8. TokenBar 参考与合规

- [x] 8.1 固定 TokenBar 参考 commit，并记录仅参考 `NSStatusItem` lifecycle、popover chrome 和 layout 思路。
- [x] 8.2 使用 search audit 证明 Swift target 不引用 `TBCore`、TokenBar report DTO、scanner/pricing/quota/cache、bundle ID、UserDefaults key 或品牌资源。
- [x] 8.3 审计第三方 asset；没有明确 license/NOTICE 的资源不得进入 release bundle。
- [x] 8.4 若未来确需复制 TokenBar 文件，先新增 reuse manifest、`ThirdPartyNotices/TokenBar.md` 和 dependency audit 任务。

## 9. Build、Bundle 与发布验证

- [x] 9.1 建立 `scripts/build-menubar.sh`，构建 `omx-menubar-ffi` release staticlib 并运行 `swift build --package-path apps/omx-menubar`。
- [x] 9.2 建立 `scripts/bundle-menubar.sh`，读取 Cargo workspace version，组装 `.app`，写入 `LSUIElement=true`、`LSMinimumSystemVersion=14.0`、bundle ID 和 version。
- [x] 9.3 增加 version consistency check，验证 `omx --version`、Cargo workspace version、Menubar `CFBundleShortVersionString` 和 release tag 使用同一个值；不得在 Swift/Info.plist/script 中维护独立 app version。
- [x] 9.4 执行本地 ad-hoc codesign 并验证 `codesign --verify`；Developer ID、notarization、Sparkle 和 Homebrew cask 自动 bump 不作为 v1 gate。
- [x] 9.5 建立 macOS CI job：Rust staticlib、Rust tests、Swift build/tests、link smoke、bundle smoke、version consistency 和 privacy check。
- [x] 9.6 审计最终 `.app` linked symbols/resources，证明未包含 TokenBar 数据引擎、动画资源、独立 quota fetcher 或第二份 usage cache。

## 10. 文档与验收

- [x] 10.1 更新 `docs/ARCHITECTURE.md`，记录 `omx-core -> omx-app -> omx-menubar-ffi -> Swift` 边界和 Menubar 账号优先定位。
- [x] 10.2 更新 `docs/PRD.md` 和用户文档，说明 Menubar v1 能力、macOS/arm64 要求、CLI-only 管理动作和 usage 最小边界。
- [x] 10.3 文档化 macOS 本地文件/Keychain 权限、签名/notarization、日志脱敏和故障排查，不要求用户分享 auth 文件。
- [x] 10.4 运行 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features`、Swift build/test 和 OpenSpec validation。
- [x] 10.5 执行 acceptance review，确认 v1 不含 TokenBar fork、账号 CRUD、自动切换、伪 account attribution 或完整 analytics dashboard。
