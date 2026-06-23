## Context

`provider-usage-statistics` 已实现或正在实现以下基础链路：

```text
local client artifacts
  -> vendored tokscale-core
  -> omx-usage-tokscale adapter
  -> OpenMux UsageEvent
  -> SQLite usage_events
  -> UsageSummary query
  -> omx usage table / JSON
```

当前默认 CLI 直接按 `client + model_provider + model` 输出 Input、Output、Cache R/W、Reasoning、Total、Provider Total、Cost、Quality、Events 宽表。字段完整，但用户需要自行理解大量 accounting 细节，无法快速回答“这段时间用了多少、主要消耗在哪里、数据是否可信”。

外部开源项目提供了三种不同参考：

- tokscale 已实现完整 parser core、复杂 Ratatui TUI 和 Overview/Models/Daily/Hourly/Stats/Agents 等分析 lens。
- ccusage 采用 daily/weekly/monthly/session 等表格报告，信息密度高，适合显式分析命令。
- TokenBar 使用 vendored `tokscale-core` + Rust FFI + SwiftUI/AppKit，提供 macOS Menubar、quota pace、趋势和多 lens 图形界面。

OpenMux 的核心仍是 account/profile switch。Usage 必须服务于快速判断和未来 UI 数据供给，不能使 CLI 演化为独立 analytics 产品，也不能让 CLI 与 Menubar 各自维护一套 parser、pricing 和聚合逻辑。

## Goals / Non-Goals

**Goals:**

- 建立轻量、渐进披露的 `omx usage` 信息架构。
- 保持 table 与 versioned JSON 来自同一聚合结果。
- 复用 tokscale 最昂贵的 source discovery/parser 能力，同时保持 OpenMux 对 domain、storage、query 和 public contract 的所有权。
- 明确 token consumption、cost、quota 三种口径。
- 为 account/profile attribution 定义“只在有证据时归因”的安全边界。
- 形成可验证的 TokenBar fork/二次开发蓝图，使未来 Menubar 复用 UI 而不复制数据引擎。

**Non-Goals:**

- 不把 tokscale 的完整 CLI/TUI 源码或交互入口嵌入 `omx`。
- 不调用 tokscale/ccusage 子进程并解析其 human-readable 输出。
- 不在 CLI 中实现 heatmap、3D contribution graph、动画、全屏交互或 session browser。
- 不在本变更中承诺完整 Menubar 发布、跨平台桌面应用或团队 usage 服务。
- 不维护 OpenMux 自有的全量模型 pricing 数据库。
- 不按当前 active account 反推历史 event 的 account/profile 归属。

## Decisions

### 1. Usage 是 switch 产品的辅助能力，不是第二核心

默认 `omx usage` 必须在单屏内回答：

1. 查询的时间窗口；
2. total tokens 和可用时的 estimated/provider-reported cost；
3. 主要 client/model 消耗；
4. 数据更新时间以及 scan/coverage 异常。

默认视图不展示全部 accounting 字段。详细 token breakdown 通过 `--details` 或 JSON 暴露；分组分析通过 `--group-by` 显式请求。

备选方案是直接复刻 ccusage 宽表。未选择，因为 ccusage 的核心任务就是 usage analytics，而 OpenMux 用户默认任务是查看状态并决定是否切换。

### 2. CLI 使用“摘要、分组、详情”三级渐进披露

建议命令面：

```text
omx usage
omx usage --period today|7d|30d|all
omx usage --since YYYY-MM-DD --until YYYY-MM-DD
omx usage [client]
omx usage --group-by client|day|model|project|session
omx usage --details
omx usage --no-scan
omx usage --json
```

- 默认等价于 `--period today --group-by client`，输出总览和紧凑 client rows。
- `--period` 是常用时间预设；`--since/--until` 继续用于精确范围，二者冲突时返回清晰错误。
- `--group-by model` 借鉴 TokenBar/tokscale Models lens 和 ccusage model breakdown。
- `--group-by day` 提供趋势所需的最小表格，不增加 TUI chart。
- `project`、`session` 只有数据存在且查询稳定时开放；无元数据时使用 `unknown`，不丢弃 token。
- `--details` 增加 input/output/cache read/cache write/reasoning/provider total/events/cost status/quality。
- diagnostics 正常时折叠为 freshness/coverage 摘要；异常时才逐条展示安全诊断。

不移除既有 `--since`、`--until`、`--json`、`--no-scan`，避免不必要的 breaking change。

### 3. JSON 是稳定数据契约，不复制 human table 结构

JSON 保持 versioned OpenMux-owned schema，并扩展以下概念：

```text
window: 查询时间边界与预设标签
totals: 跨分组总计
groups: 当前 group-by 的 rows
freshness: 最近成功扫描/入库时间
coverage: requested clients、available clients、missing/partial sources
accounting: quality 与 cost 状态说明
diagnostics: 已脱敏异常
```

human table 和 Menubar 都从同一个 query/result model 派生，但不要求 UI 镜像 JSON 的全部字段。Schema 升级遵循 additive-first；删除或重命名字段需要提升 major `schema_version`。

### 4. 复用 tokscale core，不复用 tokscale application shell

保持当前依赖边界：

```text
tokscale-core parser APIs
  -> omx-usage-tokscale anti-corruption adapter
  -> OpenMux UsageEvent
  -> OpenMux state/query
```

理由：

- parser/source discovery/pricing mapping 是跨 client 最昂贵且最适合上游共享的部分；
- OpenMux 需要稳定 account/profile、quota 和 switch 领域语义，不能依赖 tokscale TUI view model；
- 直接嵌入 TUI 会引入终端状态、布局、键盘事件和 snapshot 维护面；
- 子进程方案依赖外部安装和 human/third-party JSON 输出兼容，错误处理与跨平台更差。

