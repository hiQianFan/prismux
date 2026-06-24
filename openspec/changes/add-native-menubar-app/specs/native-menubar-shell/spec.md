## ADDED Requirements

### Requirement: 原生菜单栏应用生命周期
OpenMux SHALL 提供 macOS 14+ Apple Silicon 原生菜单栏应用，以 accessory activation policy 运行且默认不显示 Dock 图标，并通过 `NSStatusItem` 打开单实例 popover。

#### Scenario: 用户打开 Menubar
- **WHEN** 用户点击 OpenMux 菜单栏图标
- **THEN** 应用 SHALL 打开或聚焦同一个 popover
- **AND** SHALL NOT 为每次打开创建重复 status item 或独立窗口实例

### Requirement: 最小信息架构
Popover SHALL 在单页内提供 active account、quota、account list、today usage、refresh、Settings 和 Quit 入口，并 SHALL NOT 要求用户进入全屏 analytics TUI 才能完成主要任务。

#### Scenario: 用户判断是否切换账号
- **WHEN** 用户打开已有数据的 popover
- **THEN** 用户 SHALL 能在当前页面看到 active account、主要 quota/reset 和可切换账号
- **AND** SHALL 能从同一页面发起刷新或切换

### Requirement: 明确的加载与降级状态
Shell SHALL 使用最小状态模型区分 `loading`、`ready(report, stale?)` 和 `failed(lastGood?)`；后台或交互刷新失败时 SHALL 优先保留最近成功数据。refreshing、partial 和 empty SHALL 作为 report 字段或 view 派生状态处理。

#### Scenario: 刷新失败但存在历史数据
- **WHEN** backend refresh 失败且应用已有最近成功 report
- **THEN** popover SHALL 继续展示最近成功数据
- **AND** SHALL 标记 stale 状态、最近成功时间和安全错误码

#### Scenario: 首次加载失败
- **WHEN** 应用启动后没有可展示的历史数据且 backend 查询失败
- **THEN** popover SHALL 展示可恢复的 unavailable 状态
- **AND** SHALL 提供 Retry 和安全诊断而不是空白窗口

### Requirement: 主线程不得执行阻塞后端调用
Swift UI SHALL 在非 MainActor executor 上调用阻塞 C ABI，并 SHALL 防止旧请求结果覆盖更新的用户选择或账号状态。

#### Scenario: 用户在旧刷新完成前切换筛选状态
- **WHEN** 一个较早请求在较新请求之后返回
- **THEN** store SHALL 丢弃不匹配当前 generation 的旧结果
- **AND** UI SHALL 保持较新状态

### Requirement: 最小本地设置
应用 SHALL 提供 tray display mode 和 background refresh cadence 的本地设置，并 SHALL 使用 OpenMux 自有 bundle/key namespace。v1 SHALL NOT 提供 Sparkle/update channel preference。

#### Scenario: 用户关闭菜单栏文字
- **WHEN** 用户将 tray display mode 设置为 icon-only
- **THEN** status item SHALL 停止显示账号或 quota 文字
- **AND** popover 数据和后台安全调度 SHALL 不受影响
