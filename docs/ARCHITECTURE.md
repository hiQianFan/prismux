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

- `omx-core`：共享领域对象、错误、报告、账号池 summary、账号状态、登录/保存 options 和 `PlatformPlugin` trait。
- `omx-plugin-codex`：Codex 专属实现，包括 Codex home 解析、临时 `CODEX_HOME` 登录、numbered registry、auth snapshot、hash 去重、account/plan metadata 解析和 active auth 切换。
- `omx-plugin-claude`：Claude Code 专属实现，包括 profile registry、settings env patch、macOS Keychain/plaintext `.credentials.json` account snapshot、`oauthAccount` metadata 备份/恢复。
- `omx-cli`：`omx` 命令行前端，消费 core/plugin API，不拥有业务状态。

## Module Boundaries

`omx-core` 按领域拆分为 `account`、`profile`、`platform`、`plugin`、`report`、`storage` 和 `usage`。plugin crate 不应重复实现私有目录、原子写入、snapshot hash、路径展示和时间戳这类跨平台基础能力。

`omx-cli` 保持薄层：`main.rs` 只启动应用，`app.rs` 负责命令路由和输出展示，`input.rs` 负责 import 内容读取。平台行为必须留在 plugin crate。

`omx-plugin-codex` 和 `omx-plugin-claude` 的主流程保留在 `plugin.rs`，registry 文本编解码拆到 `registry_io.rs`，测试拆到 `tests.rs`。后续继续扩展 Gemini 或新的 Claude backend 时，应优先复用 core storage 和 plugin capability 模型，并把 provider-specific parser/backend 维持在对应 plugin 内。

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

账号编号是平台内 selector。`codex #1` 和 `claude #1` 没有身份上的关联。alias 是可选 metadata，不是账号创建前置条件。

`availability` 是给旧展示和单账号保守状态判断使用的摘要字段，不能替代结构化 `usage`。平台插件应该把原始 provider quota 映射成多个 `UsageLimit`，再由最紧的可用窗口派生单账号 summary；CLI overview 的 `Overall` 则优先对结构化 limit 的剩余额度做账号池聚合。`refreshed_at_unix` 记录 OpenMux 本次获取 usage 的本地时间，不是 provider quota reset time。Codex 当前会把 `primary_window`、`secondary_window` 和 `additional_rate_limits` 解析为多个 limit，并通过 `limit_window_seconds` 识别 `5h`、`weekly` 等窗口；Claude/Gemini 后续可以复用同一模型，但保留各自的 scope、kind 和 raw provider key。provider-specific 字段只有在跨平台语义明确后才进入 core，否则应该留在插件内部或 detail formatter 中。

## Local State

OpenMux state 位于用户平台数据目录下，并按平台隔离：

```text
<data-local-dir>/openmux/platforms/codex/
  registry.omx
  accounts/1.auth.json
  accounts/2.auth.json
  backups/auth.json.bak.<timestamp>
  login/codex-login-<pid>-<timestamp>/
```

`registry.omx` 是轻量文本 metadata 文件：

```text
schema_version 1
active_number 1
previous_active_number 2
next_number 3
account 1 <alias-or-empty> <account-or-empty> <plan-or-empty> <auth_hash> <snapshot_path> <imported_at> <last_activated_at>
```

raw auth payload 只存在于 snapshot / active auth / backup 文件中，不写入 registry metadata。

registry 写入时会在 account 行中保存可安全展示的 account 和 plan metadata。

Claude Code 使用同一个 `<openmux-state>/platforms/claude/` 平台目录，但 profile 与 OAuth account 分开记录：

```text
<data-local-dir>/openmux/platforms/claude/
  registry.omx
  account-registry.omx
  profiles/1.profile.json
  accounts/1.credentials.snapshot
  accounts/1.oauth-account.json
  backups/settings.json.bak.<timestamp>
  backups/credentials.snapshot.bak.<timestamp>
```

`registry.omx` 记录 Claude profile 编号、active profile、auth type、base URL、model、secret hash 和 snapshot path。`account-registry.omx` 记录 Claude OAuth account 编号、active account、脱敏 email、account/org UUID 短 hash、scopes、expiresAt、snapshot hash、metadata path 和 backup 相关路径。raw token 只存在于 private snapshot、active `.credentials.json` 或 backup 文件中。

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

## Save Flow

`omx save codex [--alias <alias>]` 是恢复/高级路径：

1. 读取当前 Codex active `auth.json`。
2. 计算 SHA-256 并做重复检测。
3. 保存或更新账号 snapshot。
4. 因为保存来源就是当前 active auth，所以 registry active number 指向该账号。

未来可以扩展 `--file` 和 `--dir` 作为显式恢复来源。

## Env Import Flow

`omx import codex "<TOML-or-KV>"` 用于从外部导入中转站、API key 或 provider/profile 配置。配置内容放在命令最后，普通用户可以直接粘贴中转站网页给出的 Codex TOML 片段，或官方/事实标准变量名，例如 `OPENAI_API_KEY`、`OPENAI_BASE_URL` 和 `OPENAI_MODEL`。

