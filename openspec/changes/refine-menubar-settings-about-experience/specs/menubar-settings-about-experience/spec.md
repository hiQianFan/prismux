## ADDED Requirements

### Requirement: Menubar SHALL expose Settings as a native single-instance window
OpenMux Menubar SHALL provide a native Settings window instead of limiting configuration to a popover menu. Repeated Settings invocations SHALL focus the existing window rather than creating duplicate windows.

#### Scenario: User opens Settings twice
- **WHEN** user clicks the Settings action twice
- **THEN** OpenMux SHALL show one Settings window
- **AND** the second invocation SHALL focus the existing window.

### Requirement: Settings SHALL separate General, Providers, and About
Settings SHALL initially expose `General`、`Providers` 和 `About` tabs. Additional tabs such as `Display`、`Advanced` 或 `Debug` SHALL only be added when their settings exceed the current tab responsibility.

#### Scenario: User opens Settings
- **WHEN** Settings opens
- **THEN** the window SHALL show General, Providers, and About tabs
- **AND** no empty placeholder tab SHALL be shown.

### Requirement: Shared settings SHALL be owned by the Rust control plane
Refresh cadence, provider enablement, provider source preference, privacy display preference, and future schema version SHALL be served by the Rust control plane through a typed settings contract. Purely visual Menubar preferences MAY remain Swift-local unless another frontend needs the same semantics.

#### Scenario: User changes refresh cadence
- **WHEN** user changes refresh cadence in Menubar Settings
- **THEN** Swift SHALL submit `update_settings` to the backend
- **AND** backend SHALL validate and persist the setting atomically
- **AND** CLI or future frontends SHALL observe the same semantics.

### Requirement: Swift Settings UI SHALL render backend settings DTOs
First-phase provider settings SHALL be represented as fixed frontend-safe DTO fields: provider identity, enabled state, status, source preference options, and diagnostics. Swift SHALL render these DTOs but SHALL NOT read provider private files or implement provider business logic.

#### Scenario: Provider exposes source preference
- **WHEN** backend returns source preference options for a provider
- **THEN** Swift SHALL render a picker from the DTO
- **AND** disabled options SHALL show backend-provided reasons.

#### Scenario: Provider lacks approved remote source
- **WHEN** provider has no approved remote data source
- **THEN** backend SHALL NOT expose `remote_only` as a selectable option
- **AND** Swift SHALL NOT invent or enable that option locally.

### Requirement: Privacy display SHALL use backend-safe labels or coarse hiding
When hide-personal-identifiers is enabled, frontends SHALL either consume backend-provided safe display labels or perform coarse UI hiding/replacement. Swift SHALL NOT parse raw labels with ad hoc sensitive-data heuristics.

#### Scenario: User hides personal identifiers
- **WHEN** user enables privacy display
- **THEN** account/profile labels SHALL be replaced with backend-safe labels or coarse placeholders
- **AND** Swift SHALL NOT inspect auth files or raw provider payloads to determine what to hide.

### Requirement: About SHALL expose version, compatibility, runtime, and support information
About SHALL show app version, build metadata when available, control-plane schema version, state schema version, settings schema version, backend runtime mode, state root display path, project links, copy version info, and copy redacted support report.

#### Scenario: User needs to report a bug
- **WHEN** user opens About
- **THEN** user SHALL be able to copy version information
- **AND** user SHALL be able to copy a redacted support report.

### Requirement: Support report SHALL be redacted before reaching Swift
Support report data SHALL be generated and redacted by Rust before crossing the FFI boundary. It SHALL exclude tokens, raw auth payloads, Cookie headers, Authorization headers, API keys, and raw provider responses.

#### Scenario: Diagnostic includes Authorization header
- **WHEN** backend diagnostic source contains an Authorization header
- **THEN** support report SHALL replace the sensitive content with a redacted marker
- **AND** Swift SHALL only receive the redacted report.

### Requirement: Settings SHALL avoid low-value or premature controls
Settings SHALL NOT include controls merely because CodexBar or another reference app has them. Launch at Login, keyboard shortcuts, CLI helper installation, debug log viewer, provider ordering, destructive account actions, and secret input fields SHALL be excluded from the first Settings phase unless separately approved.

#### Scenario: Product has no implementation decision for Launch at Login
- **WHEN** Settings is rendered
- **THEN** Launch at Login SHALL NOT be shown as an inactive placeholder
- **AND** user SHALL NOT see controls that cannot work.

### Requirement: Settings update failures SHALL rollback visible state
Settings changes SHALL be immediate-save, but failed backend validation or persistence SHALL rollback the visible control to the last saved backend value and show a safe inline error.

#### Scenario: Backend rejects a settings update
- **WHEN** user changes a setting and backend returns an error
- **THEN** Swift SHALL restore the previous saved value
- **AND** SHALL show the backend-provided safe error near the changed control.

### Requirement: Popover SHALL only provide entry points for low-frequency settings
The main popover SHALL keep high-frequency account switching and refresh tasks primary. It MAY expose Settings and About entry points, but SHALL NOT embed the full Settings/About experience inside the popover.

#### Scenario: User clicks gear in popover
- **WHEN** user clicks the gear action
- **THEN** OpenMux SHALL open Settings
- **AND** the popover SHALL NOT expand into a full settings editor.
