# Prismux Architecture

## 总体形态

Prismux 当前是一个 Rust monorepo/workspace。长期产品架构目标是：

```text
prismux-core -> prismux-control-plane -> frontends
```

Phase 1 保留 `crates/prismux-app` crate name，把它作为 `prismux-control-plane` 的实现载体；代码边界先拆成 `api`、`query`、`mutation`、`runtime`、`mapper`、`diagnostics`、`compatibility` 和 `settings`。后续是否重命名 crate 不阻塞当前验收。

当前 Phase 1 不做 WebKit/browser cookie、Sparkle、WidgetKit、HTTP server、独立分发 artifact、大量 provider registry 或完整 managed account migration。`system_active_target`、`selected_ui_target`、`refresh_scope_target`、`observed_target` 先作为 DTO/model 预留字段，account-scoped runtime 后续再实现。

现有 CLI 形态仍是：

```text
prismux-cli
  |
  v
platform plugins
  |
  v
prismux-core
```

CLI 只负责命令解析和输出展示。跨平台共享概念放在 `prismux-core`。每个 AI coding tool 的路径、登录、账号池和 auth 文件处理放在独立 plugin crate。

## Crates

- `prismux-core`：共享领域对象、错误、报告、账号池 summary、账号状态、登录/保存 options、SQLite `StateStore` 和 `PlatformPlugin` trait。
- `prismux-plugin-codex`：Codex 专属实现，包括 Codex home 解析、临时 `CODEX_HOME` 登录、auth snapshot、provider subject 去重、account/plan metadata 解析、SQLite account/profile 状态和 active auth 切换。
- `prismux-plugin-claude`：Claude Code 专属实现，包括 profile import、settings env patch、macOS Keychain/plaintext `.credentials.json` account snapshot、`oauthAccount` metadata 备份/恢复，以及共享 SQLite account/profile 状态。
- `prismux-app`：Phase 1 control-plane application service，输出 dashboard/provider/target/action/diagnostics/compatibility view model 和 operation result。
- `prismux-menubar-ffi`：Menubar transport 层，只负责 JSON envelope、schema gate、panic-safe error、JSON transport 和 memory free。
- `prismux-cli`：`prismux` 命令行前端，消费 core/plugin API，不拥有业务状态。

## Module Boundaries

`prismux-core` 按领域拆分为 `account`、`profile`、`platform`、`plugin`、`report`、`storage` 和 `usage`。plugin crate 不应重复实现私有目录、原子写入、snapshot hash、路径展示和时间戳这类跨平台基础能力。

`prismux-cli` 保持薄层：`main.rs` 只启动应用，`app.rs` 负责命令路由和输出展示，`input.rs` 负责 import 内容读取。平台行为必须留在 plugin crate。Phase 1 允许 CLI 逐步迁移到 control-plane view model，但 `status/list/save/use/switch` 和 JSON/machine output 只能 additive 演进，不能破坏现有脚本语义。

`prismux-app` 的 public control-plane API 使用 provider-agnostic 名称：`dashboard_view`、`provider_view`、`refresh_provider`、`activate_target`、`compatibility_view`。`refresh_all`、`remove_target`、`settings_view`、`update_settings`、`support_report` 在 Phase 1 只保留 contract/DTO 边界，不要求完成全部行为。

Phase 1 调用链盘点：

```text
prismux-cli -> prismux-app/api -> provider plugin -> prismux-core
Swift Menubar -> prismux-menubar-ffi -> prismux-app/api -> provider plugin -> prismux-core
```

业务解释重复点需要继续收敛：CLI overview 仍有 table/human rendering 专用汇总，Swift `DashboardView` 仍有 provider row/status 的视觉层汇总。Phase 1 已把 Menubar FFI transport 改为调用 control-plane API，并把 schema/target/diagnostics/version 字段放在 Rust DTO；后续迁移应继续把 action eligibility、provider health 和 quota health 从 Swift/CLI presentation 中移出。

