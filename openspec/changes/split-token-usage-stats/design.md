## Context

本次要把 token 用量统计从主分支剥离,核心的两个设计问题:

1. **用什么形式剥离**才能"主分支干净 + 以后能拉回继续迭代"?
2. **切割线画在哪**才能删掉统计而不误伤账号额度展示?

问题 2 已通过代码调研确认可干净切割(见下文接缝清单)。本文件重点论证问题 1 的选型。

## Decision 1: 剥离形式 —— 删净主分支 + tag 冻结,不做长期并行分支

调研了三种候选形式:

### 候选 A:维护一个长期并行的 `feature/usage` 分支持续迭代(否决)

直觉上"另开分支继续写"最符合"以后还要迭代"的诉求,但代码耦合证据否决了它:

统计代码**焊进了核心数据地基**——`AccountRecord.usage` 字段(account.rs:21)、`state_store.rs` 约 1000 行、`omx-core/src/lib.rs` 的 `pub use usage::*`。而这些文件(account.rs / state_store.rs)**正是打磨账号切换时要反复改动的地基**。

若主分支在打磨切号、并行分支在同一批 core 文件里写统计,两条线改的是同一处地基,回头 merge 会把冲突集中在最核心、最不能出错的文件上。**分支适合"冻结保存",不适合"在共享地基上长期并行开发"。** 这是纯亏。

### 候选 B:主分支保留统计代码但用 feature flag 关闭(否决)

保留即维护:编译要过、测试要跑、重构要连带改。它没有减少"打磨切号时的心智与改动面",与本次目标(收敛、精细)相悖。且死代码会持续腐化。

### 候选 C(采用):主分支物理删净 + git tag / branch 冻结现场

git tag/branch 本身就是不可变的时间点快照。在当前 HEAD 打 `usage-stats-v0`(可附带 `feature/usage-stats` 分支),统计代码全部永久可回溯,一行不丢。然后主分支物理删除。

- **主分支干净**:切号就是切号,打磨时没有统计的心智负担与改动牵连。
- **以后能拉回**:`git checkout usage-stats-v0 -- <文件>` 取单文件,或 cherry-pick 冻结点提交取整体。届时统计作为一个**重新设计过的、与新地基对齐的**功能重新引入,而不是背着一年前的旧实现强行 merge。
- **"删除"就是"剥离"的实现手段**,二者非对立:tag 负责"不丢",删除负责"主分支干净"。

**结论:采用 C。** tag `usage-stats-v0` 冻结 → 主分支删净。是否额外保留 `feature/usage-stats` 分支属可选(tag 已足够,分支只是更易发现)。

## Decision 2: 切割线 —— 按"额度(保留) vs 统计(删除)"两套数据流切

调研确认二者在各层都是独立数据流,可干净切割:

| 层 | 保留(额度 / quota·limits) | 删除(统计 / token stats) |
| --- | --- | --- |
| core 类型 (`usage.rs`) | `UsageSnapshot` `Availability` `UsageLimit` `UsageResetCredits` `UsageDiagnostic` `UsageSource` | `UsageEvent` `UsageSummary` `UsageSummaryQuery` `UsageQuery` `UsagePeriod` `UsageGroupBy` `UsageReport` `UsageTokenBreakdown` `UsageScan*` `UsageSourceFingerprint` watermark 族 |
| core storage (`state_store.rs`) | `save_quota_snapshot` `latest_quota_snapshot`(额度快照缓存 + 刷新失败回退) | `usage_events` 表 + 5 索引、`scan_watermarks` 表、`insert_usage_event` `ingest_usage_events` `usage_summaries*` `update_scan_watermark` `scan_watermark` 及相关测试 |
| plugin (codex) | `parse_codex_usage_snapshot` → limits/reset、`fetch_codex_usage` | usage_events 的本地采集 / ingest |
| omx-app (`query.rs`/`dto.rs`) | `quota_health_rollup`(只读 `account.quota` + `usage.limits`)、`QuotaHealthRollup` | `usage_headline` `UsageSummaryView` `ProviderUsageSummary` `usage_summaries_by`(query.rs 唯一调用点在 `usage_hourly_summaries`) |
| FFI | dashboard payload 的 `aggregate.quota_health` 分支 | `refresh_usage_cache` `usage_refresh_since_unix`、payload 的 `usage`(total_tokens/top_model/coverage)分支 |
| CLI (`app.rs`) | `usage_cell`(provider 表额度健康度列) | `Command::Usage` 子命令、`print_usage*` `usage_report` `usage_window` `usage_group_by`、tokscale 依赖 |
| Swift | 账号卡额度渲染(`5h/7d %`、reset) | `UsageCard.swift` `UsageSeries.swift`、`DashboardView` 挂载点、`UsageStatsStrip.swift`(按实际用途判定) |

### 关键接缝(逐点切,不能整文件删)

1. **`state_store.rs`**:同一文件里 `save_quota_snapshot`/`latest_quota_snapshot`(保留)与 `insert_usage_event`/`usage_summaries`(删)混居 → 按方法切。
2. **plugin-codex `plugin.rs`**:`parse_codex_usage_snapshot`(限流,保留)与 usage_events 采集(统计,删)混居 → 按函数切。
3. **omx-app `dto.rs`**:`DashboardAggregateView` 内 `quota_health`(保留)与 `usage_headline`(删)并排 → 删字段而非删结构;`DashboardReport.usage`/`provider_usage` 整删。
4. **FFI dashboard payload**:同一 op 同时产出 `aggregate`(含 quota_health,保留)与 `usage`(stats,删)→ 删 payload 分支与 `refresh_usage_cache` 调用,保留 aggregate 组装。
5. **`persist-usage-snapshot-on-refresh-failure`**:该 spec 的"额度快照回退"依赖 `save_quota_snapshot`/`latest_quota_snapshot`(保留侧)。需评估此 spec 哪些部分属额度回退(保留)、哪些属统计(归档)。
6. **旧 OpenSpec/docs**:`add-native-menubar-app`、`compose-overview-aggregate-primitives`、`evaluate-tokenbar-menubar-spike`、`docs/menubar-v1*.md` 仍有 today usage / UsageHeadline / `omx usage --json` 描述 → 同步改成 quota-only 或标记为已剥离,避免后续实现按旧文档把统计拉回来。

## Risks / Trade-offs

- **误删额度链路** → 编译期(删统计类型后额度代码仍需引用完整)+ 运行期(账号卡额度必须仍显示)双重验证兜底。切割顺序建议**自上而下**(先删 Swift/CLI/FFI 消费点 → 再删 omx-app → 最后删 core 类型),让编译器暴露每一处遗漏引用。
- **拉回时的漂移** → 冻结点的统计实现基于当前地基。未来拉回需与届时的账号切换新结构重新对齐,视为"重新设计引入"而非"无痛 merge"。这是采用 C 的自觉代价,已接受。
- **DB 迁移** → 主分支删表后,老用户 SQLite 里残留的 `usage_events` / `scan_watermarks` 表不会自动清理但也无害(不再读写)。可选:加一步清理迁移 `DROP TABLE IF EXISTS usage_events; DROP TABLE IF EXISTS scan_watermarks;`,或留存不动。倾向留存不动(零风险)。

## Rollback / Recovery

- 冻结:`git tag usage-stats-v0 <HEAD>`(+ 可选 `git branch feature/usage-stats`)。
- 拉回单点:`git checkout usage-stats-v0 -- crates/omx-usage-tokscale`。
- 拉回整体:从 `usage-stats-v0` cherry-pick 或以其为基重开分支。
