## 1. Workspace 与 CLI 入口硬切

- [x] 1.1 将 workspace package、crate path 和 Rust module 命名从 `omx-*` 改为 `prismux-*`，包括 CLI、core、app、provider plugin 和 menubar FFI crates；不要使用 `pmx-*` 作为内部包名。
- [x] 1.2 将公开 binary 从 `omx` 改为 `prismux`，并新增同逻辑短 binary `pmx`。
- [x] 1.3 更新 Clap command name/about/help，使正式帮助文本以 `prismux` 为主，并确认 `pmx` 执行相同 command tree。
- [x] 1.4 删除 `omx` binary 声明、测试期望和用户可见示例，不保留 `omx` fallback。
- [x] 1.5 更新 CLI help/version 相关测试，覆盖 `prismux --version`、`pmx --version` 和至少一个共享子命令解析。

## 2. Menubar 与 macOS bundle

- [x] 2.1 将 app bundle 从 `OpenMux.app` 改为 `Prismux.app`，Menubar executable 从 `OpenMux` 改为 `Prismux`。
- [x] 2.2 将 bundle 内 CLI helper 改为 `Contents/MacOS/prismux` 和 `Contents/MacOS/pmx`，并移除 `Contents/MacOS/omx`。
- [x] 2.3 更新 Menubar Settings 的 CLI install/status 逻辑，创建/检测 `prismux` 和 `pmx` symlink。
- [x] 2.4 确保已有外部 `pmx` 或 `prismux` 不是指向 bundled helper 时，Menubar 不静默覆盖，并显示手动处理指引。
- [x] 2.5 更新 `scripts/build-menubar.sh`、`scripts/bundle-menubar.sh`、bundle audit 脚本和 Swift contract tests 中的 app/helper 路径。

## 3. 文档与发布面

- [x] 3.1 将 README、中文 README、INSTALL、RELEASE、BUILD、ROADMAP、CHANGELOG、CONTRIBUTING 中的公开品牌改为 `Prismux`。
- [x] 3.2 将命令示例从 `omx ...` 改为 `prismux ...`，并在合适位置说明 `pmx` 是短命令。
- [x] 3.3 将中文短名统一为 `棱镜`，常用展示为 `棱镜（Prismux）` 或 `Prismux 棱镜`。
- [x] 3.4 更新 PRD、ARCHITECTURE、menubar v1 文档中的公开产品名、bundle 名、helper 路径和 release artifact 名称。
- [x] 3.5 更新 release workflow 和 release 文档，使 artifact、自检和 checksum 说明使用 `Prismux.app`、`prismux`、`pmx`，不再出现 `omx` 入口。
- [x] 3.6 将 `CHANGELOG.md` 里的旧品牌、旧命令和旧环境变量全部改为新命名，不保留 `OpenMux`/`openmux`/`omx`/`OMUX` 历史称呼。
- [x] 3.7 将 Cargo metadata、README、安装文档和 release workflow 中的 GitHub URL 从 `hiQianFan/openmux` 改为 `hiQianFan/prismux`。

## 4. State、FFI 与剩余命名

- [x] 4.1 将 `OMUX_STATE_ROOT` 改为 `PRISMUX_STATE_ROOT`，不保留 `OMUX_STATE_ROOT` 兼容读取。
- [x] 4.2 将 FFI exported symbols、schema 名称、fixture 路径和生成 artifact 中的 `omx` 命名改为 `prismux`。
- [x] 4.3 用 `rg -n "OpenMux|openmux|OMUX|omx"` 审计剩余命中，只保留历史 changelog 或第三方 URL 中不可改的文本。
- [x] 4.4 用 `rg -n "PrismUX|PrismX"` 确认默认品牌写法不是 `PrismUX` 或 `PrismX`。
- [x] 4.5 在代码和文档验证通过后执行 `gh repo rename prismux`，再执行 `git remote set-url origin https://github.com/hiQianFan/prismux.git` 或对应 SSH URL。

## 5. 验证

- [x] 5.1 运行 `cargo fmt --all`。
- [x] 5.2 运行 `cargo test --locked`。
- [x] 5.3 运行 `cargo clippy --all-targets --all-features -- -D warnings`。
- [x] 5.4 运行 `cargo build --release -p prismux-cli --locked`，确认生成 `prismux` 和 `pmx` 且不生成 `omx`。
- [x] 5.5 使用隔离目录运行 `PRISMUX_STATE_ROOT=/tmp/prismux-state CODEX_HOME=/tmp/prismux-codex-home CLAUDE_CONFIG_DIR=/tmp/prismux-claude-home cargo run -p prismux-cli --bin prismux -- status`。
- [x] 5.6 运行 `scripts/build-menubar.sh`，确认 bundle 内存在 `Prismux`、`prismux`、`pmx`，不存在 `omx` helper。
- [x] 5.7 运行 `openspec validate adopt-prismux-brand-aliases`。
