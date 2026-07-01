## ADDED Requirements

### Requirement: Native Codex Usage Parser
OpenMux SHALL parse Codex local session JSONL directly for production usage scans without requiring tokscale in the default runtime path.

#### Scenario: Parse token count event
- **WHEN** a Codex JSONL source contains an `event_msg` record with `payload.type` equal to `token_count` and valid usage info
- **THEN** OpenMux SHALL emit one `UsageEvent` with timestamp, session, model, token breakdown, source path, source offset, parser schema version, and event hash

#### Scenario: Prefer last token usage
- **WHEN** a Codex `token_count` record contains both `last_token_usage` and `total_token_usage`
- **THEN** OpenMux SHALL use `last_token_usage` as the event token increment unless total-delta correction is required for a documented fork, replay, reset, or total-only fallback case

#### Scenario: Event-level fact unit, not session-level
- **WHEN** a Codex session file contains multiple `token_count` events across multiple turns
- **THEN** OpenMux SHALL emit one `UsageEvent` per turn-level `token_count` event, each carrying its own `occurred_at` timestamp, and SHALL NOT collapse a session into a single aggregated record

#### Scenario: Stale or replayed total snapshot
- **WHEN** a `token_count` event carries a `last_token_usage` increment but its `total_token_usage` does not advance beyond the previous accepted total (out-of-order, replayed, or stale snapshot)
- **THEN** OpenMux SHALL NOT count that `last_token_usage` again, SHALL preserve the prior monotonic total, and SHALL record a safe diagnostic

#### Scenario: Input, cached, and reasoning token split
- **WHEN** a `token_count` usage record reports `input_tokens` that already include `cached_input_tokens`, plus `reasoning_output_tokens`
- **THEN** OpenMux SHALL store `cache_read = min(cached_input_tokens, input_tokens)`, `input = input_tokens - cache_read`, and `reasoning` separately from `output`, without folding reasoning into output

#### Scenario: Do not use quota as token usage
- **WHEN** Codex local logs or remote sources expose rate limit or quota window data
- **THEN** OpenMux SHALL store that data outside `usage_events` and SHALL NOT use it to derive token totals

### Requirement: Source Checkpointed Scanning
OpenMux SHALL maintain per-source parser checkpoints so repeated refreshes parse only changed local usage sources.

#### Scenario: Unchanged source is skipped
- **WHEN** a source path has the same size, mtime, parser schema version, and fingerprint as its stored checkpoint
- **THEN** OpenMux SHALL skip reparsing that source

#### Scenario: Appended source is parsed incrementally
- **WHEN** a source file grew and its stored prefix/fingerprint still matches
- **THEN** OpenMux SHALL resume parsing from `parsed_until_byte` using the stored parser state

#### Scenario: Rewritten source is rebuilt
- **WHEN** a source file shrinks, changes prefix, changes fingerprint incompatibly, or parser schema version changes
- **THEN** OpenMux SHALL rebuild events and affected hourly rollups for that source in one transaction

### Requirement: Event Ledger
OpenMux SHALL store usage facts as event-level records before any product aggregation.

#### Scenario: Idempotent event ingest
- **WHEN** the same source is scanned more than once
- **THEN** OpenMux SHALL keep one `usage_events` row per stable event hash and SHALL NOT double count usage

#### Scenario: Conflicting event hash
- **WHEN** an incoming event has an existing event hash but a different payload
- **THEN** OpenMux SHALL reject the conflicting event, record a safe diagnostic, and preserve existing usage totals

#### Scenario: Account attribution is unknown
- **WHEN** OpenMux cannot prove which account produced a historical usage event
- **THEN** OpenMux SHALL store `account_id` as `NULL` rather than inferring an account

#### Scenario: Session total reconciliation
- **WHEN** all accepted `UsageEvent` increments for a single Codex session are summed
- **THEN** the sum SHALL equal the last monotonic `total_token_usage` accepted for that session, per token bucket, and OpenMux SHALL cover this invariant with a test so stale double-counts and dropped increments are detected

### Requirement: Hourly Rollup Query Layer
OpenMux SHALL maintain hourly rollups for product queries instead of aggregating raw events on every dashboard read.

#### Scenario: Today query
- **WHEN** the product requests today usage
- **THEN** OpenMux SHALL read local-hour rollups for the current local day and return hourly buckets plus totals

#### Scenario: Seven and thirty day query
- **WHEN** the product requests 7d or 30d usage
- **THEN** OpenMux SHALL read hourly rollups for the requested window and MAY fold them into day-level presentation without scanning raw source files

#### Scenario: Rollup transaction
- **WHEN** new events are ingested or a source is rebuilt
- **THEN** OpenMux SHALL update affected hourly rollups in the same transaction as event and checkpoint writes

#### Scenario: Single aggregation source
- **WHEN** the product renders today, 7d, 30d, all, or a headline total
- **THEN** OpenMux SHALL derive every one of them by folding the same hourly rollup buckets, and SHALL NOT maintain a second parallel aggregation path that could disagree with the hourly buckets

#### Scenario: Cross-hour and cross-day session events
- **WHEN** a single session's turn events span more than one local hour or cross a local-day boundary
- **THEN** OpenMux SHALL assign each event to the hourly bucket of its own `occurred_at`, and SHALL NOT place a whole session into one bucket keyed by session start or end

### Requirement: Tokscale Replacement Boundary
OpenMux SHALL treat tokscale as a migration-time comparison tool, not as the target production usage data layer.

#### Scenario: Default runtime path
- **WHEN** `omx usage` or menubar refresh scans Codex usage after this change is applied
- **THEN** OpenMux SHALL use the native Codex parser and SHALL NOT require `omx-usage-tokscale` for Codex

#### Scenario: Regression comparison
- **WHEN** native parser fixtures are tested during migration
- **THEN** OpenMux MAY compare native output with tokscale, CodexBar, or tokenuse-derived expectations at the per-event and per-source level rather than totals alone, since stale-snapshot double counts can make session totals look coincidentally close, and SHALL persist OpenMux-owned `UsageEvent` records

### Requirement: Privacy-Preserving Usage Storage
OpenMux SHALL avoid storing or printing sensitive raw source content while parsing usage.

#### Scenario: Sensitive source content
- **WHEN** a Codex source contains raw prompts, responses, auth payloads, access tokens, refresh tokens, API keys, or complete JSONL lines
- **THEN** OpenMux SHALL NOT persist or print those raw values in SQLite, stdout, stderr, or diagnostics
