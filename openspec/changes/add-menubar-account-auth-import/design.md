## Context

后端能力齐备、菜单栏够不到，是一个纯粹的「接线 + UI 入口」缺口，不是新功能设计。本提案只解决「让 menubar 能调用已实现的 login/save_current/import_config，并给用户入口」。

三个能力的后端语义（已实现，开工时核对最新签名）：

- `login(LoginOptions{ device_auth, alias, activate }) -> AccountRef`：复用本机官方 CLI 的登录流程，登录完成后导入凭据快照。
- `save_current(SaveOptions{ alias }) -> AccountRef`：读取本机已存在的官方登录产物（Codex 读 `~/.codex/auth.json`；Claude 读 keychain / `.credentials.json`），导入为受管账号。不触发任何登录。
- `import_config(ImportConfigOptions{ name, content }) -> ImportedConfig`：解析中转/网关配置文本，写入 profile snapshot/registry；Codex import 还可能把 provider projection 写入 live `config.toml`。纯文本进、结构化出，无需 TTY。

`AccountRef{ platform, local_id, number, alias }` 与 `ImportedConfig{ profile_name, number, auth_type, base_url, ... }` 是 operation message 的来源；FFI 响应仍返回 menubar mutation 形态，便于 Swift 一次请求更新 dashboard。

## Goals / Non-Goals

**Goals:**

- 定义三个新 FFI op（`login` / `save_existing_login` / `import_profile`）的请求/响应契约与错误回传。
- 定义 menubar 三条接入路径的交互：账号卡 Sign in、账号卡 Use existing login、profile 卡 Import。
- 定义 login 的官方 CLI 缺失引导。
- 定义接入成功后 dashboard 的刷新与可见性。

**Non-Goals:**

- 自动探测未接管官方登录产物并主动弹窗。
- OAuth device code 的进度流、验证码展示。
- OAuth/PKCE/token 自研、Gemini、凭据编辑轮换。
- 后端三能力的算法与凭据存储边界（已实现）。

## Decisions

### 1. 三个新 FFI op

在 `crates/omx-menubar-ffi/src/lib.rs` 的 `dispatch()` 增加三个 op，复用现有 `payload` / `json_value` 通道，错误经 `application_error` sanitize 回传：

| op | payload | 后端调用 | 响应 |
|---|---|---|---|
| `login` | `{ provider, alias?, activate?, device_auth? }` | `plugin.login(LoginOptions{..})` | `OnboardingOperationReport` |
| `save_existing_login` | `{ provider, alias? }` | `plugin.save_current(SaveOptions{..})` | `OnboardingOperationReport` |
| `import_profile` | `{ provider, name?, content }` | `plugin.import_config(ImportConfigOptions{..})` | `OnboardingOperationReport` |

provider 解析复用现有按 id 取 plugin 的逻辑（Codex/Claude）。`OnboardingOperationReport` 与现有 `SwitchReport` / `RemoveReportView` 同形：包含 `control_plane_schema_version`、`state_schema_version`、`generated_at_unix`、`provider`、`operation`、`dashboard`、`accounts`，并可附带 `account` 或 `profile` 摘要字段。

Swift 现有 `RustBackendClient` 已在 detached task 中调用 FFI；`login` SHALL 不在主线程执行。第一版不做进度流，只显示 provider-scoped in-flight 状态和“Follow the browser or official CLI prompt.” 类提示；官方 CLI 退出后返回成功或脱敏错误。

`login.activate` 在 menubar 中默认 `true`。Codex 因 CLI 语义默认不切 active，menubar 明确传 `activate: true`；Claude 当前 plugin login 会导入后立即 `switch_to`，实现时 SHALL 在 UI 文案里把登录成功视为 active。

### 2. 账号卡「+ 登录」入口

账号卡（含空状态）SHALL 提供「+」/「Sign in」入口触发 `login`。流程：用户授权 → 官方 CLI 完成回调 → menubar 接管账号 → 刷新列表，新账号出现并按 `activate` 决定是否设为当前。

menubar SHALL 在触发 login 时检测或捕获对应官方 CLI（`codex` / `claude`）缺失错误；缺失时 SHALL 给出安装引导，SHALL NOT 静默失败或卡住 UI。

### 3. 账号卡「Use existing login」入口

账号卡 SHALL 提供「Use existing login」入口触发 `save_existing_login`。该入口不启动官方登录流程，只读取 provider 已存在的官方登录产物并导入 OpenMux 账号池。

第一版不在 dashboard projection 新增 `unmanaged_login_detected`，也不主动弹窗。用户点按钮后：

- 如果本机存在可导入官方登录产物，导入并刷新 dashboard。
- 如果不存在或内容不完整，展示后端返回的脱敏错误。
- 如果 provider capability 暂未声明 `account_save`，入口 SHALL 隐藏或禁用；Claude 实现已具备 `save_current`，开工时应先校正 capability，再展示入口。

### 4. profile 卡「Import」入口

profile 卡（含空状态）SHALL 提供「Import」入口，让用户粘贴文本、选择文件，或把配置文件拖入当前 provider 页面，提交非空 `content` 触发 `import_profile`。导入成功后新 profile 出现在卡片，展示 `profile_name` / `auth_type` / `base_url`（受隐私脱敏）。格式与解析由后端 `import_config` 决定，前端不重写 parser。

前端仅做轻校验：

- `content` 必须非空，避免 Claude 空 content 被解释为 account import。
- 文件必须可读且是 UTF-8 文本。
- 文件大小设置保守上限（例如 256 KiB），避免误拖大文件。
- UI 不回显完整 secret；错误展示只使用后端脱敏结果。

## Risks / Trade-offs

- **依赖官方 CLI 在 PATH**：login 不装 `codex`/`claude` 就不可用 → 用缺失错误引导兜底，且 import 与 save 路径不依赖它，仍可用。
- **login 可能长时间等待浏览器授权**：后台任务 + provider in-flight 状态兜底；login 子进程以可取消、有超时上限的方式等待（见 `cancel_login` 与核心 `run_cancellable_login`），关闭浏览器或点取消都会终止子进程并释放操作锁。
- **菜单栏环境没有可见 TTY**：第一版依赖官方 CLI 的浏览器登录能力；device-auth 只作为 payload 字段保留，不在 UI 默认暴露。
- **不主动探测已登录账号**：少做 schema 和 credential hash 比对；代价是用户需要点一次「Use existing login」。
