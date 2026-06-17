## Why

OpenMux 已经支持 Codex 的账号池和中转 profile 导入，但 Claude Code 的认证与配置模型不同：Claude Code 的一方登录凭据由官方 CLI 管理，macOS 上存储在 Keychain，非 macOS 上存储在 `.credentials.json`，而中转、网关和 API key 场景主要通过环境变量和 settings 配置完成。

调研结论是：Claude Code 需要分成两个层次推进。第一阶段先支持中转/API profile 导入和切换，覆盖最常见的 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY` 和 gateway 需求；第二阶段继续推进 Claude.ai/Console OAuth account snapshot 导入和切换，但必须把 Keychain、`.credentials.json`、`oauthAccount` metadata、token refresh race 和备份恢复作为一等安全边界处理。

这不是简单复制 Codex 的 `auth.json` 方案。Claude Code 源码显示，OAuth login 使用 Authorization Code + PKCE，token 存储在 secure storage 的 `claudeAiOauth` 字段，macOS 优先 Keychain，非 macOS 使用 `<claude-home>/.credentials.json`；账号 metadata 则在 global config 的 `oauthAccount` 中。OpenMux 不能把 `settings.json` 切换误认为官方账号切换，也不能打印或普通化处理 OAuth token。

## What Changes

- 先补齐 OpenMux core/plugin API：明确区分 account、profile、credential backend 和 settings patch，不再让 profile 切换伪装成 account 切换失败后的 fallback。
- 抽出可复用的安全文件能力：私有目录、私有原子写入、snapshot hash、backup 路径和 hash 校验，供 Codex、Claude、Gemini 等 plugin 复用。
- 新增 Claude Code plugin 能力，用于发现 Claude Code home、user settings 和 credential backend。
- 第一阶段新增 `omx import claude "<KV-or-JSON>"`，导入 Claude Code 中转/API profile。
- 第一阶段新增 Claude profile registry，与 Codex profile registry 逻辑保持一致：profile 有编号、名称、provider/base URL/model metadata 和 active 状态。
- 第一阶段新增 `omx use claude <selector>`，当 selector 命中 profile 时，将 profile 应用到 Claude Code user settings。
- 第一阶段新增 `omx list claude` 展示 Claude profile 明细。
- 第二阶段新增 Claude OAuth account registry，用于导入当前官方登录快照、展示安全 metadata、切换 account snapshot。
- 第二阶段在统一 `claude` 入口下聚合 OAuth account：`omx login claude` 包装官方 Claude Code 登录并自动导入 account snapshot；`omx import claude` 没有配置内容时导入本机已有 Claude Code 官方登录产物；`omx use claude <selector>` 在 account/profile 中自动推断，唯一命中时执行对应切换。OpenMux 不自研第三方 OAuth token exchange。
- 不打印、不保存 raw API key 或 OAuth token 到 registry；profile/account snapshot 可在私有权限目录保存 secret payload，但 registry 只保存安全 metadata 和 hash。

## Capabilities

### New Capabilities

- `claude-profile`: Claude Code 中转/API profile 导入、展示和切换。
- `claude-auth-account`: Claude Code 官方 OAuth account snapshot 导入、展示和切换。

### Modified Capabilities

- 无。

## Impact

- 更新 `crates/omx-core` 的 profile/account 模型、platform capabilities 和 storage helper，让新 plugin 可以声明自己支持 profile、account、login、save 或 account import 的子集。
- 更新 `crates/omx-cli` 的路由和展示层，按 plugin capability 展示可用命令，避免 profile-only 平台被迫暴露 OAuth account 操作。
- 新增 `crates/omx-plugin-claude`，实现 Claude Code 平台适配。
- 新增 Claude profile registry、profile config 文件和备份/原子写入逻辑。
- 新增 Claude account registry、credential snapshot 文件、OAuth metadata 文件和备份/恢复逻辑。
- 更新 README、PRD、ARCHITECTURE 中关于 Claude Code profile 和 OAuth account 的边界、风险和操作说明。

## Research Notes

- `ChinaSiro/claude-code-sourcemap` 是从公开 npm 包 sourcemap 还原的 Claude Code TypeScript 源码。其 `restored-src/src/services/oauth/client.ts` 显示 OAuth 使用 Authorization Code + PKCE、token exchange、refresh token；`restored-src/src/utils/auth.ts` 显示 token 保存、读取、401 后强制刷新、跨进程锁和 cache invalidation；`restored-src/src/utils/secureStorage/*` 显示 macOS Keychain + plaintext fallback。
- `liuup/claude-code-analysis` 的分析文档把 OAuth 账户缓存、`src/services/oauth`、`src/services/mcp/auth.ts` 作为安全与用户数据分析对象，可作为二手索引；最终实现判断以 source map 还原源码为准。
- Claude Code 源码里的 `getClaudeAIOAuthTokens()` 读取顺序包括 `CLAUDE_CODE_OAUTH_TOKEN`、file descriptor token 和 secure storage。`CLAUDE_CODE_OAUTH_TOKEN` 只生成 inference-only token，没有 refresh token 和 expiresAt，不能作为完整 account snapshot。
- Claude Code secure storage 的 plaintext 后端使用 `<claude-home>/.credentials.json`，保存 JSON 并设置 `0600`；源码中的 macOS 路径使用 Keychain generic password，并有 stale-while-error cache。OpenMux 落地 credential backend trait、plaintext backend、测试 fake backend 和 macOS Security.framework Keychain backend；Keychain 写入通过内存 buffer，不通过命令行参数传递 payload。
- Claude Code 的账号 metadata 不是只存在 credential payload 中。`oauthAccount` 存在 global config，用于 `accountUuid`、`organizationUuid`、email、display name、billing/subscription 等信息。
- `farion1231/cc-switch` 对 Claude Code 的 provider 切换主要写 `~/.claude/settings.json` 的 `env`，例如 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`；这验证了 profile/gateway 切换是一条成熟路径，但不等同于官方 OAuth account 切换。
- `cc-switch` 对 Codex 明确区分 `auth.json` 和 `config.toml`：Codex 登录态留在 `auth.json`，provider 路由、endpoint、provider-scoped bearer token 写 `config.toml`，切换 provider 不应覆盖用户 ChatGPT 登录缓存。这个经验说明 OpenMux 也应区分 Claude profile 和 Claude OAuth account。
- 本提案不新增对 Anthropic 私有或未文档化 endpoint 的 API 调用。`omx login claude` 只调用官方 Claude Code CLI 登录；OAuth account 导入只读取本机已有官方 CLI 登录产物；切换只恢复 OpenMux 已导入的本地 snapshot。
