# 安装 OpenMux

[English](INSTALL.md)

## GitHub Releases

macOS 首选分发形态是 full app bundle。它同时包含 Menubar App 和同版本 `omx` CLI helper。

1. 从 Releases 下载 macOS app archive：

   ```text
   https://github.com/hiQianFan/openmux/releases
   ```

2. 解压后把 `OpenMux.app` 拖到 `/Applications`。

3. 从 Finder 打开 `OpenMux.app`。

4. 如果希望在 Terminal 中使用 `omx` 命令，在 Menubar Settings 中点击 `Enable omx command`。

OpenMux 不会静默修改 shell 启动文件。App 内置 CLI helper 路径是：

```text
/Applications/OpenMux.app/Contents/MacOS/omx
```

`Enable omx command` 会创建 symlink，默认形态是：

```text
$HOME/.local/bin/omx -> /Applications/OpenMux.app/Contents/MacOS/omx
```

如果 `$HOME/.local/bin` 不在 `PATH` 中，请自行加入：

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

验证：

```sh
omx --version
omx status
```

### 手动安装 CLI 链接

如果不使用 Menubar 的安装按钮：

```sh
mkdir -p "$HOME/.local/bin"
if [ -L "$HOME/.local/bin/omx" ] || [ ! -e "$HOME/.local/bin/omx" ]; then
  ln -sfn "/Applications/OpenMux.app/Contents/MacOS/omx" "$HOME/.local/bin/omx"
else
  echo "$HOME/.local/bin/omx already exists; remove it manually first" >&2
fi
```

symlink 会让 Terminal 中的 `omx` 跟随已安装 App 的版本。

## 从 Git 使用 Cargo 安装

如果本机已有 Rust：

```sh
cargo install --git https://github.com/hiQianFan/openmux -p omx-cli --locked
omx --version
```

## 暂未提供

- Homebrew 会在 macOS release 稳定后加入。
- crates.io 会在 crate 命名和 API 边界稳定后考虑。
- Linux 和 Windows official binaries 会在平台验证后加入。
- Sparkle 自动更新、Developer ID notarization 和 provenance attestations 不属于第一版公开 App bundle。

## 卸载

删除 CLI symlink：

```sh
rm -f "$HOME/.local/bin/omx"
```

然后从 `/Applications` 删除 `OpenMux.app`。

如果通过 Cargo 安装：

```sh
cargo uninstall omx-cli
```

OpenMux state 默认位于平台本地数据目录，除非设置了 `OMUX_STATE_ROOT`。只有在确认不再需要账号 snapshot 或 backup 时才删除 state。
