## REMOVED Requirements

### Requirement: menubar Token Usage 面板

**Reason**: token 用量统计从主分支剥离,冻结于 tag `usage-stats-v0`。移除 menubar 中的 token 统计面板(`Codex Token Usage / Today·7d·30d / N tokens`),**保留** 账号卡的额度窗口(`5h/7d %`、reset credits)与 `quota_health` 概览。

**Migration**: 无需迁移。菜单栏继续展示账号额度信息。FFI dashboard payload 移除 `data.usage` / `data.provider_usage` 字段属破坏性契约变更,须与 Swift `Decodable` 删字段同批发布,并 bump `SCHEMA_VERSION`(1 → 2)。恢复:从冻结点拉回 `UsageCard.swift` / `UsageSeries.swift` 及对应 FFI/omx-app 组装。

menubar 不再展示按 client/model 聚合的 token 消耗面板、时间序列图与 Today/7d/30d 切换。

#### Scenario: 额度概览在面板移除后仍展示

- **WHEN** 用户打开 menubar dashboard
- **THEN** OpenMux SHALL 继续展示每账号的额度窗口(5h/7d 剩余、reset credits)与 quota_health 概览
- **AND** dashboard payload SHALL NOT 包含 token 统计字段(`usage` / `provider_usage`)
- **AND** Swift SHALL NOT 依赖已删除的 `UsageSummary` / `ProviderUsageSummary` DTO 解码 dashboard
- **AND** payload `schema_version` SHALL 反映契约变更。
