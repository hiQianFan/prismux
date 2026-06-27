## ADDED Requirements

### Requirement: Menubar SHALL use layered component architecture
Swift Menubar SHALL organize UI into shell, screen, section, shared component, and primitive layers. Business logic SHALL remain in store/control-plane view models, not in visual components.

#### Scenario: Dashboard renders
- **WHEN** Menubar renders the dashboard
- **THEN** shell components SHALL manage AppKit popover/status item lifecycle
- **AND** screen components SHALL compose sections
- **AND** section/shared/primitive components SHALL receive explicit props and callbacks.

### Requirement: Parent components SHALL own business state
Parent store/screen layers SHALL own dashboard data, selection state, operation state, compatibility state, and last-good stale state. Child components SHALL own only visual-local state such as hover, focus, pressed, inline confirmation, and animation.

#### Scenario: Target row is clicked
- **WHEN** user clicks a target row action
- **THEN** the row SHALL invoke a callback with target identity
- **AND** SHALL NOT mutate active state or infer action eligibility locally.

#### Scenario: Provider tab changes
- **WHEN** user switches provider tabs during an in-flight operation
- **THEN** operation state SHALL remain in the parent store
- **AND** affected rows or global banners SHALL render pending state from parent-owned operation state.

### Requirement: Components SHALL use typed props and callbacks
Reusable components SHALL accept narrow typed props such as `TargetRowProps`, `StatusBannerProps`, `QuotaMeterProps`, and `ProviderSelectorProps`. Components SHALL expose callbacks for user intent and SHALL NOT depend on backend DTOs directly when a view model layer exists.

#### Scenario: TargetRow renders account
- **WHEN** `TargetRow` renders account data
- **THEN** it SHALL receive a target row view model or props object
- **AND** SHALL not know whether the source was Codex, Claude, or future provider except for provider-neutral display fields.

#### Scenario: Diagnostic banner renders
- **WHEN** `StatusBanner` renders a diagnostic
- **THEN** it SHALL receive structured banner props with severity, title/message, and recovery action
- **AND** SHALL not parse raw backend error strings.

### Requirement: Global and feature-local components SHALL be separated
Components reused across providers or pages SHALL live under shared/global component folders. Components that only make sense for one feature SHALL live under that feature and SHALL compose shared primitives instead of redefining styles.

#### Scenario: Provider page needs account row
- **WHEN** ProviderPage renders account/profile targets
- **THEN** it SHALL use the shared `TargetRow` component
- **AND** SHALL not define provider-page-only row styling for the same target concept.

#### Scenario: Overview needs aggregate stats
- **WHEN** Overview renders aggregate stats
- **THEN** it MAY use a feature-local `OverviewStatsGrid`
- **AND** that grid SHALL use shared typography, spacing, badge, and card primitives.

### Requirement: Design tokens SHALL be the only source of visual constants
Spacing, typography, color roles, radius, control sizing, row height, animation duration, and status colors SHALL be centralized in design tokens.

#### Scenario: Two pages render warning status
- **WHEN** Overview and Provider pages both render warning state
- **THEN** they SHALL use the same tokenized warning color and component style
- **AND** SHALL not define ad-hoc orange/yellow values locally.

### Requirement: Page lifecycle SHALL be explicit
Menubar SHALL define lifecycle behavior for popover open, popover close, background refresh, foreground refresh, tab switch, operation success, operation failure, stale snapshot load, and backend compatibility failure.

#### Scenario: Popover closes during switch
- **WHEN** popover closes while a switch operation is in flight
- **THEN** the switch operation SHALL continue in backend/store state
- **AND** reopening the popover SHALL show the latest authoritative operation result or pending state.

#### Scenario: User changes provider tab during refresh
- **WHEN** user changes provider tab while refresh is pending
- **THEN** the selected tab SHALL update immediately
- **AND** refresh pending state SHALL remain visible in provider rows, header, or global status according to parent store state.

### Requirement: Accessibility and motion SHALL be component requirements
Reusable components SHALL define accessibility labels, focus behavior, keyboard affordances where applicable, and reduced-motion behavior.

#### Scenario: Reduce Motion is enabled
- **WHEN** the system Reduce Motion setting is enabled
- **THEN** custom Menubar transitions SHALL degrade to opacity or identity transitions
- **AND** status changes SHALL remain visible through text and icons.
