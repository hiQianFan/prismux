## Context

Menubar popover 当前把提示信息堆在首屏，来源有三处，都由彩色 `StatusBanner` 渲染：

1. **Operation notice**：`AppStore.request` 只要收到 `OperationResult` 就创建 `OperationNotice`（`AppStore.swift:249-251`），在 `DashboardView`（`DashboardView.swift:53-61`）header 下方渲染 banner。`skipped → .warning + "Operation skipped"`、`failed → .error`、其余 `.info`。`operationNotice` 被赋值后**没有任何地方置 nil**，banner 常驻。
2. **Overview "Needs attention" 卡**（`DashboardView.swift:195-201`）与 **Provider 页 "Diagnostics" 卡**（`DashboardView.swift:339-350`），展示 provider/dashboard 级诊断，内部也是 `StatusBanner`（经 `DiagnosticView`）。
3. **账号行 inline 诊断**（`TargetRows.swift:88-94` / `:218-224`）：`Label("\(code): \(message)")`，直接暴露 snake_case `code`、单行截断。

后端 `mutation.rs:431-437` 里 skip 的 message 是 `format!("Refresh skipped: {reason}.")`，所以 `fresh_enough` 会原样漏给用户。`StatusItemController.togglePopover()`（`:64`）每次打开还无条件 `store.refresh(kind: "interactive")`，让刷新按钮频繁 loading。

### Apple HIG 调研（本次决策依据）

- **《The menu bar》→ Menu bar extras**："Display a menu — not a popover ... **unless the app functionality you want to expose is too complex for a menu**, avoid presenting it in a popover." → Prismux 已因复杂度用 popover，说明处于复杂度上限，应做减法、保持轻量，不应再加导航深度。
- **《Popovers》**："limit the amount of functionality in the popover to a **few related tasks**"；"**Some popovers provide both condensed and expanded views of the same information.** If you adjust the size of a popover, **animate the change**"；"Avoid making a popover too big"；"Never show a cascade or hierarchy of popovers"；"Avoid using a popover to show a warning ... use an Alert instead." → popover 只做少量相关任务、反对在 transient popover 里做二级导航页。condensed/expanded 虽被 HIG 允许，但本次评估认为账号卡没有"只在展开区才有、值得看"的独有内容，故最终**不做 disclosure**（见决策 4）。
- **《Color》**："**Avoid relying solely on color** to differentiate ... provide the same information in alternative ways ... use text labels or **glyph shapes**"；"**Apply color sparingly** ... reserve it for elements that truly benefit from emphasis, such as **status indicators**"；用系统色（`.red`/`.orange`）自动适配浅/深/Increase Contrast；"Avoid using the same color to mean different things." → 颜色 + 形状双编码、文字中性、色要省、用系统色。

## Goals / Non-Goals

**Goals:**

- 移除 popover 首屏的所有 operation-notice banner；操作失败静默（不弹顶部彩条）。
- 移除 Overview "Needs attention" 与 Provider "Diagnostics" 两张诊断卡。
- 将 target 级诊断归到对应账号/配置卡片，用一条常驻诊断行呈现（不做展开/折叠）。
- 诊断/情况用两档颜色 + 不同图标形状表达严重度。
- 打开 popover 默认展示 last-good，不无条件刷新。
- Liquid Glass 采用原生材料优先策略；重功能归 Settings 窗口。
- 更新 `DESIGN.md` 与 UX checklist。

**Non-Goals:**

- 不重写 Menubar 为完整 analytics dashboard。
- 不改动 Rust control-plane / DTO / 测试夹具。
- 不删除 backend diagnostics 数据、support report 或 CLI/doctor 能力。
- 本次不在 popover 内实现账号二级详情页（重功能未规划；若将来需要，归 Settings 窗口）。

## Decisions

### 1. 移除所有 operation-notice banner；失败静默

删除 `operationNotice` 属性、`request()` 中的赋值、`OperationNotice` 结构体。成功/无变化/skip/失败一律不再产生顶部提示。`envelope.data?.operation` 仍被解码但不再读取。

- **skip（含 `fresh_enough`/`stale_request`/`error_backoff`）**：不显示。snake_case 泄漏问题随之消失（banner 不再出现）。
- **成功**：靠 header freshness（`Updated HH:MM`）与行状态体现，不常驻 banner。
- **失败（switch/delete/reset/import/refresh）**：**完全静默**（用户明确选择）。target 级失败若产生带 `target_id` 的诊断（如 `refresh_failed`），会经账号卡的诊断展示体现（见决策 4）；非 target 级失败不弹顶部 banner。

