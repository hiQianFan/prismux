# Capability: Claude Profile Management

## ADDED Requirements

### Requirement: Plugin capability 边界

OpenMux SHALL 让 Claude plugin 明确声明 profile 与 account 能力，CLI SHALL 只展示和调用当前阶段支持的能力。

#### Scenario: 统一入口展示 profile 与 account 能力

- GIVEN Claude profile 与 OAuth account 能力均已实现
- WHEN 用户查看 Claude 相关命令或运行 `omx list claude`
- THEN CLI 展示 Claude profiles 和 accounts
- AND `omx login claude` 作为 account 添加入口
- AND `omx import claude "<KV>"` 作为 profile 添加入口。

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

#### Scenario: 按当前列表编号切换 profile

- GIVEN 已导入两个 Claude OAuth accounts
- AND Claude profile `gateway-work` 已导入
- AND `omx list claude` 将该 profile 展示为 `#3`
- WHEN 用户运行 `omx use claude 3`
- THEN OpenMux 将 profile 的 env 写入 `~/.claude/settings.json`
- AND 写入前备份原 settings
- AND 使用原子写入保存 settings
- AND 将 `gateway-work` 标记为 active profile。

#### Scenario: 按名称切换 profile

- GIVEN Claude profile `gateway-work` 已导入
- WHEN 用户运行 `omx use claude gateway-work`
- THEN OpenMux 将 `gateway-work` 应用为 active profile。

#### Scenario: 名称不与 OAuth account selector 混用

- GIVEN Claude profile `work` 已导入
- AND Claude OAuth account alias `work` 也已导入
- WHEN 用户运行 `omx use claude work`
- THEN OpenMux 返回 selector 歧义错误
- AND 不应用 profile `work`
- AND 不切换 OAuth account。

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
- THEN 输出按 accounts 与 profiles 分组展示
- AND profile section 包含 active marker、当前列表编号、名称、base URL、auth type 和 model
- AND profiles 的当前列表编号接在 account 编号之后
- AND 不输出 raw secret。

### Requirement: OAuth account switching deferred

OpenMux SHALL 在第一阶段拒绝直接切换 Claude.ai/Console OAuth credentials，并提示用户该能力将在 Claude auth account 阶段实现。

#### Scenario: 不直接替换 Claude OAuth 凭据

- GIVEN 用户请求切换 Claude.ai OAuth 账号
- WHEN 当前实现阶段只包含 Claude profile
- THEN OpenMux 不读取或替换 macOS Keychain credential
- AND 不直接修改 `.credentials.json`
- AND 提示该能力属于 Claude auth account 阶段。
