## ADDED Requirements

### Requirement: Menubar SHALL use Overview plus provider pages
Menubar SHALL provide a top selector with an Overview page and one focused page per OpenMux tool provider. It SHALL NOT stack all provider detail sections into one long mixed page.

#### Scenario: User opens the popover
- **WHEN** dashboard contains one or more providers
- **THEN** the popover SHALL show an Overview selector item and provider selector items
- **AND** Overview SHALL show aggregate health/usage/quota summaries
- **AND** selecting Codex SHALL show only Codex account details and actions.

#### Scenario: User reviews Overview
- **WHEN** the Overview page is active
- **THEN** it SHALL show whole-pool account aggregation
- **AND** SHALL show each provider's active target summary
- **AND** SHALL show token usage aggregation by provider/client
- **AND** SHALL NOT expose per-account switch controls.

#### Scenario: User reviews one provider
- **WHEN** the Codex provider page is active
- **THEN** it SHALL show Codex overview first
- **AND** SHALL show account/profile selection before provider token usage
- **AND** SHALL use one scroll page rather than a second-level tabs row.

### Requirement: Provider account and profile rows SHALL show quota beside identity
Provider account/profile rows SHALL place identity and state on the left, with quota rings plus percent/reset text on the right.

#### Scenario: Account has quota windows
- **WHEN** a Codex account has 5h/session and 7d/weekly quota data
- **THEN** Menubar SHALL show both percentages and reset times in the account row
- **AND** MAY include activity-ring visuals as supplemental scan aids
- **AND** SHALL keep text values available for accessibility.

### Requirement: Provider SHALL have one active target across accounts and profiles
Each provider SHALL expose a single active target slot. Account targets and profile targets are mutually exclusive candidates for that slot.

#### Scenario: User switches from active account to profile
- **GIVEN** Codex account `work` is active
- **WHEN** user switches Codex to profile `api-key-backend`
- **THEN** backend SHALL return `active_after.target_kind = profile`
- **AND** profile `api-key-backend` SHALL be active
- **AND** account `work` SHALL be inactive in the returned dashboard.

#### Scenario: User switches from active profile to account
- **GIVEN** Codex profile `api-key-backend` is active
- **WHEN** user switches Codex to account `work`
- **THEN** backend SHALL return `active_after.target_kind = account`
- **AND** account `work` SHALL be active
- **AND** profile `api-key-backend` SHALL be inactive in the returned dashboard.

### Requirement: Tray title SHALL show aggregate signal instead of account email
The collapsed menu bar title SHALL prioritize usage rate, quota urgency, provider health, stale/error state, or a compact fallback. It SHALL NOT default to email/account label when that is not the highest-value status.

#### Scenario: Active account has an email label
- **WHEN** active account label is an email
- **THEN** tray title SHALL prefer usage, aggregate health, or quota/status text
- **AND** email SHALL remain available only inside the popover details.

### Requirement: Backend SHALL provide presentation-ready dashboard view
`omx-app` SHALL provide a dashboard view model that contains provider groups, account target display fields, action eligibility, quota/status text, local usage summary, and diagnostics so CLI and Menubar do not duplicate business interpretation.

#### Scenario: Swift decodes dashboard
- **WHEN** Menubar receives dashboard data
- **THEN** it SHALL render backend-provided display/status/action fields
- **AND** SHALL NOT infer account health from raw quota or local usage fields.

### Requirement: Mutation responses SHALL include operation result and full dashboard
Switch and refresh operations SHALL return an operation result plus backend-confirmed dashboard state.

#### Scenario: Switch succeeds
- **WHEN** user switches to a non-active account
- **THEN** backend SHALL return `operation.status = success`
- **AND** include `active_before`, `active_after`, `changed`, and full dashboard
- **AND** Menubar SHALL update active UI from the returned dashboard.

#### Scenario: Switch fails
- **WHEN** backend cannot switch account
- **THEN** backend SHALL return a safe failure result or error envelope
- **AND** Menubar SHALL keep previous active state visible
- **AND** SHALL show a脱敏 failure message.

### Requirement: Account quota and local usage SHALL remain distinct
Menubar SHALL display provider account quota/status separately from local token usage summary.

#### Scenario: Today usage is available
- **WHEN** dashboard includes local usage summary
- **THEN** Menubar MAY show today tokens/top model/coverage as local usage
- **AND** SHALL NOT present those tokens as provider quota remaining
- **AND** SHALL NOT use `model_provider` for provider grouping.
