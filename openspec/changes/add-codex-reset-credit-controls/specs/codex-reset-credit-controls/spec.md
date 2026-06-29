## ADDED Requirements

### Requirement: Codex reset credit count SHALL be parsed from usage refresh
OpenMux SHALL parse Codex reset credit metadata from the Codex usage response when `rate_limit_reset_credits.available_count` is present. The parsed count SHALL be associated with the same account quota snapshot as the usage windows.

#### Scenario: Usage response includes reset credits
- **WHEN** Codex usage refresh returns `rate_limit_reset_credits.available_count = 2`
- **THEN** OpenMux SHALL expose reset credit count `2` on that account quota snapshot
- **AND** OpenMux SHALL NOT store the raw provider response, access token, refresh token, or Authorization header in SQLite.

#### Scenario: Usage response omits reset credits
- **WHEN** Codex usage refresh succeeds but omits `rate_limit_reset_credits`
- **THEN** OpenMux SHALL keep reset credit metadata absent
- **AND** Menubar SHALL NOT infer a zero-credit state from the missing field.

### Requirement: Menubar account cards SHALL show reset credit count as metadata
Menubar SHALL show Codex reset credit count as compact account metadata only when a positive count is known, folded into the existing identity subtitle line. The metadata SHALL explain on hover/help that the count is Codex reset credit capacity, not token usage, billing balance, or weekly quota.

#### Scenario: Account has one reset credit
- **WHEN** an account quota snapshot includes reset credit count `1`
- **THEN** the account card SHALL show `1 credit` in the identity subtitle line
- **AND** the hover/help text SHALL explain that each can be consumed once to reset eligible usage limits for that account.

#### Scenario: Account has multiple reset credits
- **WHEN** an account quota snapshot includes reset credit count `3`
- **THEN** the account card SHALL show `3 credits`
- **AND** the text SHALL fit without overlapping the account action cluster.

#### Scenario: Account has no known positive reset credits
- **WHEN** reset credit metadata is absent or the count is `0`
- **THEN** the account card SHALL NOT show a reset credit badge or placeholder.

### Requirement: Reset action SHALL live in the account overflow menu
Menubar SHALL expose `Reset usage limit` as an account overflow menu action, visually grouped with other low-frequency account actions such as Delete. It SHALL NOT be shown as a primary account card button or provider-level overview action. The menu item SHALL always be present and SHALL be disabled (greyed) rather than hidden when reset is not currently possible.

#### Scenario: Account can be reset
- **WHEN** an account has reset credit count greater than `0` AND at least one usage window is limited or exhausted
- **THEN** the account `⋯` menu SHALL show an enabled `Reset usage limit` item
- **AND** the action SHALL target that account only.

#### Scenario: Account has no available reset credit
- **WHEN** an account has reset credit count `0` or unknown
- **THEN** the `⋯` menu SHALL show a disabled `Reset usage limit` item
- **AND** the disabled help text SHALL state that no reset credits are available.

#### Scenario: Account has credit but no active limit
- **WHEN** an account has reset credit count greater than `0` AND no usage window is limited or exhausted
- **THEN** the `⋯` menu SHALL show a disabled `Reset usage limit` item
- **AND** the disabled help text SHALL state that there is no active limit to reset.

### Requirement: Reset action SHALL require confirmation before consume
Menubar SHALL require explicit confirmation before consuming a Codex reset credit. The confirmation SHALL state both the reset scope (eligible usage limits for the selected account) and the cost (one Codex reset credit).

#### Scenario: User chooses reset
- **WHEN** user selects `Reset usage limit` from an account menu
- **THEN** Menubar SHALL show a confirmation popover or dialog stating that eligible usage limits will be reset and one reset credit consumed
- **AND** no backend consume request SHALL be sent until user confirms.

#### Scenario: User cancels reset
- **WHEN** user cancels the confirmation
- **THEN** Menubar SHALL close the confirmation
- **AND** no reset credit SHALL be consumed.

### Requirement: Backend SHALL consume reset credit with idempotency
OpenMux SHALL consume Codex reset credits through the Codex backend consume endpoint (`POST /backend-api/wham/rate-limit-reset-credits/consume`) using a non-empty `redeem_request_id`. The operation SHALL map the response `code` field to a structured outcome and produce safe diagnostics.

#### Scenario: Consume succeeds
- **WHEN** Codex backend returns `code = "reset"`
- **THEN** OpenMux SHALL report reset outcome `reset` and carry the response `windows_reset` count
- **AND** OpenMux SHALL refresh the affected account quota before returning the dashboard when possible.

#### Scenario: Nothing is eligible for reset
- **WHEN** Codex backend returns `code = "nothing_to_reset"`
- **THEN** OpenMux SHALL report outcome `nothing_to_reset`
- **AND** OpenMux SHALL report that no reset credit was consumed
- **AND** Menubar SHALL show a safe message that no active limit is eligible for reset.

#### Scenario: No credit is available
- **WHEN** Codex backend returns `code = "no_credit"`
- **THEN** OpenMux SHALL report outcome `no_credit`
- **AND** Menubar SHALL not claim that usage was reset.

#### Scenario: Same idempotency key was already redeemed
- **WHEN** Codex backend returns `code = "already_redeemed"`
- **THEN** OpenMux SHALL report outcome `already_redeemed`
- **AND** OpenMux SHALL NOT send a second consume request with a new key for the same UI attempt.

#### Scenario: Consume request fails
- **WHEN** the consume request fails due to network, auth, HTTP, timeout, or schema error (no recognized `code`)
- **THEN** OpenMux SHALL return a failed operation result with a safe diagnostic and no business outcome
- **AND** the diagnostic SHALL NOT include tokens, raw auth payloads, Cookie headers, Authorization headers, or raw provider response bodies.

### Requirement: Reset SHALL not imply forced weekly usage clearing
OpenMux SHALL only expose service-granted reset credit consumption. It SHALL NOT present or implement a generic force-reset action for weekly usage, local token usage, billing usage, or provider-side counters.

#### Scenario: User has no reset credit
- **WHEN** Codex reports weekly usage is exhausted but no reset credit is available
- **THEN** Menubar SHALL NOT offer a force reset action
- **AND** OpenMux SHALL continue to show the provider-reported reset time or unknown status.
