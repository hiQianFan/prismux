# 发布指南

[English](RELEASE.md)

Prismux v0.1 使用 version bump 自动发布。正常路径下维护者不手动创建 release tag。

## 正常发布流程

1. 从短分支创建 release PR。
2. 更新 `Cargo.toml` 中的 workspace version。
3. 将 `CHANGELOG.md` 的 `## Unreleased` 内容提升为：

   ```md
   ## vX.Y.Z - YYYY-MM-DD
   ```

4. 保留新的空 `## Unreleased` section。
5. CI 通过后合并 PR。
6. release workflow 检测 `vX.Y.Z` 不存在，提取对应 changelog 段落，创建 tag，构建 macOS artifacts，运行自检，生成 checksums，并创建 GitHub Release。

## 发布前检查

合并 release PR 前本地运行：

```sh
cargo fmt --all
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release -p prismux-cli --locked
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

Menubar 构建需要完整 Xcode。Command Line Tools 不包含 App 使用的 SwiftUI macro toolchain。

使用隔离状态运行 safe smoke tests：

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- status
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list codex
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- list claude
PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p prismux-cli -- doctor
```

除非明确就是要测试真实账号，否则不要对真实工具 home 运行 `login`、`use` 或 credential switching。

## macOS Artifacts

- `Prismux-vX.Y.Z-macos-arm64.zip` 和 `Prismux-vX.Y.Z-macos-x86_64.zip`，每个 archive 内包含 `Prismux.app`
- 内置 CLI helper：`Prismux.app/Contents/MacOS/prismux`
- `SHA256SUMS`

macOS app bundle 是首选分发路径。它同时包含 Menubar 和同版本 CLI helper。需要 Terminal 使用时，用户通过 Menubar 显式创建 PATH symlink；release 不复制 auth/state 文件，也不修改 shell 启动文件。

第一版公开 App bundle 不发布 Linux binary、Windows binary、Homebrew formula、crates.io package、Sparkle 更新、Developer ID notarization 自动化、独立签名或 provenance attestation。

## Bundle Layout

```text
Prismux.app/
  Contents/
    MacOS/
      Prismux
      prismux
    Resources/
      ...
```

`Contents/MacOS/prismux` 是可执行代码，不是 resource。发布验证必须检查：

- `Prismux.app` 设置了 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`。
- `CFBundleShortVersionString` 与 Cargo workspace version 一致。
- `Prismux.app/Contents/MacOS/prismux --version` 输出同一版本。
- bundled `prismux status` 在隔离 `PRISMUX_STATE_ROOT`、`CODEX_HOME`、`CLAUDE_CONFIG_DIR` 下通过。
- 解压 release zip 后，`Contents/MacOS/Prismux` 和 `Contents/MacOS/prismux` 仍保留可执行权限。
- bundle privacy/audit scripts 没有发现 raw auth、token、API key、raw log 或被排除的第三方数据引擎。

## Source Build

GitHub 自动生成的 source archive 可用于开发。`cargo install --git https://github.com/hiQianFan/prismux -p prismux-cli --locked` 只安装 CLI。完整 Menubar app 需要 macOS + full Xcode：

```sh
scripts/build-menubar.sh
scripts/bundle-menubar.sh
```

更完整步骤见 [源码构建](BUILD.md)。

## Artifact 能力边界

发布说明必须逐项声明 artifact 包含能力、平台和依赖：

| Artifact | 包含能力 | 平台 | 说明 |
| --- | --- | --- | --- |
| `Prismux.app` full bundle | Menubar dashboard、refresh、显式 account/profile activation、onboarding actions、bundled `prismux` helper、共享 state root | macOS 14+ | 首选公开 macOS artifact。 |
| standalone CLI tarball | CLI-only 账号/profile 管理和脚本能力 | 后续 | 有真实 standalone 需求后再加。 |
| Windows/Linux packages | 平台专用 CLI/App packaging | 后续 | 独立提案；不复用 macOS `.app` layout。 |

发布包不得暗示未包含的 optional module 可用。缺少 CLI、helper、Menubar 或 future `serve` 模块时，对应前端必须展示 unavailable view 和安装/切换指引；state-changing operation 只能在 `compatibility_view` 通过 schema gate 后启用。

## 回滚

如果 workflow 在创建 tag 前失败，修复 PR 或 workflow 后再次合并。

如果 release 已创建但 artifact 有问题，删除错误 artifact，发布 patch version，并在 `CHANGELOG.md` 记录问题。
