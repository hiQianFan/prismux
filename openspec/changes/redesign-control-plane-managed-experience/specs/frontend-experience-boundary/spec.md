## ADDED Requirements

### Requirement: Menubar SHALL be an independent native frontend
OpenMux Menubar SHALL be maintained as an independent native frontend with its own state machine, component structure, design tokens, and interaction behavior, while consuming only control-plane data for business state.

#### Scenario: Menubar renders dashboard
- **WHEN** Swift receives a dashboard view model
- **THEN** it SHALL render through reusable components for header, provider selector, overview, provider detail, target row, usage summary, status banner, and footer
- **AND** SHALL NOT place all layout, styling, and business conditions in one monolithic view.

### Requirement: Menubar SHALL cover primary user journeys explicitly
Menubar SHALL define and test primary user journeys: first launch, no accounts, account list review, explicit switch, refresh failure, missing provider CLI, missing permissions, expired account, stale background data, and CLI handoff.

#### Scenario: First launch has no accounts
- **WHEN** user opens Menubar with no managed accounts
- **THEN** Menubar SHALL show a focused empty state
- **AND** SHALL provide the safest next action, such as CLI login guidance or supported in-app login when implemented.

#### Scenario: Provider CLI is missing
- **WHEN** a provider requires a local CLI that is not installed
- **THEN** Menubar SHALL show provider-unavailable state with installation guidance
- **AND** SHALL keep other providers usable.

### Requirement: Frontends SHALL not reinterpret business facts
Frontends SHALL NOT independently derive account health, provider availability, target action eligibility, quota risk, or safety diagnostics from raw lower-level fields when control-plane fields exist.

#### Scenario: Target cannot switch
- **WHEN** control plane marks `can_switch = false` with a disabled reason
- **THEN** Menubar and CLI SHALL render that disabled state
- **AND** SHALL NOT override it based on local UI assumptions.

### Requirement: Menubar SHALL use explicit operation states
Menubar SHALL model loading, ready, stale, failed with last-good, refreshing, switching, deleting, and backend-unavailable states explicitly. It SHALL keep previous authoritative state visible when a mutation fails.

#### Scenario: Switch fails
- **WHEN** backend returns a failed switch operation
- **THEN** Menubar SHALL keep the previous active target marker
- **AND** SHALL show a safe inline or banner error
- **AND** SHALL NOT optimistically mark the requested target active.

#### Scenario: Background refresh fails
- **WHEN** background refresh fails but a last-good dashboard exists
- **THEN** Menubar SHALL continue showing the last-good dashboard as stale
- **AND** SHALL expose the refresh failure as a safe status message.

#### Scenario: Older response returns after newer response
- **WHEN** Menubar receives a response with an older generation or request identity than the current state
- **THEN** it SHALL discard that response
- **AND** SHALL keep the newer authoritative state.

### Requirement: UI SHALL be consistent through shared components and tokens
Menubar UI SHALL use shared components and design tokens for spacing, typography, colors, status levels, buttons, rows, quota indicators, empty states, and banners.

#### Scenario: Two provider pages show accounts
- **WHEN** Codex and Claude provider pages both show account rows
- **THEN** row height, left/right layout, status colors, action placement, and disabled states SHALL be consistent
- **AND** provider-specific branding SHALL be limited to safe identity/icon accents.

### Requirement: CLI SHALL remain a first-class frontend
CLI SHALL consume the same control-plane facts as Menubar while preserving scriptable commands, stable JSON output, and terminal-specific human rendering.

#### Scenario: CLI lists accounts
- **WHEN** user runs `omx list`
- **THEN** CLI SHALL display the same active target, status, and safe diagnostics as Menubar for the same state root
- **AND** JSON output SHALL remain stable and machine-readable.

### Requirement: Frontend settings SHALL use shared config semantics
Frontend-specific UI preferences MAY remain frontend-local, but provider enablement, source mode, refresh cadence, account selection, debug/recovery switches, and feature flags SHALL use shared config semantics.

#### Scenario: User disables a provider
- **WHEN** user disables a provider in Menubar settings
- **THEN** CLI and future frontends SHALL observe the provider as disabled through shared config
- **AND** Menubar-only layout preferences SHALL NOT leak into provider business config.

### Requirement: Frontend actions SHALL map to explicit backend operations
Every user action that changes state SHALL call an explicit backend operation and SHALL surface pending, success, skipped, or failure feedback.

#### Scenario: User refreshes provider
- **WHEN** user clicks provider refresh in Menubar
- **THEN** Menubar SHALL show provider-scoped pending state
- **AND** SHALL update from backend operation result
- **AND** SHALL show skipped or failed states distinctly from success.
