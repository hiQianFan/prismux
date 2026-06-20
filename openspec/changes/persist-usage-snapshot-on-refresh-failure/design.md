# 设计

## 调研结论

额度、限流和用量信息通常来自 provider 的在线接口或响应头，因此“最新数据”和“可展示数据”不是同一个概念。直接清空旧值会造成不必要的认知成本；静默沿用旧值又会误导用户。

参考主流设计后，本变更采用三段式语义：

- `fresh`: 本次刷新成功，展示最新额度，并记录 `refreshed_at_unix`。
- `stale with error`: 本次刷新失败，但存在上一次成功 snapshot，展示旧额度、旧刷新时间，并在 `Status` 中显示当前失败诊断。
- `unavailable`: 本次刷新失败且没有历史 snapshot，只展示 unknown/error。

这个设计接近监控系统里的 stale/no-data/error 显式建模，也符合 API 客户端对 429、timeout、network error 做保守降级和退避的实践。

## 统一数据口径

OpenMux 后续同时有 CLI、menubar、provider quota refresh 和 `tokscale-core` 本地 token usage ingestion。为了避免每个入口各自维护状态，本变更采用单一数据口径：

- `accounts`: OpenMux 管理的 authenticated account，例如 Codex OAuth account、Claude OAuth account。
- `profiles`: OpenMux 管理的 config profile，例如 Claude/Codex 的 API profile 或 provider profile。
- `quota_snapshots`: provider 返回的账号额度视图，只表示“某个账号最近一次成功查询到的额度”。
- `refresh_attempts`: 每一次额度刷新请求或跳过记录，表示“这次有没有尝试、结果是什么、用了哪个旧 snapshot 兜底”。
- `usage_events`: 后续 `tokscale-core` 从本地日志解析出的 token/cost 事件，表示“实际发生过的本地消耗”。
- `scan_watermarks`: usage event ingestion 的增量扫描水位。

这几个概念不互相替代：quota snapshot 不是 token 消耗账本，refresh attempt 不是 quota snapshot，profile 也不是 account。CLI 和 menubar 都只从这个统一状态层读数据；plugin 负责把 provider-specific 数据映射成这些领域对象。

## 存储模型

### 正式存储策略

本变更的正式 usage/quota/refresh 存储采用 SQLite。JSON 文件不再作为新的 usage snapshot 存储格式。

保留文件系统的范围只限于 auth-bearing payload 和 profile/config snapshot：

```text
<state-root>/platforms/codex/accounts/<auth_hash>.auth.json
<state-root>/platforms/codex/configs/default.config.toml
<state-root>/platforms/claude/accounts/<snapshot_hash>.credentials.snapshot
<state-root>/platforms/claude/accounts/<snapshot_hash>.oauth-account.json
<state-root>/platforms/<provider>/profiles/<config_hash>.<profile-format>
```

auth/account/profile snapshot 必须使用私有权限；SQLite 保存安全 metadata、hash、alias、provider subject 摘要、timestamps、active/archived 状态、额度快照、刷新尝试和后续 token usage event。新实现不读写旧 registry 或 `<number>.usage.json`，不设计迁移和双写。

### 稳定身份

OpenMux 应引入稳定 `local_id`：

```text
local_id: OpenMux 生成的稳定账号 ID，例如 acc_01J...
provider: codex / claude / gemini
target_kind: account / profile
provider_subject_id: 从官方配置或安全 metadata 中可获得的账号/用户/org/workspace id，允许为空
display_number: 当前 CLI 展示和选择用编号，不进入长期 usage 主键
```

`display_number` 可保存在 SQLite 中作为 CLI selector/display number，但不得作为 quota/usage 的长期身份。`local_id` 一旦创建不得复用；重复导入现存同一 auth/profile 时复用原 `local_id`。remove 后重新导入同一 auth/profile 会创建新的 OpenMux lifecycle，不恢复旧 quota/refresh history。

账号去重不能依赖整份 auth snapshot hash。OAuth/OIDC 和 JWT 的通用实践是使用 issuer-scoped subject 作为账号身份；token、refresh token 或本地 auth 文件会轮换，适合作为 credential fingerprint，不适合作为 account identity。OpenID Connect 明确 `iss + sub` 才是可依赖的稳定 End-User 标识，email 不保证稳定或唯一。因此 OpenMux 的 account identity 优先级应为：

