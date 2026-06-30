## ADDED Requirements

### Requirement: menubar 暴露 login / save_existing_login / import_profile 三个接入操作

OpenMux Menubar 后端契约 SHALL 暴露 `login`、`save_existing_login`、`import_profile` 三个操作，分别映射到 plugin 的 `login`、`save_current`、`import_config`。每个操作成功时 SHALL 返回包含 `OperationResult` 和刷新后 `DashboardReport` 的响应，失败时 SHALL 返回可读、已脱敏的错误。SHALL NOT 要求用户离开 menubar 使用 CLI 才能完成接入。

#### Scenario: 调用 login 操作

- **WHEN** 客户端以某 provider 调用 `login`
- **THEN** 后端 SHALL 调用该 provider plugin 的 `login`
- **AND** menubar 发起的 login SHALL 默认请求 `activate = true`
- **AND** 成功时 SHALL 返回 operation 和刷新后 dashboard，使新接管账号可见
- **AND** 失败时 SHALL 返回可读且脱敏的错误

#### Scenario: 调用 save_existing_login 操作

- **WHEN** 客户端以某 provider 调用 `save_existing_login`
- **THEN** 后端 SHALL 调用该 provider plugin 的 `save_current`，导入本机已存在的官方登录产物
- **AND** SHALL NOT 触发任何重新登录流程
- **AND** 成功时 SHALL 返回 operation 和刷新后 dashboard

#### Scenario: 调用 import_profile 操作

- **WHEN** 客户端以某 provider 与非空配置文本调用 `import_profile`
- **THEN** 后端 SHALL 调用该 provider plugin 的 `import_config`
- **AND** 成功时 SHALL 返回 operation 和刷新后 dashboard，使导入 profile 可见
- **AND** 空配置文本 SHALL 被拒绝为可读错误

#### Scenario: 未知 provider

- **WHEN** 操作携带的 provider 无对应 plugin
- **THEN** 后端 SHALL 返回可读错误，SHALL NOT panic

### Requirement: 账号卡提供应用内登录入口与 CLI 缺失引导

账号卡（含零账号空状态）SHALL 提供触发 `login` 的入口。menubar SHALL 检测或捕获对应官方 CLI 缺失错误；不可用时 SHALL 提示安装引导，SHALL NOT 静默失败或阻塞 UI 无反馈。

#### Scenario: 从空状态登录

- **WHEN** 某 provider 无受管账号且用户点击账号卡的登录入口
- **THEN** menubar SHALL 发起该 provider 的 `login`
- **AND** 登录成功后新账号 SHALL 出现在账号卡

#### Scenario: 官方 CLI 缺失

- **WHEN** 用户触发 login 但本机缺少对应官方 CLI
- **THEN** menubar SHALL 展示安装引导
- **AND** SHALL NOT 让 UI 卡死或仅静默失败

#### Scenario: 登录成功后刷新

- **WHEN** `login` 成功返回
- **THEN** menubar SHALL 从响应 dashboard 使新账号可见
- **AND** SHALL 按返回结果反映其是否为当前 active

### Requirement: login 等待可取消且不会无限阻塞

menubar 发起的 `login` 拉起官方 CLI 后会等待浏览器 OAuth 回调。该等待 SHALL 可被用户取消，并 SHALL 有时间上限，使关闭浏览器或放弃登录都不会让 menubar 永久卡在登录中。后端 SHALL 暴露 `cancel_login` 操作；in-flight login 在收到取消或超过等待上限时 SHALL 终止其官方 CLI 子进程并释放操作锁，返回可读结果。

#### Scenario: 用户取消登录

- **WHEN** login 正在等待浏览器回调且用户点击取消
- **THEN** menubar SHALL 调用 `cancel_login`
- **AND** 后端 SHALL 终止官方 CLI 子进程并结束等待
- **AND** 操作锁 SHALL 被释放，其余 menubar 操作 SHALL 恢复可用

#### Scenario: 等待超时

- **WHEN** login 等待浏览器回调超过等待上限
- **THEN** 后端 SHALL 终止官方 CLI 子进程
- **AND** SHALL 返回可读的超时错误，SHALL NOT 让 UI 永久卡在登录中

#### Scenario: 浏览器被关闭

- **WHEN** 用户关闭浏览器、官方 CLI 回调永不到达
- **THEN** menubar SHALL 通过取消或超时结束等待
- **AND** SHALL NOT 永久持有操作锁或保持登录中状态

### Requirement: 账号卡提供 Use existing login 接管入口

账号卡（含零账号空状态）SHALL 提供 `Use existing login` 入口，用于接管本机已存在的官方登录产物。该入口 SHALL 调用 `save_existing_login`，SHALL NOT 触发官方 CLI 登录。

#### Scenario: 接管本机已登录账号

- **WHEN** 用户点击某 provider 的 `Use existing login`
- **THEN** menubar SHALL 发起该 provider 的 `save_existing_login`
- **AND** 成功后该账号 SHALL 出现在账号卡

#### Scenario: 没有可接管登录产物

- **WHEN** 用户点击 `Use existing login` 但本机没有可导入的官方登录产物
- **THEN** menubar SHALL 展示后端返回的可读错误
- **AND** SHALL NOT 触发重新登录流程

### Requirement: profile 卡提供应用内配置导入入口

profile 卡（含零 profile 空状态）SHALL 提供触发 `import_profile` 的入口，允许用户粘贴文本、选择文件或拖放文件作为配置内容提交。导入成功后新 profile SHALL 出现在 profile 卡，并展示其安全 metadata。

#### Scenario: 从空状态导入 profile

- **WHEN** 某 provider 无 profile 且用户在 profile 卡提交非空配置内容
- **THEN** menubar SHALL 发起该 provider 的 `import_profile`
- **AND** 导入成功后新 profile SHALL 出现在 profile 卡

#### Scenario: 按当前 provider 导入

- **WHEN** 用户在 Codex 页面提交 profile 配置
- **THEN** menubar SHALL 以 `provider = "codex"` 调用 `import_profile`
- **AND** 后端 SHALL 按 Codex profile 规则校验配置

#### Scenario: 按 Claude provider 导入

- **WHEN** 用户在 Claude 页面提交 profile 配置
- **THEN** menubar SHALL 以 `provider = "claude"` 调用 `import_profile`
- **AND** 后端 SHALL 按 Claude profile 规则校验配置

#### Scenario: 展示导入结果

- **WHEN** `import_profile` 成功返回
- **THEN** profile 卡 SHALL 展示 profile 名称与 auth 类型
- **AND** base URL 等敏感字段 SHALL 受隐私脱敏设置约束

#### Scenario: 导入内容无效

- **WHEN** 提交的配置内容不被后端接受
- **THEN** menubar SHALL 展示后端返回的可读错误
- **AND** SHALL NOT 写入无效 profile

#### Scenario: 拖放非文本文件

- **WHEN** 用户拖放不可读取或非 UTF-8 的文件到 profile 导入入口
- **THEN** menubar SHALL 展示可读错误
- **AND** SHALL NOT 调用后端导入

### Requirement: menubar 不主动探测未接管登录产物

OpenMux Menubar 第一版 SHALL NOT 为自动接管提示新增 `unmanaged_login_detected` dashboard 字段。接管已登录账号 SHALL 由用户显式点击 `Use existing login` 触发。

#### Scenario: 首次加载

- **WHEN** menubar 加载且本机存在未接管的官方登录产物
- **THEN** menubar SHALL NOT 主动弹出接管提示
- **AND** 用户 SHALL 可通过账号卡的 `Use existing login` 入口接入
