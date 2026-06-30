## ADDED Requirements

### Requirement: Settings 信息架构

OpenMux Menubar SHALL 使用清晰的一等 Settings 窗口承载长期偏好、provider 配置、显示偏好、工具配置和产品信息。

#### Scenario: 打开 Settings

- **WHEN** 用户从 Menubar 打开 Settings
- **THEN** Settings SHALL 展示顶部 tab 式一级导航
- **AND** 一级 tab SHALL 包含 `General`、`Providers` 和 `About`
- **AND** Settings SHALL NOT 设独立 `Display` 或 `Tools` 顶层 tab，也 SHALL NOT 保留空壳 `General`
- **AND** Settings SHALL 使用原生 grouped `Form` 而非自造 card，并 SHALL NOT 使用与内容密度不匹配的左侧 sidebar 作为第一阶段默认导航。

#### Scenario: General 承载本机行为与显示偏好

- **WHEN** 用户查看基础行为和显示偏好
- **THEN** tray display、personal identifier masking、launch at login 和 command-line tool 配置 SHALL 位于 `General`
- **AND** launch at login SHALL 反映真实的 `SMAppService` 注册状态
- **AND** background refresh cadence SHALL NOT be exposed as a minute-level user setting in the first phase
- **AND** 单个分组 SHALL NOT 混合无关概念导致用户无法判断设置归属。

#### Scenario: Providers 承载只读 provider 配置

- **WHEN** 用户打开 `Providers`
- **THEN** each provider SHALL expose enablement、status、diagnostics and source policy only when the choice is understandable
- **AND** `Providers` SHALL expose read-only provider profile/gateway summaries when the provider supports imported profiles
- **AND** profile/account import、use、remove、login、delete、alias and switch actions SHALL remain in provider/dashboard context rather than Settings，`Providers` SHALL 只提供 `Manage in dashboard` 跳转。

### Requirement: Provider profile 与网络代理共享配置源

OpenMux Menubar SHALL manage CLI-visible provider profiles and network proxy settings through Rust shared state/control-plane rather than a Menubar-only preference store.

#### Scenario: CLI 导入的 gateway profile 出现在 Menubar

- **WHEN** 用户通过 CLI 执行 `omx import codex` 或 `omx import claude` 导入 gateway/API profile
- **THEN** Menubar `Providers` SHALL list the imported profile after refresh
- **AND** the listed metadata SHALL come from the same OpenMux state/profile registry used by CLI `omx list`
- **AND** Menubar SHALL display redacted profile metadata such as name、base URL、model、auth type and active state
- **AND** Menubar SHALL NOT display raw API keys、auth tokens、snapshot bytes or raw profile content。

#### Scenario: Menubar 管理的 profile 可被 CLI 看到

- **WHEN** 用户在 Menubar dashboard 中 import、use 或 remove a provider profile
- **THEN** the operation SHALL call Rust control-plane or plugin APIs equivalent to CLI import/use/remove
- **AND** a subsequent CLI `omx list <provider>` SHALL reflect the same profile state
- **AND** Settings `Providers` SHALL 只读展示同一来源摘要，SHALL NOT 重复实现 import/use/remove
- **AND** Menubar SHALL NOT persist a second copy of profile configuration in Swift `UserDefaults` or Menubar-private JSON。

#### Scenario: Gateway profile 与 HTTP proxy 分开展示

- **WHEN** Settings displays provider endpoint configuration
- **THEN** OpenAI/Anthropic-compatible endpoint values such as `OPENAI_BASE_URL` and `ANTHROPIC_BASE_URL` SHALL be labeled as `Gateway profile` or `API profile`
- **AND** HTTP proxy configuration SHALL be labeled as `Network proxy`
- **AND** the UI SHALL NOT use a single ambiguous `Proxy` setting for both concepts。

#### Scenario: Network proxy 状态

- **WHEN** 用户打开 `General` 的 command-line tool 分组
- **THEN** Menubar SHALL show the effective network proxy source for OpenMux refresh requests, such as shared settings、`OMUX_HTTPS_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` or none
- **AND** persistent network proxy edits, if implemented, SHALL be written through shared Rust settings/control-plane
- **AND** persistent network proxy edits SHALL NOT be stored only in Menubar `UserDefaults`。

### Requirement: General 中配置 Terminal command

OpenMux Menubar SHALL 将 App 内置 CLI helper 与 Terminal command 配置解释为两个不同概念，配置位于 `General` 的 command-line tool 分组，而非独立 tab。

#### Scenario: 展示 bundled helper

