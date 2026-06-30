## 1. Release artifact

- [x] 1.0 将公开 bundle identity 从本地旧名 `OpenMux Menubar.app` 迁移为 `OpenMux.app`：更新 `scripts/bundle-menubar.sh`、`scripts/check-menubar-version.sh`、`scripts/audit-menubar-bundle.sh`、`CFBundleExecutable`、`CFBundleName` 和 smoke test 路径。
- [x] 1.1 更新 `scripts/bundle-menubar.sh`，将 `target/release/omx` 复制到 `OpenMux.app/Contents/MacOS/omx`。
- [x] 1.2 验证 `OpenMux.app/Contents/MacOS/omx --version` 与 Cargo workspace version 一致。
- [x] 1.3 更新 release workflow，构建 CLI、使用 full Xcode 构建 Menubar、运行 Swift contract tests、组装 `.app` archive、运行 bundle smoke test。
- [x] 1.4 使用能保留 macOS bundle layout 和 executable permissions 的 zip 流程打包，上传 `OpenMux-vX.Y.Z-macos-<arch>.zip` 和 `SHA256SUMS`。
- [x] 1.5 在 release notes 中声明 artifact 包含 Menubar + bundled CLI helper。
- [x] 1.6 解压 release zip 后验证 `Contents/MacOS/OpenMux` 和 `Contents/MacOS/omx` 都存在、可执行，且 helper smoke test 使用隔离 `OMUX_STATE_ROOT`、`CODEX_HOME`、`CLAUDE_CONFIG_DIR`。

## 2. CLI install UX

- [x] 2.1 在 Menubar Settings 增加 CLI command 配置入口；若同时实施 `redesign-menubar-settings-footer-experience`，入口放在 `General` 的 command-line tool 分组，不另建重复 `Tools` tab。
- [x] 2.2 检测 bundled helper path：`Bundle.main.bundleURL/Contents/MacOS/omx`。
- [x] 2.3 检测 PATH 中的 `omx`、真实路径和版本。
- [x] 2.4 实现 `Install CLI`，默认创建 `~/.local/bin/omx` symlink 到 bundled helper。
- [x] 2.5 当 `~/.local/bin` 不在 PATH 时显示 `Copy PATH command`。
- [x] 2.6 对版本不一致显示 `Update CLI link` / `Different omx found`，不要覆盖非 symlink 的用户安装。
- [x] 2.7 创建 symlink 前确保 `~/.local/bin` 存在；如果 `~/.local/bin/omx` 已存在且不是指向 bundled helper 的 symlink，只显示 guidance，不自动替换。

## 3. Menubar interaction

- [x] 3.1 Dashboard 空账号状态提供 `Sign in`、`Use existing login`、`Import profile` 操作入口。
- [x] 3.2 Footer 提供 CLI handoff 或 copyable command；若同时实施 `redesign-menubar-settings-footer-experience`，不要保留醒目的 `Manage in CLI` 主按钮，改由状态串和溢出菜单引导到 Settings/General。
- [x] 3.3 About 显示 App version、CLI helper version、state root、release/docs links。
- [x] 3.4 所有 credential-changing 操作仍需要用户显式点击，不在后台静默执行。

## 4. Documentation

- [x] 4.1 更新 `docs/INSTALL.md` 和 `docs/INSTALL.zh-CN.md`，说明 full bundle 安装、CLI symlink 和 PATH。
- [x] 4.2 更新 `docs/RELEASE.md` 和 `docs/RELEASE.zh-CN.md`，说明 app artifact、helper layout 和验收。
- [x] 4.3 更新 `docs/menubar-v1.md`，记录 Menubar 交互链路。
- [x] 4.4 更新 README/ROADMAP，将 CLI-only 口径改为 macOS full bundle 主路径。
- [x] 4.5 更新 `CHANGELOG.md`，为实际发布版本新增 `## vX.Y.Z - YYYY-MM-DD`，记录 full bundle、bundled CLI helper、Menubar onboarding、已知限制和安全行为。
- [x] 4.6 更新 `CONTRIBUTING.md`，补充 Menubar/SwiftPM/Xcode 构建要求、bundle smoke、文档更新要求和 OpenSpec change 要求。
- [x] 4.7 新增或更新 source build 文档（建议 `docs/BUILD.md`），说明从 source archive/clone 构建 CLI、Menubar、full bundle，以及 full Xcode requirement。
- [x] 4.8 统一 repository URL：Cargo metadata、README、Menubar About links、INSTALL/RELEASE 文档必须指向同一个 GitHub repo。
- [x] 4.9 审查公开仓库说明文件：确认 `LICENSE`、`SECURITY.md`、issue/PR templates、vendor notes、CodexBar/reference boundary 可被外部贡献者理解。
- [x] 4.10 决定是否新增 `CODE_OF_CONDUCT.md`；若暂不新增，至少在 `CONTRIBUTING.md` 写明基本协作行为期望。

## 5. Validation

- [x] 5.1 运行 `cargo fmt --all`、`cargo test --locked`、`cargo clippy --all-targets --all-features -- -D warnings`。
- [x] 5.2 运行 `scripts/build-menubar.sh` 和 `scripts/bundle-menubar.sh`。
- [x] 5.3 对 bundle 运行 `codesign --verify`、version check、privacy check 和 bundle audit。
- [x] 5.4 使用隔离 `OMUX_STATE_ROOT`、`CODEX_HOME`、`CLAUDE_CONFIG_DIR` 运行 bundled helper `omx status` smoke test。
- [x] 5.5 执行 public repo surface audit：`git ls-files` 不应包含本地 agent/BMad 输出、auth payload、token、snapshot、backup、private account files 或无说明的生成物。
- [x] 5.6 执行 release readiness audit：README、INSTALL、RELEASE、ROADMAP、CHANGELOG、workflow artifact 名称和 bundle layout 互相一致。
- [x] 5.7 执行 secret scan（gitleaks 或等价命令）；如果工具不可用，记录手动 grep 审计命令和结果。
- [x] 5.8 验证 GitHub source archive build path：从 clean checkout/source archive 能执行 Rust checks，且 macOS + full Xcode 环境能执行 Menubar bundle script。

## 6. GitHub repository setup

- [ ] 6.1 确认 GitHub repo description、website/about、topics、license detection 和 default branch 正确。
- [ ] 6.2 启用 branch protection：required CI checks、PR-only、禁止 force push、禁止删除。
- [ ] 6.3 启用 GitHub private vulnerability reporting（如果当前 repo 支持）。
- [x] 6.4 确认 issue templates 不要求用户粘贴 auth/token/snapshot/backups，并提供环境/version/path override 字段。
- [x] 6.5 确认 release workflow 权限最小化；PR CI 不需要 `contents: write`。

> 远端状态：本地当前没有配置 git remote；文档中的 `hiQianFan/openmux` 仓库对 `gh repo view` 不可解析，branch protection 和 private vulnerability reporting API 返回 404。因此 6.1-6.3 需要在 GitHub 仓库创建/授权/remote 配置完成后再核验并勾选。
