# 设计

## 调研结论

Claude Code 与 Codex 的关键差异在认证存储：

- Codex 当前实现依赖 `CODEX_HOME/auth.json`，可以通过 snapshot + 原子替换实现账号切换。
- Claude Code 源码说明一方 OAuth 凭据由 secure storage 管理：
  - macOS：`security` CLI 读写 Keychain generic password；
  - 非 macOS：`<claude-home>/.credentials.json`，权限 `0600`；
  - secure storage payload 中的关键字段是 `claudeAiOauth`，包含 `accessToken`、`refreshToken`、`expiresAt`、`scopes`、`subscriptionType`、`rateLimitTier`；
  - global config 中的 `oauthAccount` 保存 `accountUuid`、`organizationUuid`、email、display name、billing/subscription metadata。
- Claude Code 的中转/API 场景有明确官方配置面：
  - `settings.json` 的 `env`；
  - `ANTHROPIC_BASE_URL`；
  - `ANTHROPIC_AUTH_TOKEN`；
  - `ANTHROPIC_API_KEY`；
  - `apiKeyHelper`；
  - Bedrock/Vertex/Foundry 等 provider-specific env。
- `CLAUDE_CODE_OAUTH_TOKEN` 在源码中被当作 inference-only token，缺少 refresh token 和 expiresAt，不应被 OpenMux 当作完整官方账号。
- `cc-switch` 的 Claude Code provider 切换主要写 `~/.claude/settings.json` 的 env；它没有把 `settings.json` 切换等价为 Claude.ai OAuth account 切换。其 Codex 逻辑反而明确区分 `auth.json` 登录态和 `config.toml` provider 路由。

因此本变更采用两阶段设计：第一阶段实现 Claude profile import/switch；第二阶段在同一能力线继续实现 Claude.ai/Console OAuth account snapshot import/switch，但账号切换必须独立于 profile，并使用更严格的安全、备份和验证流程。

## 用户模型

OpenMux 对 Claude Code 建模为两个层次：

```text
Claude Platform
  profiles:
    - number: 1
      name: gateway-work
      kind: anthropic-compatible
      base_url: https://gateway.example.com
      auth_env: ANTHROPIC_AUTH_TOKEN
      model: optional
      active: true

  accounts:
    - number: 1
      name: work-max
      kind: claude-ai-oauth
      email: user@example.com
      account_uuid: redacted metadata
      organization_uuid: redacted metadata
      scopes: user:inference ...
      active: true
```

第一阶段 `omx list claude` 展示 profiles。第二阶段新增 `omx list claude-account` 或在 `omx list claude --accounts` 中展示 OAuth accounts；默认输出只展示安全 metadata，不输出 token、refresh token 或完整 credential JSON。

## 路径发现

Claude home 解析顺序：

1. 测试注入路径。
2. `$CLAUDE_CONFIG_DIR`，用于 Linux/Windows 官方 credential location；对 settings 也可作为 OpenMux 测试隔离目录。
3. `~/.claude`。

Claude user settings：

```text
<claude-home>/settings.json
```

OpenMux state：

```text
<openmux-state>/platforms/claude/
  registry.omx
  profiles/<number>.settings.json
  profiles/<number>.env.omx
  accounts/<number>.credentials.snapshot
  accounts/<number>.oauth-account.json
  backups/settings.json.bak.<timestamp>
  backups/credentials.snapshot.bak.<timestamp>
```

`registry.omx` 保存 version、next number、active profile、active account、安全 metadata、snapshot path 和 secret hash。raw secret 只允许存在于私有权限 snapshot 文件，不进入 registry 和 stdout。

## Import Flow

`omx import claude "<KV-or-JSON>"`：

1. 接收命令尾部内容、`--file`、`@path`、`--clipboard` 或 stdin。
2. 解析 KV 或 JSON/TOML。
3. 识别 Claude Code 相关字段：
   - `ANTHROPIC_BASE_URL`
   - `ANTHROPIC_AUTH_TOKEN`
   - `ANTHROPIC_API_KEY`
   - `ANTHROPIC_MODEL`
   - `CLAUDE_CODE_USE_BEDROCK`
   - `CLAUDE_CODE_USE_VERTEX`
   - `CLAUDE_CODE_USE_FOUNDRY`
   - provider-specific base URL 和 skip-auth env。
4. 生成 profile snapshot。
5. registry 只保存安全 metadata：
   - profile number/name；
   - provider kind；
   - base URL；
   - model；
   - auth type：`bearer-token`、`api-key`、`api-key-helper`、`cloud-provider`；
   - secret key names，不保存 raw secret value。
6. raw secret 如果必须持久化，写入私有权限 profile snapshot，不进入 registry 和 stdout。
7. 基于 normalized metadata + secret hash 做重复检测。
8. 输出导入的 profile 编号和名称。

默认 profile name：

- 如果输入包含显式 `OMUX_PROFILE` 或 `name`，使用该名称。
- 否则从 base URL host 派生，例如 `gateway-example-com`。
- 若冲突，追加编号后缀。

## Use Flow

`omx use claude <selector>`：

1. selector 优先解析 profile number。
2. 再按 profile name 精确匹配。
3. 读取目标 profile snapshot。
4. 读取 `<claude-home>/settings.json`；不存在则创建最小 settings。
5. 在写入前备份当前 settings。
6. 更新 user settings 的 `env` 字段：
   - 设置 profile 中的 `ANTHROPIC_BASE_URL` 等 env；
   - 设置 `ANTHROPIC_AUTH_TOKEN` 或 `ANTHROPIC_API_KEY`，如果 profile snapshot 保存了 secret；
   - 对 profile 管理的旧 key 做清理，避免旧 profile 残留。
7. 使用原子写入保存 settings。
8. 更新 registry active profile。
9. 输出 active profile 和 settings path。