vendor 继续锁定 upstream commit；通用 parser 修复优先贡献 tokscale，上游暂未提供稳定 SDK 时仅在 adapter 内吸收 API 变化。ccusage 只用于 fixture 对照和表格信息架构参考，不进入 runtime dependency。

### 5. Account/profile attribution 必须证据驱动

token usage 与 subscription quota 是独立数据。Local session logs 通常不能可靠说明某条历史 event 使用了哪个 OpenMux account；因此：

- `UsageEvent` 可以预留 `account_local_id`/`profile_local_id` 和 attribution evidence/status；
- 只有 provider 记录、稳定 session metadata 或 OpenMux 自己记录的可验证 active timeline 能建立归属；
- 无证据时必须存储/展示 `unknown`；
- 不允许把扫描时的 current account 应用到历史 event；
- 第一阶段 CLI 不把 account 作为默认 group-by，直到 attribution coverage 达到可接受门槛。

这保留了 OpenMux 的潜在差异化能力，同时不制造伪精确统计。

### 6. Menubar 复用 TokenBar UI，OpenMux 保持唯一数据引擎

Menubar 采用两阶段接入：

1. **Spike:** fork TokenBar，最小替换一个 Overview 数据入口，通过 `omx usage --json --no-scan` 验证 UI 与 OpenMux contract 的适配成本。
2. **Production candidate:** 将 OpenMux query 封装为 Rust staticlib/C ABI 或等价稳定本地接口，由 Swift 直接调用；Swift 不直接查询 SQLite schema，也不重新扫描 provider logs。

可复用范围：

- `NSStatusItem`/popover 生命周期；
- SwiftUI 视图、时间范围和 lens 切换；
- 图表、quota pace、live trace 的交互模式；
- Sparkle/Homebrew 等 macOS 工程经验，采用前需单独审查发布策略。

必须替换或删除：

- TokenBar 自带的 local session scanning；
- 独立 pricing/model mapping；
- 独立 usage cache/aggregation；
- 与 OpenMux 不一致的 account/quota 定义。

采用门槛：

- MIT license、copyright、NOTICE 和第三方资源可合规再分发；
- 一个 Overview screen 能在限定 spike 内改接 OpenMux contract；
- UI 与 `tokscale-core`/TokenBar report model 的耦合可通过 adapter 隔离；
- 能制定 upstream sync 策略，且 fork 差异不会要求持续大规模 cherry-pick。

停止线：如果替换数据层需要重写大部分 view model、FFI 和状态管理，或 fork 发布链路与 OpenMux 产品边界冲突，则停止 fork，只借鉴 UX/视觉并创建更小的原生 shell。

### 7. CLI 与 Menubar 共用 query service，不共用 presentation

建议增加 OpenMux-owned `UsageQuery`/`UsageReport`：

```text
UsageQuery { window, filters, group_by, details }
UsageReport { totals, groups, freshness, coverage, diagnostics }
```

StateStore 负责 SQL，application service 负责扫描编排和 report 组装，CLI 只负责参数映射与渲染。未来 Menubar 通过 FFI/本地接口调用同一 service。这样两端数值一致，但可以采用不同的信息密度。

## Risks / Trade-offs

- [TokenBar 仓库和 UI 变化快，fork 持续分叉] → 先做限定 spike，记录 upstream commit、fork delta 和同步预算，通过停止线决定继续或自建 shell。
- [tokscale-core 没有稳定 SDK] → vendor pin + adapter + fixture contract tests，第三方类型不得进入 core/public JSON。
- [默认摘要隐藏 accounting 细节] → `--details` 与 versioned JSON 完整保留，并在异常时主动显示 quality/coverage。
- [account attribution 被误解为精确] → 默认关闭 account group-by，要求 evidence/status/coverage，unknown 不参与具名账号统计。
- [CLI JSON 子进程用于 Menubar 带来启动开销] → 仅限 spike；生产候选使用 Rust FFI 或稳定本地 service。
- [Menubar 需求反向膨胀 CLI] → CLI capability 与 Menubar capability 分开验收，复杂 lens 不自动进入 CLI。
- [SQLite schema 泄漏给 Swift] → Swift 只依赖 versioned query contract，不直接读取表结构。

## Migration Plan

1. 先完成或冻结 `provider-usage-statistics` 的 `UsageEvent`、SQLite 和基础 summary contract。
2. 引入 `UsageQuery`/`UsageReport` application model，并保持旧 JSON 字段兼容或提供明确 schema migration。
3. 增加 period preset、group-by 和 details；先以 tests 固定 table/JSON 行为，再替换默认宽表。
4. 增加 freshness/coverage，确保 scan failure 与 empty usage 可区分。
5. 独立执行 TokenBar license/coupling spike，不阻塞 CLI 交付。
6. Spike 通过后再创建 Menubar implementation change；未通过则保存 UX mapping 和自建 shell 建议。

回滚策略：CLI 可暂时保留 legacy detailed renderer，通过内部 feature flag/单独函数切回；数据 schema 和历史 events 不回滚、不删除。TokenBar spike 只存在于独立目录或分支，不进入默认构建链路。

## Open Questions

- `--period 7d/30d` 使用 rolling window 还是本地自然日；建议 CLI 明示为包含今天的本地自然日集合。
- 默认摘要是否显示 estimated cost；建议仅在 pricing coverage 完整时显示总 cost，否则显示 `partial/missing`，不展示误导性总额。
- account attribution 的最小证据类型和 coverage 门槛应在独立 proposal 中定义，避免本变更暗含 timeline inference。
- Menubar production contract 最终采用 Rust FFI、local socket 还是 CLI JSON；本提案只要求 spike 给出基准与推荐。
