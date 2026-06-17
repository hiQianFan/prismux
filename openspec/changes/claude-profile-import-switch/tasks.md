## 0. Core/CLI 可复用基础

- [x] 0.1 扩展 `omx-core` platform capabilities，让 plugin 显式声明 accounts、login、save、profiles、profile import、account import 能力。
- [x] 0.2 扩展 profile 领域模型，支持 profile number、auth type、active profile 和安全 metadata 摘要。
- [x] 0.3 抽出 shared storage helper：私有目录、私有原子写入、snapshot hash、hash 校验和路径展示。
- [x] 0.4 调整 CLI 路由：profile use 与 account use 不再通过 fallback 混用；selector 歧义时返回明确错误。
- [x] 0.5 调整 CLI 展示：按 capability 展示 account/profile section，移除 Codex-only 文案。
- [x] 0.6 为 alias/profile 同名、snapshot hash mismatch、registry 保存失败回滚和临时登录目录清理增加回归测试。
- [x] 0.7 聚合平台 UX：`omx list/use/import claude` 统一展示和分发 account/profile，selector 唯一命中时自动推断，同时命中时返回歧义错误。
- [x] 0.8 实现聚合 target resolver：`omx list claude` 分组展示 accounts/profiles 但使用连续选择编号，`omx use claude <number>` 按当前展示编号解析到底层 account/profile selector。

## 1. Phase 1 - Claude Profile 基础

- [x] 1.1 新增 `crates/omx-plugin-claude` crate，并接入 workspace。
- [x] 1.2 实现 Claude path discovery：测试注入路径、`$CLAUDE_CONFIG_DIR`、`~/.claude`。
- [x] 1.3 实现 Claude user settings 读写：解析 JSON、保留未知字段、只修改 OpenMux 管理的 `env` keys。
- [x] 1.4 实现 settings 写入安全流程：写入前备份、原子写入、私有权限、错误时保留 backup path。
- [x] 1.5 设计并实现 Claude profile registry：schema version、active profile number、next number、profile metadata、secret hash、snapshot path。

## 2. Phase 1 - Profile Import/Use/List

- [x] 2.1 实现 `omx import claude` 的 KV 输入解析。
- [x] 2.2 实现 `omx import claude` 的 JSON/TOML 输入解析。
- [x] 2.3 支持 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL` 的 profile metadata 提取。
- [x] 2.4 支持 Bedrock、Vertex、Foundry、gateway 相关 env 的安全 metadata 提取。
- [x] 2.5 实现 profile 重复检测，基于 normalized metadata 和 secret hash 更新已有 profile。
- [x] 2.6 实现 `omx use claude <selector>`：按 number/name 解析、应用 profile 到 user settings `env`、清理旧 profile 管理的 env、更新 active profile。
- [x] 2.7 实现 `omx list claude` profile 详情输出，并确保不输出 raw secret。
- [x] 2.8 更新全局 `omx list`，让 Claude 行展示 active profile、profile 数量和 status。
- [x] 2.9 确保 `omx use claude <selector>` 统一解析 account/profile selector，唯一命中 profile 时不替换 OAuth credential，同时命中时返回歧义错误。

## 3. Phase 1 - Profile 测试

- [x] 3.1 增加 Anthropic-compatible gateway profile 导入测试。
- [x] 3.2 增加 API key profile 和 bearer token profile 导入测试。
- [x] 3.3 增加重复 profile 不追加编号测试。
- [x] 3.4 增加 `use claude` 写入 settings env、保留未知字段、备份旧 settings 测试。
- [x] 3.5 增加 registry 不保存 raw secret 测试。
- [x] 3.6 增加 selector 支持 number/name 和无 profile 时可操作提示测试。

## 4. Phase 2 - Claude OAuth Account 基础

- [x] 4.1 设计并实现 Claude account registry：schema version、active account number、next number、safe metadata、credential snapshot hash、oauthAccount metadata path。
- [x] 4.2 实现 credential backend trait：macOS Keychain backend、plaintext `.credentials.json` backend、测试 fake backend。
- [x] 4.3 实现 Claude secure storage payload 解析，只识别 `claudeAiOauth`，并拒绝 inference-only `CLAUDE_CODE_OAUTH_TOKEN` 作为完整 account。
- [x] 4.4 实现 `oauthAccount` metadata 读写抽象，保留未知 global config 字段。
- [x] 4.5 实现 account snapshot 私有权限写入、hash 校验、重复检测和 metadata 脱敏。

## 5. Phase 2 - Account Import/Use/List

- [x] 5.0 实现 `omx login claude [--alias <name>] [--use]`：调用官方 `claude auth login`，登录成功后导入 account snapshot，并可立即切换。
- [x] 5.1 实现 `omx import claude [--name <name>]` 在无配置内容时从本机已有 Claude Code 官方登录产物导入 account snapshot。
- [x] 5.2 实现导入输出：编号、名称、脱敏 email/organization、expiresAt，不输出 token。
- [x] 5.3 实现 `omx use claude <selector>` 唯一命中 account 时：校验 snapshot hash、备份当前 credential 和 `oauthAccount`、写入目标 account、更新 active account。
- [x] 5.4 实现 account 切换失败回滚：credential 写入失败、`oauthAccount` 写入失败、hash mismatch 均有明确错误和恢复路径。
- [x] 5.5 实现 `omx list claude` 的 account section，展示 active marker、编号、名称、脱敏 email、auth type、expiresAt。
- [x] 5.6 更新全局 `omx list`，在第二阶段展示 Claude active account 和 account 数量。

## 6. Phase 2 - Account 测试

- [x] 6.1 增加 plaintext `.credentials.json` account 导入测试，验证 registry 不保存 raw token。
- [x] 6.2 增加 fake Keychain backend account 导入和切换测试。
- [x] 6.3 增加缺少 refresh token、缺少 expiresAt、inference-only env token 的拒绝导入测试。
- [x] 6.4 增加 account 切换备份、原子写入、`0600` 权限和回滚测试。
- [x] 6.5 增加 `oauthAccount` metadata 保留未知字段和写入失败回滚测试。
- [x] 6.6 增加 snapshot hash mismatch 拒绝切换测试。

## 7. 文档与验证

- [x] 7.1 更新 README，说明 Claude profile 和 Claude OAuth account 是两个不同层次。
- [x] 7.2 更新 PRD，记录 Phase 1 profile 先行、Phase 2 account auth 后续完整推进。
- [x] 7.3 更新 ARCHITECTURE，记录 Claude secure storage backend、registry、snapshot、备份恢复和禁止私有 endpoint 的安全边界。
- [x] 7.4 运行 `cargo fmt --all`。
- [x] 7.5 运行 `cargo test`。
- [x] 7.6 运行 `cargo clippy --all-targets --all-features`。
