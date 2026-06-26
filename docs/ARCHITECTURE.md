# OpenMux Architecture

## 总体形态

OpenMux 当前是一个 Rust monorepo/workspace：

```text
omx-cli
  |
  v
platform plugins
  |
  v
omx-core
```

CLI 只负责命令解析和输出展示。跨平台共享概念放在 `omx-core`。每个 AI coding tool 的路径、登录、账号池和 auth 文件处理放在独立 plugin crate。

## Crates

- `omx-core`：共享领域对象、错误、报告、账号池 summary、账号状态、登录/保存 options、SQLite `StateStore` 和 `PlatformPlugin` trait。
- `omx-plugin-codex`：Codex 专属实现，包括 Codex home 解析、临时 `CODEX_HOME` 登录、auth snapshot、provider subject 去重、account/plan metadata 解析、SQLite account/profile 状态和 active auth 切换。
- `omx-plugin-claude`：Claude Code 专属实现，包括 profile import、settings env patch、macOS Keychain/plaintext `.credentials.json` account snapshot、`oauthAccount` metadata 备份/恢复，以及共享 SQLite account/profile 状态。
- `omx-cli`：`omx` 命令行前端，消费 core/plugin API，不拥有业务状态。

## Module Boundaries

`omx-core` 按领域拆分为 `account`、`profile`、`platform`、`plugin`、`report`、`storage` 和 `usage`。plugin crate 不应重复实现私有目录、原子写入、snapshot hash、路径展示和时间戳这类跨平台基础能力。

`omx-cli` 保持薄层：`main.rs` 只启动应用，`app.rs` 负责命令路由和输出展示，`input.rs` 负责 import 内容读取。平台行为必须留在 plugin crate。

`omx-plugin-codex` 和 `omx-plugin-claude` 的主流程保留在 `plugin.rs`，测试拆到 `tests.rs`。后续继续扩展 Gemini 或新的 Claude backend 时，应优先复用 core storage、SQLite `StateStore` 和 plugin capability 模型，并把 provider-specific parser/backend 维持在对应 plugin 内。

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

当平台同时暴露 accounts 和 profiles 时，CLI 不直接把用户输入的数字传给插件。`omx-core::TargetCatalog` 会把 accounts 和 profiles 聚合成当前列表 target：

1. accounts 按插件返回顺序编号；
2. profiles 接在 accounts 后继续编号；
3. 数字 selector 按当前展示编号解析；
4. 非数字 selector 按 account alias 与 profile name 精确匹配；
5. 命中多个 target 时返回歧义错误。

展示编号不作为长期 usage 身份。plugin 仍只接收自己的底层 selector：account number/alias，或 profile number/name；SQLite 中的 `local_id` 才是 quota snapshot、refresh attempt 和 future usage event 的稳定归属。

`availability` 是给旧展示和单账号保守状态判断使用的摘要字段，不能替代结构化 `usage`。平台插件应该把原始 provider quota 映射成多个 `UsageLimit`，再由最紧的可用窗口派生单账号 summary；CLI overview 的 `Overall` 则优先对结构化 limit 的剩余额度做账号池聚合。`refreshed_at_unix` 记录 OpenMux 本次获取 usage 的本地时间，不是 provider quota reset time。Codex 当前会把 `primary_window`、`secondary_window` 和 `additional_rate_limits` 解析为多个 limit，并通过 `limit_window_seconds` 识别 `5h`、`weekly` 等窗口；Claude/Gemini 后续可以复用同一模型，但保留各自的 scope、kind 和 raw provider key。provider-specific 字段只有在跨平台语义明确后才进入 core，否则应该留在插件内部或 detail formatter 中。

本地 token usage 使用另一条数据链路：

```text
tokscale-core parser
  -> omx-usage-tokscale adapter
  -> omx-core UsageEvent
  -> SQLite usage_events / scan_watermarks
  -> UsageQuery / UsageReport
  -> CLI human table / versioned JSON / future Menubar contract
```

`omx usage` 默认只展示轻量 summary：window、total tokens、按 `client` 的紧凑 rows、cost status、freshness/coverage。`--group-by day|model` 和 `--details` 负责渐进披露；human output 和 `--json` 必须来自同一个 `UsageReport`。JSON schema 采用 additive-first v1，保留 `totals`、`groups`、`freshness`、`coverage`、`accounting` 和脱敏 diagnostics。

token consumption、cost 和 subscription quota 是三种不同口径。`UsageEvent` 来自本地日志解析，不能推断 provider quota remaining；cost 只能来自 provider-reported value 或按 `model + provider + token buckets` 与 cached pricing table 估算，未知价格保持 missing。缺少可验证 evidence 时，也不能把历史 usage 归因到当前 active account/profile。未来 Menubar 不应直接查询 SQLite 表或复写 scanner/pricing/aggregation，而应消费同一 `UsageQuery`/`UsageReport` contract。

## Local State

OpenMux state 位于用户平台数据目录下。SQLite 是 account/profile/active/quota/refresh 的统一状态源，auth-bearing payload 仍保存在私有 snapshot 文件中：

