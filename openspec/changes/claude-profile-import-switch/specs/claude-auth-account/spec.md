## ADDED Requirements

### Requirement: Claude OAuth account 导入

OpenMux SHALL 支持从本机已有 Claude Code 官方登录产物导入 Claude.ai/Console OAuth account snapshot。

#### Scenario: 包装官方 Claude Code 登录

- GIVEN 用户希望添加一个新的 Claude OAuth account
- WHEN 用户运行 `omx login claude --alias work`
- THEN OpenMux 调用官方 Claude Code CLI 登录流程
- AND 官方登录成功后导入当前 credential backend 中的 `claudeAiOauth` snapshot
- AND account 名称为 `work`
- AND registry 不保存 raw access token 或 refresh token。

#### Scenario: 登录后立即切换

- GIVEN 用户希望添加并立即使用一个新的 Claude OAuth account
- WHEN 用户运行 `omx login claude --alias work --use`
- THEN OpenMux 调用官方 Claude Code CLI 登录流程
- AND 导入 account snapshot
- AND 将该 account 标记为 active。

#### Scenario: 从 plaintext credentials 导入 account

- GIVEN 当前 Claude home 存在 `.credentials.json`
- AND `.credentials.json` 包含 `claudeAiOauth.accessToken`、`refreshToken`、`expiresAt` 和 scopes
- WHEN 用户运行 `omx import claude --name work` 且没有提供 KV/JSON/TOML 配置内容
- THEN OpenMux 创建 Claude account snapshot
- AND registry 保存脱敏 metadata 和 snapshot hash
- AND registry 不保存 raw access token 或 refresh token
- AND 命令输出包含 account 编号、名称和脱敏账号信息。

#### Scenario: 从 macOS Keychain 导入 account

- GIVEN 当前平台为 macOS
- AND Claude Code secure storage 在 Keychain 中存在完整 `claudeAiOauth`
- WHEN 用户运行 `omx import claude --name work` 且没有提供 KV/JSON/TOML 配置内容
- THEN OpenMux 通过 credential backend 导入 account snapshot
- AND 不在 stdout、日志或 registry 中输出 Keychain payload。

#### Scenario: 拒绝 inference-only token

- GIVEN 当前认证来源只有 `CLAUDE_CODE_OAUTH_TOKEN`
- WHEN 用户运行 `omx import claude` 且没有提供 KV/JSON/TOML 配置内容
- THEN OpenMux 拒绝导入 account
- AND 提示该 token 缺少 refresh token 和 expiresAt，不能作为完整 OAuth account snapshot。

### Requirement: Claude OAuth account 切换

OpenMux SHALL 支持通过 `omx use claude <selector>` 在 selector 唯一命中 account 时切换已导入的 Claude OAuth account snapshot。

#### Scenario: 按当前列表编号切换 account

- GIVEN Claude account `work` 已导入
- AND `omx list claude` 将该 account 展示为 `#1`
- WHEN 用户运行 `omx use claude 1`
- THEN OpenMux 校验 account snapshot hash
- AND 备份当前 credential payload 和 `oauthAccount` metadata
- AND 写入目标 account snapshot
- AND 将 `work` 标记为 active account。

#### Scenario: 按名称切换 account

- GIVEN Claude account `work` 已导入
- WHEN 用户运行 `omx use claude work`
- THEN OpenMux 将 `work` 应用为 active account。

#### Scenario: snapshot 校验失败

- GIVEN Claude account snapshot 文件与 registry 中记录的 hash 不匹配
- WHEN 用户运行 `omx use claude <selector>`
- THEN OpenMux 拒绝切换
- AND 不修改当前 Claude credential。

#### Scenario: 切换失败回滚

- GIVEN Claude account `work` 已导入
- AND 写入目标 credential 或 `oauthAccount` metadata 时失败
- WHEN 用户运行 `omx use claude 1`
- THEN OpenMux 恢复写入前备份
- AND 返回包含恢复状态的错误
- AND 不将 `work` 标记为 active account。

#### Scenario: registry 更新失败回滚

- GIVEN Claude account `work` 已导入
- AND 目标 credential 已经写入成功
- BUT 更新 OpenMux registry active account 失败
- WHEN 用户运行 `omx use claude 1`
- THEN OpenMux 尝试恢复写入前 credential 和 `oauthAccount`
- AND 返回包含回滚结果和 backup path 的错误
- AND 不将 `work` 标记为 active account。

### Requirement: Claude OAuth account 展示

OpenMux SHALL 在 account 列表中展示安全 metadata，且不得输出 raw credential。

#### Scenario: 查看 account 列表

- GIVEN 已导入两个 Claude OAuth accounts
- WHEN 用户运行 `omx list claude`
- THEN 输出包含 active marker、当前列表编号、名称、脱敏 email、plan 和 status
- AND 不输出 access token、refresh token 或 `.credentials.json` 内容。

### Requirement: Claude OAuth account 安全边界

OpenMux SHALL 只包装官方 Claude Code 登录流程或处理本机已有官方 Claude Code 登录产物，不实现自己的 Claude OAuth token exchange，也不调用 Anthropic 私有或未文档化 endpoint。

#### Scenario: 不自研 OAuth login

- GIVEN 用户没有可导入的 Claude Code 官方登录产物
- WHEN 用户运行 `omx import claude` 且没有提供 KV/JSON/TOML 配置内容
- THEN OpenMux 提示先使用官方 Claude Code 登录命令完成登录
- AND 不打开浏览器
- AND 不调用 token exchange 或 profile 私有 endpoint。
