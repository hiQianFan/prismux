## ADDED Requirements

### Requirement: Accounts SHALL support isolated runtime scopes
OpenMux SHALL support a model where managed accounts can have isolated runtime scopes, such as managed `CODEX_HOME`, for account-scoped refresh and metadata resolution without changing the system active account.

#### Scenario: Refresh inactive account quota
- **WHEN** Menubar refreshes quota for an inactive Codex account
- **THEN** backend SHALL use that account's isolated runtime scope when available
- **AND** SHALL NOT replace the user's system `~/.codex/auth.json` as part of refresh.

#### Scenario: Isolated scope is unavailable
- **WHEN** an account lacks a usable isolated runtime scope
- **THEN** backend SHALL report an unavailable or stale account-scoped refresh state
- **AND** SHALL NOT silently fall back to another account's credentials.

### Requirement: Existing snapshot accounts SHALL migrate to managed runtime scopes safely
OpenMux SHALL define a migration path from existing auth snapshot accounts to managed runtime scopes. Migration SHALL be lazy or explicit, preserve existing snapshots until activation and refresh are verified, and never delete the only usable credential copy during migration.

#### Scenario: Existing saved Codex account has no managed home
- **WHEN** account-scoped refresh needs that account
- **THEN** backend MAY create a managed runtime scope from the verified snapshot
- **AND** SHALL keep the original snapshot until the managed scope is verified.

#### Scenario: Managed scope creation fails
- **WHEN** backend cannot create or verify the managed runtime scope
- **THEN** the account SHALL remain available for existing snapshot-based activation
- **AND** account-scoped refresh SHALL report a recoverable unavailable state.

### Requirement: System active account SHALL change only by explicit activation
The user's system active home SHALL change only when the user explicitly activates a target through CLI, Menubar, or future frontend operation.

#### Scenario: User opens Menubar
- **WHEN** Menubar opens and refreshes provider state
- **THEN** backend MAY refresh managed account metadata
- **AND** SHALL NOT change the system active account.

#### Scenario: User clicks Switch
- **WHEN** user explicitly switches to a target
- **THEN** backend SHALL perform the provider-specific safe activation flow
- **AND** SHALL return backend-confirmed active target state.

### Requirement: Credential refresh SHALL update the correct account scope
When provider credential refresh rotates access or refresh material, OpenMux SHALL persist the updated credential material back to the account scope that produced it and SHALL not overwrite unrelated account scopes.

#### Scenario: Inactive managed account token refreshes
- **WHEN** quota refresh for an inactive managed account rotates tokens
- **THEN** backend SHALL update that managed account's credential material and fingerprint
- **AND** SHALL NOT update system active credentials unless that account is also the system active target.

#### Scenario: Active system account matches managed account
- **WHEN** refreshed system active credentials match a stored managed account by provider subject
- **THEN** backend SHALL reconcile the matching managed account metadata or fingerprint
- **AND** SHALL avoid downgrading to stale stored credentials.

### Requirement: Activation SHALL preserve OpenMux safety guarantees
Account activation SHALL preserve provider subject validation, snapshot hash verification, private permissions, backup, atomic replacement, concurrency checks, and rollback behavior where supported by the provider.

#### Scenario: Target snapshot hash mismatches
- **WHEN** backend detects that a target account snapshot or managed auth material no longer matches its recorded fingerprint
- **THEN** activation SHALL fail
- **AND** SHALL NOT replace the system active credentials.

#### Scenario: Active auth changes during activation
- **WHEN** system active auth changes after backend prepared activation but before replacement
- **THEN** activation SHALL fail or retry through a safe path
- **AND** SHALL NOT overwrite newer external changes.

### Requirement: Account identity SHALL prefer provider subject over display labels
Managed account reconciliation SHALL use stable provider subject or workspace identity when available. Email, alias, or display label SHALL be presentation metadata and SHALL NOT be the primary automatic merge key when a stronger identity exists.

#### Scenario: Same email has multiple workspaces
- **WHEN** two Codex accounts share the same email but have different provider account IDs or workspace IDs
- **THEN** OpenMux SHALL treat them as distinct managed accounts
- **AND** Menubar SHALL show enough workspace metadata to distinguish them.

#### Scenario: Legacy account has only email identity
- **WHEN** a legacy saved account has no provider subject
- **THEN** OpenMux MAY use normalized email as a fallback identity
- **AND** SHALL upgrade reconciliation to provider subject when later credentials expose one.

### Requirement: Account-scoped refresh SHALL be independent from local usage
Account quota/status refresh SHALL be modeled separately from local token usage parsing. Local usage MAY appear in account or provider views only as local parsed usage, not as provider quota or remaining capacity.

#### Scenario: Local logs show high token usage
- **WHEN** local usage reports many tokens for today
- **THEN** backend SHALL NOT infer that a provider quota is low unless provider quota data confirms it
- **AND** UI SHALL label the metric as local parsed usage.

### Requirement: Managed account lifecycle SHALL be recoverable
OpenMux SHALL support clear lifecycle states for managed accounts: healthy, stale, missing runtime scope, unreadable runtime scope, auth expired, activation failed, and removed.

#### Scenario: Managed home is deleted externally
- **WHEN** a managed account's runtime home no longer exists
- **THEN** control plane SHALL mark the account as missing or unavailable
- **AND** SHALL provide a safe diagnostic and recovery action instead of crashing or hiding the account silently.

#### Scenario: Account reauthentication replaces old managed home
- **WHEN** user reauthenticates an existing managed account
- **THEN** backend SHALL atomically update account metadata to the new verified runtime scope
- **AND** SHALL delete or archive the old runtime scope only after the new scope is committed.
