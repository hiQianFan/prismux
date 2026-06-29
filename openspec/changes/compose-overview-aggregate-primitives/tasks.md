## 1. 边界与模型定义

- [ ] 1.1 在 design 中固定 Layer Responsibility Matrix，并在实现评审中按矩阵检查代码归属。
- [ ] 1.2 定义 surface-agnostic projection 类型：`QuotaHealthRollup`、`UsageHeadline`、`ProviderAggregateView`、`DashboardAggregateView`。
- [ ] 1.3 明确哪些 helper 留在 `omx-app`，哪些 facts 继续来自 `omx-core` / provider plugin / `StateStore`。
- [ ] 1.4 全量把共享 `Menubar*` 类型重命名为 surface-agnostic 命名，删除旧名，不保留别名或兼容垫片。
- [ ] 1.5 检查 core 新增/迁移函数签名：不得接收 `MenubarAccount`、surface DTO 或 `omx-app` 类型。
- [ ] 1.6 明确 display 字段归属：machine semantic status/tone 必须先存在，`status_text`、`provider_display_label` 只能作为 display projection。

## 2. Quota facts 与 control-plane health projection

- [ ] 2.1 在 `omx-app` 实现 quota facts 折叠 helper：account_count、reporting_count、avg/min/max remaining、soonest_reset、reset_credit_total。
- [ ] 2.2 在 `omx-app` 实现 product policy：healthy/low/exhausted 分类、provider status/tone、worst target、best alternative。
- [ ] 2.3 全局平均必须折叠原始 reporting accounts，禁止 provider 平均再平均。
- [ ] 2.4 best alternative 只使用 control-plane action eligibility，不在 core 里判断。
- [ ] 2.5 禁止一个 `quota_rollup` 同时返回 facts 与 policy；实现上拆成 facts rollup 和 control-plane policy projection。
- [ ] 2.6 单测：无 quota 不计入均值、全部无 quota 返回 None、全局均值不可由 provider 均值再平均、best alternative tie-break、reset credit 单列。

## 3. Usage projection 与 period headline

- [ ] 3.1 保持 `StateStore::usage_summaries_by` 作为最小查询事实，不改 scan/ingest。
- [ ] 3.2 在 control-plane 生成 hourly bucket projection，补 `estimated_cost_usd` 与 `cost_status`。
- [ ] 3.3 在 control-plane 实现 `UsageHeadline`：period、total tokens、estimated cost、cost status、top model、breakdown。
- [ ] 3.4 实现 `TargetRecommendation`：用 action eligibility + quota facts 选 best alternative，输出推荐原因和可执行 action（control-plane policy，core 不参与）。
- [ ] 3.5 移除 headline 钉死 `today_window()` 的独立快照路径，headline 与 chart 使用同源数据。
- [ ] 3.6 hourly atom 的 cost 使用机器可消费数值，不存 `"$1.23"` 这类预格式化字符串。
- [ ] 3.7 单测：today 与旧口径一致、7d/30d headline 随 period 变化、missing cost 不显示 0、mixed cost status 正确、top model 来自同源 series。

## 4. Provider grouping 与 dashboard projection

- [ ] 4.1 在 `omx-app` 抽 `group_targets_by_provider(accounts, profiles)`。
- [ ] 4.2 `provider_views`、active target/count、per-provider quota health、diagnostics scope 统一从分组结果取数。
- [ ] 4.3 `dashboard_view` 输出全局 `DashboardAggregateView` 和 per-provider `ProviderAggregateView`。
- [ ] 4.4 diagnostics 数据结构携带 `provider_id` / `target_id` / scope，删除 `message.contains(provider)` 关联方式。
- [ ] 4.5 单测：provider 排序稳定、空 provider 保留 planned/unavailable 语义、多 provider active 状态不串、diagnostics scope 不靠 message 文本匹配。

## 5. Frontend 消费迁移

- [ ] 5.1 Swift DTO 解码新增 neutral aggregate 字段。
- [ ] 5.2 删除 Swift `DashboardView.lowestQuota`、`lowestQuotaSummary`、`OverviewProviderRow.quotaColor` 的业务推断。
- [ ] 5.3 Swift provider/overview 渲染改读 control-plane aggregate projection；只保留 layout、tone-to-color、loading/pending state。
- [ ] 5.4 CLI `list/status/usage` 改读 control-plane aggregate projection；只保留 table/JSON renderer。
- [ ] 5.5 删除 CLI `menubar_overall_availability`、`menubar_window_availability` 或将其降级为纯 renderer helper。
- [ ] 5.6 删除或改造 CLI `usage_groups`、`usage_total` 中与 control-plane period headline 重复的业务聚合。

## 6. Contract 与验证

- [ ] 6.1 全量重写 FFI fixtures 为重命名后的中性字段；不保留旧名 fixture。
- [ ] 6.2 增加同一 state root 下 CLI 与 Menubar aggregate 口径一致的 contract 测试。
- [ ] 6.3 bump `CONTROL_PLANE_SCHEMA_VERSION`（重命名属破坏式变更）；旧名 fixture 全部替换为新名，contract 测试断言新版本与同口径聚合值。
- [ ] 6.4 `cargo fmt --all`、`cargo test`、`cargo clippy --all-targets --all-features` 通过。
- [ ] 6.5 menubar contract / Swift build 脚本通过。
- [ ] 6.6 `openspec validate compose-overview-aggregate-primitives` 通过。