```text
1. provider 明确暴露的 account/workspace subject，例如 Codex chatgpt_account_id
2. 标准 OIDC `iss + sub`
3. provider user subject，例如 Codex chatgpt_user_id
4. provider auth metadata 中的 account_id
5. fallback: auth_hash
```

SQLite 保存 subject 的 hash，而不是 raw subject：

```text
provider_subject_kind: codex_chatgpt_account / oidc_subject / codex_chatgpt_user / codex_account_id
provider_subject_hash: sha256(provider + ":" + kind + ":" + subject)
provider_subject_label: 可安全展示的简短 label，例如 account/user/account_id，不包含 token
```

`auth_hash` 继续保留，用于 snapshot 文件名、完整凭证指纹和篡改校验。导入、login、save 的 upsert 逻辑必须先按 `provider_subject_kind + provider_subject_hash` 查找既有 account；只有拿不到 subject 时才按 `auth_hash` fallback。相同 email 不允许作为自动合并依据，因为同一 email 可能对应 personal、team 或 enterprise 等不同 provider account。

### SQLite 本地状态库

为了支持 menubar、账号 usage 查看、refresh 历史和后续 `tokscale-core` 事件索引，本变更定义目标本地状态库：

```text
<state-root>/omx-state.sqlite
```

SQLite 只保存非敏感索引和历史，不保存 access token、refresh token、raw auth payload、API key 或完整 provider 原始响应。为避免过度设计，第一版只需要以下表；复杂 rollup、FTS、retention policy 和自定义 source 可以等 menubar/token history 真正需要时再加。

```sql
accounts(
  local_id text primary key,
  provider text not null,
  provider_subject_kind text,
  provider_subject_hash text,
  provider_subject_label text,
  alias text,
  label text,
  plan_label text,
  auth_hash text,
  secret_ref text,
  imported_at_unix integer not null,
  updated_at_unix integer not null,
  last_activated_at_unix integer,
  archived_at_unix integer
);

profiles(
  local_id text primary key,
  provider text not null,
  name text not null,
  label text,
  profile_kind text not null,
  config_hash text,
  secret_ref text,
  imported_at_unix integer not null,
  updated_at_unix integer not null,
  last_activated_at_unix integer,
  archived_at_unix integer
);

active_targets(
  provider text not null,
  target_kind text not null,
  local_id text not null,
  previous_local_id text,
  activated_at_unix integer not null,
  primary key(provider, target_kind)
);

quota_snapshots(
  id integer primary key,
  local_id text not null,
  provider text not null,
  captured_at_unix integer not null,
  source text not null,
  summary_state text not null,
  summary_display text not null,
  limits_json text not null,
  diagnostic_json text,
  refresh_attempt_id integer,
  foreign key(local_id) references accounts(local_id)
);

refresh_attempts(
  id integer primary key,
  local_id text not null,
  provider text not null,
  refresh_kind text not null,
  trigger text not null,
  attempted_at_unix integer not null,
  completed_at_unix integer,
  status text not null,
  error_code text,
  error_message text,
  http_status integer,
  duration_ms integer,
  used_snapshot_id integer,
  foreign key(local_id) references accounts(local_id)
);

usage_events(
  id integer primary key,
  provider text not null,
  local_id text,
  session_id text,
  request_id text,
  project_path text,
  model text,
  input_tokens integer,
  output_tokens integer,
  total_tokens integer,
  cost_usd real,
  occurred_at_unix integer not null,
  source_kind text not null,
  source_path text,
  source_offset integer,
  event_hash text unique
);

scan_watermarks(
  source_id text primary key,
  provider text not null,
  source_kind text not null,
  source_path text not null,
  last_offset integer,
  last_inode text,
  last_scanned_at_unix integer
);
```

`quota_snapshots` 是 provider quota 视图，回答“账号额度现在/上次是什么”；`usage_events` 是 token/cost 事件账本，回答“过去消耗了多少”。两者语义不同，不能混为一个表。

推荐最小索引：

```sql
create index idx_accounts_provider_active
  on accounts(provider, archived_at_unix);
create index idx_profiles_provider_active
  on profiles(provider, archived_at_unix);
create index idx_quota_latest
  on quota_snapshots(local_id, captured_at_unix desc);
create index idx_refresh_attempts_latest
  on refresh_attempts(local_id, attempted_at_unix desc);
create index idx_usage_events_time
  on usage_events(provider, occurred_at_unix desc);
create index idx_usage_events_project
  on usage_events(provider, project_path, occurred_at_unix desc);
```