## OAuth Account Import Flow

`omx import claude-account [--name <name>]`：

1. 解析 Claude home 和 user/global config path。
2. 检测当前官方登录状态：
   - macOS：通过受控的 Keychain 读路径读取 Claude Code secure storage payload；
   - 非 macOS：读取 `<claude-home>/.credentials.json`；
   - 如果只存在 `CLAUDE_CODE_OAUTH_TOKEN` 这类 inference-only env token，拒绝导入为 account，并提示它不是完整 OAuth account。
3. 校验 payload 中存在 `claudeAiOauth.accessToken`、`refreshToken`、`expiresAt` 和 scopes。
4. 读取 `oauthAccount` metadata；若缺失，不调用私有 endpoint 补全，只将 metadata 标记为 partial，并提示用户可先运行官方 `claude auth login` 刷新。
5. 生成 account snapshot：
   - `accounts/<number>.credentials.snapshot` 保存 secure storage payload；
   - `accounts/<number>.oauth-account.json` 保存 `oauthAccount` metadata；
   - 文件权限设置为私有；
   - registry 只保存 email、account/org UUID 的短 hash、scopes 摘要、expiresAt、snapshot hash 和 partial flag。
6. 基于 refresh token hash、account UUID hash 和 organization UUID hash 做重复检测。
7. 输出 account 编号、名称、email/organization 摘要、到期时间；不输出 token。

## OAuth Account Use Flow

`omx use claude-account <selector>`：

1. selector 按 number/name 解析到 account。
2. 读取目标 account snapshot，并校验 snapshot hash。
3. 在写入前备份当前 secure storage payload 和 `oauthAccount` metadata：
   - macOS 备份当前 Keychain payload 到 OpenMux 私有 backup snapshot；
   - 非 macOS 备份 `.credentials.json`；
   - 备份 global config 中的 `oauthAccount`。
4. 写入目标 account：
   - macOS 使用受控 Keychain 写路径恢复 secure storage payload；
   - 非 macOS 使用原子写入恢复 `.credentials.json` 并设置 `0600`；
   - 恢复或更新 global config 的 `oauthAccount` metadata。
5. 更新 registry active account。
6. 输出 active account metadata 和 credential backend；不输出 token。

失败处理：

- secure storage 写入失败时必须恢复备份，或明确报告 backup path。
- `oauthAccount` 写入失败时必须回滚 credential snapshot，避免 token 与 metadata 不一致。
- snapshot hash 不匹配时拒绝切换。
- 不执行 `/login`、不打开浏览器、不调用 Anthropic 私有 endpoint。

## List Flow

`omx list`：

- Claude 行显示平台级 overview：
  - active profile；
  - active account（第二阶段）；
  - profile 数量；
  - account 数量（第二阶段）；
  - status。

`omx list claude`：

```text
Claude profiles: 2 total, active gateway-work
╭───┬───┬──────────────┬─────────────────────────────┬──────────────┬────────╮
│ * ┆ # ┆ Name         ┆ Base URL                    ┆ Auth         ┆ Model  │
╞═══╪═══╪══════════════╪═════════════════════════════╪══════════════╪════════╡
│ * ┆ 1 ┆ gateway-work ┆ https://gateway.example.com ┆ bearer-token ┆ sonnet │
│ - ┆ 2 ┆ api-direct   ┆ -                           ┆ api-key      ┆ -      │
╰───┴───┴──────────────┴─────────────────────────────┴──────────────┴────────╯
```

`omx list claude-account`：

```text
Claude accounts: 2 total, active work-max
╭───┬───┬──────────┬──────────────────┬──────────────┬────────────╮
│ * ┆ # ┆ Name     ┆ Email            ┆ Auth         ┆ Expires    │
╞═══╪═══╪══════════╪══════════════════╪══════════════╪════════════╡
│ * ┆ 1 ┆ work-max ┆ u***@example.com ┆ oauth/full   ┆ 2026-06-17 │
│ - ┆ 2 ┆ personal ┆ p***@example.com ┆ oauth/full   ┆ 2026-06-17 │
╰───┴───┴──────────┴──────────────────┴──────────────┴────────────╯
```

`omx list claude` 可以在第二阶段显示 profile 与 account 的 summary，但不默认展开 credential-sensitive metadata。

## Phase Sequencing

### Phase 1: Profile

先实现 `claude-profile`，只操作 `settings.json` 的 user scope env。该阶段不读取 Keychain、不读取 `.credentials.json`，不会影响官方 Claude.ai 登录。

### Phase 2: Account Auth

在 profile 逻辑稳定后实现 `claude-auth-account`。该阶段只导入本机已有的官方登录产物，不创建新的 OAuth login flow，不调用私有 endpoint。实现时需要先完成平台 credential backend 抽象、snapshot 权限策略、回滚测试和跨平台手工验证。

## 安全规则

- 不打印 raw API key、bearer token、OAuth token 或 `.credentials.json` 内容。
- registry 不保存 raw secret。
- profile snapshot 和 backup 使用私有权限。
- account snapshot 和 backup 使用私有权限；后续如引入 OS keyring/加密封装，不改变 registry schema。
- settings 替换使用原子写入。
- `.credentials.json` 替换使用原子写入，并设置 `0600`。
- macOS Keychain 读写必须通过独立 backend 封装，禁止在日志、错误、命令行参数中暴露 payload。
- 修改 settings 前必须备份。
- 修改 credential 和 `oauthAccount` 前必须备份。
- 不覆盖用户非 OpenMux 管理的 settings 字段。
- 不在项目 `.claude/settings.json` 写入个人 secret；只修改 user scope settings。
- 不新增对 Anthropic 私有或未文档化 endpoint 的 API 调用。
