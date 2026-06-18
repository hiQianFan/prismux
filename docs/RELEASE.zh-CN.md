# 发布指南

[English](RELEASE.md)

OpenMux v0.1 使用 version bump 自动发布。正常路径下维护者不手动创建 release tag。

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
cargo build --release -p omx-cli --locked
```

使用隔离状态运行 safe smoke tests：

```sh
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- status
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list codex
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- list claude
OMUX_STATE_ROOT=/tmp/openmux-state CODEX_HOME=/tmp/codex-home CLAUDE_CONFIG_DIR=/tmp/claude-home cargo run -p omx-cli -- doctor
```

除非明确就是要测试真实账号，否则不要对真实工具 home 运行 `login`、`use` 或 credential switching。

## v0.1 Artifacts

- macOS Apple Silicon archive
- macOS Intel archive
- `SHA256SUMS`

v0.1 不发布 Linux binary、Windows binary、Homebrew formula、crates.io package、独立签名或 provenance attestation。

## 回滚

如果 workflow 在创建 tag 前失败，修复 PR 或 workflow 后再次合并。

如果 release 已创建但 artifact 有问题，删除错误 artifact，发布 patch version，并在 `CHANGELOG.md` 记录问题。

