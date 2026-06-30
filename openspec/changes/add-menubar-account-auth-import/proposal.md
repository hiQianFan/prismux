# 提案：menubar 补齐账号登录、profile 导入与已登录接管

## 背景

后端三件能力——`login`（OAuth 登录并导入凭据）、`save_current`（接管本机已登录的官方账号）、`import_config`（导入中转/网关 profile）——在 `omx-core` 的 `PlatformPlugin` trait 上对 Codex、Claude 都已**完整实现**，CLI（`omx login` / `omx save` / `omx import`）也全部可用。

但 menubar 完全够不到它们：

- `crates/omx-menubar-ffi/src/lib.rs:130` 的 dispatch 表只有 `accounts/dashboard/switch/refresh/remove/consume_reset_credit/settings/about/support`，**没有 `login`、`save`、`import` 三个 op**，其余一律 `unknown_op`。
- Swift 侧账号/profile 卡的空状态（`DashboardView.swift:257`、`:289`）只有一句灰字，没有任何「添加账号 / 登录 / 导入 / 接管」入口；唯一的 onboarding 是页脚把命令复制到剪贴板的「Manage in CLI」。

结果：**新用户打开 menubar 是一组空卡片，必须先离开 app 去命令行才能用**。这是阻断首次使用的逻辑漏洞，本提案补齐它。

## 目标

让用户**完全在 menubar 内**完成首次接入，三条路径：

1. **登录（login）**：账号卡点「Sign in」→ 官方 CLI 打开浏览器或执行其原生登录流程 → 用户授权 → 回调完成 → menubar 接管该账号并自动刷新列表。底层复用本机 `codex` / `claude` CLI，不自研 OAuth。
2. **接管已登录账号（save_existing_login）**：账号卡点「Use existing login」→ 调用 `save_current` 读取本机已有官方登录产物（`~/.codex/auth.json` / Claude keychain 或 `.credentials.json`）→ 成功后导入账号池。失败时展示“未发现可接管的官方登录产物”类错误，不自动弹窗打扰用户。
3. **导入 profile（import_profile）**：profile 卡点「Import」→ 粘贴文本、选择文件，或把配置文件拖入 menubar → 当前 provider 为 Codex 时按 Codex profile 解析，当前 provider 为 Claude 时按 Claude profile 解析 → 后端校验并导入。

配套：

- **CLI 依赖处理**：login 依赖本机装有对应官方 CLI（`codex` / `claude`）。menubar SHALL 在触发 login 时检测或捕获二进制缺失错误，给出明确的安装引导，而不是静默失败或卡死。
- 三个能力都以新 FFI op 暴露，并返回现有 menubar mutation 形态：`OperationResult + DashboardReport`。错误回到 UI 并可读（沿用现有 `application_error` sanitize 通道）。
- 操作成功后新账号/profile 立即出现在对应卡片。

## 非目标

- **不自研任何 provider 的 OAuth/PKCE/token exchange**：登录交给官方 CLI，OpenMux 只负责拉起、等待完成、导入产物。
- 不新增 Gemini 支持（FFI 当前只挂 Codex + Claude，`lib.rs:229`）。
- 不改后端 `login` / `save_current` / `import_config` 的算法与凭据存储边界（已实现，本提案只接线 + 加 UI）。
- 不改账号/profile 卡现有的 Use/Refresh/Delete/Reset 行为与样式。
- 不做凭据编辑、轮换、多账号批量导入。
- 不在 menubar 内嵌浏览器或自建 localhost 回调服务器（回调由官方 CLI 自己处理）。
- 不做自动 `unmanaged_login_detected` 探测和主动接管弹窗；第一版用显式「Use existing login」按钮覆盖该需求。

## 用户价值

- 新用户**零命令行**即可接入：装了官方 CLI 就能在 menubar 里登录。
- 已经在用 Codex/Claude 的老用户，可以点「Use existing login」一键接管，零重复登录。
- 中转/网关用户粘贴文本、选择文件或拖入配置文件即可导入 profile，自助完成。
