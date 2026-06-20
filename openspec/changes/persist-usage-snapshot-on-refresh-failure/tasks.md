## 0. 设计收敛

- [x] 0.1 对比 TokenBar 的 SQLite usage event/watermark 模型和 ClaudeBar 的 account/snapshot/refresh kind 模型。
- [x] 0.2 明确 OpenMux 统一数据口径：accounts/profiles、quota_snapshots、refresh_attempts、usage_events、scan_watermarks。
- [x] 0.3 明确 SQLite 是 usage/quota/refresh 的正式存储，不保留旧 JSON usage snapshot 迁移或双写。
- [x] 0.4 明确 remove 默认语义：删除 OpenMux 管理对象、secret/config snapshot 和账号级 quota/refresh 记录。

## 1. SQLite Fallback 实现

- [x] 1.1 为 Codex account 建立 SQLite quota snapshot 记录。
- [x] 1.2 在线 usage refresh 成功后写入 `quota_snapshots`。
- [x] 1.3 usage refresh 成功/失败时写入 `refresh_attempts`。
- [x] 1.4 usage refresh 失败时读取历史 quota snapshot 并标记 `UsageSource::StoredSnapshot`。
- [x] 1.5 保留历史 `refreshed_at_unix`、summary 和 limits，同时用本次失败 diagnostics 替换旧 diagnostics。
- [x] 1.6 没有历史 snapshot 时保持 unknown/error 行为。
- [x] 1.7 删除新的 per-account usage JSON 写入路径。

## 2. CLI 展示

- [x] 2.1 账号明细表新增 `Refresh` 列。
- [x] 2.2 `Refresh` 显示最后一次成功 usage refresh 时间，没有数据时显示 `-`。
- [x] 2.3 `Status` 在降级展示时显示本次失败 diagnostic code。

## 3. 测试与验证

- [x] 3.1 增加 Codex usage refresh 失败时沿用历史 snapshot 的回归测试。
- [x] 3.2 更新 CLI account table header 测试。
- [x] 3.3 运行 `cargo fmt --all`。
- [x] 3.4 运行 `cargo test`。
- [x] 3.5 运行 `cargo clippy --all-targets --all-features`。

## 4. 稳定账号身份

- [x] 4.1 在 core 账号/profile 记录中增加稳定 `local_id`，并明确 `number` 只用于 CLI selector/display。
- [x] 4.2 为 Codex account import 生成并持久化 `local_id`，重复导入同一 auth 时复用原 `local_id`。
- [x] 4.3 为 Claude account/profile 生成并持久化 `local_id`，profile 与 account 不复用 identity。
- [x] 4.4 将 usage snapshot identity 从 `<number>` 改为 SQLite `local_id` 外键，停止写入新的 usage JSON 文件。
- [x] 4.5 为 account 增加 `provider_subject_kind`、`provider_subject_hash`、`provider_subject_label`，并建立 subject 唯一索引。
- [x] 4.6 Codex import/login/save 优先使用 `chatgpt_account_id`、`iss+sub`、`chatgpt_user_id`、`tokens.account_id` 去重，`auth_hash` 仅作为 fallback。
- [x] 4.7 增加同一 Codex subject 不同 token hash 不新增账号的回归测试。
- [x] 4.8 增加同一 email 不同 Codex account subject 必须保留两个账号的回归测试。
- [ ] 4.9 增加新增/删除/重排账号后 SQLite quota snapshot 仍绑定原账号的回归测试。

## 5. SQLite 本地状态库

- [x] 5.1 选择 Rust SQLite 依赖并在 core 层封装连接、迁移和查询，CLI/plugin 不直接管理数据库文件。
- [x] 5.2 实现 `accounts`、`profiles`、`active_targets`、`quota_snapshots`、`refresh_attempts` 的 schema、indexes 和 migration。
- [x] 5.3 usage refresh 成功时写入 `quota_snapshots`，成功/失败写入 `refresh_attempts`。
- [ ] 5.4 refresh 失败且存在历史 quota snapshot 时，在 `refresh_attempts.used_snapshot_id` 记录本次兜底来源。
- [x] 5.5 `omx list <platform>` 从 SQLite 读取最近成功 quota snapshot；SQLite 缺失时不读取旧 JSON。
- [x] 5.6 保持 auth snapshot、raw token、API key 和完整 provider 原始响应不进入 SQLite。
- [x] 5.7 增加 SQLite fallback 回归测试，确认不依赖旧 usage JSON。

## 6. Refresh 命令与调度语义

- [ ] 6.1 增加 `omx refresh <platform> [selector]`，用于显式刷新账号额度并输出每个账号的 refresh attempt 结果。
- [ ] 6.2 在领域模型中增加 `RefreshKind`：`interactive` 和 `background`。
- [ ] 6.3 为 timeout、network、HTTP 429 和 schema error 定义 retry/cooldown 语义，供 menubar 后续复用。
- [ ] 6.4 增加 `list` 使用旧 snapshot 时仍显示本次 `refresh_attempt` 失败状态的测试。
- [ ] 6.5 增加 provider floor/cooldown 命中时记录 skipped refresh attempt 的测试。

## 7. Account/Profile Remove

- [x] 7.1 增加 `omx remove <platform> <selector>` CLI 入口，复用现有 account/profile target resolver。
- [x] 7.2 通过 SQLite hard delete 实现 account remove：删除 OpenMux 管理的 auth-bearing snapshot，清理 active target，删除 account row、`quota_snapshots`、`refresh_attempts`。
- [x] 7.3 通过 SQLite hard delete 实现 profile remove：删除 OpenMux 管理的 profile/config snapshot，清理 active target，并删除 profile row。
- [x] 7.4 默认 `list/use/refresh` 不再包含已删除 account/profile。
- [x] 7.5 增加 CLI help、Codex account/profile remove、Claude account/profile remove 回归测试。
- [x] 7.6 不引入 `purge`/archive 双命令；当前 `remove` 即完整删除 OpenMux 管理状态。
- [x] 7.7 增加 remove 后 active target、account/profile row、账号级 quota snapshot、refresh attempt 被清理的状态库测试。

## 8. Menubar 与 `tokscale-core` Roadmap

- [ ] 8.1 实现 `usage_events` 和 `scan_watermarks` schema，支持 provider/project/session/model/token/cost 聚合。
- [ ] 8.2 设计 `tokscale-core` 适配边界：输入本地日志扫描结果，输出去重后的 OpenMux usage event。
- [ ] 8.3 定义 menubar 读取策略：UI 只读 SQLite 聚合结果，后台任务负责 refresh attempts 和 usage event ingestion。
- [ ] 8.4 定义 no-activity 降频策略：长期无 usage event 或 active account 变化时降低 background quota refresh 频率。
- [ ] 8.5 补充跨进程安全策略：CLI 和 menubar 同时读写 SQLite 时使用事务、busy timeout 和短事务。
