## MODIFIED Requirements

### Requirement: macOS full bundle artifact

Prismux SHALL publish a macOS full bundle artifact that includes Menubar and same-version `prismux` and `pmx` CLI helpers.

#### Scenario: Public bundle identity

- **WHEN** a macOS app bundle is prepared for public release
- **THEN** the bundle SHALL be named `Prismux.app`
- **AND** the Menubar executable SHALL be located at `Prismux.app/Contents/MacOS/Prismux`
- **AND** `CFBundleExecutable`, `CFBundleName`, release archive names and documentation SHALL use the same `Prismux` / `Prismux.app` identity
- **AND** public release artifacts SHALL NOT use `OpenMux`, `OpenMux Menubar.app`, or `omx` as the public bundle identity.

#### Scenario: Release app archive

- **WHEN** a macOS GitHub Release is prepared
- **THEN** the release SHALL include a `Prismux.app` archive for supported macOS architectures
- **AND** the app SHALL include the Menubar executable
- **AND** the app SHALL include bundled `prismux` and `pmx` CLI helpers
- **AND** the app SHALL NOT include `omx` as a supported helper.

#### Scenario: Helper location

- **WHEN** `Prismux.app` is assembled
- **THEN** the bundled primary CLI helper SHALL be located at `Prismux.app/Contents/MacOS/prismux`
- **AND** the bundled short CLI helper SHALL be located at `Prismux.app/Contents/MacOS/pmx`
- **AND** both helpers SHALL be executable
- **AND** both helpers SHALL report the same product version as `Prismux.app`.

#### Scenario: Archive preserves bundle executables

- **WHEN** the release app archive is downloaded and unpacked
- **THEN** `Prismux.app/Contents/MacOS/Prismux` SHALL still exist and be executable
- **AND** `Prismux.app/Contents/MacOS/prismux` SHALL still exist and be executable
- **AND** `Prismux.app/Contents/MacOS/pmx` SHALL still exist and be executable
- **AND** the unpacked helpers SHALL pass `prismux --version`, `pmx --version`, and isolated `prismux status` smoke tests.

### Requirement: Explicit CLI installation

Prismux SHALL expose CLI PATH installation as an explicit user action, not as a silent first-launch side effect.

#### Scenario: User installs CLI from Menubar

- **WHEN** user clicks `Install CLI`
- **THEN** Menubar SHALL create symlinks from user-visible PATH locations to the bundled `prismux` and `pmx` helpers
- **AND** SHALL NOT create an `omx` symlink
- **AND** SHALL NOT copy auth files, state files, snapshots or backups
- **AND** SHALL NOT silently modify shell rc files.

#### Scenario: Existing external CLI is preserved

- **WHEN** `~/.local/bin/prismux` or `~/.local/bin/pmx` already exists and is not a symlink to the bundled helper
- **THEN** Menubar SHALL NOT overwrite it
- **AND** SHALL show guidance for manually replacing or keeping the existing CLI intentionally.

#### Scenario: PATH guidance

- **WHEN** the selected install directory is not on PATH
- **THEN** Menubar SHALL show a copyable PATH command
- **AND** SHALL leave shell configuration changes to the user.

### Requirement: CLI status in Menubar

Menubar SHALL explain whether Terminal usage is ready and whether the installed CLIs match the bundled helpers.

#### Scenario: CLI not installed

- **WHEN** PATH lookup cannot find `prismux`
- **THEN** Menubar SHALL show `CLI not installed`
- **AND** SHALL offer `Install CLI`.

#### Scenario: Short CLI not installed

- **WHEN** PATH lookup can find `prismux` but cannot find `pmx`
- **THEN** Menubar SHALL show that the primary CLI is ready and the short command is missing
- **AND** SHALL offer guidance to install or repair the short command.

#### Scenario: CLI version mismatch

- **WHEN** PATH lookup finds `prismux` or `pmx` but its version differs from the bundled helper
- **THEN** Menubar SHALL show a version mismatch state
- **AND** SHALL offer guidance to update the symlink or keep the external CLI intentionally.

### Requirement: Public GitHub readiness

Prismux SHALL treat public GitHub repository readiness as part of the full bundle release gate.

#### Scenario: Repository documentation is consistent

- **WHEN** maintainer prepares the full bundle release
- **THEN** README, INSTALL, RELEASE, ROADMAP and CHANGELOG SHALL describe the same primary macOS artifact
- **AND** artifact names, helper paths, supported platforms, known limitations and install commands SHALL agree.

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
- **THEN** it SHALL build the Rust CLI helpers, build the Swift Menubar with a full Xcode toolchain, assemble `Prismux.app`, include the bundled helpers, package zip artifacts and generate `SHA256SUMS`
- **AND** it SHALL NOT publish CLI-only tarballs as the primary macOS artifact for this release path.

#### Scenario: Public surface audit

- **WHEN** maintainer prepares to publish source
- **THEN** tracked files SHALL be audited for local agent tooling, generated outputs, raw tokens, auth payloads, snapshots, backups and private account files
- **AND** third-party vendored or referenced code SHALL have license/source notes before release.
