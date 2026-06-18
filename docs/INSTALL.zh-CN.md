# 安装 OpenMux

[English](INSTALL.md)

## GitHub Releases

v0.1 只发布 macOS 官方 binary。

1. 从 Releases 下载你的 Mac 对应的 archive：

   ```text
   https://github.com/hiQianFan/openmux/releases
   ```

2. 解压并把 `omx` 放到 `PATH` 中，例如：

   ```sh
   mkdir -p "$HOME/.local/bin"
   mv omx "$HOME/.local/bin/omx"
   chmod +x "$HOME/.local/bin/omx"
   ```

3. 验证：

   ```sh
   omx --version
   omx status
   ```

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

## 卸载

删除 binary：

```sh
rm -f "$HOME/.local/bin/omx"
```

如果通过 Cargo 安装：

```sh
cargo uninstall omx-cli
```

OpenMux state 默认位于平台本地数据目录，除非设置了 `OMUX_STATE_ROOT`。只有在确认不再需要账号 snapshot 或 backup 时才删除 state。

