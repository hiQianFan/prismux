## Why

OpenMux 的长期架构已经明确为 `omx-core -> omx-control-plane -> presentation surfaces`。CLI、Menubar 和未来 Desktop 都应该消费同一套 control-plane 事实；它们只负责操作入口和展示形态，不应该各自重新解释 quota health、usage headline、provider status 或 action eligibility。

当前 Overview 聚合暴露了几个边界问题：

- **Frontend 仍在自算业务口径。** Swift `DashboardView.swift` / `OverviewProviderRow.swift` 仍计算 `lowestQuota`、quota color、provider 过滤和局部状态文案；CLI 也还有 `menubar_overall_availability` / `menubar_window_availability` 这类 presentation 层汇总。两边可能继续漂移。
- **Control-plane 类型仍带 Menubar 历史命名。** `MenubarDashboardReport`、`MenubarUsageSummary` 等已经被 CLI 和 Menubar 共同消费，但命名仍让人误以为只服务 Menubar。本变更借重构一次性把这些共享类型全量重命名为 surface-agnostic 命名，不保留 `Menubar*` 别名或兼容垫片。
- **Usage headline 和图表口径不同源。** `menubar_usage_for_client` 的 headline 取 `today_window()`，图表桶是 30 天 hourly 数据；period 切换时 headline 与图表容易不一致。
- **Core / control-plane 边界需要重新收紧。** `core` 应提供最小事实、持久化查询和 provider 安全原子；Overview 的阈值、最佳备选、告警、headline、operation result 属于 control-plane 产品语义，不应下沉到 core 或 frontend。任何放入 core 的 helper 都不能接收 `MenubarAccount` 或 app DTO，也不能把 facts 和 policy 揉进同一个返回结构。
- **Contract 现在缺少硬边界。** diagnostics 不能靠 `message.contains(provider)` 关联；schema/FFI 字段变更需要版本、fixture 和 CLI/Menubar contract 测试兜住。

本变更把“Overview 聚合”重新定义为 **control-plane aggregate primitives**：core/plugin/store 提供最小事实，`omx-app` 组合成 CLI、Menubar、future Desktop 共用的业务口径，各 presentation surface 只渲染。

## What Changes

- **新增 Layer Responsibility Matrix。** 明确 `omx-core`、provider plugin、`omx-app` control-plane、transport、frontend 各自能做什么、不能做什么，并把本次聚合字段逐项归属。
- **新增 control-plane 共享聚合模型。** 在 `omx-app` 定义 surface-agnostic 的 `QuotaHealthRollup` / `UsageHeadline` / `ProviderAggregateView` 等共享 projection；不新增新的 `Menubar*` 命名。
- **core 只暴露最小事实和中性查询。** 复用 `AccountStatus`、`UsageSnapshot`、`UsageLimit`、`StateStore::usage_summaries_by`、`TargetCatalog`、`PlatformPlugin::refresh_account` 等事实原子。若需要纯计算 helper，只能是不含 product policy 的数据折叠。
- **product policy 留在 control-plane。** quota 阈值、status/tone、best alternative、reset escape hatch、provider aggregate status、usage headline 都由 `omx-app` 组合输出。
- **facts 与 display 分离。** hourly cost 等事实字段使用机器可消费数值；`status_text`、`provider_display_label` 只能作为 display projection，不能替代 semantic status、provider id 或 diagnostics scope。
- **CLI / Menubar 共用同一聚合口径。** Menubar Overview 是第一个 UI 消费方；CLI `list/status/usage` 和未来 Desktop 后续应复用同一 control-plane projection，而不是各自重算。
- **加厚 usage hourly projection。** control-plane 的 hourly bucket projection 增加 cost 和 cost status，使任意 period 的 headline 和 chart 能从同一份窗口数据折叠。（不含趋势/环比——那是另行评估的业务功能，不在本次 scope。）
- **收敛错误逻辑。** 本次重构应删除或替代 Swift `lowestQuota` / `quotaColor` / provider filter 业务推断，以及 CLI 中与 quota/usage headline 重复的聚合逻辑；剩余 terminal/UI layout 逻辑保留在 frontend。

## Capabilities

### New Capabilities

- `control-plane-aggregate-primitives`: 在 control-plane 提供 surface-agnostic 的容量、用量、成本和 provider 聚合 projection，供 CLI、Menubar 和未来 Desktop 共享同一业务口径。

### Modified Capabilities

- `frontend-experience-boundary`: 进一步明确 frontend 只能渲染 control-plane 事实，不得重新定义 quota risk、usage headline 或 provider aggregate status。
- `provider-usage-statistics`: hourly projection 携带 cost 与 cost status；底层 scan/ingest 不变。

## Impact

- `crates/omx-core`: 不新增 Menubar/Overview 专属业务模型；只复用或补充最小事实查询和不含 product policy 的纯 helper。
- `crates/omx-app`: 全量重命名共享 `Menubar*` 类型为 surface-agnostic 命名，新增共享聚合 projection 与 DTO。不保留旧名别名或兼容垫片。
- `crates/omx-cli`: 改为消费 control-plane 聚合事实；表格/JSON 输出保持 CLI presentation，但不再自算 quota health / headline / usage total 口径。
- `apps/omx-menubar`: 删除业务聚合推断与旧 `Menubar*` 解码名，对接中性 DTO；保留 Swift state machine、layout、component 和交互状态。
- `crates/omx-menubar-ffi`: 继续只做 transport，不承载聚合或业务判断；随重命名同步更新 op 名/类型。
- 测试：新增 control-plane 聚合单测、CLI/Menubar 同口径 contract 测试、hourly cost/headline period 折叠测试、旧 today 口径回归测试、schema version/fixture 测试。
- 不做：实现 Overview UI 布局；改 usage scan/ingest；新增网络或 account/profile 级 usage 归因；趋势/环比等新业务指标。
