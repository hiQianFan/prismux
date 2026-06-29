## 1. 后端数据模型与解析

- [x] 1.1 在 `omx-core` 为 `UsageSnapshot` 增加可选 `UsageResetCredits { available_count }` 字段。
- [x] 1.2 在 `omx-plugin-codex` 解析 `rate_limit_reset_credits.available_count`(int 或十进制 string),缺失/非法时整个 `reset_credits` 保持 `None`。
- [x] 1.3 确认 quota snapshot 持久化不保存 raw provider response、token 或 Authorization header。
- [x] 1.4 增加 Codex usage payload 解析测试：有 credits、无 credits、无效 count、string count。

## 2. Menubar Contract

- [x] 2.1 在 `omx-app` 的 `MenubarQuota` 增加可选 `reset_credits`(`MenubarResetCredits { available_count }`)。
- [x] 2.2 在 mapper (`quota_from_usage`) 中从 `UsageSnapshot.reset_credits` 映射到 `MenubarQuota.reset_credits`。
- [x] 2.3 更新 Swift `Quota` DTO，新增 optional `resetCredits` 解码字段。
- [x] 2.4 增加 Rust/Swift DTO decode 测试，确认缺字段兼容。

## 3. Consume Reset Credit Operation

- [x] 3.1 在 `omx-core` `PlatformPlugin` trait 新增 `consume_reset_credit(selector, idempotency_key) -> Result<ResetCreditOutcome>`,带"默认 unsupported"实现(对照 `remove_target`),新增 `ResetCreditOutcome` enum。
- [x] 3.2 在 `omx-plugin-codex` 把 `fetch_codex_usage` 的 curl-config 鉴权 + 执行抽成 `codex_backend_request(auth, method, url, body)` helper;usage 改走该 helper。
- [x] 3.3 在 `parse_codex_usage_auth` 解出 fedramp flag,helper 按需追加 `X-OpenAI-Fedramp: true`。
- [x] 3.4 实现 Codex `consume_reset_credit`:POST `/wham/rate-limit-reset-credits/consume`,body `{ "redeem_request_id": key }`,按响应 `code` 映射 `reset`(读 `windows_reset`)/`nothing_to_reset`/`no_credit`/`already_redeemed`,未知 code → `Err`。
- [x] 3.5 在 `omx-app` 新增 `consume_reset_credit`(对照 `menubar_remove`):取 `OPERATION_LOCK`、调 plugin、成功后刷新该账号 quota、组 `MenubarConsumeResetCreditReport`;`Err` → `Failed` operation + 脱敏 diagnostic。
- [x] 3.6 在 `omx-app` 新增 `MenubarConsumeResetCreditCommand` / `MenubarConsumeResetCreditReport` / `MenubarResetCreditOutcome` DTO。
- [x] 3.7 在 `omx-menubar-ffi` `dispatch` 增加 `"consume_reset_credit"` op arm,保持 panic-safe 和脱敏。
- [x] 3.8 在 Swift `BackendRequest` 增加 `consumeResetCredit` case(生成 UUID idempotency key),`BackendData` 增加 outcome 解码。
- [x] 3.9 增加 consume outcome、HTTP/auth/network/schema failure 的后端测试,确认 `code` 映射正确且不泄露敏感内容。

## 4. Menubar UI

- [x] 4.1 在 `TargetIdentity` 副标题行尾(`metaText`)显示 `N credits`(单数 `1 credit`),caption2/secondary + `arrow.counterclockwise.circle`,仅 `available_count > 0` 时展示;窄宽度降级为单独一行。
- [x] 4.2 为 credit 标注增加 hover/help：明确是 Codex reset credit、可消费一次、作用于 eligible usage limits、不是 token 或余额。
- [x] 4.3 在 `overflowMenu` 增加 `Reset usage limit`(置于 Delete 上 + `Divider`);启用条件 = `available_count > 0` 且 有活动限额;否则灰掉(非隐藏),按原因给 disabledReason(`No reset credits available` / `No active limit to reset`)。
- [x] 4.4 增加 reset 确认 popover(复用 `DeleteConfirmPopover` 形态):文案说明重置 eligible usage limits + 消费 1 credit;确认前不发送 consume 请求。
- [x] 4.5 实现 reset in-flight 态:overflow 图标转 `hourglass`,同账号 Reset/Delete/Use 全禁,避免重复提交;reset 结果(reset N / no_credit / already_redeemed / failed)用 operation result 安全展示。

## 5. 验证

- [x] 5.1 运行 `cargo fmt --all`。
- [x] 5.2 运行 `cargo test`。
- [x] 5.3 运行 `cargo clippy --all-targets --all-features`。
- [x] 5.4 运行 Swift 构建或现有 menubar 验证命令。
- [ ] 5.5 手动检查 account card 长账号名、无 credit、1 credit、多 credit、reset pending、reset failed 的布局。