备选：保留失败 banner、只改样式。拒绝原因：用户明确要求做减法、失败静默；且顶部彩条正是要移除的噪音来源。

### 2. Swift 展示层做减法，Rust 不动

Rust 继续返回 `OperationResult`/`skipped_reason`/`Diagnostic` 语义与字段，DTO 不变，不影响 CLI 与测试夹具。所有改动都在 Swift 展示层。**本次不新增任何 DTO 字段**（诊断行所需数据均已存在于现有 DTO，见决策 4 的字段清单）。

### 3. 打开 popover 不等于刷新

`togglePopover()` 不无条件 `refresh(kind: "interactive")`。默认展示 last-good；后台 timer 继续低频静默刷新。仅在以下触发刷新：

- 用户显式点击 header Refresh（可显示 loading）。
- 无 dashboard/last-good 数据（首屏加载，loading 限定为初次加载）。
- 数据明显 stale：可作为后台静默刷新，不让 header Refresh 按钮进入显眼 loading（除非用户点了 Refresh）。

### 4. Target 级诊断归到账号卡片，用一条常驻诊断行（不做展开/折叠）

**诊断归属**（`Diagnostic` 有 `providerId`/`targetId`/`scope`）：

- **target 级（有 `target_id`）** → 落到对应账号/配置卡片。`account.diagnostic` 已是 target 级（`mapper.rs:account_from_status` 取 `usage.diagnostics.first()`，`scope="target"`），target 刷新失败也从这里冒出来。
- **provider / dashboard 级（无 `target_id`）** → 从 popover 移除（见决策 5）。

**账号卡结构**：

- **折叠态（默认，且是唯一形态）**：identity 行（active dot、`#N 标签`、`plan · 刷新时间`、Reset 徽章、Use/Active、overflow）+ 5h/7d 额度条。**此部分完全不变。**
- **诊断行**：当 `account.diagnostic != nil` 时，在额度条下方显示**一条常驻的诊断行**（`TargetDiagnosticLine`）：severity 小图标（形状 + 颜色）+ `diagnostic.message`（人话，非 snake_case）。`recoveryAction` 收进 `.help()` 悬浮提示，不占正文；`message` 用 `.secondary` 中性灰、`lineLimit(2)`；无填充背景。没情况的账号不显示这一行。

**不做展开/折叠 disclosure**（相较早期方案的修正）：

- 早期设想的"点击展开、展开区放完整额度窗口 + 操作按钮"被否决。原因：
  - 展开区若重复列出 `quota.windows`，等于把折叠态已有的 usage 数据再放一遍，属噪音。
  - Retry / Sign in 按钮与账号行 `⋯` overflow（Refresh usage）和 Accounts 卡 `+` 菜单（Sign in / Use existing login）重复。
  - 目前没有"只在展开区才有、且值得看"的独有内容，展开机制无法自证其价值。
- 因此本次只做"一条常驻诊断行"。操作一律走既有入口（overflow 菜单、Accounts 卡 `+`），诊断行不带按钮。
- **将来**若出现真正独有的账号详情（用量历史、单账号配置、日志等），优先放 Settings 窗口（决策 7），而不是在 transient popover 里加展开层。

**可用字段核对（均在现有 DTO，无需新增）**：`TargetAccount.diagnostic{code,message,recoveryAction}`、`status`、`actions`。usage 相关字段（`quota.windows`/`summary`/`resetAtUnix`）仅供折叠态既有展示，不进诊断行。

### 5. provider / dashboard 级诊断从 popover 移除

删除 Overview "Needs attention" 卡与 Provider "Diagnostics" 卡。无 `target_id` 的诊断挂不到任何账号行，从 popover 移除。理由：

- 全局信号已被 header 的橙色 "Stale HH:MM" + 警告图标、footer 的 "CLI ready / not configured" 覆盖。
- 完整诊断仍保留在 Settings/About 的 support report 与 CLI/doctor 排障路径。

不为孤儿诊断引入任何替代 banner/卡片（会抵消减法目标）。

### 6. 颜色两档 + 形状双编码

遵循《Color》"不能只靠颜色 + 用 glyph 形状 + 色要省 + 用系统色"：

