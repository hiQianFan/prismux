## 1. Core Backup Model

- [ ] 1.1 在 `prismux-core` 新增 portable backup manifest、target entry、export/import/restore report 领域类型。
- [ ] 1.2 实现 snapshot hex encode/decode、schema version 校验和 per-target SHA-256 校验。
- [ ] 1.3 增加私有原子写入 backup 文件的 helper，并默认拒绝覆盖已有输出路径。
- [ ] 1.4 增加 unit tests 覆盖 manifest roundtrip、hash mismatch、future schema 和 invalid hex。

## 2. Target Collection And Restore

- [ ] 2.1 从 `StateStore` 读取 accounts、profiles 和 active_targets，生成可导出的 target entries。
- [ ] 2.2 导出时按 `secret_ref` 读取 snapshot bytes，缺失或 hash 不匹配时失败且不创建 backup。
- [ ] 2.3 导入时将 snapshot 写入当前机器 provider snapshot 目录，并使用私有权限。
- [ ] 2.4 导入时复用 account/profile hash 与 provider subject 去重规则，更新已有 target 而不是重复创建。
- [ ] 2.5 处理 profile name 冲突：内容不同不得静默覆盖。

## 3. Provider Integration

- [ ] 3.1 为 Codex account snapshot 导出/导入提供最小 provider hook 或 core helper。
- [ ] 3.2 为 Claude profile snapshot 导出/导入提供最小 provider hook 或 core helper。
- [ ] 3.3 为 Claude OAuth account snapshot 导出/导入保留现有 credential backend 安全边界，导入只写 managed snapshot，不写 active credential。
- [ ] 3.4 对 unsupported provider 或 unsupported target kind 生成可读 skip/error reason。

## 4. App Orchestration

- [ ] 4.1 在 `prismux-app` 增加 provider-agnostic `export_backup`、`import_backup`、`restore_backup` API。
- [ ] 4.2 app API 使用现有 `OPERATION_LOCK` 串行化 backup import/restore 与账号切换。
- [ ] 4.3 app API 返回 frontend-safe report DTO：provider 摘要、新增/更新/跳过数量、resolved path、warning 和 activation 结果。
- [ ] 4.4 `restore_backup` 导入完成后调用现有 `use_target`，不新增直接写 active auth 的路径。
- [ ] 4.5 Menubar/Desktop 后续入口必须调用 app API，不能重新实现 backup 解析或 provider 写入。

## 5. CLI

- [ ] 5.1 增加 `prismux backup export --to <path> [--provider <id>] [--force]`。
- [ ] 5.2 增加 `prismux backup import <path>`。
- [ ] 5.3 增加 `prismux backup restore <path>`。
- [ ] 5.4 CLI 输出导出/导入/恢复摘要，不打印 snapshot payload 或 raw secret。
- [ ] 5.5 导出成功时输出 resolved absolute output path，并提示 backup 包含账号凭据。

## 6. Tests And Verification

- [ ] 6.1 增加 app-level tests：多 Codex accounts 导出后导入到空 state root。
- [ ] 6.2 增加 mixed targets test：Codex account + Claude profile 同 backup 导入。
- [ ] 6.3 增加 duplicate import test：第二次导入更新而不是新增编号。
- [ ] 6.4 增加 corrupt backup test：hash mismatch 不写 active auth。
- [ ] 6.5 增加 CLI smoke tests 覆盖 `backup export/import/restore` 参数语义。
- [ ] 6.6 运行 `cargo fmt --all`、`cargo test --locked`、`cargo clippy --all-targets --all-features -- -D warnings`。
