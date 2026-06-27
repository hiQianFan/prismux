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

## Artifact 能力边界

当前 v0.1 release 仍以 CLI artifact 为主，Menubar 通过 macOS 本地 bundle 构建脚本验证。后续如果拆成独立产物，发布说明必须逐项声明包含能力、平台和依赖：

| Artifact | 包含能力 | 平台 | 依赖 |
| --- | --- | --- | --- |
| `CLI-only` | `login`、`save`、`use`、`import`、`alias`、`doctor`、`usage`、JSON/machine output | macOS；后续再评估 Linux/Windows | 已安装的目标 AI tool CLI |
| `Menubar-only` | dashboard、refresh、显式 activation、last-good snapshot、upgrade-required/unavailable view、CLI handoff 文案 | macOS 14+ Apple Silicon | embedded staticlib、helper binary 或 installed `omx` CLI 之一 |
| `full bundle` | CLI + Menubar + 共享 state root + compatibility gate | macOS | 已安装的目标 AI tool CLI |

发布包不得暗示未包含的 optional module 可用。缺少 CLI、helper、Menubar 或 future `serve` 模块时，对应前端必须展示 unavailable view 和安装/切换指引；state-changing operation 只能在 `compatibility_view` 通过 schema gate 后启用。

## 回滚

如果 workflow 在创建 tag 前失败，修复 PR 或 workflow 后再次合并。

如果 release 已创建但 artifact 有问题，删除错误 artifact，发布 patch version，并在 `CHANGELOG.md` 记录问题。
