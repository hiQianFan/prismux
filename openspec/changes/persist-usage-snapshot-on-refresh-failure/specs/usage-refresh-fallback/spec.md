# Capability: Usage Refresh Fallback

## ADDED Requirements

### Requirement: 成功刷新后持久化 quota snapshot

OpenMux SHALL 在账号额度在线刷新成功后把该账号最后一次成功的结构化 quota snapshot 保存到 SQLite。

#### Scenario: 保存最后一次成功额度到 SQLite

- GIVEN Codex account `#1` 具有稳定 `local_id`
- AND Codex usage endpoint 返回可解析的 quota payload
- WHEN 用户运行 `omx list codex`
- THEN OpenMux SHALL 将本次成功额度写入本地 SQLite `quota_snapshots`
- AND OpenMux SHALL 将本次刷新尝试写入 `refresh_attempts`
- AND `refresh_attempts.status` SHALL be `success`
- AND SQLite 记录不包含 raw auth payload、access token、refresh token、API key 或完整 provider 原始响应。

### Requirement: 刷新失败时沿用最后一次成功 snapshot

OpenMux SHALL 在账号额度刷新失败且 SQLite 中存在历史 quota snapshot 时展示历史额度，并同时暴露本次失败诊断。

#### Scenario: timeout 后展示旧额度

- GIVEN Codex account `#1` 已有一次成功 SQLite quota snapshot
- AND 该 quota snapshot 的 `refreshed_at_unix` 是 `1785000000`
- WHEN 用户运行 `omx list codex`
- AND 本次 Codex usage refresh 超时
- THEN account `#1` 的 5h 和 weekly 额度来自历史 snapshot
- AND account `#1` 的刷新时间显示为 `1785000000` 对应的本地时间
- AND account `#1` 的 status 显示本次失败诊断 `timeout`
- AND OpenMux 不把本次失败时间写成新的成功刷新时间。
- AND OpenMux 记录一条失败的 refresh attempt，包含 attempted time、error code 和 refresh kind。

#### Scenario: auth 缺失但存在旧 snapshot

- GIVEN Codex account `#1` 已有一次成功 SQLite quota snapshot
- AND 该账号保存的 auth snapshot 缺少 usage 查询所需字段
- WHEN 用户运行 `omx list codex`
- THEN account `#1` 继续展示历史额度
- AND account `#1` 的 status 显示 `auth`
- AND account `#1` 的 usage source 标记为 stored snapshot。

### Requirement: 没有历史 quota snapshot 时保持 unavailable

OpenMux SHALL 在刷新失败且 SQLite 中没有历史 quota snapshot 时展示 unknown/error，而不是合成虚假额度。

#### Scenario: 首次刷新失败

- GIVEN Codex account `#1` 没有历史 SQLite quota snapshot
- WHEN 用户运行 `omx list codex`
- AND 本次 usage refresh 失败
- THEN account `#1` 的额度显示为 `-`
- AND account `#1` 的刷新时间显示为 `-`
- AND account `#1` 的 status 显示本次失败诊断。

### Requirement: 平台明细展示刷新时间

OpenMux SHALL 在平台账号明细列表中显示额度数据最后一次成功刷新时间。

#### Scenario: 展示 refresh 列

- GIVEN Codex account `#1` 的 SQLite quota snapshot 包含 `refreshed_at_unix`
- WHEN 用户运行 `omx list codex`
- THEN account table 包含 `Refresh` 列
- AND `Refresh` 列显示最后一次成功刷新时间
- AND `Status` 列仍显示当前诊断或 availability state。

### Requirement: 稳定账号身份绑定 usage 数据

OpenMux SHALL 使用稳定 `local_id` 绑定账号/profile、quota snapshot、refresh attempt 和 usage event；CLI number 只作为展示和选择用途。

#### Scenario: 账号编号变化后仍读取原 usage

- GIVEN Codex account `A` 的 `local_id` 是 `acc_aaa`
- AND account `A` 当前显示为 `#1`
- AND account `A` 已有成功 quota snapshot
- WHEN 用户新增或删除其他账号导致 account `A` 的显示编号变化
- THEN account `A` 的 quota snapshot 仍通过 `acc_aaa` 读取
- AND OpenMux 不把其他账号的 usage snapshot 展示到 account `A`。

