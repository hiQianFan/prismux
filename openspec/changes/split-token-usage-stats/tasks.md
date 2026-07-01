# Tasks — Split Token Usage Stats from Main

> 执行顺序刻意"自上而下"(消费层 → omx-app → core),让编译器在删除底层类型前先暴露每一处遗漏的引用,避免误伤额度链路。本提案仅规划,**执行时**才动代码。

## 1. 冻结现场(先做,零风险)

- [ ] 1.1 在当前 HEAD 打 tag:`git tag usage-stats-v0`
- [ ] 1.2 (可选)留分支便于发现:`git branch feature/usage-stats`
- [ ] 1.3 把未提交的 `openspec/changes/redesign-usage-ledger-rollups/` 先提交进冻结点(否则 tag 不含它),再从主分支移除
- [ ] 1.4 `assets/`(app 图标,与统计无关)正常提交进主分支,**不**纳入冻结/移除范围
- [ ] 1.5 确认 `git checkout usage-stats-v0 -- crates/omx-usage-tokscale` 能取回,验证冻结有效

## 2. 删除消费层(Swift / CLI / FFI)

- [ ] 2.1 Swift:删除 `UsageCard.swift`、`UsageSeries.swift`;从 `DashboardView.swift` 移除挂载与引用;评估 `UsageStatsStrip.swift` 用途后删除或保留;同步 `OmxMenubarContractTests/main.swift`
- [ ] 2.2 Swift DTO:从 `DTO.swift` 删除 `UsageSummary`/`UsageHeadline`/`UsageModelBreakdown`/`ProviderUsageSummary`/`Coverage` 及 `DashboardReport.usage`/`providerUsage`;**保留** `QuotaHealthRollup`/`Quota`/`ResetCredits`/`QuotaWindow`
- [ ] 2.3 CLI:删除 `Command::Usage` 及其 arg 类型(`UsagePeriodArg`/`UsageGroupByArg`)、`print_usage*`/`usage_report`/`usage_window`/`usage_group_by`;移除 `use omx_usage_tokscale::*`;**保留** `usage_cell`(provider 表额度列)
- [ ] 2.4 CLI 测试:重写/删除 `crates/omx-cli/tests/usage_cli.rs` 中针对 `omx usage` 的用例
- [ ] 2.5 FFI:删除 `refresh_usage_cache`/`usage_refresh_since_unix`、dashboard payload 的 `usage`(total_tokens/top_model/coverage)分支及相关调用;移除 `use omx_usage_tokscale::*`;**保留** `aggregate.quota_health` 组装;更新对应测试
- [ ] 2.6 **契约版本**:bump `SCHEMA_VERSION`(1 → 2);确保 2.2(Swift DTO 删字段)与 2.5(Rust 删 payload 字段)在**同一次提交/发布**内完成,同步更新 `OmxMenubarContractTests/main.swift` 的 golden/断言,避免旧 app 解码新 payload 运行时失败

## 3. 删除聚合层(omx-app)

- [ ] 3.1 `query.rs`:删除 `usage_hourly_summaries`(唯一 `usage_summaries_by` 调用点)、`usage_headline`/`usage_total`/hourly 相关 helper、`menubar_today_usage`;从 dashboard 组装中移除 `usage`/`provider_usage`/`usage_headline` 字段填充;**保留** `quota_health_rollup`/`quota_facts_rollup`/`window_averages`
- [ ] 3.2 `dto.rs`:删除 `UsageSummaryView`/`UsageHeadline`/`ProviderUsageSummary`/`Coverage`/`UsageModelBreakdown` 及 `DashboardView.usage`/`provider_usage`、`*Aggregate.usage_headline`;**保留** `QuotaHealthRollup`/`quota_health`
- [ ] 3.3 检查 `mutation.rs`/`api.rs`/`compatibility.rs`/`settings.rs` 的残留引用并清理

## 4. 删除 plugin 采集

- [ ] 4.1 plugin-codex:删除 usage_events 的本地采集/ingest 路径;**保留** `parse_codex_usage_snapshot`→limits/reset、`fetch_codex_usage`、`cached_usage`/`usage_from_snapshot`(喂额度)
- [ ] 4.2 plugin-codex `tests.rs`:清理 usage_events 相关用例
- [ ] 4.3 plugin-claude:确认 `usage: None`(plugin.rs:656)在字段保留后仍成立

