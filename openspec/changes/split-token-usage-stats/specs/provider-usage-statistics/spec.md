## REMOVED Requirements

### Requirement: 按 client 聚合 parsed token usage

**Reason**: token 用量统计功能线从主分支剥离,冻结于 tag `usage-stats-v0`,后续重新设计后再引入。本次迭代主分支只服务账号切换。

**Migration**: 无需迁移。额度信息(quota/limits)不受影响,继续由 `UsageSnapshot`/`UsageLimit`/`UsageResetCredits` 提供。历史 `usage_events` / `scan_watermarks` SQLite 数据保留不动(不再读写),或由可选清理迁移移除。恢复统计能力从冻结点拉回:`git checkout usage-stats-v0 -- <path>`。

OpenMux 主分支不再按本地 client 汇总 parsed token usage,亦不再提供 `codex`/`claude`/`gemini` 的 token summary。

### Requirement: usage backend 适配第三方本地解析器

**Reason**: 随统计功能线整体剥离;`omx-usage-tokscale` 适配器与 vendored `tokscale-core` 一并从主分支移除。

**Migration**: 无。冻结点保留完整适配器实现。
