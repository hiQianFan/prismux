# Install OpenMux

[简体中文](INSTALL.zh-CN.md)

## GitHub Releases

The preferred macOS distribution is the full app bundle. It contains both the
Menubar app and the matching `omx` CLI helper.

1. Download the macOS app archive from:

   ```text
   https://github.com/hiQianFan/openmux/releases
   ```

2. Unpack it and move `OpenMux.app` to `/Applications`.

3. Open `OpenMux.app` from Finder.

4. In Menubar Settings, use `Enable omx command` if you want the `omx` command
   in Terminal.

OpenMux does not silently modify your shell startup files. The app includes the
CLI helper at:

```text
/Applications/OpenMux.app/Contents/MacOS/omx
```

`Enable omx command` creates a symlink, normally:

```text
$HOME/.local/bin/omx -> /Applications/OpenMux.app/Contents/MacOS/omx
```

If `$HOME/.local/bin` is not on your `PATH`, add it yourself:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

Verify:

```sh
omx --version
omx status
```

### Manual CLI Link

If you prefer not to use the Menubar installer:

```sh
mkdir -p "$HOME/.local/bin"
if [ -L "$HOME/.local/bin/omx" ] || [ ! -e "$HOME/.local/bin/omx" ]; then
  ln -sfn "/Applications/OpenMux.app/Contents/MacOS/omx" "$HOME/.local/bin/omx"
else
  echo "$HOME/.local/bin/omx already exists; remove it manually first" >&2
fi
```

The symlink keeps the Terminal command on the same version as the installed app.

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
- Sparkle auto-update, Developer ID notarization, and provenance attestations are
  not part of the first public app bundle.

## Uninstall

Remove the CLI symlink:

```sh
rm -f "$HOME/.local/bin/omx"
```

Then remove `OpenMux.app` from `/Applications`.

If installed with Cargo:

```sh
cargo uninstall omx-cli
```

OpenMux state lives under the platform local data directory unless
`OMUX_STATE_ROOT` is set. Remove state only when you are sure you no longer need
saved account snapshots or backups.