`prismux-plugin-codex` 和 `prismux-plugin-claude` 的主流程保留在 `plugin.rs`，测试拆到 `tests.rs`。后续继续扩展 Gemini 或新的 Claude backend 时，应优先复用 core storage、SQLite `StateStore` 和 plugin capability 模型，并把 provider-specific parser/backend 维持在对应 plugin 内。

## Modular Distribution

Prismux 的长期分发形态允许拆成 `CLI-only`、`Menubar-only` 和 `full bundle`，但所有产物必须共享同一个 state root、control-plane schema、state schema、safe diagnostics 和 provider capability matrix。分发拆分只能改变入口和打包方式，不能引入第二套账号状态或前端自行推断。

| Artifact | 能力 | 平台 | 依赖 |
| --- | --- | --- | --- |
| `CLI-only` | `login`、`save`、`use`、`import`、`alias`、`doctor`、`usage` 等高级管理入口 | macOS；后续可扩展 Linux/Windows | 已安装的目标 AI tool CLI |
| `Menubar-only` | dashboard、refresh、显式 activation、last-good snapshot、CLI handoff 文案 | macOS 14+ Apple Silicon | embedded staticlib 或 helper binary 提供 control-plane runtime |
| `full bundle` | CLI + Menubar + 共享状态 + compatibility gate | macOS | 已安装的目标 AI tool CLI；bundle 内含前端和 backend runtime |

Menubar 获取 backend/control-plane runtime 的优先模式：

1. `embedded_staticlib`：当前推荐路径，`prismux-menubar-ffi` 作为 transport 层，Swift 只收发 JSON envelope，所有 mutation 先通过 schema gate。
2. `helper_binary`：未来可选 packaging 方式，helper 必须复用同一 state root、control-plane contract、request timeout 和 last-good cache。
3. `installed_cli`：兜底方式，只能调用文档化的 machine-readable 命令；不得解析 human output。

`compatibility_view` 是独立分发的 gate。它返回 control-plane schema、state schema、minimum backend/frontend version、artifact capability、backend runtime option、optional module status 和 provider capability matrix。不兼容 schema 时只能进入 read-only safe snapshot 或 upgrade-required 状态，不允许执行 state-changing operation。缺失 optional module 时前端必须展示 unavailable view 和安装指导，例如提示安装 `prismux` CLI 或切换到 embedded staticlib；不能隐藏入口、静默失败或崩溃。

## Core Domain

核心模型是 platform account pool：

```text
PlatformPoolSummary
  platform
  account_count
  active: optional AccountRef
  availability

AccountRef
  platform
  number
  alias: optional

AccountStatus
  account
  active
  account_label: optional
  plan_label: optional
  auth_type: optional
  expires_at_unix: optional
  availability
  usage: optional UsageSnapshot

UsageSnapshot
  source: RemoteApi | LocalSession | StoredSnapshot | Unavailable
  refreshed_at_unix: optional local fetch timestamp
  summary: Availability
  limits: list UsageLimit
  diagnostics: list safe diagnostic

UsageLimit
  id
  label
  scope: Account | Workspace | Project | Model | Feature | Unknown
  kind: RollingWindow | CalendarWindow | CreditBalance | RequestRate | TokenRate | Unknown
  window_seconds: optional
  used_percent_x100: optional
  remaining_percent_x100: optional
  reset_at_unix: optional
  exhausted: optional
  raw_provider_key: optional
```

account 持久编号是平台内 account selector。`codex account #1` 和 `claude account #1` 没有身份上的关联。alias 是可选 metadata，不是账号创建前置条件。

当平台同时暴露 accounts 和 profiles 时，CLI 不直接把用户输入的数字传给插件。`prismux-core::TargetCatalog` 会把 accounts 和 profiles 聚合成当前列表 target：

