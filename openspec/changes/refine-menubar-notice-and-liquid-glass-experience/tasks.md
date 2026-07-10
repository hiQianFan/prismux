# Implementation Tasks

给实现者（Codex）的可执行步骤。全部改动在 Swift 展示层，**不动 Rust / DTO / 测试夹具**。行号为编辑前的近似位置，以符号为准定位。除特别说明外，路径相对 `apps/prismux-menubar/Sources/PrismuxMenubarCore/`。

## 1. 移除 operation-notice 机制（`Features/Dashboard/AppStore.swift`）

- [x] 1.1 删除属性 `@Published private(set) var operationNotice: OperationNotice?`（约 `:16`）。
- [x] 1.2 在 `request(_:)` 中删除赋值块（约 `:249-251`）：
      ```swift
      if let operation = envelope.data?.operation {
          operationNotice = OperationNotice(operation: operation)
      }
      ```
      保留 `envelope.data?.operation` 被解码但不读取——这就是"失败静默"：成功/skip/失败都不再产生任何顶部提示状态。
- [x] 1.3 删除 `struct OperationNotice`（约 `:331-350`，含其 `init(operation:)`）。
- [x] 1.4 确认删除后 `AppStore` 无对 `OperationNotice` / `operationNotice` 的残留引用。

## 2. 清理 `Features/Dashboard/DashboardView.swift`

- [x] 2.1 删除顶部 notice banner 块（约 `:53-61`）：`if let notice = store.operationNotice { StatusBanner(...) }` 整段（含其 `.padding`）。
- [x] 2.2 在 `overview(_:)`（约 `:173-203`）删除 `let alerts = aggregatedDiagnostics(report)`（约 `:175`）与整个 `if !alerts.isEmpty { Card(title: "Needs attention") { ... } }`（约 `:195-201`）；保留 `Providers` 卡。
- [x] 2.3 删除函数 `aggregatedDiagnostics(_:)`（约 `:217-223`，含其上方 doc 注释）。
- [x] 2.4 在 `providerPage(...)`（约 `:225-234`）删除对 `diagnostics(provider:report:accounts:)` 的调用（约 `:232`）；**保留** `let accounts = accounts(for:in:)` 绑定（`accountTargets` 仍在用）。
- [x] 2.5 删除函数 `diagnostics(provider:report:accounts:)`（约 `:339-350`）。
- [x] 2.6 删除死代码 `providerAttentionCount(_:)` 与 `isProviderAttention(_:)`（约 `:601-607`）——已确认无调用点。
- [x] 2.7 确认 `OverviewPage` / `ProviderPage` 仅是布局包装，删除后两页面仍返回非空 `VStack`。

## 3. 删除失效组件文件

- [x] 3.1 删除 `Components/Shared/DiagnosticView.swift`（唯一消费者是被删的两张卡）。
- [x] 3.2 删除 `Components/Shared/StatusBanner.swift`（消费者只剩 notice banner 与 `DiagnosticView`，均已删；`StatusBannerProps.Severity` 仅被 `OperationNotice` 用，也已删）。
- [x] 3.3 全局搜索确认 `StatusBanner`、`StatusBannerProps`、`DiagnosticView` 无其他引用。

## 4. 账号卡片：一条常驻诊断行（不做展开/折叠）（`Components/Target/TargetRows.swift`）

> 修正记录：早期方案是"折叠 + 点击展开、展开区放完整额度窗口 + 操作按钮"。评审否决——展开区重复列 usage、Retry/Sign in 与既有 overflow/`+` 菜单重复、没有独有内容支撑展开。改为只做一条常驻诊断行。

- [x] 4.1 新增 severity 映射辅助（`private`），输入 `Diagnostic.code`，输出 `(Color, systemImage: String)`：
      - `managed_runtime_auth`、`auth`、`http_401`、`http_403` → `(PrismuxTokens.StatusColor.failed, "exclamationmark.octagon.fill")`
      - 其余（含 `refresh_failed`/`network`/`timeout`/`curl`/`schema`/`state`/`json`/`managed_runtime_unavailable`/HTTP 5xx·429 与未知 code）→ `(PrismuxTokens.StatusColor.warning, "exclamationmark.triangle.fill")`
