## ADDED Requirements

### Requirement: CLI and Menubar SHALL be independently distributable frontends
OpenMux SHALL support a long-term distribution model where `omx` CLI and OpenMux Menubar can be installed, updated, and used independently while sharing compatible state and control-plane semantics.

#### Scenario: User installs only CLI
- **WHEN** user installs only `omx`
- **THEN** account login, import, list, switch, usage, and diagnostics SHALL remain usable from CLI
- **AND** missing Menubar SHALL NOT reduce CLI correctness.

#### Scenario: User installs only Menubar
- **WHEN** user installs only Menubar in a future supported package
- **THEN** Menubar SHALL include or access the required backend/control-plane runtime
- **AND** SHALL clearly guide users for operations that require CLI or bundled helper support.

### Requirement: Shared state SHALL remain compatible across frontends
Independently distributed frontends SHALL use the same state root, schema versions, migration rules, provider registry semantics, and safe diagnostics.

#### Scenario: CLI switches account while Menubar is running
- **WHEN** CLI changes the active account
- **THEN** Menubar SHALL refresh from shared state and show the backend-confirmed active target
- **AND** SHALL NOT maintain a conflicting private active state.

### Requirement: Independent artifacts SHALL enforce compatibility versions
CLI, Menubar, helper binaries, and embedded backends SHALL declare compatible control-plane schema versions, state schema versions, and minimum supported frontend/backend versions.

#### Scenario: Menubar talks to older backend
- **WHEN** Menubar detects that the backend does not support the required control-plane schema
- **THEN** it SHALL show an upgrade-required state
- **AND** SHALL NOT attempt state-changing operations through an incompatible backend.

#### Scenario: Backend sees future state schema
- **WHEN** backend opens a state store with an unsupported future schema
- **THEN** it SHALL fail closed
- **AND** SHALL not rewrite or downgrade the state.

### Requirement: Distribution packaging SHALL not duplicate business logic
Packaging MAY bundle static libraries, helper binaries, or CLI tools, but SHALL NOT create divergent implementations of account switching, usage scanning, provider refresh, or diagnostics.

#### Scenario: Menubar bundle includes backend
- **WHEN** Menubar ships with an embedded Rust backend or helper
- **THEN** that backend SHALL implement the same control-plane contract as CLI
- **AND** SHALL pass the same contract fixtures for dashboard and operations.

### Requirement: Optional modules SHALL degrade gracefully
If a frontend, provider, helper, or optional feature is absent, OpenMux SHALL expose an actionable unavailable state instead of crashing or hiding the missing capability.

#### Scenario: Menubar cannot find CLI helper
- **WHEN** Menubar needs a CLI helper that is not installed
- **THEN** it SHALL show a clear unavailable state and install guidance
- **AND** SHALL keep read-only dashboard features usable when possible.

#### Scenario: Optional provider helper is missing
- **WHEN** a provider-specific helper required for quota refresh is unavailable
- **THEN** OpenMux SHALL mark only that capability unavailable
- **AND** SHALL preserve account switching if switching does not depend on that helper.

### Requirement: Release artifacts SHALL advertise capability scope
Each future release artifact SHALL declare whether it includes CLI, Menubar, backend/control-plane runtime, provider helpers, auto-update support, and supported platforms.

#### Scenario: User downloads Menubar artifact
- **WHEN** user reviews installation docs or release notes
- **THEN** documentation SHALL state whether the artifact includes CLI commands
- **AND** SHALL state which operations require separate installation or setup.