- **WHEN** 用户打开 `General` 的 command-line tool 分组
- **THEN** Menubar SHALL 显示 bundled `omx` helper 的路径
- **AND** 路径 SHALL 指向 `OpenMux.app/Contents/MacOS/omx`
- **AND** Menubar SHALL 显示 helper version 或 helper unavailable 状态。

#### Scenario: Terminal command 未配置

- **WHEN** PATH lookup 找不到 `omx`
- **THEN** Menubar SHALL 显示 Terminal command 未配置
- **AND** SHALL 提供 `Enable omx command` 或 `Copy manual command`
- **AND** SHALL NOT 表示需要下载另一个 CLI。

#### Scenario: 外部 omx 已存在

- **WHEN** PATH lookup 找到 `omx` 但它不指向 bundled helper 或版本不匹配
- **THEN** Menubar SHALL 显示 `Different omx found` 或等价状态
- **AND** SHALL NOT 覆盖该 binary 或 symlink
- **AND** SHALL 提供手动处理 guidance。

#### Scenario: 启用 omx command

- **WHEN** 用户显式点击 `Enable omx command`
- **THEN** Menubar SHALL 只创建从用户 PATH 目录到 bundled helper 的 symlink
- **AND** SHALL NOT 复制 auth、state、snapshot 或 backup 文件
- **AND** SHALL NOT 静默修改 `.zshrc`、`.bashrc` 或 shell profile。

### Requirement: About 面向用户可信信息

OpenMux Menubar SHALL 提供面向用户的 About 页面，优先展示产品身份、版本、链接和支持入口。

#### Scenario: 查看 About

- **WHEN** 用户打开 `About`
- **THEN** About SHALL 展示 OpenMux 名称、app icon 或稳定占位、产品版本、runtime/build 摘要和简短产品描述
- **AND** SHALL 展示 GitHub、Documentation、Issues 和 Releases 链接
- **AND** 链接 SHALL 使用 `https://github.com/hiQianFan/openmux` 作为仓库根。

#### Scenario: 作者署名链接

- **WHEN** 用户打开 `About`
- **THEN** About SHALL 展示作者 GitHub 链接
- **AND** 作者 GitHub SHALL 指向 `https://github.com/hiQianFan`
- **AND** 作者 personal website SHALL 指向 `https://blog.mapin.net/`
- **AND** 作者 Twitter/X SHALL 指向 `https://x.com/hiQianFan`
- **AND** About MAY 展示已配置的 email link
- **AND** About SHALL NOT render empty placeholder links for author URLs that are not configured。

#### Scenario: 支持信息

- **WHEN** 用户需要反馈问题
- **THEN** About SHALL 提供 `Copy Version Info`
- **AND** SHALL 提供 `Copy Redacted Support Report`
- **AND** copied support data SHALL NOT include raw auth payloads、tokens、API keys、snapshots、backups 或 private account file contents。

#### Scenario: 内部 schema 信息

- **WHEN** About 展示版本和支持信息
- **THEN** control-plane schema、state schema 和 settings schema MAY appear in copied version/support info
- **AND** SHALL NOT dominate the first visible About content over product identity and user-facing links。

### Requirement: Dashboard footer 工具栏

OpenMux Menubar dashboard footer SHALL provide low-friction utility access without competing with account/provider content.

#### Scenario: 查看 Dashboard footer

- **WHEN** 用户打开 dashboard
- **THEN** footer SHALL show a concise status or freshness summary
- **AND** SHALL provide a `⋯` overflow menu giving access to About、Copy omx command、Open Releases，plus a visible Quit
- **AND** footer SHALL NOT duplicate the Settings entry already in the header
- **AND** footer SHALL NOT use `Manage in CLI` as the only prominent non-quit action。

#### Scenario: Terminal command 未准备好

- **WHEN** 用户从 footer `⋯` 选择 Copy omx command 且 Terminal command 未配置
- **THEN** Menubar SHALL copy the command and MAY guide the user to `Settings -> General` command-line tool 分组
- **AND** SHALL NOT silently run credential-changing CLI commands。

### Requirement: 非范围能力不进入第一阶段

OpenMux SHALL keep Settings/Footer redesign independent from update and installer systems that are not yet part of the product decision.

#### Scenario: 更新入口

- **WHEN** About 或 General 提供 release/update information
- **THEN** 第一阶段 SHALL open or copy the GitHub Releases link
- **AND** SHALL NOT implement Sparkle、appcast、background update download、Homebrew cask management 或 Developer ID notarization UI。

#### Scenario: CLI 下载器

- **WHEN** 用户需要 Terminal `omx`
- **THEN** Menubar SHALL use the bundled helper and symlink/copy-command guidance
- **AND** SHALL NOT download a standalone CLI binary from GitHub Releases in this change。
