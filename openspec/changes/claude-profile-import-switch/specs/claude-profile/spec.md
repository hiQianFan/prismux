# Capability: Claude Profile Management

## ADDED Requirements

### Requirement: Claude profile 导入

OpenMux SHALL 支持通过 `omx import claude` 导入 Claude Code 中转/API profile。

#### Scenario: 导入 Anthropic-compatible gateway profile

- GIVEN 用户提供包含 `ANTHROPIC_BASE_URL` 和 `ANTHROPIC_AUTH_TOKEN` 的 KV 内容
- WHEN 用户运行 `omx import claude "<KV>"`
- THEN OpenMux 创建 Claude profile
- AND profile metadata 包含 base URL 和 auth type
- AND registry 不保存 raw `ANTHROPIC_AUTH_TOKEN`
- AND 命令输出包含 profile 编号和名称。

#### Scenario: 导入 Anthropic API key profile

- GIVEN 用户提供包含 `ANTHROPIC_API_KEY` 的 KV 内容
- WHEN 用户运行 `omx import claude "<KV>"`
- THEN OpenMux 创建 Claude profile
- AND auth type 为 `api-key`
- AND registry 不保存 raw API key。

#### Scenario: 重复导入

- GIVEN 已存在一个 Claude profile
- AND 用户再次导入等价的 profile 内容
- WHEN 用户运行 `omx import claude "<KV>"`
- THEN OpenMux 更新已有 profile
- AND 不创建新的 profile 编号。

### Requirement: Claude profile 切换

OpenMux SHALL 支持通过 `omx use claude <selector>` 将 Claude profile 应用到 Claude Code user settings。

#### Scenario: 按编号切换 profile

- GIVEN Claude profile `#1` 已导入
- WHEN 用户运行 `omx use claude 1`
- THEN OpenMux 将 profile 的 env 写入 `~/.claude/settings.json`
- AND 写入前备份原 settings
- AND 使用原子写入保存 settings
- AND 将 profile `#1` 标记为 active。

#### Scenario: 按名称切换 profile

- GIVEN Claude profile `gateway-work` 已导入
- WHEN 用户运行 `omx use claude gateway-work`
- THEN OpenMux 将 `gateway-work` 应用为 active profile。

#### Scenario: 保留非 OpenMux settings

- GIVEN `~/.claude/settings.json` 包含用户自定义权限、hooks 或其他 settings
- WHEN 用户运行 `omx use claude <selector>`
- THEN OpenMux 只更新 OpenMux 管理的 `env` keys
- AND 保留其他 settings 字段。

### Requirement: Claude profile 展示

OpenMux SHALL 在 `omx list claude` 中展示 Claude profiles。

#### Scenario: 查看 profile 列表

- GIVEN 已导入两个 Claude profiles
- WHEN 用户运行 `omx list claude`
- THEN 输出包含 active marker、编号、名称、base URL、auth type 和 model
- AND 不输出 raw secret。

### Requirement: OAuth account switching deferred

OpenMux SHALL 在第一阶段拒绝直接切换 Claude.ai/Console OAuth credentials，并提示用户该能力将在 Claude auth account 阶段实现。

#### Scenario: 不直接替换 Claude OAuth 凭据

- GIVEN 用户请求切换 Claude.ai OAuth 账号
- WHEN 当前实现阶段只包含 Claude profile
- THEN OpenMux 不读取或替换 macOS Keychain credential
- AND 不直接修改 `.credentials.json`
- AND 提示该能力属于 Claude auth account 阶段。
