## REMOVED Requirements

### Requirement: `omx usage` token 用量统计命令

**Reason**: token 用量统计从主分支剥离,冻结于 tag `usage-stats-v0`。仅移除 **token 统计** 命令 `omx usage`(扫描本地 session、按 Today/7d/30d/model 出 token 报表),**不影响** 账号额度(quota/limits)展示。

**Migration**: 无需迁移。查看账号剩余额度改用/继续用既有命令:
- `omx list` —— 总览表,含 `Overall` / `5h`(额度剩余百分比)/ `Status` 列。
- `omx list <platform>` —— 单平台下每个账号的额度明细。
- `omx refresh [platform] [selector]` —— 刷新并展示最新额度。
恢复 token 统计:从冻结点拉回,`git checkout usage-stats-v0 -- crates/omx-cli`。

主分支不再提供 `omx usage [client]` 命令及其 `--since` / `--until` / `--json` / `--group-by` / `--no-scan` 选项与对应的表格 / JSON token 报表输出。

#### Scenario: 额度展示不受统计移除影响

- **WHEN** 用户运行 `omx list` 或 `omx refresh`
- **THEN** OpenMux SHALL 继续展示账号额度剩余百分比(5h/7d 窗口)与 healthy/limited/exhausted 状态
- **AND** OpenMux SHALL NOT 依赖已移除的 `usage_events` 统计数据来渲染额度列。
