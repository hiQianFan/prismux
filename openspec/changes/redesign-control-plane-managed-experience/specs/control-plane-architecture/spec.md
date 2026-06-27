## ADDED Requirements

### Requirement: OpenMux SHALL use a three-layer product architecture
OpenMux SHALL define its long-term architecture as `omx-core`, `omx-control-plane`, and frontends. `omx-core` SHALL own provider domain logic and safety operations, `omx-control-plane` SHALL own product-level application services and view models, and frontends SHALL own interaction and rendering.

#### Scenario: Frontend requests account dashboard
- **WHEN** CLI, Menubar, or a future API needs account status
- **THEN** it SHALL request a control-plane dashboard or provider view
- **AND** SHALL NOT call provider plugin internals directly.

#### Scenario: Core switches active account
- **WHEN** a target activation requires auth replacement or profile patching
- **THEN** the safety-critical operation SHALL execute in `omx-core` or provider plugin code
- **AND** the frontend SHALL only receive an operation result and refreshed view model.

### Requirement: Phase 1 SHALL preserve current user workflows while strengthening foundations
Phase 1 of the control-plane redesign SHALL improve internal boundaries without regressing current user-visible CLI and Menubar workflows. Lower-priority future surfaces SHALL NOT block Phase 1 completion.

#### Scenario: User keeps using existing CLI workflows during Phase 1
- **WHEN** Phase 1 control-plane modules are introduced
- **THEN** existing account list, status, save, activate/switch, and refresh workflows SHALL remain available
- **AND** machine-readable CLI output SHALL remain backward-compatible except for additive fields.

#### Scenario: Menubar opens during Phase 1 migration
- **WHEN** the Swift Menubar consumes the new control-plane/FFI boundary
- **THEN** it SHALL still render current accounts, active state, refresh state, and safe failure fallback
- **AND** it SHALL NOT require managed account migration, independent distribution artifacts, HTTP server, WebKit scraping, or additional provider registry work.

#### Scenario: Future capability is deferred
- **WHEN** a feature belongs to managed account migration, independent distribution, future desktop/widget/http, or private provider source strategies
- **THEN** Phase 1 MAY define compatible DTO fields or extension points
- **AND** SHALL NOT require full implementation before the foundation is accepted.

### Requirement: Control plane SHALL provide frontend-independent view models
`omx-control-plane` SHALL expose presentation-ready but frontend-independent view models for dashboards, provider pages, targets, actions, usage, quota, freshness, diagnostics, and operation results.

#### Scenario: Menubar renders provider row
- **WHEN** Menubar receives a provider target view
- **THEN** it SHALL use backend-provided display label, secondary label, status level, status text, action eligibility, and diagnostics
- **AND** SHALL NOT infer business health from raw auth or provider-specific fields.

#### Scenario: CLI renders same provider
- **WHEN** CLI renders the same provider state
- **THEN** it SHALL use the same control-plane status and diagnostics
- **AND** MAY choose a terminal-specific layout without changing the underlying meaning.

### Requirement: Control plane SHALL own operation contracts
All user-visible operations SHALL return a control-plane operation result that includes status, changed flag when applicable, active before/after when applicable, message, diagnostics, and the authoritative post-operation view when state may have changed.

#### Scenario: Switch succeeds
- **WHEN** user switches to a different account
- **THEN** the control plane SHALL return `operation.status = success`
- **AND** SHALL include backend-confirmed active target after the switch
- **AND** SHALL include the updated provider or dashboard view.

#### Scenario: Refresh is skipped
- **WHEN** refresh is skipped because data is fresh enough or under backoff
- **THEN** the control plane SHALL return `operation.status = skipped`
- **AND** SHALL include a safe reason code and user-facing message.

### Requirement: Control plane SHALL distinguish domain, application, and presentation models
OpenMux SHALL keep provider domain records, application operation models, and frontend presentation models as separate layers. The control plane SHALL map domain state into application and presentation-safe view models without moving provider safety logic into UI code.

#### Scenario: Provider returns raw account state
- **WHEN** a provider plugin returns account/profile/usage records
- **THEN** the control plane SHALL map those records into frontend-safe view models
- **AND** SHALL preserve stable domain identifiers separately from display labels.

#### Scenario: Frontend needs layout
- **WHEN** Menubar needs row layout, colors, or control styling
- **THEN** Swift SHALL choose layout using its design system
- **AND** Rust SHALL only provide semantic status, action, and display fields.

### Requirement: Control plane SHALL model active, selected, observed, and refresh-scope targets separately
The control plane SHALL distinguish the system active target, the frontend selected target, observed account states, and the target scope used for refresh.

#### Scenario: User views inactive account
- **WHEN** user selects an inactive account row in Menubar
- **THEN** the selected UI target MAY change
- **AND** the system active target SHALL remain unchanged until explicit activation.

#### Scenario: Background refresh observes inactive account
- **WHEN** backend refreshes an inactive managed account
- **THEN** the refresh-scope target SHALL be that account
- **AND** the system active target SHALL remain unchanged in the returned dashboard.

### Requirement: Provider expansion SHALL use core and control-plane contracts
New provider support SHALL be added through provider plugin capabilities and control-plane mapping before frontend exposure. Frontends SHALL NOT contain provider-specific credential or safety logic.

#### Scenario: New provider is added
- **WHEN** a future provider such as Gemini is implemented
- **THEN** it SHALL provide account/profile/usage/status data through core plugin contracts
- **AND** the control plane SHALL map that data into the shared provider view model
- **AND** Menubar and CLI SHALL consume that shared view.

#### Scenario: Provider has partial capability
- **WHEN** a provider only supports detection and profile switching
- **THEN** the control plane SHALL expose only those supported actions
- **AND** SHALL mark unsupported account or quota operations unavailable with safe reasons.

### Requirement: Sensitive state SHALL remain behind Rust boundaries
Auth payloads, tokens, provider raw responses, SQLite internals, usage log scanning, and provider endpoint calls SHALL remain in Rust backend layers. Frontends SHALL receive only safe metadata, safe diagnostics, and view model fields.

#### Scenario: Swift Menubar loads dashboard
- **WHEN** Swift Menubar renders account details
- **THEN** it SHALL NOT read auth files, SQLite files, usage logs, browser cookies, or provider endpoints
- **AND** it SHALL only use FFI/control-plane responses.

### Requirement: Control plane SHALL provide persisted safe snapshots for frontend resilience
The control plane SHALL support persisted non-sensitive last-good view snapshots that frontends can display when live backend calls fail, provided the snapshot includes schema version, generated time, stale status, and redaction guarantees.

#### Scenario: Menubar backend call fails on launch
- **WHEN** Menubar cannot obtain a live dashboard but has a compatible safe snapshot
- **THEN** it MAY render that snapshot as stale
- **AND** SHALL clearly show backend unavailable state and retry action.
