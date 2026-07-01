# Roadmap

[简体中文](ROADMAP.zh-CN.md)

## v0.1: macOS Public Release

- Codex account login, save, list, alias, switch, and profile import.
- Claude Code profile import/switch and OAuth account snapshot import/switch.
- macOS `Prismux.app` full bundle releases for Apple Silicon, with a
  bundled same-version `prismux` helper.
- `cargo install --git` developer install path.
- Bilingual README and user docs.
- Repository cleanup, CI, issue/PR templates, and security policy.

## v0.1 Hardening

- Improve diagnostics and recovery guidance.
- Add more real-world smoke-test coverage.
- Harden release automation based on first public release feedback.
- Evaluate optional dependency/license checks such as `cargo deny`.

## v0.2: Linux Validation

- Validate Codex and Claude behavior on Linux.
- Verify credential file permissions and external CLI behavior.
- Add official Linux binary releases when stable.

## v0.3: Windows Validation

- Validate path discovery, file replacement behavior, process lookup, and
  credential storage behavior on Windows.
- Decide whether Windows needs additional ACL/private-permission handling.
- Add official Windows binary releases when stable.

## Later

- Homebrew tap after macOS releases stabilize.
- crates.io distribution after crate names and public API boundaries are stable.
- Standalone CLI tarballs if there is real demand beyond the app-bundled helper.
- Gemini CLI plugin.
- Artifact signing or provenance beyond v0.1 checksums.
- More provider/profile import formats.

## Non-Goals

- Prismux is not an API gateway, model router, or provider marketplace.
- v0.1 does not include a daemon, watcher, dynamic plugin loading, Sparkle
  auto-update, or Developer ID notarization.
- Prismux does not call private provider APIs to enrich account metadata.
