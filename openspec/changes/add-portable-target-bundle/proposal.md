## Why

Prismux 现在已经能在本机管理 Codex accounts、Claude accounts 和 Claude profiles，但这些 target 只存在于本机 state root 与 plugin snapshot 目录中。用户已有 5 个账号/profile 时，换新电脑需要逐个重新登录或重新导入配置，既慢也容易把 alias、编号、active target 和 profile metadata 搞丢。

## What Changes

- 新增 portable target backup：将 Prismux 管理的 accounts/profiles、脱敏 metadata、secret snapshots 和 manifest 一次性导出到本地 credential-bearing backup 文件。
- 新增导入入口：在新电脑上读取 backup 文件，校验 manifest、schema version、snapshot hash 和目标平台兼容性后，重建 Prismux state 与 plugin snapshot 文件。
- CLI 使用 `backup` 命名空间表达 credential-bearing 迁移包：`prismux backup export`、`prismux backup import`、`prismux backup restore`。
- 支持全量导出和平台/target 子集导出，默认导出所有已管理 targets。
- `backup import` 默认不立即切换 active target；`backup restore` 才按 backup 中记录恢复 active target。
- 导入时保留 alias/name 等用户可见 metadata，并通过现有 hash/subject 规则做重复检测，避免重复导入。
- 不新增云同步、自动上传、跨设备加密密钥管理或第三方私有 API 调用。

## Capabilities

### New Capabilities

- `portable-target-backup`: Prismux 管理的 account/profile targets 的本地 backup 导出、导入、恢复、校验和安全边界。

### Modified Capabilities

- 无。

## Impact

- `crates/prismux-core`: 增加 backup manifest 类型、hash 校验、文件格式编解码和安全文件写入原语。
- `crates/prismux-app`: 增加 provider-agnostic backup export/import/restore orchestration 和 report DTO；CLI、Menubar、Desktop 都从这里调用，不在前端重复实现。
- `crates/prismux-cli`: 增加 `backup export` / `backup import` / `backup restore` 命令，输出导出/导入摘要且不打印 raw secret。
- `crates/prismux-plugin-codex`: 暴露已管理 account snapshot 的导出读取和导入写入路径，复用现有 save/use 安全规则。
- `crates/prismux-plugin-claude`: 暴露 Claude account/profile snapshot 的导出读取和导入写入路径，遵守 Keychain/plaintext credential 的现有边界。
- Menubar 与 Desktop：通过 `prismux-app` API 接入同一套 backup 逻辑；第一阶段 UI 可后置，但 API 边界必须先支持。
- 不新增运行时网络依赖；archive 格式优先使用 Rust 标准库与现有依赖能完成的最小实现。
