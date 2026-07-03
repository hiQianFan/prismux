# Changelog

All notable changes to Prismux will be documented in this file.

The project follows Semantic Versioning. During the `0.x` phase, minor versions
may include breaking changes to the CLI or internal plugin API.

## Unreleased

### Added

- GitHub launch readiness plan, including repository cleanup, bilingual user
  documentation, release automation, and macOS v0.1 distribution policy.

## v0.2.2 - 2026-07-03

### Added

- GitHub Releases now include a standalone macOS arm64 CLI tarball with
  `prismux`, `pmx`, `install.sh`, and package documentation.
- Prismux Settings now includes a Network proxy control that stores refresh proxy
  settings in the local control-plane settings file.
- README and CLI documentation now include clearer install, command, and
  screenshot-driven onboarding paths.

### Changed

- Menubar Settings now uses a sidebar-style layout with grouped panes and
  streamlined provider, command-line tool, and About sections.
- Menubar refreshes can run concurrently with long-running operations; stale
  read responses are filtered client-side while writes always apply their own
  responses.

### Fixed

- Removed quota and refresh-attempt writes for accounts that were already
  deleted, avoiding stale rows after account removal.
- Codex usage refresh now reads proxy configuration from Prismux Settings instead
  of process environment variables.
- macOS bundle packaging now strips extended attributes before codesigning,
  avoiding local resource-fork signing failures.

## v0.2.1 - 2026-07-02

### Fixed

- Menubar no longer crashes on launch (SIGTRAP) when rendering provider icons in
  downloaded builds. `ProviderIcon` previously loaded assets through the
  SwiftPM-generated `Bundle.module`, whose accessor calls `fatalError()` when it
  cannot locate the resource bundle — which happens for the flat, `Info.plist`-less
  resource bundle produced by the CI `swift build`, especially under App
  Translocation. Assets are now resolved by direct filesystem lookup that handles
  both flat and nested bundle layouts and degrades to a blank glyph instead of
  crashing.


## v0.2.0 - 2026-07-02

### Changed

- macOS app bundles now install bundled CLI helpers under
  `Prismux.app/Contents/SharedSupport/bin/`, keeping user-facing commands out of
  `Contents/MacOS`.
- The macOS Menubar status item now uses an icon-only square item instead of a
  variable-width quota summary title.

### Fixed

- Bundle creation now guards against case-insensitive filename collisions between
  `Prismux` and `prismux`, which could otherwise clobber the Menubar executable.
- Release and bundle audit scripts now validate the SharedSupport helper path.

## v0.1.1 - 2026-07-02

### Fixed

- macOS app bundle now installs the Menubar executable as
  `Prismux.app/Contents/MacOS/Prismux`, matching the public bundle identity used
  by release workflow checks and documentation.
- Release workflow now audits the app bundle before and after zipping so
  executable, helper, icon, and resource drift fail before publishing.

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