Codex plugin 会把导入内容写入 `<codex-home>/<profile>.config.toml`，符合 Codex 官方 profile 文件模型，避免覆盖用户现有 `config.toml`。TOML 片段会原样保存；OpenAI-compatible KV 会转换为 `[model_providers.<id>]` 下的 `base_url`、`env_key` 和 `wire_api`。OpenMux registry 不保存 raw API key；KV 导入只把 `OPENAI_API_KEY` 这类 env var 名写入 profile。

Profile 名解析顺序：显式 `--name`、`base_url` host、`model_provider` 或 `[model_providers.<id>]`、最后回退到 `codex-import`。例如 `https://api.apikey.fun/v1` 会生成 `api-apikey-fun.config.toml`。

Claude profile import 使用 `omx import claude "<KV-or-JSON-or-TOML>"`。插件识别 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL` 和 Bedrock/Vertex/Foundry 相关 env，写入 OpenMux 私有 profile snapshot；`omx use claude <selector>` 只 patch user `settings.json` 的 OpenMux 管理 env keys，并保留 permissions、hooks 等未知字段。

## Claude OAuth Account Flow

`omx login claude --alias <name>` 通过官方 Claude Code CLI 执行 `claude auth login`。OpenMux 继承当前终端 stdin/stdout/stderr，让官方流程负责 browser/OAuth、PKCE、token exchange 和 secure storage 写入。官方登录成功后，OpenMux 读取本机 credential backend 并导入 account snapshot。

`omx import claude --name <name>` 在没有外部 KV/TOML/JSON 内容时读取本机已有官方 Claude Code credential：

1. 校验 payload 包含 `claudeAiOauth.accessToken`、`refreshToken` 和 `expiresAt`。
2. 拒绝只有 `CLAUDE_CODE_OAUTH_TOKEN` 或缺少 refresh token/expiresAt 的 inference-only token。
3. 读取 `settings.json` 中的 `oauthAccount` metadata；缺失时保存 partial metadata，不调用私有 endpoint。
4. 将 credential payload 写入 private `accounts/<number>.credentials.snapshot`。
5. 将 `oauthAccount` metadata 写入 private `accounts/<number>.oauth-account.json`。
6. registry 只保存脱敏 email、短 hash、scopes、expiresAt、snapshot hash 和 metadata path。

`omx use claude <selector>` 唯一命中 OAuth account 时：

1. 按 number/name 解析 account。
2. 校验 credential snapshot hash。
3. 备份当前 `.credentials.json` 和 `settings.json`。
4. 原子写入目标 `.credentials.json`，并恢复 `settings.json.oauthAccount`。
5. 更新 `account-registry.omx` active account；registry 更新失败时尝试回滚 credential/settings。

`omx use claude <selector>` 在 account/profile 中自动推断。数字 selector 按当前 `omx list claude` 的分组展示编号解析：accounts 先编号，profiles 接在后面；该编号只属于 CLI 展示/选择层，不写入 registry，也不改变底层 account/profile 持久编号。非数字 selector 按 account alias 与 profile name 精确匹配；唯一命中 profile 时只 patch `settings.json.env`，唯一命中 OAuth account 时恢复 credential snapshot 和 `oauthAccount` metadata，同时命中时返回歧义错误。内部仍保留独立 account/profile plugin 逻辑，CLI 只做统一入口聚合。

## Switch Flow

`omx use codex <selector>`：

1. selector 优先按平台内编号解析。
2. 如果不是编号，则按 alias 精确匹配。
3. 读取目标账号 snapshot。
4. 如果当前 active auth 存在且内容不同，先写入 backup。
5. 将目标 snapshot 原子写入 Codex `auth.json`。
6. 更新 active number、previous active number 和 activation timestamp。

## Safety Rules

- 不打印 raw auth。
- registry 不保存 raw auth。
- account/plan 只保存从 JWT claims 中提取的可展示 metadata，例如 email、plan、user/account id。
- capacity/availability 通过 Codex 官方源码中使用的 ChatGPT backend usage endpoint 做 best-effort 查询；请求失败、超时或响应无法解析时保持 `unknown`，并在 safe diagnostic 中记录不含 token 的失败类型。
- usage 请求使用 auth snapshot 中的 access token 和 account id，但不得把 token 放进 stdout/stderr 或 registry。
- alias 不能是全数字，避免和编号 selector 混淆。
- snapshot、backup、registry 使用私有权限。
- registry 和 auth replacement 使用原子写入。
- 切换前备份已有 active auth。
- 切换前校验 snapshot hash，hash mismatch 时拒绝写入 active auth/credentials。
- registry 更新失败时尽力回滚已写入的 active auth/credentials/settings。
- 拒绝当前程序不支持的未来 registry schema。
