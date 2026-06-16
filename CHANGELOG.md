# Changelog

All notable changes to OpenMux will be documented in this file.

The project follows Semantic Versioning. During the `0.x` phase, minor versions
may include breaking changes to the CLI or core plugin API.

## Unreleased

### Added

- Initial Rust workspace with `omx-core`, `omx-plugin-codex`, and `omx-cli`.
- Account-pool CLI command shape: `login`, `save`, `list [platform]`, `current
  [platform]`, `status`, `use`, `alias`, and `doctor [platform]`.
- Project README, contribution guide, and CI skeleton.
- Codex `auth.json` detection, official login wrapping, `--device-auth`,
  numbered account registry, optional aliases, safe account/plan metadata parsing,
  best-effort usage availability, content-hash duplicate detection, account
  listing, snapshot switching, backups, and basic diagnostics.
- Codex gateway/profile import via `omx import codex "<TOML-or-KV>"`, writing
  official Codex profile config files without storing raw API keys.
- Product, architecture, and agent guidance documents.
