## ADDED Requirements

### Requirement: Control-plane SHALL expose provider-agnostic public APIs
The control plane SHALL expose public APIs grouped by query, mutation, runtime, settings, diagnostics, and compatibility. API names SHALL describe product actions rather than provider-specific implementation details.

#### Scenario: Frontend loads dashboard
- **WHEN** a frontend needs the current dashboard
- **THEN** it SHALL call a provider-agnostic query such as `dashboard_view(query)`
- **AND** SHALL NOT call Codex-specific or Claude-specific functions directly.

#### Scenario: Frontend activates target
- **WHEN** a frontend needs to switch account or profile
- **THEN** it SHALL call a provider-agnostic mutation such as `activate_target(command)`
- **AND** provider-specific activation SHALL remain behind plugin/control-plane dispatch.

#### Scenario: Frontend requests settings
- **WHEN** a frontend needs provider enablement, source mode, refresh cadence, or feature flags
- **THEN** it SHALL call `settings_view` or `update_settings`
- **AND** SHALL NOT read or write provider behavior settings through private frontend storage.

### Requirement: Control-plane modules SHALL separate queries, mutations, runtime, settings, diagnostics, and mapping
The control-plane implementation SHALL be split into modules that prevent provider logic, DTO mapping, runtime scheduling, and transport from accumulating in a single file.

#### Scenario: Developer adds new operation
- **WHEN** a new user-visible operation is added
- **THEN** command parsing, provider dispatch, operation result mapping, diagnostics, and transport encoding SHALL live in separate modules
- **AND** the operation SHALL have focused tests for each boundary.

#### Scenario: Developer edits FFI transport
- **WHEN** FFI schema handling changes
- **THEN** provider refresh eligibility, target activation, source strategy, and diagnostics redaction SHALL remain unchanged
- **AND** tests SHALL verify transport still delegates to control-plane APIs.

### Requirement: Control-plane SHALL use layered DTOs
The control plane SHALL keep domain records, application state, and frontend-safe view DTOs separate. Frontend-safe DTOs SHALL be stable, versioned, and redacted.

#### Scenario: Plugin returns account record
- **WHEN** a plugin returns an account or profile record
- **THEN** mapper code SHALL convert it into an application target model
- **AND** view mapper code SHALL convert it into frontend-safe target row data.

### Requirement: Provider-specific code SHALL stay in plugins or provider adapters
Provider-specific file paths, auth parsing, credential refresh, runtime scope construction, and endpoint/source selection SHALL live in provider plugins or provider adapters, not in generic control-plane views or Swift components.

#### Scenario: Codex managed runtime is implemented
- **WHEN** Codex needs a managed `CODEX_HOME`
- **THEN** Codex-specific scope creation SHALL live in Codex plugin/adapter code
- **AND** generic control-plane APIs SHALL refer only to runtime scope capabilities and target identity.

#### Scenario: Claude runtime scope is added
- **WHEN** Claude receives account-scoped refresh support
- **THEN** Claude-specific credential backend and settings patching SHALL stay in Claude plugin/adapter code
- **AND** generic control-plane APIs SHALL remain unchanged.

### Requirement: Transport SHALL not own business behavior
FFI, future HTTP, and CLI JSON transport SHALL only encode/decode requests, enforce schema compatibility, call control-plane APIs, and encode safe responses.

#### Scenario: FFI receives refresh request
- **WHEN** FFI receives a provider refresh request
- **THEN** it SHALL decode the envelope and call the control-plane refresh API
- **AND** SHALL NOT compute refresh eligibility, source strategy, or provider status itself.
