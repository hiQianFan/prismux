## 1. 后端模型与解析

- [x] 1.1 在 `omx-core` 扩展 `UsageResetCredits`，增加默认空列表的 per-credit metadata，并定义 `expires_at_unix` / `granted_at_unix` 字段。
- [x] 1.2 在 `omx-plugin-codex` 新增 `fetch_codex_reset_credit_details(auth)`，调用 `GET https://chatgpt.com/backend-api/wham/rate-limit-reset-credits`，复用现有 `codex_backend_request` 鉴权 helper。
- [x] 1.3 新增 detail payload parser：读取 `credits[]`，解析 `status`、`reset_type`、`granted_at`、`expires_at`，RFC3339 转 Unix seconds，按 `expires_at_unix` 升序排序。
- [x] 1.4 在 `usage_from_snapshot` 中仅当 `/wham/usage` 得到 `available_count > 0` 时补充 detail；detail 失败不得让 quota refresh 失败。
- [x] 1.5 确认 SQLite quota snapshot 只保存结构化 expiry timestamp，不保存 raw provider response、token、Authorization header 或 Cookie。

## 2. Contract 与 DTO

- [x] 2.1 在 `omx-app` `ResetCreditsView` 增加 `credits: Vec<ResetCreditView>`，保持缺字段 decode/serialize 兼容。
- [x] 2.2 在 `quota_from_usage` 中映射 reset credit expiry metadata；聚合层 `reset_credit_total` 继续只统计 count。
- [x] 2.3 更新 `crates/omx-menubar-ffi` fixture，覆盖 count-only 和带两个 `expires_at_unix` 的 reset credit 样例。
- [x] 2.4 更新 Swift `ResetCredits` DTO，新增默认空数组的 `credits`，并新增 `ResetCredit` 解码结构。
- [x] 2.5 增加 Rust/Swift contract decode 测试，确认旧 fixture 缺 `credits` 时仍通过。

## 3. Menubar Hover 展示

- [x] 3.1 为 account card reset credit 标注生成 hover 文案：有 expiry 时显示最多两条，按最早过期排序；无 expiry 时显示 count + `Expiry unavailable`。
- [x] 3.2 使用本地时区格式化 expiry，口径与现有 `fullDateTimeLabel` 对齐。
- [x] 3.3 若 `.help` 在 transient `NSPopover` 中不可稳定展示，实现 popover 内轻量 hover overlay，确保 hover 到 reset credit 标注时可见。
- [x] 3.4 保持 account card 常态布局不新增显式 expiry 行，不影响 Use/Delete/Reset action cluster。
- [x] 3.5 确认 hover 文案明确区分 reset credit expiry 与 5h/7d quota window reset time。

## 4. 验证

- [x] 4.1 增加 Codex detail parser 单元测试：两个 available credits、redeemed credit、缺失 `expires_at`、非法 timestamp、空 `credits[]`。
- [x] 4.2 增加 refresh 降级测试：detail endpoint 网络/HTTP/schema 失败时仍返回 `/wham/usage` quota 与 count。
- [x] 4.3 增加 Menubar hover 文案测试或可提取 formatter 测试：两条 expiry、超过两条、count-only。
- [x] 4.4 运行 `cargo fmt --all`。
- [x] 4.5 运行 `cargo test`。
- [x] 4.6 运行 `cargo clippy --all-targets --all-features`。
- [x] 4.7 运行 Swift menubar 构建/contract 测试，确认 DTO 与 UI 编译通过。
- [ ] 4.8 手动用本地有 2 次 reset credit 的账号验证 hover 显示两个本地时间，且无 credit 账号不额外展示 expiry。
