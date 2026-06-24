## ADDED Requirements

### Requirement: App 构建与 CLI 发布边界独立
`omx-menubar` SHALL 作为独立 macOS App 产物构建，Swift App 不作为 Cargo workspace member；Swift 开发 SHALL 使用 SwiftPM、scripts 和 CLI 命令，不提交 `.xcodeproj`、storyboard 或 Interface Builder 产物。Menubar 构建失败 SHALL NOT 改变 CLI runtime 行为或状态格式。

#### Scenario: 仅构建 Rust CLI
- **WHEN** 开发者运行标准 Cargo build/test
- **THEN** Rust workspace SHALL 不要求 Xcode/Swift 工具链即可完成非 Menubar 构建
- **AND** Menubar 专用 CI SHALL 在 macOS job 单独运行

### Requirement: Menubar 版本号统一使用仓库版本
Menubar App 的 `CFBundleShortVersionString` SHALL 使用仓库/Cargo workspace version，并与 CLI release tag 保持一致。

#### Scenario: 组装本地 app bundle
- **WHEN** `scripts/bundle-menubar.sh` 生成 `.app`
- **THEN** `Info.plist` SHALL 写入当前 workspace version
- **AND** 不得在 Swift 源码中硬编码独立 app version

### Requirement: v1 使用最小本地 bundle 和手动分发
v1 SHALL 提供 CLI 驱动的 build/bundle 流程、本地 ad-hoc signing 和 GitHub Releases 手动分发；Sparkle、appcast、notarization 自动化和 Homebrew cask 自动 bump SHALL NOT 是 v1 gate。

#### Scenario: 本地生成 app bundle
- **WHEN** 开发者运行 Menubar bundle script
- **THEN** script SHALL 组装 `.app`、写入 `LSUIElement=true` 和 `LSMinimumSystemVersion=14.0`
- **AND** SHALL 执行 ad-hoc codesign 并可用 `codesign --verify` 检查

### Requirement: 复制第三方源码前必须记录来源
v1 SHALL NOT 复制 TokenBar 源码或资源。任何未来从 TokenBar 或其他项目复制的源码、资源或设计实现 MUST 先在 reuse manifest/NOTICE 中记录 upstream repository、固定 commit、原文件、local file、license 和修改摘要，并保留许可证要求的 copyright notice。

#### Scenario: 未来采摘 TokenBar popover helper
- **WHEN** 实现需要将 TokenBar 文件复制到 `apps/omx-menubar`
- **THEN** change SHALL 先新增 `ThirdPartyNotices/TokenBar.md` 或等价 NOTICE 记录该文件及固定 upstream commit
- **AND** review SHALL 验证该文件不再依赖 TokenBar 数据引擎或品牌资源

### Requirement: 发布包不得包含被排除的数据引擎
Menubar release SHALL NOT 链接或打包 TokenBar `tb_core_ffi`、TokenBar vendored tokscale、独立 scanner/pricing/quota fetcher、动画资源或第二份 usage cache。

#### Scenario: 检查 release contents
- **WHEN** CI 审计最终 `.app` 和 linked libraries
- **THEN** 产物 SHALL 只包含 OpenMux backend 与明确批准的 Swift/第三方依赖
- **AND** SHALL 不包含 TokenBar 数据引擎符号或被排除资源

### Requirement: 发布前执行跨层验证
Menubar candidate SHALL 通过 Rust fmt/test/clippy、Swift unit/contract tests、临时 state root integration、bundle smoke test 和 privacy regression。

#### Scenario: 准备发布候选
- **WHEN** 团队标记 Menubar build 为 production candidate
- **THEN** 所有必需验证 SHALL 成功
- **AND** diagnostics/log 检查 SHALL 证明不含 raw auth、token、API key 或 raw provider log