1. accounts 按插件返回顺序编号；
2. profiles 接在 accounts 后继续编号；
3. 数字 selector 按当前展示编号解析；
4. 非数字 selector 按 account alias 与 profile name 精确匹配；
5. 命中多个 target 时返回歧义错误。

展示编号不作为长期状态身份。plugin 仍只接收自己的底层 selector：account number/alias，或 profile number/name；SQLite 中的 `local_id` 才是 quota snapshot 和 refresh attempt 的稳定归属。

`availability` 是给旧展示和单账号保守状态判断使用的摘要字段，不能替代结构化 `usage`。平台插件应该把原始 provider quota 映射成多个 `UsageLimit`，再由最紧的可用窗口派生单账号 summary；CLI overview 的 `Overall` 则优先对结构化 limit 的剩余额度做账号池聚合。`refreshed_at_unix` 记录 Prismux 本次获取 usage 的本地时间，不是 provider quota reset time。Codex 当前会把 `primary_window`、`secondary_window` 和 `additional_rate_limits` 解析为多个 limit，并通过 `limit_window_seconds` 识别 `5h`、`weekly` 等窗口；Claude/Gemini 后续可以复用同一模型，但保留各自的 scope、kind 和 raw provider key。provider-specific 字段只有在跨平台语义明确后才进入 core，否则应该留在插件内部或 detail formatter 中。

本地 token usage stats 已从主分支剥离，冻结点为 `usage-stats-v0`。当前主分支不提供 `prismux usage`、本地 session parser、`usage_events` 或 `scan_watermarks`；后续若重新引入，必须作为独立设计重新进入，而不是混入 quota/limit 链路。

## Local State

Prismux state 位于用户平台数据目录下。SQLite 是 account/profile/active/quota/refresh 的统一状态源，auth-bearing payload 仍保存在私有 snapshot 文件中：

```text
<data-local-dir>/prismux/
  prismux.sqlite
  platforms/codex/
    accounts/<auth_hash>.auth.json
    backups/auth.json.bak.<timestamp>
    backups/config.toml.bak.<timestamp>
    login/codex-login-<pid>-<timestamp>/
  platforms/claude/
    profiles/<config_hash>.profile.json
    accounts/<snapshot_hash>.credentials.snapshot
    accounts/<snapshot_hash>.oauth-account.json
    backups/settings.json.bak.<timestamp>
    backups/credentials.snapshot.bak.<timestamp>
```

`prismux.sqlite` 包含 `accounts`、`profiles`、`active_targets`、`quota_snapshots` 和 `refresh_attempts`。SQLite 只保存非敏感 metadata、hash、`secret_ref`、display number、timestamps、active target 状态和 quota/refresh 历史；raw auth payload、access token、refresh token、API key 和完整 provider 原始响应不进入 SQLite。

账号或 profile remove 使用 hard delete 语义：删除 Prismux 管理的 secret/config snapshot，清除 active target，并删除对应 `accounts`/`profiles` 行。account remove 同时删除该账号的 `quota_snapshots` 和 `refresh_attempts`。

账号唯一身份优先使用 provider subject，而不是整份 auth 文件 hash。Codex 从 `id_token` 和 `tokens.account_id` 中按优先级提取 `chatgpt_account_id`、`iss+sub`、`chatgpt_user_id/user_id`、`account_id`，SQLite 只保存 `provider_subject_hash` 和 kind/label。`auth_hash` 继续作为 snapshot 文件名、凭证指纹和切换时的篡改校验；当 auth 中没有可用 subject 时才作为去重 fallback。email 只作为展示 metadata，不能作为自动合并依据。

## Codex Path Resolution

Codex home 解析顺序：

1. 测试或嵌入方注入的显式路径。
2. `$CODEX_HOME`。
3. `~/.codex`。

Prismux state root 解析顺序：

1. 测试或嵌入方注入的显式路径。
2. `$PRISMUX_STATE_ROOT`。
3. OS 平台本地数据目录下的 `prismux`。

