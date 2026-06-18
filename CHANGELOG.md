# Changelog

All notable changes to OpenMux will be documented in this file.

The project follows Semantic Versioning. During the `0.x` phase, minor versions
may include breaking changes to the CLI or internal plugin API.

## Unreleased

### Added

- GitHub launch readiness plan, including repository cleanup, bilingual user
  documentation, release automation, and macOS v0.1 distribution policy.

## v0.1.0 - 2026-06-17

### Added

- Initial Rust workspace with `omx-core`, `omx-plugin-codex`,
  `omx-plugin-claude`, and `omx-cli`.
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
- Product, architecture, OpenSpec, contribution, security, roadmap, install, and
  release documentation.
- macOS GitHub Release automation for official v0.1 binaries.

### Security

- Registry files store safe metadata and hashes, not raw auth payloads.
- Active credentials are backed up before replacement.
- Snapshot hashes are verified before switching.
- Snapshot, backup, and registry writes use private permissions where the
  platform supports them.
- Future registry schema versions are rejected instead of modified.

### Known Limitations

- v0.1 official binaries target macOS only.
- Linux and Windows official binaries are planned after platform validation.
- Homebrew and crates.io distribution are planned but not available in v0.1.
- GitHub Release artifacts include SHA-256 checksums, but not independent
  signing or provenance in v0.1.
