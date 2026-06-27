## ADDED Requirements

### Requirement: Provider runtime SHALL have an explicit lifecycle
Each provider SHALL expose a runtime lifecycle through the control plane, including detection, availability, configured enablement, refresh eligibility, in-flight state, cancellation, timeout, backoff, last success, last failure, and source label.

#### Scenario: Provider refresh starts
- **WHEN** a frontend requests provider refresh
- **THEN** the control plane SHALL create or join a provider-scoped refresh task
- **AND** SHALL expose in-flight state without launching duplicate equivalent refreshes.

#### Scenario: Provider refresh times out
- **WHEN** a provider refresh exceeds its timeout
- **THEN** the control plane SHALL cancel or terminate the underlying provider work where possible
- **AND** SHALL return a failed or stale operation result with safe diagnostics.

### Requirement: Refresh requests SHALL be coalesced and generation-safe
Provider refreshes and mutations SHALL use generation or request identity so older responses cannot overwrite newer authoritative state.

#### Scenario: User triggers manual refresh during background refresh
- **WHEN** a background refresh is in flight and user triggers manual refresh for the same provider
- **THEN** the control plane SHALL either replace or coalesce the earlier request according to operation kind
- **AND** stale older responses SHALL NOT overwrite the newer result.

#### Scenario: Mutation changes active account during refresh
- **WHEN** account activation completes while a prior refresh for the old account is still running
- **THEN** the prior refresh result SHALL be discarded unless its account scope still matches the current view target.

### Requirement: Usage source strategy SHALL be explicit
Each provider SHALL declare available source strategies, source priority, foreground/background eligibility, timeout, fallback rules, source labels, confidence, and safe failure reasons.

#### Scenario: Codex usage auto mode runs
- **WHEN** Codex usage source is `auto`
- **THEN** backend SHALL record which source was attempted, selected, skipped, or failed
- **AND** SHALL expose the resolved source label and confidence in the view model.

#### Scenario: Source is not suitable for background refresh
- **WHEN** a source may open UI, launch an interactive TUI, require high battery usage, or prompt for permissions
- **THEN** backend SHALL mark it foreground-only or opt-in
- **AND** SHALL NOT use it during routine background refresh.

### Requirement: Last-good cache SHALL have provider and account scope
The control plane SHALL maintain safe last-good snapshots scoped by provider and, when identity is known, by account/runtime scope. Cached data SHALL include freshness and stale reason.

#### Scenario: Account-scoped refresh fails
- **WHEN** refresh for one managed account fails
- **THEN** backend MAY show that account's last-good snapshot as stale
- **AND** SHALL NOT substitute another account's last-good quota.

#### Scenario: Cache TTL expires
- **WHEN** cached data exceeds its allowed stale TTL
- **THEN** control plane SHALL mark data unavailable or expired
- **AND** SHALL provide a recovery action or refresh action when possible.

### Requirement: Settings SHALL be shared and typed
Provider enablement, provider order, source preference, refresh cadence, display preference, debug/recovery switches, and feature flags SHALL live in a shared typed config model consumed by CLI, Menubar, and future frontends.

#### Scenario: User changes refresh cadence in Menubar
- **WHEN** Menubar updates refresh cadence
- **THEN** the change SHALL persist through the shared config/settings model
- **AND** CLI or future frontends SHALL observe compatible semantics rather than a private Menubar-only setting.

#### Scenario: Config version is unsupported
- **WHEN** a frontend or backend sees a future unsupported config version
- **THEN** it SHALL fail closed with a safe diagnostic
- **AND** SHALL NOT silently rewrite the config.

### Requirement: Diagnostics SHALL be structured and redacted
Diagnostics SHALL distinguish user-facing message, machine code, severity, recovery action, debug detail, source, and redaction status. Sensitive values including tokens, cookie headers, authorization headers, raw auth payloads, email where inappropriate, and API keys SHALL be redacted before crossing frontend or log boundaries.

#### Scenario: Provider returns authorization error
- **WHEN** a provider refresh fails due to expired credentials
- **THEN** control plane SHALL return a user-facing auth-expired diagnostic with recovery action
- **AND** SHALL NOT include access tokens, refresh tokens, Cookie headers, Authorization headers, or raw provider responses.

#### Scenario: User exports support diagnostics
- **WHEN** user requests a support report
- **THEN** OpenMux SHALL include versions, provider status, safe operation history, config shape, and redacted logs
- **AND** SHALL exclude raw credentials and raw provider payloads.

### Requirement: Provider maturity SHALL gate frontend exposure
Each provider SHALL declare maturity capabilities such as detected-only, account-switchable, profile-switchable, quota-readable, account-scoped-refreshable, local-usage-readable, and menubar-ready.

#### Scenario: Provider lacks quota support
- **WHEN** a provider is account-switchable but not quota-readable
- **THEN** Menubar SHALL still allow supported account actions
- **AND** SHALL show quota unavailable explicitly rather than inventing quota from local usage.

#### Scenario: Provider is not menubar-ready
- **WHEN** a provider lacks required view model fields for Menubar
- **THEN** it SHALL remain hidden or disabled in Menubar with a safe explanation
- **AND** CLI support MAY still be available if its contract is satisfied.