SQLite 是新的单一状态来源。为了避免长期双写复杂度，本变更不设计 `accounts/<local_id>/usage.json`，也不保留旧 `<number>.usage.json` 读取或迁移路径。

## 刷新流程

`list_accounts()` 构建每个 `AccountStatus` 时仍触发一次在线 usage refresh：

1. 读取账号 auth snapshot。
2. 从 auth snapshot 提取 usage 查询所需的 access token 和 account id。
3. 调用 Codex usage endpoint。
4. 解析 provider payload 为 `UsageSnapshot`。
5. 先写入 `refresh_attempts(status = 'running')` 或在内存中持有 attempt context。
6. 成功时把 quota snapshot 写入 `quota_snapshots`，并把 `refresh_attempts` 更新为 `success`。

任意失败分支进入降级流程：

- auth snapshot 缺失或缺少 usage auth 字段：`code = auth`。
- curl 超时：`code = timeout`。
- 网络或进程错误：`code = network`。
- HTTP 非 2xx：`code = http_<status>`。
- JSON 解析失败：`code = schema`。
- provider 响应没有已知 quota 字段：`code = schema`。

降级流程先把 `refresh_attempts` 记录为 `error`，再尝试从 SQLite 读取该 `local_id` 的最近成功 quota snapshot：

- 如果读取成功，将 `source` 改为 `StoredSnapshot`，保留旧 `summary`、`limits` 和 `refreshed_at_unix`，并把 `diagnostics` 替换为本次失败诊断。
- 同时在 `refresh_attempts.used_snapshot_id` 记录本次展示兜底使用了哪个历史 snapshot。
- 如果读取失败，返回 `UsageSnapshot::unknown(UsageSource::RemoteApi, diagnostic)`。

失败时不会更新 `refreshed_at_unix`，因为该字段表示最后一次成功刷新时间，不表示本次尝试时间。本次尝试时间应进入 `refresh_attempts.attempted_at_unix`。

`omx list <platform>` 不应无条件制造多次请求：同一进程内对同一 `local_id` 只做一次 best-effort refresh；如果后续 menubar 加入后台任务，应通过 `refresh_attempts` 和 provider floor 判断是否跳过。

## Refresh Kind 与调度

CLI 阶段只需要同步刷新：

- `omx list <platform>`：执行一次 best-effort interactive refresh。成功则更新最近成功 snapshot；失败则展示旧 snapshot 和本次错误。
- `omx refresh <platform> [selector]`：后续可新增显式刷新命令，语义是用户主动确认额度；它可以比 `list` 更详细地展示每个账号的失败原因和耗时。

menubar 阶段必须区分：

- `interactive`: 用户打开菜单、点击刷新、切换账号前触发；允许做更积极的 provider 查询。
- `background`: 常驻后台刷新；必须 obey provider floor、TTL、失败退避和 no-activity 降频。

建议策略：

- quota refresh 设置 provider floor，例如 Codex/Claude 至少数分钟级，不因 UI 高频刷新而重复请求。
- 最近没有 `usage_events` 或没有 active account 变化时，后台刷新降频；打开菜单时再 interactive refresh。
- timeout、HTTP 429、network error 记录到 `refresh_attempts`，并按 error code 做冷却，避免重试风暴。
- 本地日志扫描由 file watcher + watermark 触发，不依赖 quota refresh 频率。

## Menubar 与 `tokscale-core`

TokenBar 类项目的核心经验是：menubar UI 不应实时全量扫描日志。OpenMux 后续接入 `tokscale-core` 时，应把它视为 usage event ingestion engine：

1. `tokscale-core` 或适配层读取 Claude/Codex/Gemini/OpenCode 等本地日志。
2. 按 source path、offset、event hash 去重。
3. 写入 `usage_events`。
4. 更新 `scan_watermarks`。
5. menubar 和 CLI 从 SQLite 聚合查询 token、cost、project、model、session。

账号额度和 token 消耗是两条数据流：

- quota snapshots 来自 provider quota/usage endpoint，适合展示剩余额度和 reset time。
- usage events 来自本地日志解析，适合展示历史消耗、项目分布和趋势。

二者可以通过 `local_id`、provider、session metadata 或未来 provider subject 进行弱关联；无法可靠关联时允许 `local_id` 为空，但仍按 provider/project/session 展示 usage。