#### Scenario: 重复导入复用 local_id

- GIVEN 当前 Codex auth 已导入为 account `A`
- AND account `A` 的 `local_id` 是 `acc_aaa`
- WHEN 用户再次导入同一 auth snapshot
- THEN OpenMux SHALL 复用 `acc_aaa`
- AND 不创建新的 usage history identity。

#### Scenario: Token 轮换后仍复用 Codex account

- GIVEN Codex account `A` 的 id token 包含稳定 `chatgpt_account_id = acct_123`
- AND account `A` 的 `local_id` 是 `acc_aaa`
- WHEN 用户重新 login 或 save，新的 auth snapshot hash 与旧 hash 不同
- AND 新 id token 仍包含 `chatgpt_account_id = acct_123`
- THEN OpenMux SHALL 复用 `acc_aaa`
- AND 更新该 account 的 auth hash、secret ref 和 metadata
- AND 不创建新的 account number。

#### Scenario: 同 email 但不同 provider subject 不自动合并

- GIVEN 两个 Codex auth snapshot 展示相同 email
- AND 它们的 `chatgpt_account_id` 不同
- WHEN 用户分别导入这两个 snapshot
- THEN OpenMux SHALL 创建两个不同 account
- AND 不得只按 email 自动合并。

#### Scenario: 缺少 provider subject 时使用 auth hash fallback

- GIVEN Codex auth snapshot 无法提取 `chatgpt_account_id`、`iss+sub`、`chatgpt_user_id` 或 `tokens.account_id`
- WHEN 用户导入该 snapshot
- THEN OpenMux SHALL 使用 auth hash 作为 fallback identity
- AND 相同 auth hash SHALL 复用同一 account。

#### Scenario: Profile 使用独立 local_id

- GIVEN 用户导入一个 Claude profile `work`
- WHEN OpenMux 保存该 profile
- THEN OpenMux SHALL 为该 profile 生成稳定 `local_id`
- AND 该 `local_id` SHALL NOT 与 account `local_id` 复用
- AND profile 的 secret/config snapshot 不进入 SQLite。

### Requirement: 本地 SQLite 状态库只保存非敏感状态

OpenMux SHALL 使用本地 SQLite 保存可查询的非敏感状态，并继续把 auth-bearing payload 留在私有文件或未来 Keychain。

#### Scenario: SQLite 保存 quota 和 refresh history

- GIVEN Codex account 具有 `local_id`
- WHEN 账号额度刷新成功或失败
- THEN OpenMux SHALL 在 SQLite 中保存 `quota_snapshots` 和 `refresh_attempts`
- AND SQLite SHALL NOT 保存 raw auth payload、access token、refresh token、API key 或完整 provider 原始响应。

#### Scenario: SQLite 统一管理 active 和 removed 状态

- GIVEN OpenMux 管理多个 account 和 profile
- WHEN 用户运行 `omx list <platform>`
- THEN OpenMux SHALL 从 SQLite 读取现存 account/profile 作为默认展示集合
- AND active account/profile SHALL come from SQLite active target state
- AND removed account/profile SHALL NOT participate in default `use` or `refresh` resolution。

#### Scenario: 不读取旧 JSON snapshot

- GIVEN 旧版本状态目录中存在 `<number>.usage.json`
- AND SQLite 中没有该账号的 quota snapshot
- WHEN OpenMux 读取该账号 usage
- THEN OpenMux SHALL NOT 迁移或读取旧 JSON snapshot
- AND OpenMux SHALL 在本次在线 refresh 失败时展示 unknown/error。

### Requirement: Refresh attempt 与 quota snapshot 分离

OpenMux SHALL 分开记录最后一次成功 quota snapshot 和每一次 refresh attempt。

#### Scenario: 失败不会覆盖成功快照时间

- GIVEN account `A` 在 `1785000000` 成功刷新过 quota snapshot
- WHEN 后续 refresh 在 `1785000300` 因 timeout 失败
- THEN `quota_snapshots` 中最近成功 snapshot 的 captured time 仍是 `1785000000`
- AND `refresh_attempts` 中存在 `1785000300` 的失败记录
- AND CLI 展示旧额度时同时展示旧 refresh time 和当前失败状态。