```text
<data-local-dir>/openmux/
  omx-state.sqlite
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

`omx-state.sqlite` 包含 `accounts`、`profiles`、`active_targets`、`quota_snapshots` 和 `refresh_attempts`。SQLite 只保存非敏感 metadata、hash、`secret_ref`、display number、timestamps、active target 状态和 quota/refresh 历史；raw auth payload、access token、refresh token、API key 和完整 provider 原始响应不进入 SQLite。

账号或 profile remove 使用 hard delete 语义：删除 OpenMux 管理的 secret/config snapshot，清除 active target，并删除对应 `accounts`/`profiles` 行。account remove 同时删除该账号的 `quota_snapshots` 和 `refresh_attempts`；本地 token usage event 不在当前版本绑定 account lifecycle，因此不随 account remove 清理。

账号唯一身份优先使用 provider subject，而不是整份 auth 文件 hash。Codex 从 `id_token` 和 `tokens.account_id` 中按优先级提取 `chatgpt_account_id`、`iss+sub`、`chatgpt_user_id/user_id`、`account_id`，SQLite 只保存 `provider_subject_hash` 和 kind/label。`auth_hash` 继续作为 snapshot 文件名、凭证指纹和切换时的篡改校验；当 auth 中没有可用 subject 时才作为去重 fallback。email 只作为展示 metadata，不能作为自动合并依据。

## Codex Path Resolution

Codex home 解析顺序：

1. 测试或嵌入方注入的显式路径。
2. `$CODEX_HOME`。
3. `~/.codex`。

OpenMux state root 解析顺序：

1. 测试或嵌入方注入的显式路径。
2. `$OMUX_STATE_ROOT`。
3. OS 平台本地数据目录下的 `openmux`。

Codex active auth path：

```text
<codex-home>/auth.json
```

## Login Flow

`omx login codex [--device-auth] [--alias <alias>] [--use]`：

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

`login` 负责获取并登记凭据，`use` 负责改变当前运行上下文。默认不激活使添加账号保持最小副作用，并允许脚本先完成多账号登记；`--use` 是将两步组合为一次操作的显式便利开关。CLI 在未传 `--use` 时必须打印可直接执行的 `omx use codex <selector>`。如果新登录结果与当前 active account 是同一 provider subject，则应同步更新 active auth，因为这不改变账号上下文，只是替换该账号已经轮换的凭据。

## Save Flow

`omx save codex [--alias <alias>]` 是恢复/高级路径：

1. 读取当前 Codex active `auth.json`。
2. 计算 SHA-256 并做重复检测。
3. 保存或更新账号 snapshot 与 SQLite account metadata。
4. 因为保存来源就是当前 active auth，所以 SQLite active target 指向该账号。

未来可以扩展 `--file` 和 `--dir` 作为显式恢复来源。

## Env Import Flow

`omx import codex "<TOML-or-KV>"` 用于从外部导入中转站、API key 或 provider/profile 配置。配置内容放在命令最后，普通用户可以直接粘贴中转站网页给出的 Codex TOML 片段，或官方/事实标准变量名，例如 `OPENAI_API_KEY`、`OPENAI_BASE_URL` 和 `OPENAI_MODEL`。

Codex plugin 会把导入内容写入 `<codex-home>/<profile>.config.toml` 作为 OpenMux 管理的 provider 来源片段，并把 provider section 安装到唯一的 live `<codex-home>/config.toml` 中。安装后的 section 使用 `openmux-<profile-name>` 命名，导入本身不改变当前 `model_provider`。OpenAI-compatible KV 会转换为该 section 下的 `base_url`、`env_key` 和 `wire_api`。OpenMux SQLite 只保存 profile metadata、hash 和 snapshot ref，不保存 raw API key；KV 导入只保存 `OPENAI_API_KEY` 这类 env var 名。

`config.toml` 是用户习惯配置的唯一事实源，不按账号复制，也不维护多份完整配置。Codex App、CLI 或用户手工写入的 plugins、skills、MCP、UI、comments 和未知字段都保留在 live 文件中。`omx use codex <profile>` 只持久修改 OpenMux 管理的 provider selector，并在缺失时补回对应 provider section；写入前会检查文件是否被其他进程并发修改。显式删除 profile 时只删除仍与导入快照语义一致的 OpenMux provider section；如果该 section 已被外部修改，则拒绝删除。

Profile 名解析顺序：显式 `--name`、`base_url` host、`model_provider` 或 `[model_providers.<id>]`、最后回退到 `codex-import`。例如 `https://api.apikey.fun/v1` 会生成 `api-apikey-fun.config.toml`。

