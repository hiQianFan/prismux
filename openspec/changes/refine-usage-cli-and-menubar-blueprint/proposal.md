## Why

OpenMux 的核心任务是安全、可靠地切换 AI coding tools 的 account/profile；`usage` 是帮助用户理解消耗并辅助切换决策的附加能力，不应把产品扩张为另一个 tokscale 或 ccusage。现有 `provider-usage-statistics` 已建立采集、标准化、SQLite 和基础 CLI，但默认宽表信息密度过高、缺少清晰层级，也尚未把 CLI 与未来 Menubar 的职责和开源复用边界固化为可执行蓝图。

## What Changes

- 将 `omx usage` 定位为轻量决策入口：默认回答“时间窗口内用了多少、主要由谁消耗、数据是否新鲜可信”，复杂分析不进入默认视图。
- 将 CLI 分为默认摘要、显式分组和详细 token breakdown 三层；提供 `today`、`7d`、`30d` 等时间预设，并保留精确日期范围与 versioned JSON。
- 默认输出收窄到单屏可读；input/output/cache/reasoning、provider total、event count 和诊断细节仅在 `--details`、显式 group-by 或 JSON 中出现。
- 明确区分 local parsed token consumption、estimated/provider-reported cost 与 subscription quota，禁止用 token consumption 推断订阅剩余额度。
- 将 account/profile attribution 定义为 OpenMux 的差异化方向，但只有存在可验证证据时才归因；无法可靠判断的历史 event 必须保持 `unknown`，不得按当前 active account 反推。
- 不把 tokscale 的完整 CLI/TUI 或 ccusage CLI 嵌入 OpenMux，也不通过子进程解析其展示输出；继续复用 `tokscale-core` 的 source discovery/parser，并保留 OpenMux-owned domain、SQLite、query 和 JSON contract。
- 建立 Menubar 蓝图：优先评估 fork/二次开发 TokenBar 的原生 macOS shell、视图和交互，替换其数据入口，使其消费 OpenMux 稳定查询契约；不得形成第二套日志扫描、pricing、聚合或持久化实现。
- 定义 TokenBar fork 的采用门槛与停止线，包括 MIT license/NOTICE、UI-data coupling、上游同步成本、签名/更新链路和跨平台产品边界。
- 复杂趋势图、hourly heatmap、session/agent drill-down 和动画只作为 Menubar 后续能力，不进入本次 CLI 打磨的必选范围。

## Capabilities

### New Capabilities

- `usage-cli-experience`: 定义轻量默认摘要、时间预设、分组/详情层级、数据新鲜度与 versioned JSON 的稳定行为。
- `menubar-usage-blueprint`: 定义 Menubar 与 OpenMux 数据层的集成边界、TokenBar 二次开发评估门槛及 CLI/Menubar 一致性要求。

### Modified Capabilities

- 无。现有 `provider-usage-statistics` 变更仍负责 usage 采集、标准化、持久化与基础查询；本变更在其上增加独立的体验和集成能力。

## Impact

- `crates/omx-cli`: 重构 `omx usage` 参数、默认摘要、分组表格、details 和 diagnostics 展示。
- `crates/omx-core`: 可能扩展聚合 query/result，增加 daily bucket、freshness、coverage/completeness 和可验证 account attribution 字段；不引入 UI 类型。
- `crates/omx-usage-tokscale` 与 `vendor/tokscale-core`: 继续作为采集/解析层；通用 parser 修复优先贡献上游，adapter 保持防腐边界。
- SQLite state store: 继续作为 usage 唯一事实来源；CLI 与未来 Menubar 共享 query contract。
- Menubar 原型：新增独立技术 spike，评估 TokenBar fork、数据入口替换、macOS 签名与更新链路；本提案不直接承诺完整 GUI 产品交付。
- 文档与测试：增加 CLI snapshot/integration、JSON contract、account attribution 安全性和 Menubar contract 一致性验证。
- 依赖关系：本变更依赖 `provider-usage-statistics` 的稳定 `UsageEvent`、SQLite 和基础聚合能力，应在其收尾或接口冻结后进入实现。
