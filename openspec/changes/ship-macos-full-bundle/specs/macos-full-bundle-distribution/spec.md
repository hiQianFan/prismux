## ADDED Requirements

### Requirement: macOS full bundle artifact

OpenMux SHALL publish a macOS full bundle artifact that includes Menubar and a same-version `omx` CLI helper.

#### Scenario: Public bundle identity

- **WHEN** a macOS app bundle is prepared for public release
- **THEN** the bundle SHALL be named `OpenMux.app`
- **AND** the Menubar executable SHALL be located at `OpenMux.app/Contents/MacOS/OpenMux`
- **AND** `CFBundleExecutable`, `CFBundleName`, release archive names and documentation SHALL use the same `OpenMux` / `OpenMux.app` identity
- **AND** public release artifacts SHALL NOT use the internal legacy name `OpenMux Menubar.app`.

#### Scenario: Release app archive

- **WHEN** a macOS GitHub Release is prepared
- **THEN** the release SHALL include an `OpenMux.app` archive for supported macOS architectures
- **AND** the app SHALL include the Menubar executable
- **AND** the app SHALL include a bundled `omx` CLI helper.

#### Scenario: Helper location

- **WHEN** `OpenMux.app` is assembled
- **THEN** the bundled CLI helper SHALL be located at `OpenMux.app/Contents/MacOS/omx`
- **AND** it SHALL be executable
- **AND** it SHALL report the same product version as `OpenMux.app`.

#### Scenario: Archive preserves bundle executables

- **WHEN** the release app archive is downloaded and unpacked
- **THEN** `OpenMux.app/Contents/MacOS/OpenMux` SHALL still exist and be executable
- **AND** `OpenMux.app/Contents/MacOS/omx` SHALL still exist and be executable
- **AND** the unpacked helper SHALL pass `omx --version` and isolated `omx status` smoke tests.

### Requirement: Explicit CLI installation

OpenMux SHALL expose CLI PATH installation as an explicit user action, not as a silent first-launch side effect.

#### Scenario: User installs CLI from Menubar

- **WHEN** user clicks `Install CLI`
- **THEN** Menubar SHALL create a symlink from a user-visible PATH location to the bundled helper
- **AND** SHALL NOT copy auth files, state files, snapshots or backups
- **AND** SHALL NOT silently modify shell rc files.

#### Scenario: Existing external CLI is preserved

- **WHEN** `~/.local/bin/omx` already exists and is not a symlink to the bundled helper
- **THEN** Menubar SHALL NOT overwrite it
- **AND** SHALL show guidance for manually replacing or keeping the existing CLI intentionally.

#### Scenario: PATH guidance

- **WHEN** the selected install directory is not on PATH
- **THEN** Menubar SHALL show a copyable PATH command
- **AND** SHALL leave shell configuration changes to the user.

### Requirement: CLI status in Menubar

Menubar SHALL explain whether Terminal usage is ready and whether the installed CLI matches the bundled helper.

#### Scenario: CLI not installed

- **WHEN** PATH lookup cannot find `omx`
- **THEN** Menubar SHALL show `CLI not installed`
- **AND** SHALL offer `Install CLI`.

#### Scenario: CLI version mismatch

- **WHEN** PATH lookup finds `omx` but its version differs from the bundled helper
- **THEN** Menubar SHALL show a version mismatch state
- **AND** SHALL offer guidance to update the symlink or keep the external CLI intentionally.

### Requirement: Menubar onboarding chain

Menubar SHALL provide a direct first-use chain from downloaded app to account management without requiring users to discover CLI commands first.

#### Scenario: First launch with no accounts

- **WHEN** user opens Menubar and no account/profile targets exist
- **THEN** Menubar SHALL show account onboarding actions such as `Sign in`, `Use existing login`, or `Import profile`
- **AND** SHALL keep advanced management available through CLI handoff.

#### Scenario: CLI handoff

- **WHEN** a workflow remains CLI-only
- **THEN** Menubar SHALL provide clear command guidance or copyable commands
- **AND** SHALL NOT execute credential-changing commands without explicit user confirmation.

### Requirement: Cross-platform packaging separation

macOS App bundle layout SHALL NOT constrain future Windows or Linux packaging.

#### Scenario: Future Windows packaging

- **WHEN** OpenMux designs Windows packaging
- **THEN** it SHALL use a separate packaging proposal
- **AND** SHALL share product version, state schema and CLI semantics with macOS
- **AND** SHALL NOT be required to mimic `.app/Contents/MacOS` layout.

### Requirement: Public GitHub readiness

OpenMux SHALL treat public GitHub repository readiness as part of the full bundle release gate.

#### Scenario: Repository documentation is consistent

- **WHEN** maintainer prepares the full bundle release
- **THEN** README, INSTALL, RELEASE, ROADMAP and CHANGELOG SHALL describe the same primary macOS artifact
- **AND** artifact names, helper path, supported platforms, known limitations and install commands SHALL agree.

#### Scenario: Source archive is usable

- **WHEN** user downloads the GitHub source archive for a release tag
- **THEN** the archive SHALL contain the Rust workspace, SwiftPM Menubar source, vendor notes, build scripts, tests and documentation required to build from source
- **AND** source build documentation SHALL distinguish CLI-only `cargo install` from full Menubar bundle builds.

#### Scenario: Open source maintenance files exist

- **WHEN** a contributor opens the public repository
- **THEN** the repository SHALL include README, LICENSE, CONTRIBUTING, SECURITY, CHANGELOG, ROADMAP, issue templates and pull request template
- **AND** these files SHALL describe feedback, contribution, security reporting, local checks and credential-safety expectations.

#### Scenario: Release notes are human-maintained

- **WHEN** GitHub Release is created
- **THEN** release notes SHALL be derived from the matching `CHANGELOG.md` version section
- **AND** SHALL NOT be raw git log output.

#### Scenario: Release workflow matches documented artifact

- **WHEN** release automation runs for a new version
- **THEN** it SHALL build the Rust CLI, build the Swift Menubar with a full Xcode toolchain, assemble `OpenMux.app`, include the bundled helper, package zip artifacts and generate `SHA256SUMS`
- **AND** it SHALL NOT publish CLI-only tarballs as the primary macOS artifact for this release path.

#### Scenario: Public surface audit

- **WHEN** maintainer prepares to publish source
- **THEN** tracked files SHALL be audited for local agent tooling, generated outputs, raw tokens, auth payloads, snapshots, backups and private account files
- **AND** third-party vendored or referenced code SHALL have license/source notes before release.