## 5. 删除 core 数据模型与 storage

- [ ] 5.1 `state_store.rs`:删除 `usage_events` 表 + 5 索引(`idx_usage_client_time`/`idx_usage_client_model_time`/`idx_usage_client_provider_time`/`idx_usage_client_project_time`/`idx_usage_session`) + `source_record_hash` 列迁移、`scan_watermarks` 表、`insert_usage_event*`/`ingest_usage_events`/`usage_summaries*`/`update_scan_watermark`/`scan_watermark`/`usage_event_payload*`、`StoredUsageEvent`/`UsageEventPayload`、`usage_quality_name`/`usage_summary_from_row`/`scan_watermark_from_row` 及全部对应测试;**保留** `save_quota_snapshot`/`latest_quota_snapshot`/`StoredUsageSnapshot`/`usage_source_name`
- [ ] 5.2 `usage.rs`:删除统计类型族(`UsageEvent`/`UsageSummary`/`UsageSummaryQuery`/`UsageQuery`/`UsagePeriod`/`UsageGroupBy`/`UsageReport`/`UsageFreshness`/`UsageCoverage`/`UsageAccounting`/`UsageReportScan`/`UsageTokenBreakdown`/`CostStatus`/`UsageEventSource`/`UsageFileSampleHash`/`UsageRelatedSourceFingerprint`/`UsageSourceFingerprint`/`UsageScan*`/`UsageScanWatermark`/`UsageDataQuality`);**保留** `UsageSnapshot`/`Availability`/`AvailabilityState`/`UsageLimit`/`UsageLimitScope`/`UsageLimitKind`/`UsageResetCredits`/`UsageResetCredit`/`UsageSource`/`UsageDiagnostic`
- [ ] 5.3 `lib.rs`:`pub use usage::*` 若因分模块而需调整则更新
- [ ] 5.4 `account.rs`/`target.rs`:确认 `usage: Option<UsageSnapshot>` 字段(保留)仍编译

## 6. 删除独立 crate

- [ ] 6.1 删除 `crates/omx-usage-tokscale/` 整个目录
- [ ] 6.2 从根 `Cargo.toml` 的 `members` 移除;从 `omx-cli`/`omx-menubar-ffi` 的 `Cargo.toml` 移除依赖;重新生成 `Cargo.lock`
- [ ] 6.3 评估 `vendor/tokscale-core` 是否随之移除(它是 tokscale 的 vendored 源)

## 7. 验证

- [ ] 7.1 `cargo build --workspace` 通过
- [ ] 7.2 `cargo test --workspace` 通过
- [ ] 7.3 Swift 侧构建通过(menubar app)
- [ ] 7.4 手动/契约验证:账号卡额度(5h/7d %、reset credits)仍显示;CLI provider 表额度列仍显示;刷新失败时额度快照回退仍生效
- [ ] 7.5 确认 menubar 无 Token Usage 面板残留、CLI 无 `omx usage` 子命令
- [ ] 7.6 `openspec validate split-token-usage-stats --strict` 通过

## 8. 文档与 spec 归档

- [ ] 8.1 归档 `provider-usage-statistics`、`refine-usage-cli-experience`、`redesign-usage-ledger-rollups`
- [ ] 8.2 评估 `persist-usage-snapshot-on-refresh-failure`:拆出"额度快照回退"(保留)与 usage event / scan watermark roadmap(归档)
- [ ] 8.3 README:更新 Codex 能力描述(移除 "best-effort usage" 中属统计的部分,保留额度);在 roadmap/non-goals 写明"token 用量统计已剥离至 `usage-stats-v0`,后续重新设计后引入"
- [ ] 8.4 同步修正仍引用 token stats 的文档/变更:`add-native-menubar-app` 的最小 usage 摘要、`compose-overview-aggregate-primitives` 的 `UsageHeadline`/hourly projection、`evaluate-tokenbar-menubar-spike` 的 `omx usage --json` 样例、`docs/menubar-v1.md`、`docs/menubar-v1-acceptance-review.md`
