## ADDED Requirements

### Requirement: CLI refresh SHALL support optional account selector
OpenMux CLI SHALL allow `omx refresh <platform>` to keep refreshing all managed accounts for a provider, and SHALL allow `omx refresh <platform> <selector>` to refresh one account target.

#### Scenario: User refreshes provider
- **WHEN** the user runs `omx refresh codex`
- **THEN** OpenMux SHALL refresh all managed Codex account quota snapshots
- **AND** output the refreshed account summary

#### Scenario: User refreshes one account
- **WHEN** the user runs `omx refresh codex work`
- **THEN** OpenMux SHALL resolve `work` using the same selector rules as `omx use codex work`
- **AND** refresh only the matched account quota snapshot
- **AND** output the refreshed account summary

#### Scenario: Selector matches profile
- **WHEN** the user runs `omx refresh claude api-profile` and the selector matches a profile
- **THEN** OpenMux SHALL return a clear error that refresh only supports account targets
- **AND** OpenMux SHALL NOT attempt to refresh provider profiles

### Requirement: CLI SHALL expose Codex reset credit consumption
OpenMux CLI SHALL provide `omx reset-credit codex <selector>` to consume one Codex reset credit for a resolved account target. The command SHALL require an explicit selector.

#### Scenario: User consumes reset credit interactively
- **WHEN** the user runs `omx reset-credit codex work` in an interactive terminal
- **THEN** OpenMux SHALL resolve `work` to a Codex account
- **AND** ask for confirmation before consuming a reset credit
- **AND** call the provider reset credit operation only after confirmation

#### Scenario: User consumes reset credit non-interactively
- **WHEN** the user runs `omx reset-credit codex work --yes` in a script
- **THEN** OpenMux SHALL consume one reset credit without prompting
- **AND** output the structured outcome as human-readable text

#### Scenario: User omits non-interactive confirmation
- **WHEN** the user runs `omx reset-credit codex work` without a TTY
- **THEN** OpenMux SHALL fail with a message requiring `--yes`
- **AND** OpenMux SHALL NOT consume a reset credit

#### Scenario: Reset credit target is not an account
- **WHEN** the selector matches a profile or no target
- **THEN** OpenMux SHALL return a clear error
- **AND** OpenMux SHALL NOT call the reset credit operation
