## ADDED Requirements

### Requirement: Claude OAuth account 导入

OpenMux SHALL 支持从本机已有 Claude Code 官方登录产物导入 Claude.ai/Console OAuth account snapshot。

#### Scenario: 从 plaintext credentials 导入 account

- GIVEN 当前 Claude home 存在 `.credentials.json`
- AND `.credentials.json` 包含 `claudeAiOauth.accessToken`、`refreshToken`、`expiresAt` 和 scopes
- WHEN 用户运行 `omx import claude-account --name work`
- THEN OpenMux 创建 Claude account snapshot
- AND registry 保存脱敏 metadata 和 snapshot hash
- AND registry 不保存 raw access token 或 refresh token
- AND 命令输出包含 account 编号、名称和脱敏账号信息。

#### Scenario: 从 macOS Keychain 导入 account

- GIVEN 当前平台为 macOS
- AND Claude Code secure storage 在 Keychain 中存在完整 `claudeAiOauth`
- WHEN 用户运行 `omx import claude-account --name work`
- THEN OpenMux 通过 credential backend 导入 account snapshot
- AND 不在 stdout、日志或 registry 中输出 Keychain payload。

#### Scenario: 拒绝 inference-only token

- GIVEN 当前认证来源只有 `CLAUDE_CODE_OAUTH_TOKEN`
- WHEN 用户运行 `omx import claude-account`
- THEN OpenMux 拒绝导入 account
- AND 提示该 token 缺少 refresh token 和 expiresAt，不能作为完整 OAuth account snapshot。

### Requirement: Claude OAuth account 切换

OpenMux SHALL 支持通过 `omx use claude-account <selector>` 切换已导入的 Claude OAuth account snapshot。

#### Scenario: 按编号切换 account

- GIVEN Claude account `#1` 已导入
- WHEN 用户运行 `omx use claude-account 1`
- THEN OpenMux 校验 account snapshot hash
- AND 备份当前 credential payload 和 `oauthAccount` metadata
- AND 写入目标 account snapshot
- AND 将 account `#1` 标记为 active。

#### Scenario: 按名称切换 account

- GIVEN Claude account `work` 已导入
- WHEN 用户运行 `omx use claude-account work`
- THEN OpenMux 将 `work` 应用为 active account。

#### Scenario: snapshot 校验失败

- GIVEN Claude account snapshot 文件与 registry 中记录的 hash 不匹配
- WHEN 用户运行 `omx use claude-account <selector>`
- THEN OpenMux 拒绝切换
- AND 不修改当前 Claude credential。

#### Scenario: 切换失败回滚

- GIVEN Claude account `#1` 已导入
- AND 写入目标 credential 或 `oauthAccount` metadata 时失败
- WHEN 用户运行 `omx use claude-account 1`
- THEN OpenMux 恢复写入前备份
- AND 返回包含恢复状态的错误
- AND 不将 account `#1` 标记为 active。

### Requirement: Claude OAuth account 展示

OpenMux SHALL 在 account 列表中展示安全 metadata，且不得输出 raw credential。

#### Scenario: 查看 account 列表

- GIVEN 已导入两个 Claude OAuth accounts
- WHEN 用户运行 `omx list claude-account`
- THEN 输出包含 active marker、编号、名称、脱敏 email、auth type 和 expiresAt
- AND 不输出 access token、refresh token 或 `.credentials.json` 内容。

### Requirement: Claude OAuth account 安全边界

OpenMux SHALL 只处理本机已有官方 Claude Code 登录产物，不实现新的 Claude OAuth 登录流程，也不调用 Anthropic 私有或未文档化 endpoint。

#### Scenario: 不发起 OAuth login

- GIVEN 用户没有可导入的 Claude Code 官方登录产物
- WHEN 用户运行 `omx import claude-account`
- THEN OpenMux 提示先使用官方 Claude Code 登录命令完成登录
- AND 不打开浏览器
- AND 不调用 token exchange 或 profile 私有 endpoint。
