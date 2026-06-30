## 1. Menubar Freshness

- [x] 1.1 Add additive stale/snapshot fields to the FFI response envelope and set them when dashboard fallback serves last-good data.
- [x] 1.2 Decode stale/snapshot fields in Swift and set `MenubarState.ready(..., stale: true)` for fallback dashboards.
- [x] 1.3 Add regression coverage for fresh dashboard vs last-good fallback stale marking.

## 2. Menubar Usage Period

- [x] 2.1 Encode selected `usage_period` in Swift dashboard-producing requests.
- [x] 2.2 Reload dashboard when the Menubar period selector changes.
- [x] 2.3 Add coverage that dashboard request payload includes the selected period.

## 3. CLI Refresh Selector

- [x] 3.1 Extend `omx refresh <platform>` with optional selector while preserving provider-wide refresh behavior.
- [x] 3.2 Resolve selector through the existing target resolver and reject profile targets with a clear error.
- [x] 3.3 Add CLI regression coverage for provider-wide refresh, account refresh, and profile rejection.

## 4. CLI Reset Credit

- [x] 4.1 Add `omx reset-credit codex <selector> [--yes]`.
- [x] 4.2 Require confirmation in interactive terminals and require `--yes` in non-interactive use.
- [x] 4.3 Resolve selector to an account, call `consume_reset_credit`, refresh the account, and print the outcome.
- [x] 4.4 Add CLI regression coverage for confirmed reset, non-interactive missing `--yes`, and profile rejection.

## 5. Verification

- [x] 5.1 Run `cargo fmt --all`.
- [x] 5.2 Run `cargo test`.
- [x] 5.3 Run `cargo clippy --all-targets --all-features`.
