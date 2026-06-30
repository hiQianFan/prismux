# Build from Source

OpenMux can be built from a Git clone or GitHub's generated source archive.

## Requirements

- macOS 14 or newer for the Menubar app.
- Full Xcode for SwiftUI macros and app bundling. Command Line Tools alone are
  not enough.
- Stable Rust toolchain with `rustfmt` and `clippy`.

```sh
rustup default stable
rustup component add rustfmt clippy
```

## CLI Only

```sh
cargo build --release -p omx-cli --locked
./target/release/omx --version
```

Developer install from Git builds only the CLI:

```sh
cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked
```

## Menubar App

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

The bundle script writes:

```text
target/menubar/OpenMux.app
```

The app contains:

```text
OpenMux.app/Contents/MacOS/OpenMux
OpenMux.app/Contents/MacOS/omx
```

## Local Checks

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
```

Use isolated state when running manual checks:

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home ./target/release/omx status
```
