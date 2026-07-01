## ADDED Requirements

### Requirement: 聚合业务口径由 control-plane 产生

OpenMux SHALL 通过 `omx-app` control-plane 产生 CLI、Menubar 和未来 presentation surface 共用的 aggregate projection。Presentation surfaces SHALL render these projections and SHALL NOT independently redefine quota health, usage headline, provider status, action eligibility, or diagnostics.

#### Scenario: CLI 与 Menubar 使用同一聚合事实

- **WHEN** CLI 和 Menubar 读取同一个 state root
- **THEN** 它们 SHALL 从 control-plane 取得同一 quota health、usage headline、provider aggregate status 和 diagnostics
- **AND** 它们 MAY choose different layouts or terminal/native rendering
- **AND** 它们 SHALL NOT use different thresholds or independent aggregation formulas.

#### Scenario: Future Desktop 复用同一 control-plane contract

- **WHEN** future Desktop app needs dashboard or provider aggregate data
- **THEN** it SHALL consume the same control-plane aggregate projection
- **AND** it SHALL NOT introduce a second product logic implementation.

### Requirement: Core SHALL expose facts, not product overview policy

`omx-core` SHALL own domain facts, storage queries, target resolution, and safety primitives. It SHALL NOT own Overview-specific product policy such as quota risk thresholds, best alternative selection, provider aggregate tone, usage headline wording, or surface DTOs.

#### Scenario: Core function signatures use only domain facts

- **WHEN** this change introduces or moves a quota/usage aggregate helper into `omx-core`
- **THEN** the helper SHALL accept only core/domain types such as `AccountStatus`, `UsageSnapshot`, `UsageLimit`, or value structs
- **AND** it SHALL NOT accept `MenubarAccount`, `MenubarDashboardReport`, Swift DTO mirrors, or any `omx-app` DTO.

#### Scenario: Core provides quota facts

- **WHEN** control-plane needs quota information
- **THEN** it SHALL read `UsageSnapshot`, `UsageLimit`, `UsageResetCredits`, or equivalent domain facts from core/provider/store
- **AND** core SHALL NOT decide whether the account is an Overview warning, danger, or best alternative.

#### Scenario: Control-plane applies product policy

- **WHEN** control-plane builds a dashboard or provider view
- **THEN** it SHALL apply product policy for quota health, provider status/tone, best alternative, reset escape hatch, and user-facing diagnostics
- **AND** the resulting projection SHALL be frontend-safe and redacted.

#### Scenario: Fact atoms do not carry formatted display strings

- **WHEN** core/store/control-plane exposes hourly usage or cost facts
- **THEN** cost SHALL be represented as a machine-readable numeric or decimal-compatible value with an explicit status
- **AND** fact atoms SHALL NOT store preformatted display strings such as `"$1.23"` as the source value.

### Requirement: 新共享聚合类型 SHALL use surface-agnostic naming

New shared aggregate DTOs SHALL use surface-agnostic names. This change SHALL rename shared `Menubar*` DTOs to surface-agnostic names without aliases or compatibility shims.

#### Scenario: 新增 quota aggregate

- **WHEN** this change adds or renames a shared aggregate projection
- **THEN** it SHALL use a neutral name such as `QuotaHealthRollup`
- **AND** it SHALL NOT add or retain a shared type named `MenubarQuotaRollup`.

### Requirement: Quota aggregate SHALL separate facts from product policy

OpenMux SHALL separate neutral quota fact folding from control-plane product policy. Neutral fact folding MAY compute counts, reporting counts, average/min/max remaining, soonest reset, and reset credit totals. Control-plane policy SHALL compute health buckets, provider tone, worst target, and best alternative.

#### Scenario: Facts rollup has no policy fields

- **WHEN** neutral quota fact folding returns a rollup
- **THEN** the rollup MAY include account counts, reporting counts, avg/min/max remaining, soonest reset, and reset credit totals
- **AND** it SHALL NOT include health bucket names, status text, provider tone, worst target display priority, or best alternative.

#### Scenario: 全局平均折叠原始 reporting accounts

- **WHEN** provider A has 1 reporting account at 90% remaining
- **AND** provider B has 3 reporting accounts at 30% remaining
- **THEN** global average remaining SHALL be `(90+30+30+30)/4 = 45%`
- **AND** OpenMux SHALL NOT output `(90+30)/2 = 60%`.

#### Scenario: 无 quota 上报的账号不进入均值

- **WHEN** a scope has 3 accounts and only 2 report quota at 80% and 60%
- **THEN** `account_count` SHALL be 3
- **AND** `reporting_count` SHALL be 2
- **AND** average remaining SHALL be 70%.

