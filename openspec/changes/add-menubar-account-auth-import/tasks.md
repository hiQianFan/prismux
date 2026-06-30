# 任务

> 三个能力的后端已实现（`PlatformPlugin::login` / `save_current` / `import_config`）。本变更只做 FFI 接线 + menubar UI 入口 + 显式接管已登录账号。第一版不做自动 `unmanaged_login_detected` 探测。

## 1. 后端 mutation 接线（omx-app / omx-menubar-ffi）

- [ ] 1.1 `omx-app` 增加 `OnboardingOperationReport`，复用现有 mutation 形态：`OperationResult + DashboardReport + AccountsReport`，并可附带 account/profile 摘要。
- [ ] 1.2 增加 `menubar_login`：解析 `{ provider, alias?, activate?, device_auth? }`，默认 `activate = true`、`device_auth = false` → `plugin.login(LoginOptions{..})` → 返回 `OnboardingOperationReport`。
- [ ] 1.3 增加 `menubar_save_existing_login`：解析 `{ provider, alias? }` → `plugin.save_current(SaveOptions{..})` → 返回 `OnboardingOperationReport`。
- [ ] 1.4 增加 `menubar_import_profile`：解析 `{ provider, name?, content }`，拒绝空 content → `plugin.import_config(ImportConfigOptions{..})` → 返回 `OnboardingOperationReport`。
- [ ] 1.5 `crates/omx-menubar-ffi/src/lib.rs` 的 `dispatch()` 增加 `login` / `save_existing_login` / `import_profile` op，复用 `payload` / `json_value` / `application_error` 通道，错误脱敏。
- [ ] 1.6 bump fixtures：新增 `login.response.json` / `save-existing-login.response.json` / `import-profile.response.json`（`OMX_UPDATE_FIXTURES=1`），更新 contract 测试。

## 2. provider capability 与安全边界

- [ ] 2.1 校正 Claude capability：若 `save_current` 支持导入 Claude OAuth snapshot，则 `account_save` SHALL 为 true；否则 Swift 不展示「Use existing login」。
- [ ] 2.2 `import_profile` SHALL 强制 content 非空，避免 Claude 空 content 走 account import。
- [ ] 2.3 不新增 dashboard `unmanaged_login_detected` 字段，不 bump control-plane schema 仅为自动提示服务。

## 3. Swift DTO 与 Payload

- [ ] 3.1 `BackendClient.swift`：`Payload` 增加 `.login` / `.saveExistingLogin` / `.importProfile` case 与 op 名映射（`AppStore.opName`）。
- [ ] 3.2 `BackendClient.swift` / `DTO.swift`：扩展 `OperationData` 解码 onboarding 响应；继续优先读取 `operation.dashboard` 更新 UI。

## 4. AppStore 动作

- [ ] 4.1 `AppStore` 增加 `signIn(provider)` / `useExistingLogin(provider)` / `importProfile(provider, name, content)`，沿用现有 `request(_:)` 模式。
- [ ] 4.2 login/save/import 期间的 provider-scoped in-flight 态与错误展示，复用现有 toast/inline error 通道。
- [ ] 4.3 官方 CLI 缺失检测：login 触发前判定（或捕获后端「binary not found」错误），转为安装引导提示。
- [ ] 4.4 login 请求默认传 `activate: true`；第一版不在 UI 暴露 `device_auth`。

## 5. menubar UI 入口

- [ ] 5.1 账号卡（含空状态 `DashboardView.swift:257`）加「Sign in」按钮 → `signIn`。
- [ ] 5.2 账号卡加「Use existing login」按钮 → `useExistingLogin`；无 capability 时隐藏或禁用。
- [ ] 5.3 profile 卡（含空状态 `:289`）加「Import」入口 → 粘贴文本 / 选文件 / 拖放文件 → `importProfile`。
- [ ] 5.4 profile import 前端轻校验：非空、UTF-8、文件大小上限、错误与预览不回显完整 secret。
- [ ] 5.5 CLI 缺失引导文案与安装链接/命令；沿用 DESIGN.md 视觉。

## 6. 验证

- [ ] 6.1 Rust：`cargo test -p omx-app -p omx-menubar-ffi` 通过；`cargo build -p omx-cli` 通过。
- [ ] 6.2 Swift：`swift build` + `OmxMenubarContractTests` 通过（先 `xattr -cr .build`）。
- [ ] 6.3 `openspec validate add-menubar-account-auth-import` 通过。
- [ ] 6.4 人工 GUI：零账号空状态可登录；Use existing login 可接管本机已登录账号；profile 粘贴/选文件/拖放导入；缺 CLI 时给引导不卡死。
