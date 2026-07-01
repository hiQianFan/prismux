## ADDED Requirements

### Requirement: 展示账号 quota 与 reset
Menubar SHALL 将 quota 作为账号状态的一部分展示：OpenMux 最近成功 quota snapshot 中的主要窗口、used/remaining、reset time 和 refreshed time，并 SHALL 将当前 refresh diagnostic 与历史 snapshot 分开表达。

#### Scenario: 当前刷新失败但存在 quota snapshot
- **WHEN** 本次 quota refresh 失败且存在最近成功 snapshot
- **THEN** Menubar SHALL 展示最近成功 quota 数值与其 refreshed time
- **AND** SHALL 同时显示 stale/error 状态而不是清空或归零

### Requirement: usage 只作为最小附属摘要
Menubar SHALL 通过 OpenMux query service 展示用户本地自然日的 total tokens、top client、top model 和 freshness/coverage。它 SHALL NOT 提供 history drill-down、daily/model chart、session browser、account usage attribution 或完整 analytics dashboard。

#### Scenario: today usage 有完整 token 但部分 cost 缺失
- **WHEN** today usage event 可聚合但 pricing coverage 为 partial 或 missing
- **THEN** Menubar SHALL 展示 token total
- **AND** SHALL 将 cost 标为 partial/missing，不得把缺失 cost 显示为 `$0.00`

#### Scenario: today usage 缺失
- **WHEN** backend 没有可用的 today usage event
- **THEN** Menubar SHALL 将 usage summary 显示为 empty 或 unavailable
- **AND** SHALL 继续展示账号池、quota 和 switch 操作

### Requirement: usage 与 quota 必须保持不同口径
Menubar SHALL 将 provider quota snapshot 与本地 parsed token usage 分成不同 section 和标签，MUST NOT 将本地 token total 推导为订阅剩余额度。

#### Scenario: 本地没有 usage event 但 quota 可用
- **WHEN** active account 有 quota snapshot 但 today 没有本地 usage event
- **THEN** Menubar SHALL 继续展示 quota
- **AND** SHALL 将 today usage 表达为 empty，而不是 quota unavailable

### Requirement: 展示 freshness、coverage 和安全 diagnostics
Dashboard report SHALL 提供 freshness、requested/available/missing source coverage、cost status 和脱敏 diagnostics；Menubar SHALL 在异常时将其呈现为可理解状态。

#### Scenario: 一个 client source 扫描失败
- **WHEN** Codex usage 成功但 Claude source 扫描失败
- **THEN** Menubar SHALL 展示可用的 Codex 聚合结果
- **AND** SHALL 将整体 coverage 标记为 partial 并指出安全 client/error code

### Requirement: 不得伪造 account attribution
在 usage event 缺少可验证 account/profile evidence 时，Menubar MUST NOT 将历史 token 消耗归入当前 active account。

#### Scenario: 用户切换账号后查看历史 usage
- **WHEN** 历史 event 没有 account attribution evidence 且当前 active account 已改变
- **THEN** Menubar SHALL 仅按 client/model 展示该 usage 或标为 unknown attribution
- **AND** SHALL NOT 将其显示为新 active account 的消耗

### Requirement: refresh kind 与 provider 调度一致
打开菜单、点击刷新和切换前的用户动作 SHALL 使用 interactive refresh 语义；常驻刷新 SHALL 使用 background 语义并遵守 provider floor、TTL、backoff 和 no-activity 降频。

#### Scenario: background timer 高频触发
- **WHEN** Swift timer 在 provider floor 内再次请求 background refresh
- **THEN** backend SHALL 跳过实际 provider 请求并返回最近状态
- **AND** Menubar SHALL 继续展示最近成功数据
