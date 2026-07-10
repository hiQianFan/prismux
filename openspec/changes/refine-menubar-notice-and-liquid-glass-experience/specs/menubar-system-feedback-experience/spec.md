## ADDED Requirements

### Requirement: Opening Menubar does not force foreground refresh

Menubar SHALL treat opening the popover as a view action, not as an explicit refresh request.

#### Scenario: Popover opens with last-good data

- **WHEN** the user opens Menubar and last-good dashboard data is available
- **THEN** Menubar MUST show the last-good dashboard immediately
- **AND** the header Refresh button MUST NOT enter foreground loading solely because the popover opened

#### Scenario: User explicitly refreshes

- **WHEN** the user clicks the header Refresh button
- **THEN** Menubar MUST run an interactive refresh
- **AND** the header Refresh button MAY show loading until that explicit refresh completes

#### Scenario: Popover opens without usable data

- **WHEN** the user opens Menubar and no dashboard or last-good data is available
- **THEN** Menubar MUST load data for the first screen
- **AND** Menubar MUST keep the loading state scoped to the initial load, not present it as a repeated user-triggered refresh

### Requirement: Menubar shows no operation-result banner

Menubar SHALL NOT render a top-of-content banner for any operation result — success, no-change/skip, or failure. Operation completion and failure are communicated through row state, header freshness, and target-scoped diagnostics on account cards, not a persistent colored banner.

#### Scenario: Fresh-enough refresh is skipped

- **WHEN** an interactive refresh returns `operation.status = "skipped"` (including `skipped_reason = "fresh_enough"`)
- **THEN** Menubar MUST keep the dashboard visible without showing any "Operation skipped" banner
- **AND** Menubar MUST preserve or update the header freshness text

#### Scenario: Stale request is skipped

- **WHEN** an older refresh response returns after a newer dashboard generation is active
- **THEN** Menubar MUST ignore the stale response and show no user-facing banner

#### Scenario: Routine success

- **WHEN** refresh, switch, or import succeeds
- **THEN** Menubar MUST NOT show a persistent success banner
- **AND** Menubar MAY communicate completion through refreshed timestamps and row state

#### Scenario: Operation failure is silent at the top level

- **WHEN** switch, delete, reset, import, sign-in, or refresh returns a failed operation
- **THEN** Menubar MUST NOT show a top-of-content error banner
- **AND** Menubar MUST keep the prior active account/profile state unchanged until the backend confirms a change

### Requirement: Target-scoped diagnostics appear on the owning account or profile card

Menubar SHALL surface a diagnostic that carries a `target_id` on the account or profile card it belongs to, rather than in a shared banner or a separate diagnostics section.

#### Scenario: Account has a target-scoped diagnostic

- **WHEN** an account's `diagnostic` is present (for example an auth failure or a `refresh_failed` scoped to that target)
- **THEN** Menubar MUST attach the diagnostic to that account's card
- **AND** Menubar MUST NOT render it as a top-of-content banner or in a separate diagnostics card

#### Scenario: Recoverable target refresh failure

- **WHEN** a refresh of a specific account fails but last-good data is still available
- **THEN** Menubar MUST keep showing the last-good data for that account
- **AND** the account card MUST reflect the failure through its diagnostic presentation

### Requirement: Target diagnostic shows as a single compact line on the card

Menubar SHALL keep the account/profile card's existing compact layout unchanged and, when a target has a diagnostic, add a single always-visible diagnostic line rather than an expand/collapse disclosure or extra action buttons.

#### Scenario: Card layout is unchanged when there is no diagnostic

- **WHEN** an account has no diagnostic
- **THEN** the card MUST render exactly as before (identity line and 5h/7d quota bars) with no added affordance
- **AND** Menubar MUST NOT add an expand/collapse control to the card

#### Scenario: Diagnostic present

- **WHEN** an account or profile has a `diagnostic`
- **THEN** the card MUST show a single compact diagnostic line below its existing content, consisting of a severity glyph and the human-readable message
- **AND** the line MUST NOT show the raw snake_case `code` as text
- **AND** the line MUST NOT repeat the account's usage/quota data
- **AND** the line MUST NOT include action buttons; actions remain in the row's overflow menu and the Accounts card

#### Scenario: Recovery guidance and raw detail

- **WHEN** a diagnostic has a recovery action or a technical code
- **THEN** Menubar MAY expose the recovery action and raw code through a tooltip or the support report
- **AND** Menubar MUST NOT expose tokens, raw auth payloads, private account files, or secret endpoints

#### Scenario: Heavier per-account detail is deferred

- **WHEN** richer per-account detail is considered (usage history, per-account configuration, logs)
- **THEN** Menubar MUST NOT introduce an in-popover expand/collapse or detail page for it
- **AND** such detail MUST be deferred to the Settings window when it is built

### Requirement: Diagnostic severity uses both color and shape

Menubar SHALL communicate diagnostic severity with a combination of color and glyph shape, applying color sparingly, so severity is distinguishable without relying on color alone.

#### Scenario: Recoverable severity