Codex active auth path：

```text
<codex-home>/auth.json
```

Codex account-scoped refresh 使用 Prismux 管理的 runtime scope：

```text
<data-local-dir>/prismux/platforms/codex/runtime/<account-local-id>/auth.json
```

该 runtime scope 不是系统 active `CODEX_HOME`，也不复制完整用户配置。首次刷新某个已保存 account 时，Prismux 会从该 account 的私有 auth snapshot lazy migration 出最小 `auth.json`，校验 snapshot hash 后写入 managed runtime scope。后续 quota/status refresh 优先读取 managed runtime auth；如果该文件已由 refresh 流程轮换，则只保留在对应 account scope，不回写系统 active `auth.json`，也不覆盖原 snapshot。用户显式 `prismux use codex <selector>` 或 Menubar activation 时，才走 active auth replacement/promote 路径。

## Login Flow

`prismux login codex [--device-auth] [--alias <alias>] [--use]`：

1. 创建临时 login home。
2. 以该目录作为 `CODEX_HOME` 调用官方 `codex login`。
3. 如果传入 `--device-auth`，调用 `codex login --device-auth`。
4. 登录交互继承当前终端 stdin/stdout/stderr。
5. 登录成功后读取临时 home 的 `auth.json`。
6. 计算 auth bytes 的 SHA-256。
7. 从 `id_token` claims 中提取可展示 account 和 plan，例如 email 和 ChatGPT plan；不展示 raw token。
8. 如果 hash 已存在，更新已有账号 snapshot；否则分配下一个平台内编号。
9. 默认不替换当前 active auth；传入 `--use` 时才立即切换。
10. 清理临时 login home。

`login` 负责获取并登记凭据，`use` 负责改变当前运行上下文。默认不激活使添加账号保持最小副作用，并允许脚本先完成多账号登记；`--use` 是将两步组合为一次操作的显式便利开关。CLI 在未传 `--use` 时必须打印可直接执行的 `prismux use codex <selector>`。如果新登录结果与当前 active account 是同一 provider subject，则应同步更新 active auth，因为这不改变账号上下文，只是替换该账号已经轮换的凭据。

## Save Flow

`prismux save codex [--alias <alias>]` 是恢复/高级路径：

1. 读取当前 Codex active `auth.json`。
2. 计算 SHA-256 并做重复检测。
3. 保存或更新账号 snapshot 与 SQLite account metadata。
4. 因为保存来源就是当前 active auth，所以 SQLite active target 指向该账号。

未来可以扩展 `--file` 和 `--dir` 作为显式恢复来源。

## Env Import Flow

`prismux import codex "<TOML-or-KV>"` 用于从外部导入中转站、API key 或 provider/profile 配置。配置内容放在命令最后，普通用户可以直接粘贴中转站网页给出的 Codex TOML 片段，或官方/事实标准变量名，例如 `OPENAI_API_KEY`、`OPENAI_BASE_URL` 和 `OPENAI_MODEL`。

Codex plugin 会把导入内容写入 `<codex-home>/<profile>.config.toml` 作为 Prismux 管理的 provider 来源片段，并把 provider section 安装到唯一的 live `<codex-home>/config.toml` 中。安装后的 section 使用 `prismux-<profile-name>` 命名，导入本身不改变当前 `model_provider`。OpenAI-compatible KV 会转换为该 section 下的 `base_url`、`env_key` 和 `wire_api`。Prismux SQLite 只保存 profile metadata、hash 和 snapshot ref，不保存 raw API key；KV 导入只保存 `OPENAI_API_KEY` 这类 env var 名。

`config.toml` 是用户习惯配置的唯一事实源，不按账号复制，也不维护多份完整配置。Codex App、CLI 或用户手工写入的 plugins、skills、MCP、UI、comments 和未知字段都保留在 live 文件中。`prismux use codex <profile>` 只持久修改 Prismux 管理的 provider selector，并在缺失时补回对应 provider section；写入前会检查文件是否被其他进程并发修改。显式删除 profile 时只删除仍与导入快照语义一致的 Prismux provider section；如果该 section 已被外部修改，则拒绝删除。