- **可恢复（橙 `PrismuxTokens.StatusColor.warning` + `exclamationmark.triangle.fill`）**：`refresh_failed`、`network`、`timeout`、`curl`、`schema`、`state`、`json`、`managed_runtime_unavailable`、HTTP 5xx/429 等。
- **需处理（红 `PrismuxTokens.StatusColor.failed` + `exclamationmark.octagon.fill`）**：`managed_runtime_auth`、`auth`、HTTP 401/403。
- 未知 code → 归橙（默认可恢复）。
- 文字一律 `.secondary` 中性灰；**无填充背景色块**；只有小图标带色。
- 用系统色（Tokens 已是 `.orange`/`.red`），自动获得 Increase Contrast 高对比变体，不硬编码 hex。
- 不与额度条重复：额度条已用红色表达"额度告急"，诊断行只承载额度条没表达的（登录失效、刷新失败、限流）。

### 7. 菜单栏 extra 保持轻量；重功能归 Settings

依据《The menu bar》与《Popovers》：popover 只做"几个相关任务"，不做二级导航页（transient popover 点外即关，导航进去会丢失位置）。将来的重功能（完整用量历史、单账号配置、日志等）放进已有的 Settings 窗口（`MenubarSettingsWindowController`，真正的可缩放 NSWindow），而非在 popover 里加 NavigationStack。

### 8. Liquid Glass 与 HIG 系统优先

macOS 26/27：

- 菜单栏应用保持短时、轻量、可扫读。
- Settings 用 `NavigationSplitView`/`.sidebar`/grouped `Form`/`.regularMaterial` 等系统控件，让系统承担 Liquid Glass。
- 主 popover 不手搓 glass shader、不叠多层透明卡、不引入大面积 blur；可用系统 `Material`/window background。
- 遵守 Reduce Transparency、Increase Contrast、Reduce Motion。
- macOS 14/15 保持现有 `windowBackgroundColor` / 深色淡品牌底 fallback。

## Risks / Trade-offs

- [Risk] 失败静默后用户不知道操作失败。→ Mitigation: target 级失败经账号卡诊断体现；行状态（Active/Use 不变）反映未变更；用户明确接受此权衡。
- [Risk] 去掉 open refresh 后看到旧数据。→ Mitigation: 后台 timer 刷新；header 显示 freshness；仅 missing/stale 才静默刷新。
- [Risk] 移除诊断卡后找不到排障信息。→ Mitigation: target 级问题落到账号卡；完整信息在 support report 与 CLI/doctor。
- [Risk] 诊断行占用额外高度。→ Mitigation: 单行 + `lineLimit(2)`，仅在有诊断时出现；不做展开也就没有高度抖动问题。
- [Risk] 颜色仅两档不够细。→ Mitigation: 形状 + 文字承载更多语义；support report 保留完整 code。
- [Risk] 只在 Swift 层改导致与 CLI 文案不一致。→ Mitigation: Rust 保留语义；CLI 继续输出具体原因；本次不动 Rust。

## Migration Plan

1. `AppStore`：删除 `operationNotice` 属性、`request()` 赋值、`OperationNotice` 结构。
2. `DashboardView`：删除顶部 banner、"Needs attention" 卡 + `aggregatedDiagnostics`、Provider "Diagnostics" 卡函数及调用、死代码 `providerAttentionCount`/`isProviderAttention`。
3. 删除 `Components/Shared/StatusBanner.swift`、`Components/Shared/DiagnosticView.swift`。
4. `TargetRows`：新增 severity→(颜色,图标) 映射与 `TargetDiagnosticLine`；账号/配置行在额度条下渲染该行、替换旧 inline Label；移除展开相关代码与 `signInAction`。
5. `StatusItemController.togglePopover()`：打开 popover 不无条件 interactive refresh。
6. 更新 `DESIGN.md` 与 `docs/menubar-ux-checklist.md`。
7. 用完整 Xcode 环境编译 Swift package（见 tasks），做手工验收。

回滚策略：本次纯 Swift 展示层改动，无 state schema / auth 迁移；恢复被删代码即可。

## Open Questions

- stale 阈值：复用现有 backend stale/cooldown，还是 Swift 层只根据 dashboard stale flag 决定是否静默刷新。
- （已决）诊断行不带操作按钮——恢复动作走既有 overflow 菜单与 Accounts 卡 `+`；将来若需账号级详情/操作面板，归 Settings 窗口。
