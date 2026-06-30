## ADDED Requirements

### Requirement: Menubar fallback dashboard SHALL be marked stale
When the backend serves a last-good dashboard snapshot because a fresh dashboard query failed, the Menubar response SHALL include an explicit stale/snapshot signal. Menubar SHALL render the returned dashboard as stale and SHALL NOT label it as freshly updated.

#### Scenario: Backend serves last-good snapshot
- **WHEN** the dashboard operation cannot produce fresh data and a last-good snapshot exists
- **THEN** the FFI response SHALL return the dashboard data with `data_stale = true` or `served_from_snapshot = true`
- **AND** Menubar SHALL set its ready state stale flag to true
- **AND** the header and tray title SHALL indicate stale data

#### Scenario: Backend serves fresh dashboard
- **WHEN** the dashboard operation succeeds with fresh data
- **THEN** the FFI response SHALL NOT mark the dashboard as stale
- **AND** Menubar SHALL render the ready state as fresh

### Requirement: Menubar usage period SHALL be part of dashboard queries
Menubar SHALL send the selected usage period to dashboard-producing backend requests so Overview headline, provider headline, and usage chart use the same period.

#### Scenario: User selects Today
- **WHEN** the user selects the Today period in Menubar
- **THEN** the next dashboard query SHALL include `usage_period = Today`
- **AND** the returned usage headline and chart data SHALL use the Today window

#### Scenario: User changes period
- **WHEN** the user changes Menubar usage period from `7d` to `30d`
- **THEN** Menubar SHALL reload dashboard data with `usage_period = ThirtyDays`
- **AND** the displayed usage headline SHALL NOT continue using the previous period
