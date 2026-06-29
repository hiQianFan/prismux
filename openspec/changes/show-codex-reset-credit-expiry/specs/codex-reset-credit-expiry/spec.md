## ADDED Requirements

### Requirement: Codex reset credit expiry SHALL be read from detail endpoint
OpenMux SHALL read Codex reset credit expiry metadata from `GET /backend-api/wham/rate-limit-reset-credits` when an account has a known positive reset credit count. OpenMux SHALL continue to use `/backend-api/wham/usage` as the primary quota refresh source.

#### Scenario: Account has reset credits
- **WHEN** `/backend-api/wham/usage` reports `rate_limit_reset_credits.available_count = 2`
- **THEN** OpenMux SHALL request `/backend-api/wham/rate-limit-reset-credits` for that account
- **AND** OpenMux SHALL parse `credits[].expires_at` for available credits.

#### Scenario: Account has no reset credits
- **WHEN** `/backend-api/wham/usage` reports reset credit count `0` or omits reset credit metadata
- **THEN** OpenMux SHALL NOT require a reset-credit detail request before returning the quota snapshot
- **AND** OpenMux SHALL preserve the existing count-only or absent reset-credit behavior.

#### Scenario: Detail endpoint cannot replace usage endpoint
- **WHEN** OpenMux refreshes Codex quota
- **THEN** OpenMux SHALL continue to parse usage windows from `/backend-api/wham/usage`
- **AND** OpenMux SHALL NOT rely on `/backend-api/wham/rate-limit-reset-credits` as the only quota source.

### Requirement: Reset credit expiry SHALL be modeled as structured metadata
OpenMux SHALL expose reset credit expiry as structured per-credit metadata under the account quota reset-credit object. The metadata SHALL be additive and optional.

#### Scenario: Detail response includes two available credits
- **WHEN** reset-credit detail response includes two `credits[]` entries with `status = "available"` and valid RFC3339 `expires_at`
- **THEN** OpenMux SHALL expose two reset-credit expiry timestamps on that account quota
- **AND** the timestamps SHALL be normalized to Unix seconds for backend DTOs.

#### Scenario: Detail response includes unavailable or malformed credits
- **WHEN** a `credits[]` entry is redeemed, unavailable, missing `expires_at`, or has an invalid `expires_at`
- **THEN** OpenMux SHALL NOT show that entry as an available expiry time in Menubar
- **AND** OpenMux SHALL NOT fail the quota refresh solely because of that entry.

#### Scenario: Existing snapshots have only count
- **WHEN** a stored quota snapshot has `reset_credits.available_count` but no per-credit expiry list
- **THEN** OpenMux SHALL decode the snapshot successfully
- **AND** Menubar SHALL treat expiry details as unavailable.

### Requirement: Menubar SHALL show reset credit expiry on account-card hover
Menubar SHALL show available reset credit expiration times when the user hovers over the reset credit metadata on a Codex account card. The hover content SHALL distinguish reset credit expiry from usage-window reset time.

#### Scenario: Two expiry times are available
- **WHEN** an account card has reset credit count `2` and two available reset credit expiry timestamps
- **THEN** hovering the reset credit metadata SHALL show both expiry times
- **AND** the times SHALL be rendered in the user's local timezone.

#### Scenario: More than two expiry times are available
- **WHEN** an account has more than two available reset credit expiry timestamps
- **THEN** Menubar SHALL show at most the first two expiry times ordered by soonest expiration
- **AND** Menubar SHALL keep the account card layout unchanged outside hover.

#### Scenario: Count is available but expiry is unavailable
- **WHEN** an account has a positive reset credit count but no parsed expiry timestamps
- **THEN** hovering the reset credit metadata SHALL still explain the available reset credit count
- **AND** the hover content SHALL state that expiry is unavailable.

#### Scenario: No positive reset credit count is known
- **WHEN** reset credit metadata is absent or count is `0`
- **THEN** Menubar SHALL NOT show reset credit expiry hover content for that account.

### Requirement: Expiry detail failures SHALL degrade safely
OpenMux SHALL treat reset-credit detail failures as non-blocking. A failure to read expiry SHALL NOT prevent quota windows, reset credit count, account switching, or reset consume operations from functioning.

#### Scenario: Detail endpoint returns an HTTP or network error
- **WHEN** `/backend-api/wham/rate-limit-reset-credits` fails due to network, timeout, auth, HTTP, or JSON/schema error
- **THEN** OpenMux SHALL keep the quota refresh result from `/backend-api/wham/usage` when that result is valid
- **AND** Menubar SHALL show reset credit count without expiry details.

#### Scenario: Diagnostics are recorded
- **WHEN** OpenMux records a reset-credit expiry diagnostic
- **THEN** the diagnostic SHALL NOT include access tokens, refresh tokens, raw auth payloads, Cookie headers, Authorization headers, or raw provider response bodies.

#### Scenario: Consume reset credit still works
- **WHEN** reset-credit expiry detail is unavailable
- **THEN** Menubar SHALL preserve the existing reset consume menu behavior defined by `add-codex-reset-credit-controls`
- **AND** OpenMux SHALL NOT block consume solely because expiry metadata is missing.