## Remove 语义

OpenMux 需要提供 remove 能力，否则过时账号或 profile 会长期污染 `list/use/refresh`。当前版本不引入 `purge` 或用户可见 archive 命令，避免让“移除”和“彻底删除”两个概念在小型 CLI 中相互冲突。`remove` 的用户语义就是删除 OpenMux 管理的对象：

- `omx remove <platform> <selector>` 解析 account/profile selector。
- account remove：删除 OpenMux 管理的 auth-bearing snapshot 文件，清除 active target，删除 `accounts` 行，并删除该账号的 `quota_snapshots` 与 `refresh_attempts`。
- profile remove：删除 OpenMux 管理的 profile secret/config snapshot，清除 active target，并删除 `profiles` 行。
- removed account/profile 不再参与默认 `list`、`use`、`refresh`、menubar 自动选择。
- `usage_events` 当前按 client/provider/project/session/model 记录，不作为 account lifecycle 的强依赖；account remove 不清理本地 usage event。后续如果引入 account attribution，再单独定义 retention 行为。
- 如果当前 active target 被 remove，OpenMux 不自动切到另一个账号，除非用户显式 `omx use`；这样避免误切账号。

删除后重新导入同一 provider subject 会创建新的 OpenMux lifecycle。provider subject 用于避免当前 SQLite 中重复账号，不用于复活已删除账号或恢复旧 quota/refresh history。

如果未来需要“停用但保留历史归属”，应新增 `archive` 或 `disable` 语义，而不是改变既有 `remove` 行为。`archived_at_unix` 可以作为内部去重冲突处理或未来 archive 能力的 schema 余量，但不是当前用户 remove 的产品语义。

## CLI 展示

平台账号明细表新增列：

```text
* # Alias Account Plan 5h Weekly Refresh Status
```

- `5h`、`Weekly`：继续显示当前 `UsageSnapshot.limits` 中的剩余额度和 reset time。
- `Refresh`：显示 `UsageSnapshot.refreshed_at_unix`，格式复用 reset time 的本地时间格式；没有成功记录时显示 `-`。
- `Status`：优先显示 `UsageSnapshot.diagnostics[0].code`；没有诊断时按 availability state 展示。

因此当本次刷新失败且存在历史 snapshot 时，用户会看到旧额度、旧 `Refresh` 时间，以及当前错误码，例如：

```text
5h        Weekly      Refresh        Status
72% (...) 91% (...)   06-18 10:23:11 timeout
```

全局 `omx list` 暂不展开每个账号的刷新时间和诊断细节，避免 overview 过载；需要排查时使用 `omx list codex`。

## 非目标

- 不为第三方 OpenAI-compatible API key 实现统一余额/额度查询。不同网关的余额接口、鉴权方式、返回结构和成本单位差异较大，需要单独产品决策和 provider adapter。
- 本变更定义 menubar/SQLite/refresh 调度的目标设计，但不要求在当前 CLI fallback PR 中完整实现 daemon、file watcher 或后台任务。
- 不调用私有或未文档化 endpoint 之外的新接口。
- 不把 quota snapshot 当成准确计费账本；它只是“最后一次 provider 成功返回的额度视图”。准确的本地 token 消耗趋势应来自 `usage_events`。
- 不把 auth payload、raw token、API key、完整 provider 原始响应写入 SQLite。
- 不在第一版实现复杂 retention、rollup、FTS、云同步或自动最佳账号选择。

## 风险与缓解

- 旧数据误导：通过 `Refresh` 和 `Status` 显示明确 stale 语义。
- snapshot 腐坏：读取失败时回到 unknown，不阻塞账号列表。
- secret 泄露：SQLite 只保存非敏感索引；auth snapshot 继续使用私有文件或未来 Keychain；错误信息必须脱敏。
- provider schema 变化：保留旧值并显示 `schema`，后续再更新 parser。
- SQLite 迁移复杂度：先定义 `schema_version`/migrations，所有 SQL 通过 storage 封装，不让 plugin/CLI 拼接表结构。
- 账号关联不准确：以 `local_id` 为强主键，provider subject 只作为可选 metadata；无法可靠归因的 usage event 不强行绑定账号。
- 误删当前账号：remove 前展示将删除的账号/profile safe metadata；删除后不自动切换到其他账号。重新导入同一账号会产生新的 OpenMux lifecycle，不恢复旧 quota/refresh history。
