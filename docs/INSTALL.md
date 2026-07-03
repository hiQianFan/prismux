# Install Prismux

[简体中文](INSTALL.zh-CN.md)

## GitHub Releases

GitHub Releases provide two install paths:

- A desktop app archive for users who want the Prismux UI.
- A standalone CLI package for users who only want `prismux` and `pmx` in
  Terminal.

## Desktop App

1. Download the macOS app archive from:

   ```text
   https://github.com/hiQianFan/prismux/releases
   ```

2. Unpack it and move `Prismux.app` to `/Applications`.

3. Open `Prismux.app` from Finder.

4. In Menubar Settings, use `Enable prismux command` if you want the `prismux` command
   in Terminal.

Prismux does not silently modify your shell startup files. The app includes the
CLI helper at:

```text
/Applications/Prismux.app/Contents/SharedSupport/bin/prismux
```

`Enable prismux command` creates a symlink, normally:

```text
$HOME/.local/bin/prismux -> /Applications/Prismux.app/Contents/SharedSupport/bin/prismux
```

If `$HOME/.local/bin` is not on your `PATH`, add it yourself:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

Verify:

```sh
prismux --version
prismux status
```

### Manual CLI Link

If you prefer not to use the Menubar installer:

```sh
mkdir -p "$HOME/.local/bin"
if [ -L "$HOME/.local/bin/prismux" ] || [ ! -e "$HOME/.local/bin/prismux" ]; then
  ln -sfn "/Applications/Prismux.app/Contents/SharedSupport/bin/prismux" "$HOME/.local/bin/prismux"
else
  echo "$HOME/.local/bin/prismux already exists; remove it manually first" >&2
fi
```

The symlink keeps the Terminal command on the same version as the installed app.

## Standalone CLI Package

Download the CLI package from the same GitHub Release:

```text
prismux-cli-vX.Y.Z-macos-arm64.tar.gz
```

Unpack and install:

```sh
tar -xzf prismux-cli-vX.Y.Z-macos-arm64.tar.gz
cd prismux-cli-vX.Y.Z-macos-arm64
./install.sh
```

By default, `install.sh` copies `prismux` and `pmx` to:

```text
$HOME/.local/bin
```

To install somewhere else:

```sh
PRISMUX_INSTALL_DIR=/usr/local/bin ./install.sh
```

Verify:

```sh
prismux --version
prismux status
```

If `$HOME/.local/bin` is not on your `PATH`, add it yourself:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

## Cargo from Git

For developers with Rust installed:

```sh
cargo install --git https://github.com/hiQianFan/prismux -p prismux-cli --locked
prismux --version
```

## Not Yet Available

- Homebrew is planned after macOS releases stabilize.
- crates.io is planned after crate names and API boundaries stabilize.
- Linux and Windows official binaries are planned after platform validation.
- Sparkle auto-update, Developer ID notarization, and provenance attestations are
  not part of the first public app bundle.

## Uninstall

Remove the CLI symlink:

```sh
rm -f "$HOME/.local/bin/prismux"
```

Then remove `Prismux.app` from `/Applications`.

If installed with Cargo:

```sh
cargo uninstall prismux-cli
```

Prismux state lives under the platform local data directory unless
`PRISMUX_STATE_ROOT` is set. Remove state only when you are sure you no longer need
saved account snapshots or backups.
