# Install OpenMux

[简体中文](INSTALL.zh-CN.md)

## GitHub Releases

v0.1 publishes official macOS binaries only.

1. Download the archive for your Mac from:

   ```text
   https://github.com/hiQianFan/openmux/releases
   ```

2. Unpack it and move `omx` to a directory on your `PATH`, for example:

   ```sh
   mkdir -p "$HOME/.local/bin"
   mv omx "$HOME/.local/bin/omx"
   chmod +x "$HOME/.local/bin/omx"
   ```

3. Verify:

   ```sh
   omx --version
   omx status
   ```

## Cargo from Git

For developers with Rust installed:

```sh
cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked
omx --version
```

## Not Yet Available

- Homebrew is planned after macOS releases stabilize.
- crates.io is planned after crate names and API boundaries stabilize.
- Linux and Windows official binaries are planned after platform validation.

## Uninstall

Remove the installed binary:

```sh
rm -f "$HOME/.local/bin/omx"
```

If installed with Cargo:

```sh
cargo uninstall omx-cli
```

OpenMux state lives under the platform local data directory unless
`OMUX_STATE_ROOT` is set. Remove state only when you are sure you no longer need
saved account snapshots or backups.