Claude profile import 使用 `omx import claude "<KV-or-JSON-or-TOML>"`。插件识别 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL` 和 Bedrock/Vertex/Foundry 相关 env，写入 OpenMux 私有 profile snapshot；`omx use claude <selector>` 只 patch user `settings.json` 的 OpenMux 管理 env keys，并保留 permissions、hooks 等未知字段。

## Claude OAuth Account Flow

`omx login claude --alias <name>` 通过官方 Claude Code CLI 执行 `claude auth login`。OpenMux 继承当前终端 stdin/stdout/stderr，让官方流程负责 browser/OAuth、PKCE、token exchange 和 secure storage 写入。官方登录成功后，真实 Claude credential 已经被官方 CLI 激活；OpenMux 随即读取本机 credential backend、导入 account snapshot、登记该 account 为 active，并清空同平台 profile active marker。

`omx import claude --name <name>` 在没有外部 KV/TOML/JSON 内容时读取本机已有官方 Claude Code credential：

1. 校验 payload 包含 `claudeAiOauth.accessToken`、`refreshToken` 和 `expiresAt`。
2. 拒绝只有 `CLAUDE_CODE_OAUTH_TOKEN` 或缺少 refresh token/expiresAt 的 inference-only token。
3. 读取 `settings.json` 中的 `oauthAccount` metadata；缺失时保存 partial metadata，不调用私有 endpoint。
4. 将 credential payload 写入 private `accounts/<snapshot_hash>.credentials.snapshot`。
5. 将 `oauthAccount` metadata 写入 private `accounts/<snapshot_hash>.oauth-account.json`。
6. SQLite 只保存脱敏 email、expiresAt、auth hash、snapshot ref 和 active/archive metadata。

`omx use claude <selector>` 唯一命中 OAuth account 时：

1. 按 number/name 解析 account。
2. 校验 credential snapshot hash。
3. 备份当前 `.credentials.json` 和 `settings.json`。
4. 原子写入目标 `.credentials.json`，并恢复 `settings.json.oauthAccount`。
5. 更新 SQLite active account；SQLite 更新失败时尝试回滚 credential/settings。

`omx use claude <selector>` 在 account/profile 中自动推断。数字 selector 按 `TargetCatalog` 当前展示编号解析：accounts 先编号，profiles 接在后面；该编号只属于展示/选择层，不写入 registry，也不改变底层 account/profile 持久编号。非数字 selector 按 account alias 与 profile name 精确匹配；唯一命中 profile 时只 patch `settings.json.env`，唯一命中 OAuth account 时恢复 credential snapshot 和 `oauthAccount` metadata，同时命中时返回歧义错误。内部仍保留独立 account/profile plugin 逻辑，CLI 只做统一入口聚合。

## Switch Flow

`omx use codex <selector>`：

1. 如果 Codex 同时有 accounts 和 profiles，CLI 先用 `TargetCatalog` 将数字展示编号翻译为底层 account selector 或 profile selector。
2. 命中 account 时，先比较当前 `auth.json` 与 SQLite active account snapshot。内容变化时必须从两者提取 provider subject 并确认身份一致。
3. 身份一致时将当前 auth 原子写入新的 hash snapshot，更新 active account 的 `auth_hash`/`secret_ref` 后再清理旧 snapshot；身份缺失或不匹配时拒绝切换。
4. 重新解析目标账号并校验目标 snapshot hash，确保切换到当前账号时不会使用同步前的旧记录。
5. 如果当前 active auth 存在且内容不同，先写入 backup；覆盖前再次检查 active auth 未被其他进程修改。
6. 将目标 snapshot 原子写入 Codex `auth.json`。
7. 更新 SQLite active account 和 activation timestamp；账号切换不读取或写入 `config.toml`。
8. 命中 profile 时，读取唯一 live `config.toml`，只更新 `model_provider`、可选 model selector 和对应的 `[model_providers.openmux-<name>]` section。
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

Menubar v1 采用 `omx-core -> omx-app -> omx-menubar-ffi -> Swift` 的单向边界：

1. `omx-core` 继续保存领域类型、SQLite state、usage summary 查询和 provider plugin trait。
2. `omx-app` 提供普通函数：`menubar_accounts`、`menubar_dashboard`、`menubar_switch`、`menubar_refresh`。它复用 plugin 的账号枚举/切换流程和 `StateStore` usage 聚合，不引入单实现 service trait。
3. `omx-menubar-ffi` 导出 `omx_menubar_call` 和 `omx_menubar_free`，使用 `schema_version = 1` JSON envelope 暴露 `dashboard`、`accounts`、`switch`、`refresh`。
4. `apps/omx-menubar` 是 SwiftPM macOS 14 App：AppKit 管 `NSStatusItem`、`NSPopover` 和 accessory lifecycle，SwiftUI 只负责账号控制面板内容。

Swift 不读取 auth、SQLite、usage logs 或 provider endpoint；它只提交 provider/local ID/refresh intent。账号切换仍由 Rust plugin 重新解析 stable local ID，并沿用备份、atomic replacement、私有权限和 rollback 语义。Menubar 的 usage 只展示 today total tokens、top client/model 和 coverage，不做 account attribution 或完整 analytics。

本地发布产物由 `scripts/build-menubar.sh` 和 `scripts/bundle-menubar.sh` 生成。bundle script 从 Cargo workspace version 写入 `CFBundleShortVersionString`，设置 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`，并执行 ad-hoc codesign；Developer ID、notarization、Sparkle 和 Homebrew cask 自动 bump 不属于 v1 gate。
