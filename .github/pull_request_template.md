## Summary

## Tests

- [ ] `cargo fmt --all`
- [ ] `cargo test --locked`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `scripts/build-menubar.sh` / `scripts/bundle-menubar.sh` when Menubar or release packaging changes
- [ ] Docs/CHANGELOG updated where needed

## Safety

- [ ] This PR does not include tokens, auth payloads, snapshots, backups, or private account files.
- [ ] Changes touching credential replacement describe backup/rollback behavior.
- [ ] Manual tests used isolated `OMUX_STATE_ROOT`, `CODEX_HOME`, or `CLAUDE_CONFIG_DIR` where appropriate.
