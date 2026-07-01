## ADDED Requirements

### Requirement: Public brand identity

The product SHALL use `Prismux` as its public brand name.

#### Scenario: User-facing brand text

- **WHEN** user-facing product text, release notes, README content, installer text, Menubar text, or documentation names the product
- **THEN** it SHALL use `Prismux`
- **AND** it SHALL NOT use `OpenMux` as the current product brand.

#### Scenario: Brand capitalization

- **WHEN** user-facing text renders the brand
- **THEN** it SHALL use `Prismux`
- **AND** it SHALL NOT use `PrismUX` as the default spelling.

#### Scenario: Chinese product description

- **WHEN** Chinese documentation introduces the product
- **THEN** it SHALL use `棱镜` as the Chinese short name
- **AND** it MAY pair the names as `棱镜（Prismux）` or `Prismux 棱镜`
- **AND** it SHALL NOT use `棱枢`, `光枢`, `棱切`, or `棱镜 X` as the default Chinese product name.

### Requirement: CLI entrypoints

The CLI SHALL expose `prismux` as the official command and `pmx` as a short command for the same behavior.

#### Scenario: Official command

- **WHEN** user installs the CLI from source, release artifact, or Menubar helper
- **THEN** `prismux` SHALL be available as the official command
- **AND** documentation SHALL use `prismux` as the primary command in examples.

#### Scenario: Short command

- **WHEN** user installs the CLI from source, release artifact, or Menubar helper
- **THEN** `pmx` SHALL be available as a short command
- **AND** `pmx` SHALL execute the same command tree and business behavior as `prismux`.

#### Scenario: Legacy command removed

- **WHEN** user installs a new Prismux build
- **THEN** the build SHALL NOT install or document `omx`
- **AND** invoking `omx` SHALL NOT be a supported Prismux entrypoint.

### Requirement: Internal names are renamed

Internal crate, module, state and environment names SHALL remove `omx`, `OpenMux`, and `OMUX` naming as part of this development-stage hard rename.

#### Scenario: Internal crate and package names

- **WHEN** implementation updates the public brand and command names
- **THEN** Rust crate/package names and paths SHALL use `prismux-*`
- **AND** they SHALL NOT keep `omx-*` names.
- **AND** they SHALL NOT use `pmx-*` names.

#### Scenario: State and environment prefixes

- **WHEN** implementation updates public brand text and command names
- **THEN** user-configurable environment variables SHALL use `PRISMUX_*`
- **AND** `OMUX_*` SHALL NOT remain as supported compatibility aliases.

#### Scenario: FFI and schema names

- **WHEN** implementation updates public brand text and command names
- **THEN** FFI exported symbol names, schema names, fixture paths and generated artifacts SHALL use `prismux` naming where they currently use `omx`
- **AND** they SHALL NOT keep `omx` naming for compatibility.