- [x] 4.2 新增 `private struct TargetDiagnosticLine`：`Label { message } icon { severity 图标 }`，`message` 用 `.caption2` + `.secondary` + `lineLimit(2)`，图标带 severity 色；`recoveryAction`（若有）拼进 `.help()` 悬浮，不占正文；不显示 snake_case `code`；不放任何操作按钮。
- [x] 4.3 `AccountTargetRow`：**折叠态 identity + 5h/7d 额度条完全不变**。删除现有 inline `Label`（旧 `:88-94`）；当 `account.diagnostic != nil` 时，在额度条下方渲染 `TargetDiagnosticLine(diagnostic:)`。
- [x] 4.4 `ProfileTargetRow`：同样删除旧 inline `Label`，`profile.diagnostic != nil` 时渲染同一 `TargetDiagnosticLine`（当前恒为 nil，通常不出现）。
- [x] 4.5 移除展开相关的一切：`@State expanded`、`@Environment reduceMotion`（仅为展开动画引入）、`isExpandable`、`disclosureButton`、`AccountDetailDisclosure`、`DiagnosticDetail`、`DiagnosticMarker`、以及 `AccountTargetRow` 的 `signInAction` 入参。
- [x] 4.6 `DashboardView.accountTargets`：移除传给 `AccountTargetRow` 的 `signInAction:` 实参（Sign in 仍在 Accounts 卡 `+` 菜单）。
- [x] 4.7 操作不重复：Refresh/Reset/Delete 继续在行 `⋯` overflow；Sign in / Use existing login 继续在 Accounts 卡 `+`。诊断行不承载操作。
- [x] 4.8 保留 `.accessibilityElement(children: .combine)`；诊断行 `accessibilityLabel("Issue: \(message)")`。
- [ ] 4.9 （推迟）账号二级详情 / 展开：等有真正独有内容时再做，且优先放 Settings 窗口（见 design 决策 7），不在 transient popover 内加展开层。

## 5. 打开 popover 不无条件刷新（`Shell/StatusItemController.swift`）

- [x] 5.1 在 `togglePopover()`（约 `:56-66`）移除打开分支里无条件的 `Task { await store.refresh(kind: "interactive") }`。
- [x] 5.2 改为：仅当无 last-good 数据（首屏）或数据明显 stale 时触发刷新；显式 header Refresh 仍走 interactive 并可 loading。stale 判定复用 dashboard 的 stale flag（见 design Open Question），不在 Swift 层新造阈值。
- [x] 5.3 后台 timer（`scheduleBackgroundRefresh`）保持不变。

## 6. 文档

- [x] 6.1 更新 `DESIGN.md`：反馈策略（无 operation banner、失败静默）、diagnostics-on-card + disclosure、provider/dashboard 诊断不进 popover、菜单栏 extra 轻量（重功能归 Settings 窗口）、原生材料优先 Liquid Glass、open-popover 刷新策略、header/footer freshness、status item 图标。移除过时的"skip 显示为 warning banner""Diagnostics/Needs attention 卡"等描述。
- [x] 6.2 更新 `docs/menubar-ux-checklist.md`（必要时 `docs/menubar-v1.md`）：手工验收——打开 popover 不重复 loading；成功/skip/失败均无顶部 banner；target 诊断为账号卡上一条常驻单行（无展开、不重复 usage、无操作按钮）；颜色两档 + 形状可区分。

## 7. 验证

- [x] 7.1 编译 Swift package（按项目 memory：需完整 Xcode `DEVELOPER_DIR`，用 `/tmp` scratch 路径）：
      ```sh
      DEVELOPER_DIR="$(xcode-select -p)" swift build --package-path apps/prismux-menubar \
        --scratch-path /tmp/prismux-menubar-build
      ```
      （若 `xcode-select -p` 指向 CommandLineTools，需切到完整 Xcode.app 的 Developer 目录。）
- [x] 7.2 若存在 Swift 单测目标则运行；本次为纯展示层改动，若无覆盖可加最小展示/映射测试（可选，非必须）。`swift test` 已确认当前 package 无 Tests target；`swift run ... PrismuxMenubarContractTests` 通过。
- [x] 7.3 **不运行** cargo：本变更不触碰 Rust。若实现过程意外改到 Rust，则补 `cargo fmt --all` / `cargo test --locked` / `cargo clippy --all-targets --all-features -- -D warnings`。
- [ ] 7.4 手工验收：按 6.2 checklist 逐条核对。

## 完成判定

- popover 首屏无任何 operation banner；Overview 无 "Needs attention"、Provider 无 "Diagnostics"。
- target 诊断以"额度条下方一条常驻诊断行"的形式落在对应账号卡；无展开/折叠、无操作按钮、不重复 usage、无 snake_case 泄漏；无诊断时该行不出现、折叠态卡片不变。
- 严重度两档颜色 + 不同图标形状；文字中性、无填充色块。
- 打开 popover 不再让刷新按钮无条件 loading。
- `StatusBanner.swift` / `DiagnosticView.swift` 已删除，无悬挂引用；Swift package 编译通过。