#### Scenario: 跳过刷新也记录原因

- GIVEN account `A` 最近一次 background refresh 尚未超过 provider floor
- WHEN menubar background refresh 请求刷新 account `A`
- THEN OpenMux SHALL NOT 请求 provider endpoint
- AND OpenMux SHALL 记录 `refresh_attempts.status = skipped`
- AND `refresh_attempts.error_code` 或 message SHALL indicate cooldown or provider floor。

### Requirement: 区分 interactive 与 background refresh

OpenMux SHALL 为未来 CLI 和 menubar 使用统一 refresh kind 语义。

#### Scenario: CLI list 使用 interactive refresh

- WHEN 用户运行 `omx list codex`
- THEN OpenMux SHALL 将本次 best-effort 额度刷新记录为 `interactive`
- AND 该刷新失败时仍可沿用最近成功 quota snapshot。

#### Scenario: Menubar 后台刷新 obey provider floor

- GIVEN menubar 后台任务请求刷新 account `A`
- AND account `A` 的 provider floor 或 cooldown 尚未到期
- WHEN 后台任务执行
- THEN OpenMux SHALL 跳过在线 provider 请求
- AND MAY 记录 skipped refresh attempt
- AND menubar SHALL 继续展示最近成功 quota snapshot 及其刷新时间。

### Requirement: 支持后续 token usage event 索引

OpenMux SHALL 为后续接入 `tokscale-core` 预留本地 usage event 模型，且不把 quota snapshot 当作 token 消耗账本。

#### Scenario: 写入本地 usage event

- GIVEN `tokscale-core` 适配层解析到一个 Codex token usage event
- AND 该 event 包含 provider、session、model、token count、occurred time 和 source offset
- WHEN OpenMux ingest 该 event
- THEN OpenMux SHALL 写入 SQLite `usage_events`
- AND 更新对应 `scan_watermarks`
- AND 使用 event hash、request id 或 source offset 避免重复写入。

#### Scenario: 无法可靠绑定账号

- GIVEN 一个 usage event 无法可靠映射到具体 `local_id`
- WHEN OpenMux ingest 该 event
- THEN OpenMux SHALL 允许 `local_id` 为空
- AND 仍可按 provider、project、session 或 model 聚合展示。

### Requirement: 移除过时 account 和 profile

OpenMux SHALL 支持移除 OpenMux 管理的 account 和 profile，使其不再参与默认 list/use/refresh，并删除对应 OpenMux 管理状态。

#### Scenario: 移除 account

- GIVEN Codex account `A` 已导入 OpenMux
- AND account `A` 具有 auth-bearing snapshot
- AND account `A` 具有历史 `quota_snapshots` 和 `refresh_attempts`
- WHEN 用户运行 `omx remove codex <selector-for-A>`
- THEN OpenMux SHALL 删除 OpenMux 管理的 auth-bearing snapshot
- AND 删除 account `A` 的 SQLite account row
- AND 删除 account `A` 的 `quota_snapshots` 和 `refresh_attempts`
- AND account `A` SHALL NOT appear in default `omx list codex`
- AND account `A` SHALL NOT be selectable by default `omx use codex`
- AND 重新导入同一 provider subject SHALL create a new OpenMux account lifecycle without restoring old quota/refresh history。

#### Scenario: 移除当前 active account

- GIVEN Codex account `A` 是当前 active account
- WHEN 用户运行 `omx remove codex <selector-for-A>`
- THEN OpenMux SHALL clear the active account target
- AND SHALL NOT automatically switch to another account
- AND later `omx use codex <selector>` SHALL require an explicit user choice。

#### Scenario: 移除 profile

- GIVEN Claude profile `work` 已由 OpenMux 管理
- WHEN 用户运行 `omx remove claude work`
- THEN OpenMux SHALL 删除 OpenMux 管理的 profile secret/config snapshot
- AND 删除 profile 的 SQLite profile row
- AND profile SHALL NOT appear in default `omx list claude`
- AND profile SHALL NOT be selectable by default `omx use claude`。