- **WHEN** a diagnostic is recoverable (for example `refresh_failed`, `network`, `timeout`, `managed_runtime_unavailable`, or an unknown code)
- **THEN** Menubar MUST use the warning tint with the triangle glyph (`exclamationmark.triangle.fill`)

#### Scenario: Action-required severity

- **WHEN** a diagnostic indicates broken authentication (for example `managed_runtime_auth` or `auth`)
- **THEN** Menubar MUST use the failed tint with a distinct glyph shape (`exclamationmark.octagon.fill`)

#### Scenario: Restrained color treatment

- **WHEN** a diagnostic is shown on a card
- **THEN** only the small glyph MAY carry the severity color and the message text MUST stay neutral/secondary
- **AND** Menubar MUST NOT wrap the diagnostic in a filled colored background block
- **AND** Menubar MUST use system colors so they adapt to light, dark, and Increase Contrast

### Requirement: Menubar main view does not show diagnostics sections

Menubar SHALL NOT show standalone diagnostics sections or cards in the main popover pages.

#### Scenario: Overview has diagnostics

- **WHEN** dashboard data contains aggregate or provider diagnostics
- **THEN** the Overview page MUST NOT show a `Needs attention` diagnostics card
- **AND** the Overview page MUST keep its focus on provider/account status and quota summary

#### Scenario: Provider page has diagnostics

- **WHEN** a provider has provider-scoped or dashboard-scoped diagnostics without a `target_id`
- **THEN** the Provider page MUST NOT show a standalone `Diagnostics` card
- **AND** those diagnostics MUST remain available through redacted support output, Settings/About support actions, or CLI/doctor

#### Scenario: Provider page has no diagnostics

- **WHEN** a provider has no diagnostics
- **THEN** the Provider page MUST NOT show an empty diagnostics state such as "No diagnostics"

### Requirement: Menu bar extra stays lightweight

Menubar SHALL keep the popover to a few related tasks and MUST NOT introduce a secondary navigation/detail page inside the transient popover; heavier per-account features belong in the Settings window.

#### Scenario: Heavier per-account feature is requested later

- **WHEN** a richer per-account feature (such as usage history, per-account configuration, or logs) is added
- **THEN** it MUST be presented in the Settings window rather than as a drill-down page inside the transient popover

#### Scenario: No popover-over-popover hierarchy

- **WHEN** a target diagnostic is shown
- **THEN** Menubar MUST surface it as a compact line on the card, not by stacking another popover or navigation layer over the popover

### Requirement: Feedback copy is user-facing

Menubar SHALL translate internal operation reasons into user-facing copy before displaying them in the UI.

#### Scenario: Internal reason not shown raw

- **WHEN** the backend returns a snake_case reason or code such as `fresh_enough`, `error_backoff`, or `managed_runtime_auth`
- **THEN** Menubar MUST NOT display the raw snake_case string as primary user-facing text

#### Scenario: Technical detail remains available

- **WHEN** a technical reason is useful for support or debugging
- **THEN** Menubar MAY include the raw reason in a tooltip, redacted support output, or CLI/doctor output
- **AND** Menubar MUST NOT expose tokens, raw auth payloads, private account files, or secret endpoints

### Requirement: Menubar visual system follows native material first

Menubar SHALL follow Apple HIG by using native macOS materials, controls, and accessibility settings before custom visual effects when adapting to macOS 26/27 Liquid Glass.

#### Scenario: Running on macOS with Liquid Glass

- **WHEN** Prismux runs on a macOS version whose system UI uses Liquid Glass
- **THEN** Settings and Menubar chrome MUST prefer native SwiftUI/AppKit materials and controls
- **AND** Prismux MUST NOT add a custom glass shader or new rendering dependency only to imitate Liquid Glass

#### Scenario: Accessibility contrast settings are enabled

- **WHEN** Reduce Transparency, Increase Contrast, or Reduce Motion is enabled
- **THEN** Menubar MUST keep text readable and controls usable
- **AND** custom motion or translucency MUST reduce to a simpler system appearance

### Requirement: Design documentation stays aligned with implementation

The project documentation SHALL describe the current Menubar feedback policy, diagnostics placement, refresh-on-open policy, Liquid Glass strategy, and footer behavior.

#### Scenario: Design docs updated

- **WHEN** this change is implemented
- **THEN** `DESIGN.md` MUST no longer claim that operation results or harmless refresh skips appear as banners
- **AND** `DESIGN.md` MUST document that target-scoped diagnostics show as a single compact line on account cards (no expand/collapse) and that provider/dashboard diagnostics are not shown in the popover
- **AND** `DESIGN.md` MUST document the native-material-first Liquid Glass strategy and that opening Menubar does not always trigger foreground refresh

#### Scenario: UX checklist updated

- **WHEN** this change is implemented
- **THEN** the Menubar UX checklist MUST include manual checks for: opening the popover without repeated refresh-button loading, no operation banner on success/skip/failure, and a target diagnostic appearing as a single compact line on its account card (no expand/collapse, no duplicated usage, no action buttons)
