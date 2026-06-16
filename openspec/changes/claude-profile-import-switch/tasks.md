## 1. Phase 1 - Claude Profile 基础

- [ ] 1.1 新增 `crates/omx-plugin-claude` crate，并接入 workspace。
- [ ] 1.2 实现 Claude path discovery：测试注入路径、`$CLAUDE_CONFIG_DIR`、`~/.claude`。
- [ ] 1.3 实现 Claude user settings 读写：解析 JSON、保留未知字段、只修改 OpenMux 管理的 `env` keys。
- [ ] 1.4 实现 settings 写入安全流程：写入前备份、原子写入、私有权限、错误时保留 backup path。
- [ ] 1.5 设计并实现 Claude profile registry：schema version、active profile number、next number、profile metadata、secret hash、snapshot path。

## 2. Phase 1 - Profile Import/Use/List

- [ ] 2.1 实现 `omx import claude` 的 KV 输入解析。
- [ ] 2.2 实现 `omx import claude` 的 JSON/TOML 输入解析。
- [ ] 2.3 支持 `ANTHROPIC_BASE_URL`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_MODEL` 的 profile metadata 提取。
- [ ] 2.4 支持 Bedrock、Vertex、Foundry、gateway 相关 env 的安全 metadata 提取。
- [ ] 2.5 实现 profile 重复检测，基于 normalized metadata 和 secret hash 更新已有 profile。
- [ ] 2.6 实现 `omx use claude <selector>`：按 number/name 解析、应用 profile 到 user settings `env`、清理旧 profile 管理的 env、更新 active profile。
- [ ] 2.7 实现 `omx list claude` profile 详情输出，并确保不输出 raw secret。
- [ ] 2.8 更新全局 `omx list`，让 Claude 行展示 active profile、profile 数量和 status。

## 3. Phase 1 - Profile 测试

- [ ] 3.1 增加 Anthropic-compatible gateway profile 导入测试。
- [ ] 3.2 增加 API key profile 和 bearer token profile 导入测试。
- [ ] 3.3 增加重复 profile 不追加编号测试。
- [ ] 3.4 增加 `use claude` 写入 settings env、保留未知字段、备份旧 settings 测试。
- [ ] 3.5 增加 registry 不保存 raw secret 测试。
- [ ] 3.6 增加 selector 支持 number/name 和无 profile 时可操作提示测试。

## 4. Phase 2 - Claude OAuth Account 基础

- [ ] 4.1 设计并实现 Claude account registry：schema version、active account number、next number、safe metadata、credential snapshot hash、oauthAccount metadata path。
- [ ] 4.2 实现 credential backend trait：macOS Keychain backend、plaintext `.credentials.json` backend、测试 fake backend。
- [ ] 4.3 实现 Claude secure storage payload 解析，只识别 `claudeAiOauth`，并拒绝 inference-only `CLAUDE_CODE_OAUTH_TOKEN` 作为完整 account。
- [ ] 4.4 实现 `oauthAccount` metadata 读写抽象，保留未知 global config 字段。
- [ ] 4.5 实现 account snapshot 私有权限写入、hash 校验、重复检测和 metadata 脱敏。

## 5. Phase 2 - Account Import/Use/List

- [ ] 5.1 实现 `omx import claude-account [--name <name>]`，从本机已有 Claude Code 官方登录产物导入 account snapshot。
- [ ] 5.2 实现导入输出：编号、名称、脱敏 email/organization、expiresAt，不输出 token。
- [ ] 5.3 实现 `omx use claude-account <selector>`：校验 snapshot hash、备份当前 credential 和 `oauthAccount`、写入目标 account、更新 active account。
- [ ] 5.4 实现 account 切换失败回滚：credential 写入失败、`oauthAccount` 写入失败、hash mismatch 均有明确错误和恢复路径。
- [ ] 5.5 实现 `omx list claude-account`，展示 active marker、编号、名称、脱敏 email、auth type、expiresAt。
- [ ] 5.6 更新全局 `omx list`，在第二阶段展示 Claude active account 和 account 数量。

## 6. Phase 2 - Account 测试

- [ ] 6.1 增加 plaintext `.credentials.json` account 导入测试，验证 registry 不保存 raw token。
- [ ] 6.2 增加 fake Keychain backend account 导入和切换测试。
- [ ] 6.3 增加缺少 refresh token、缺少 expiresAt、inference-only env token 的拒绝导入测试。
- [ ] 6.4 增加 account 切换备份、原子写入、`0600` 权限和回滚测试。
- [ ] 6.5 增加 `oauthAccount` metadata 保留未知字段和写入失败回滚测试。
- [ ] 6.6 增加 snapshot hash mismatch 拒绝切换测试。

## 7. 文档与验证

- [ ] 7.1 更新 README，说明 Claude profile 和 Claude OAuth account 是两个不同层次。
- [ ] 7.2 更新 PRD，记录 Phase 1 profile 先行、Phase 2 account auth 后续完整推进。
- [ ] 7.3 更新 ARCHITECTURE，记录 Claude secure storage backend、registry、snapshot、备份恢复和禁止私有 endpoint 的安全边界。
- [ ] 7.4 运行 `cargo fmt --all`。
- [ ] 7.5 运行 `cargo test`。
- [ ] 7.6 运行 `cargo clippy --all-targets --all-features`。
