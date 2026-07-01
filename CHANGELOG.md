# Changelog

All notable changes to Prismux will be documented in this file.

The project follows Semantic Versioning. During the `0.x` phase, minor versions
may include breaking changes to the CLI or internal plugin API.

## Unreleased

### Added

- GitHub launch readiness plan, including repository cleanup, bilingual user
  documentation, release automation, and macOS v0.1 distribution policy.

## v0.1.0 - 2026-06-30

### Added

- Initial Rust workspace with `prismux-core`, `prismux-plugin-codex`,
  `prismux-plugin-claude`, and `prismux-cli`.
- Account-pool CLI commands: `login`, `save`, `list [platform]`, `current
  [platform]`, `status`, `use`, `alias`, `import`, and `doctor [platform]`.
- Codex official login wrapping, `--device-auth`, numbered account registry,
  optional aliases, account/plan metadata parsing, best-effort usage
  availability, duplicate auth detection, account listing, snapshot switching,
  backups, and diagnostics.
- Codex gateway/profile import from TOML or OpenAI-compatible KEY=VALUE input,
  writing Codex profile config files without storing raw API keys in registry
  metadata.
- Claude Code gateway/API profile import and switch support for Anthropic
  compatible endpoints, API keys, bearer tokens, Bedrock, Vertex, and Foundry
  related environment keys.
- Claude OAuth account snapshot import and switch support using official Claude
  Code credential artifacts; macOS uses Keychain, non-macOS or explicit
  `CLAUDE_CONFIG_DIR` uses plaintext `.credentials.json`.
- Unified Claude account/profile selector resolution with ambiguity detection.
- Native macOS Menubar app for dashboard, refresh, explicit account/profile
  activation, onboarding actions, Settings, About, and support report copying.
- macOS full bundle release path: `Prismux.app` archives for Apple Silicon,
  with bundled same-version `prismux` helper at
  `Prismux.app/Contents/MacOS/prismux`.
- Explicit `Enable prismux command` setup that creates a user-controlled symlink to
  the bundled helper without copying auth/state or modifying shell startup
  files.
- Product, architecture, OpenSpec, contribution, security, roadmap, install,
  source-build, and release documentation.
- GitHub Release automation for official v0.1 `Prismux.app` archives and
  `SHA256SUMS`.

### Security

- Registry files store safe metadata and hashes, not raw auth payloads.
- Active credentials are backed up before replacement.
- Snapshot hashes are verified before switching.
- Snapshot, backup, and registry writes use private permissions where the
  platform supports them.
- Future registry schema versions are rejected instead of modified.

### Known Limitations

- v0.1 official downloads target macOS only.
- Linux and Windows official binaries are planned after platform validation.
- Homebrew and crates.io distribution are planned but not available in v0.1.
- Sparkle auto-update, Developer ID notarization, and standalone CLI tarballs
  are not part of v0.1.
- GitHub Release artifacts include SHA-256 checksums, but not independent
  signing or provenance in v0.1.
