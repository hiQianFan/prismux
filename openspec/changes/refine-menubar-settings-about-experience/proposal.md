# 提案：优化 Menubar Settings 与 About 体验

## 背景

当前 OpenMux Menubar 的 Settings 只是 popover 顶部或 footer 里的小 `Menu`，只能修改 tray display mode 和 background refresh cadence。About 基本不存在，用户无法在图形界面里确认版本、schema 兼容性、state 位置、CLI/后端状态、诊断导出或项目链接。

但 Settings 不能变成“能想到什么就放什么”的杂物间。OpenMux 的核心用户旅程是：打开 menubar → 判断当前账号/用量状态 → 必要时切换 account/profile → 出问题时知道为什么、怎么恢复。Settings 只应该承载会改变这个旅程的长期偏好、provider 可见性/数据来源和排障支持信息。

我查看了同级目录中的 `/Users/qianfan/Desktop/Code/CodexBar`。CodexBar 的设置不是简单下拉菜单，而是独立 Preferences window，使用 `TabView` 分成 `General`、`Providers`、`Display`、`Advanced`、`About`、`Debug`。其中值得 OpenMux 学习的是：

- Settings 是一等产品界面，不塞在 popover 里。
- Provider 设置最终应 descriptor 化，但第一阶段只做固定 DTO：enabled、status、source preference、diagnostics。
- About 展示 app icon、版本、build、项目链接、更新设置。
- Debug/diagnostics 被隔离到低频页面，不污染主菜单。
- 设置窗口根据 tab 调整宽度，高信息密度页面如 Providers 可以更宽。

OpenMux 不应该照搬 CodexBar 的体量。CodexBar 是 usage/credits 多 provider 工具，OpenMux 的核心是本地账号/profile 控制平面。我们需要的是同样清晰的设置分层和 About/Support 能力，而不是 Sparkle、Widget、几十个 provider 私有设置、浏览器 Cookie 导入或大型 Swift provider registry。

CodexBar 的前后端分层也只能作为架构参考：它的 App、CLI、Widget 共享 `CodexBarCore`，App 里仍有 `UsageStore` / `SettingsStore` 负责状态编排，CLI 还能通过 `serve` 暴露 localhost JSON。OpenMux 的对应方向不是复制 Swift 业务层或引入 localhost server，而是让 Rust `omx-app` / control-plane 承担共享产品语义，Swift Menubar 只负责原生窗口、交互状态和安全 DTO 渲染。

## 目标

- 把 Settings 从 popover 小菜单升级为独立原生窗口，但只放必要配置。
- About 提供可信的版本、兼容性、运行时、支持与诊断入口。
- 区分 shared backend settings 和 frontend-local preferences，避免 CLI/Menubar 数据口径分叉。
- Provider 设置第一阶段采用固定 backend DTO，由 Rust control-plane 输出前端安全字段；通用 descriptor 留到 provider-specific 设置真正出现时再加。
- popover 只保留高频入口：Refresh、Settings、About/Support、Quit，不承担完整配置编辑。
- 不让 Swift 读取 auth、SQLite、usage logs 或 provider 私有文件。

## 非目标

- 本 change 不引入 Sparkle 自动更新。
- 本 change 不实现登录、导入、删除、alias 编辑的完整 GUI。
- 本 change 不实现 Launch at Login、keyboard shortcut、CLI helper installation 或 debug log viewer。
- 本 change 不复制 CodexBar 源码、资源、bundle ID、provider registry 或 updater 逻辑。
- 本 change 不把 provider 私有 endpoint、Cookie、token、auth payload 暴露给 Swift。
- 本 change 不做全量设置系统。第一阶段只做 General、Providers、About 三个 tab。

## 用户价值

- 用户能在图形界面里知道 OpenMux 当前是否健康、版本是否匹配、后端数据是否 stale。
- 用户能调整刷新频率、隐私显示、provider 启用状态和 provider 数据来源，而不需要猜配置文件。
- 用户能从 About/Support 复制安全诊断信息，方便反馈问题。
- 主 popover 更干净，只服务账号查看、切换和刷新，不被低频设置挤占。
