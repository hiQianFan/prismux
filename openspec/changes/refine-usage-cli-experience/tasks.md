## 1. 前置契约与现状基线

- [x] 1.1 完成或冻结 `provider-usage-statistics` 的 `UsageEvent`、SQLite schema、基础 summary query 和 versioned JSON v1 契约，并记录本变更依赖的具体接口。
- [x] 1.2 为当前 `omx usage` 默认宽表、`--json`、空数据、scan failure 和 missing cost 建立 CLI snapshot/integration 基线。
- [ ] 1.3 使用现有 Codex fixture 对照 tokscale/ccusage 可比字段，记录 total、cache、reasoning、model 和时间边界差异；Claude/Gemini 等 provider 接入后再补对应 fixture。
- [x] 1.4 固定本变更支持的 lens 为 `client`、`day`、`model`，不公开 `project`、`session` 或 `account` 分组。

## 2. OpenMux Usage Query 与 Report

- [x] 2.1 在 `omx-core` 定义 OpenMux-owned `UsageQuery`，包含 window、client/filter、group-by 和 details，不引用 tokscale/ccusage 类型。
- [x] 2.2 定义 `UsageReport`，包含 totals、groups、freshness、coverage、accounting status 和安全 diagnostics。
- [x] 2.3 扩展 `StateStore` 聚合查询，支持按 client、model 和本地日期 bucket 分组，并保证 unknown 维度不丢 token。
- [x] 2.4 增加聚合单元测试，验证 group rows 之和与 totals 一致，覆盖 unknown model、跨日、cache-only、reasoning-only 和空数据。
- [x] 2.5 增加 application service，统一 scan/ingest/query/report 组装，使 CLI table 与 JSON 不再分别拼装领域数据。

## 3. 时间窗口与 CLI 参数

- [x] 3.1 为 `omx usage` 增加 `--period today|7d|30d|all`，并保留 `--since`、`--until`、`--no-scan`、`--json`。
- [ ] 3.2 按用户本地时区实现 `7d`、`30d` 自然日窗口，包含当前日，并测试 DST/跨月/跨年边界。
- [x] 3.3 当 `--period` 与 `--since`/`--until` 同时出现时返回明确冲突错误，不隐式覆盖参数。
- [x] 3.4 增加 `--group-by client|day|model` 和 `--details` 参数，不提前公开不稳定入口。
- [x] 3.5 更新 CLI help 和示例，说明默认 today/client 摘要、时间预设、精确范围、details 与 JSON。

## 4. 紧凑 Human Output

- [x] 4.1 将默认 `omx usage` 重构为单屏摘要：window、total tokens、可用 cost status、client rows、top model、freshness/coverage。
- [x] 4.2 实现 `--group-by day` 与 `--group-by model` 的 ccusage 风格紧凑表格，不引入全屏 TUI、图表或键盘事件依赖。
- [x] 4.3 实现 `--details` renderer，展示 input/output/cache read/cache write/reasoning/provider total/events/cost status/quality。
- [x] 4.4 仅在 pricing coverage 完整时展示无歧义 total cost；partial/missing 时显示状态，禁止把缺失 cost 当 `$0.00`。
- [ ] 4.5 正常 scan 时将 diagnostics 折叠为 freshness/coverage；scan failure、partial source 或 stale data 时展示脱敏警告并保留历史 summary。
- [ ] 4.6 增加一个常规宽度和一个窄宽度 snapshot，确保默认摘要单屏可读且关键字段不丢失。

## 5. Versioned JSON 与数据质量

- [x] 5.1 从 `UsageReport` 生成 versioned JSON，增加 totals、groups、freshness、coverage 和 accounting，并制定与现有 schema v1 的兼容/升级方案。
- [ ] 5.2 增加 contract tests，验证相同 query 的 human output 与 JSON 在 window、totals、groups 和 status 上一致。
- [ ] 5.3 区分 empty usage、missing source、scan failed、partial coverage 和 stale snapshot，并分别增加 CLI/JSON integration tests。
- [x] 5.4 验证 `omx usage --json --no-scan` 在 pipe/无 TTY 环境不初始化 TUI，stdout 只包含机器可读 JSON。
- [x] 5.5 对新增 freshness、coverage 和 diagnostics 字段执行隐私回归，禁止泄漏 raw prompt/response/log、auth payload、token、API key 或未脱敏私有路径。

## 6. Account/Profile Attribution 安全护栏

- [ ] 6.1 审计 ingest/query 路径，确认扫描时 current account/profile 不会被写入历史 usage event。
- [ ] 6.2 增加回归测试：切换 active account 后扫描旧日志，历史 event 仍不归入新 active account。
- [ ] 6.3 记录 future attribution proposal 的触发条件；在 evidence/coverage 门槛确定前不公开 account group-by。

## 7. 文档、验证与交付

- [x] 7.1 更新用户文档，说明 OpenMux usage 的辅助定位、CLI lens、parsed/cost/quota 区别和非账单级准确性。
- [x] 7.2 更新架构文档，固化 `tokscale-core -> adapter -> UsageEvent -> SQLite -> UsageReport -> CLI/JSON` 边界和上游贡献策略。
- [x] 7.3 使用临时 `OMUX_STATE_ROOT` 和隔离 client homes 手动验证 today、7d、30d、model/day details、no-scan 和 JSON。
- [x] 7.4 运行 `cargo fmt --all`、`cargo test` 和 `cargo clippy --all-targets --all-features`。
- [x] 7.5 复核本变更未引入 tokscale TUI、ccusage runtime 或第二套 pricing/aggregation 实现。
