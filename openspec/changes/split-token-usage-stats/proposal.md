## Why

OpenMux 的产品定义是"AI coding 工具的本地账号切换器"(README 首句)。当前主分支里除了账号切换,还长出了一整条 **token 用量统计** 功能线:本地 session 解析、`usage_events` 落库、按 client/model/day/hour 的聚合 rollup、tokscale token 换算、CLI `omx usage` 子命令、menubar 的 Token Usage 面板。这条线:

- 与核心使命无协同:切号做得好不需要它,它做得好也不让切号更好。
- 是维护黑洞:上游(Codex/Claude)一改计费口径或日志格式,就得追着改一次。
- 体量大:Rust ~2033 行(`omx-usage-tokscale` 1176 + `omx-core/src/usage.rs` 中的统计类型族)、Swift ~740 行(UsageCard 494 + UsageSeries 246)、state_store 中约 1000 行的表/查询/测试、CLI/FFI/omx-app 多处消费点。

本次迭代目标收敛为**只服务好账号切换,把核心能力打磨精细**。因此把 token 用量统计从主分支剥离,但**不永久丢弃**——保存现场以便后续拉回继续迭代。

**关键边界(必须精确区分,二者在代码里是两套独立数据流):**

- **保留 —— 账号额度信息(quota/limits)**:账号卡上显示的 `5h 0%` / `7d 76%` 限流窗口、reset credits。数据源是 Codex `wham/usage` API 返回的 `UsageLimit` / `UsageResetCredits`,与"这个账号还能不能用"强相关,属于账号信息本身。
- **删除 —— token 用量统计(stats)**:menubar 的 `Codex Token Usage / Today·7d·30d / N tokens` 面板、CLI `omx usage`、`usage_events` 表与聚合。数据源是本地 session 解析出的 `UsageEvent` → `usage_summaries` 聚合 → tokscale 换算。

调研已确认这条切割线在数据模型层是**干净的**:`UsageSnapshot`(额度)仅由 `Availability`/`UsageLimit`/`UsageResetCredits` 组成,不引用 `UsageEvent`/`UsageSummary`/`UsageTokenBreakdown`(统计);omx-app 里 `quota_health_rollup`(额度)只读 `account.quota` 与 `usage.limits`,`usage_summaries_by`(统计)在 `query.rs` 中仅一处调用点。

## What Changes

采用 **"主分支删净 + 打 tag 冻结现场"** 的形式(选型理由见 design.md),而非维护长期并行分支。

- 打 tag `usage-stats-v0`(并可留一条 `feature/usage-stats` 分支)冻结当前 HEAD,使统计代码全部可回溯。
- 主分支删除 token 用量统计的三层实现:
  - **独立 crate**:删除 `omx-usage-tokscale` 整个 crate,从 workspace members 与各 `Cargo.toml` 依赖中移除。
  - **core 数据模型**:从 `omx-core/src/usage.rs` 删除统计类型族(`UsageEvent` / `UsageSummary` / `UsageSummaryQuery` / `UsageQuery` / `UsagePeriod` / `UsageGroupBy` / `UsageReport` / `UsageTokenBreakdown` / `UsageScan*` / `UsageSourceFingerprint` / watermark 等);**保留** `UsageSnapshot` / `Availability` / `UsageLimit` / `UsageResetCredits` / `UsageDiagnostic` / `UsageSource`。
  - **core storage**:从 `state_store.rs` 删除 `usage_events` 表及 5 个索引、`scan_watermarks` 表、`insert_usage_event` / `ingest_usage_events` / `usage_summaries*` / watermark 方法及相关测试;**保留** `save_quota_snapshot` / `latest_quota_snapshot`(额度快照缓存,用于刷新失败回退)。
  - **消费层**:删除 CLI `omx usage` 子命令及 tokscale 扫描/ingest(`app.rs`);删除 FFI 的 `refresh_usage_cache` / `usage_refresh_since_unix` 与 dashboard payload 中的 `usage` stats 分支(`omx-menubar-ffi`);删除 omx-app 的 `usage_headline` / `UsageSummaryView` / `usage_summaries_by` 调用链;删除 Swift `UsageCard.swift` / `UsageSeries.swift` 及 `DashboardView` 中的挂载点。
- 归档到冻结点、从主分支移除:未提交的 `openspec/changes/redesign-usage-ledger-rollups/`(它正是给统计做 rollup 的)。
- 保留**额度展示全链路**:账号卡的 `5h/7d %`、reset credits、`quota_health` rollup、CLI provider 表里的 `usage_cell`(额度健康度)。
- **`assets/` 与本变更无关**:经核实为 app 图标品牌资源(`openmux-app-icon-*.png`、`prismux-icon/`),应正常提交进主分支,**不纳入**统计冻结范围。

**FFI 契约破坏性变更(必须原子发布)**:menubar payload 是 versioned(`omx-menubar-ffi` 中 `SCHEMA_VERSION = 1`)。删除 payload 的 `data.usage` / `data.provider_usage` 字段是**破坏性**变更——若 Swift 侧 `DashboardReport.usage` / `providerUsage` 为非可选字段,旧 app 解码新 payload 会在运行时失败。因此:Rust 删字段与 Swift 删对应 `Decodable` 字段必须**同一次发布**;并 bump `SCHEMA_VERSION`(→ 2)以标记契约变更。

## Impact

- Affected specs/docs: 本 change 移除 `provider-usage-statistics`、`usage-cli-experience`、`menubar-usage-overview` 中的 token stats 要求;执行时同步归档/修正仍以统计为前提的 `refine-usage-cli-experience`、`add-native-menubar-app`(最小 usage 摘要部分)、`compose-overview-aggregate-primitives`(UsageHeadline/hourly projection)、`evaluate-tokenbar-menubar-spike`(`omx usage --json` 样例)、`redesign-usage-ledger-rollups`。`persist-usage-snapshot-on-refresh-failure` 保留"额度快照回退",归档 usage event / scan watermark roadmap。
- Affected code: `omx-usage-tokscale`(删)、`omx-core`(usage.rs / state_store.rs / account.rs / target.rs / lib.rs 局部删)、`omx-cli`(app.rs / usage_cli.rs)、`omx-menubar-ffi`、`omx-app`(dto.rs / query.rs)、Swift Dashboard 组件。
- 验证:`openspec validate split-token-usage-stats --strict`、`cargo build --workspace`、`cargo test --workspace` 通过;账号卡额度(5h/7d/reset)与 provider 表额度列仍正常显示;menubar 无 Token Usage 面板残留。
- 回收路径:`git checkout usage-stats-v0 -- <path>` 或 cherry-pick 冻结点提交即可拉回。

### 依赖收益(剥离统计的连带收缩)

删除 `omx-usage-tokscale` 不只是减代码,还会连带清掉两个纯为统计而存在的重依赖,与"收敛、精细"的目标一致:

- **`tokio` 异步运行时**:全工作区**只有** `omx-usage-tokscale` 一个 crate 依赖它(已核实)。删除后整个多线程 async 运行时从依赖树消失,编译体量与依赖面收缩。
- **`vendor/tokscale-core`(1.8M vendored 源码)**:随 crate 一并移除。
- **`chrono`**:在 `omx-app` 中主要用于统计的 day/hour 分桶(`local_day_offset_seconds`、`local_date_start_unix`、hourly bucketing),但 `omx-plugin-codex` / `omx-cli` 也在用,**不能整体删除**;删除统计代码后需重新编译确认 `omx-app` 是否还需要 `chrono`,若不再需要则从该 crate 的依赖中移除,否则保留。