Profile 名解析顺序：显式 `--name`、`base_url` host、`model_provider` 或 `[model_providers.<id>]`、最后回退到 `codex-import`。例如 `https://api.apikey.fun/v1` 会生成 `api-apikey-fun.config.toml`。

Claude profile import 使用 `prismux import claude "<KV-or-JSON-or-TOML>"`。插件识别 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL` 和 Bedrock/Vertex/Foundry 相关 env，写入 Prismux 私有 profile snapshot；`prismux use claude <selector>` 只 patch user `settings.json` 的 Prismux 管理 env keys，并保留 permissions、hooks 等未知字段。

## Claude OAuth Account Flow

`prismux login claude --alias <name>` 通过官方 Claude Code CLI 执行 `claude auth login`。Prismux 继承当前终端 stdin/stdout/stderr，让官方流程负责 browser/OAuth、PKCE、token exchange 和 secure storage 写入。官方登录成功后，真实 Claude credential 已经被官方 CLI 激活；Prismux 随即读取本机 credential backend、导入 account snapshot、登记该 account 为 active，并清空同平台 profile active marker。

`prismux import claude --name <name>` 在没有外部 KV/TOML/JSON 内容时读取本机已有官方 Claude Code credential：

1. 校验 payload 包含 `claudeAiOauth.accessToken`、`refreshToken` 和 `expiresAt`。
2. 拒绝只有 `CLAUDE_CODE_OAUTH_TOKEN` 或缺少 refresh token/expiresAt 的 inference-only token。
3. 读取 `settings.json` 中的 `oauthAccount` metadata；缺失时保存 partial metadata，不调用私有 endpoint。
4. 将 credential payload 写入 private `accounts/<snapshot_hash>.credentials.snapshot`。
5. 将 `oauthAccount` metadata 写入 private `accounts/<snapshot_hash>.oauth-account.json`。
6. SQLite 只保存脱敏 email、expiresAt、auth hash、snapshot ref 和 active/archive metadata。

`prismux use claude <selector>` 唯一命中 OAuth account 时：

1. 按 number/name 解析 account。
2. 校验 credential snapshot hash。
3. 备份当前 `.credentials.json` 和 `settings.json`。
4. 原子写入目标 `.credentials.json`，并恢复 `settings.json.oauthAccount`。
5. 更新 SQLite active account；SQLite 更新失败时尝试回滚 credential/settings。

`prismux use claude <selector>` 在 account/profile 中自动推断。数字 selector 按 `TargetCatalog` 当前展示编号解析：accounts 先编号，profiles 接在后面；该编号只属于展示/选择层，不写入 registry，也不改变底层 account/profile 持久编号。非数字 selector 按 account alias 与 profile name 精确匹配；唯一命中 profile 时只 patch `settings.json.env`，唯一命中 OAuth account 时恢复 credential snapshot 和 `oauthAccount` metadata，同时命中时返回歧义错误。内部仍保留独立 account/profile plugin 逻辑，CLI 只做统一入口聚合。

## Switch Flow

`prismux use codex <selector>`：

1. 如果 Codex 同时有 accounts 和 profiles，CLI 先用 `TargetCatalog` 将数字展示编号翻译为底层 account selector 或 profile selector。
2. 命中 account 时，先比较当前 `auth.json` 与 SQLite active account snapshot。内容变化时必须从两者提取 provider subject 并确认身份一致。
3. 身份一致时将当前 auth 原子写入新的 hash snapshot，更新 active account 的 `auth_hash`/`secret_ref` 后再清理旧 snapshot；身份缺失或不匹配时拒绝切换。
4. 重新解析目标账号并校验目标 snapshot hash，确保切换到当前账号时不会使用同步前的旧记录。
5. 如果当前 active auth 存在且内容不同，先写入 backup；覆盖前再次检查 active auth 未被其他进程修改。
6. 将目标 snapshot 原子写入 Codex `auth.json`。
7. 更新 SQLite active account 和 activation timestamp；账号切换不读取或写入 `config.toml`。
8. 命中 profile 时，读取唯一 live `config.toml`，只更新 `model_provider`、可选 model selector 和对应的 `[model_providers.prismux-<name>]` section。
9. provider 写入使用原子替换和乐观并发检查；Codex App 在切换期间改写配置时，本次切换失败并要求重试，不覆盖较新的内容。
10. account 与 profile active 状态彼此独立：前者表示当前认证身份，后者表示 live config 当前选择的 provider。

## Safety Rules

- 不打印 raw auth。
- SQLite 不保存 raw auth。
- account/plan 只保存从 JWT claims 中提取的可展示 metadata，例如 email、plan、user/account id。
- capacity/availability 通过 Codex 官方源码中使用的 ChatGPT backend usage endpoint 做 best-effort 查询；请求失败、超时或响应无法解析时保持 `unknown`，并在 safe diagnostic 中记录不含 token 的失败类型。
- usage 请求使用 auth snapshot 中的 access token 和 account id，但不得把 token 放进 stdout/stderr 或 SQLite。
- alias 不能是全数字，避免和编号 selector 混淆。
- snapshot、backup 和 SQLite state root 使用私有权限。
- auth replacement 使用原子写入；SQLite 更新通过 `StateStore` 集中执行。
- 切换前备份已有 active auth。
- 切换前同步同一 provider subject 的 active auth，避免恢复已经轮换过的 refresh token。
- active auth 身份不匹配、无法验证或在最终替换检查前被并发修改时拒绝切换；这是 best-effort 检测，不是 Codex 进程共同遵守的文件锁，切换前仍应关闭运行中的 Codex 实例。
- 切换前校验 snapshot hash，hash mismatch 时拒绝写入 active auth/credentials。
- SQLite active 更新失败时尽力回滚已写入的 active auth/credentials/settings。

## Menubar App Boundary

Menubar v1 采用 `prismux-core -> prismux-app -> prismux-menubar-ffi -> Swift` 的单向边界：

1. `prismux-core` 继续保存领域类型、SQLite state、usage summary 查询和 provider plugin trait。
2. `prismux-app` 提供普通函数：`menubar_accounts`、`menubar_dashboard`、`menubar_switch`、`menubar_refresh`。它复用 plugin 的账号枚举/切换流程和 `StateStore` usage 聚合，不引入单实现 service trait。
3. `prismux-menubar-ffi` 导出 `prismux_menubar_call` 和 `prismux_menubar_free`，使用 `schema_version = 1` JSON envelope 暴露 `dashboard`、`accounts`、`switch`、`refresh`。
4. `apps/prismux-menubar` 是 SwiftPM macOS 14 App：AppKit 管 `NSStatusItem`、`NSPopover` 和 accessory lifecycle，SwiftUI 只负责账号控制面板内容。

Swift 不读取 auth、SQLite、usage logs 或 provider endpoint；它只提交 provider/local ID/refresh intent。账号切换仍由 Rust plugin 重新解析 stable local ID，并沿用备份、atomic replacement、私有权限和 rollback 语义。Menubar 的 usage 只展示 today total tokens、top client/model 和 coverage，不做 account attribution 或完整 analytics。

本地发布产物由 `scripts/build-menubar.sh` 和 `scripts/bundle-menubar.sh` 生成。bundle script 从 Cargo workspace version 写入 `CFBundleShortVersionString`，设置 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`，并执行 ad-hoc codesign；Developer ID、notarization、Sparkle 和 Homebrew cask 自动 bump 不属于 v1 gate。