#### Scenario: Best alternative is control-plane policy

- **WHEN** a scope has multiple candidate accounts
- **THEN** control-plane SHALL choose best alternative using action eligibility and quota facts
- **AND** core SHALL NOT encode that product recommendation.

### Requirement: Usage headline SHALL come from control-plane period projection

OpenMux SHALL produce usage headline fields through control-plane period projection over shared usage facts. Headline totals, cost, top model, and model breakdown SHALL share the same source window as the chart data.

#### Scenario: period 切换 headline 与图表同源

- **WHEN** user chooses today, 7d, or 30d
- **THEN** control-plane SHALL produce headline values for that selected period
- **AND** chart data and headline SHALL derive from the same usage summary facts
- **AND** headline SHALL NOT remain fixed to today's snapshot when the selected period changes.

#### Scenario: Cost status remains honest

- **WHEN** pricing is missing for the selected period
- **THEN** control-plane SHALL return missing cost status
- **AND** frontend SHALL NOT display `$0.00` as a valid cost.

### Requirement: Provider grouping SHALL be a control-plane primitive

OpenMux SHALL group accounts and profiles by provider in control-plane and SHALL reuse that grouping for provider aggregate view, active target/count, quota health, and diagnostics scope.

#### Scenario: Frontend does not rederive provider business state

- **WHEN** a presentation surface renders provider pages or overview rows
- **THEN** it SHALL consume grouped/aggregated control-plane data
- **AND** it SHALL NOT filter raw accounts/profiles to compute provider health, alerts, quota health, or action eligibility.

#### Scenario: Diagnostics carry structured scope

- **WHEN** OpenMux emits a diagnostic that belongs to a provider or target
- **THEN** the diagnostic SHALL carry structured scope such as `provider_id`, `target_id`, or a scoped enum
- **AND** consumers SHALL NOT associate diagnostics to providers by checking whether `message` contains a provider label.

### Requirement: Display fields SHALL not replace semantic fields

Control-plane MAY provide display projection fields for shared copy, but machine-readable semantic fields SHALL remain the contract source for product logic and frontend rendering decisions.

#### Scenario: Status text is derived from semantic status

- **WHEN** a provider or dashboard status is returned
- **THEN** the projection SHALL include semantic status/tone fields
- **AND** any `status_text` SHALL be treated as display copy derived from those fields
- **AND** presentation surfaces SHALL NOT parse `status_text` to decide severity or actions.

#### Scenario: Provider label is not provider identity

- **WHEN** a provider display label is returned
- **THEN** provider identity and grouping SHALL use stable provider ids
- **AND** consumers SHALL NOT use `provider_display_label` for lookup, grouping, or diagnostics matching.

### Requirement: Existing misplaced aggregation SHALL be migrated

OpenMux SHALL migrate existing duplicated aggregate logic out of frontends when equivalent control-plane projection exists.

#### Scenario: Swift quota aggregation is removed

- **WHEN** control-plane provides quota health projection
- **THEN** Swift SHALL remove business calculations such as `lowestQuota`, `lowestQuotaSummary`, and quota threshold color logic
- **AND** Swift SHALL keep only semantic tone rendering and layout.

#### Scenario: CLI quota aggregation is removed

- **WHEN** control-plane provides quota health projection
- **THEN** CLI SHALL stop independently computing `menubar_overall_availability` and `menubar_window_availability`
- **AND** CLI SHALL keep only terminal/table rendering.

#### Scenario: CLI usage aggregation is removed

- **WHEN** control-plane provides period usage headline and breakdown projection
- **THEN** CLI SHALL stop independently computing business totals through `usage_groups` or `usage_total`
- **AND** CLI SHALL keep only terminal/table/JSON presentation.

### Requirement: Control-plane schema changes SHALL be versioned and contract-tested

OpenMux SHALL treat FFI/machine JSON fields as a control-plane contract. This change is a one-time breaking rename: shared types move to neutral names with no aliases or compatibility shims. The schema version SHALL be bumped and all fixtures/contract tests SHALL be rewritten to the new names.

#### Scenario: Breaking rename bumps version and replaces fixtures

- **WHEN** shared `Menubar*` types are renamed to neutral names
- **THEN** OpenMux SHALL bump `CONTROL_PLANE_SCHEMA_VERSION`
- **AND** old-name fixtures SHALL be replaced, not kept alongside new ones
- **AND** CLI/Menubar contract tests SHALL assert the new version and identical aggregate values under the same state root.
