## Context

当前 Prismux 的可迁移资产分成两类：

- state store：`prismux.sqlite` 中的 accounts、profiles、active_targets、usage/refresh 历史。
- plugin snapshots：Codex `auth.json` snapshot、Claude OAuth credential snapshot、Claude profile snapshot 等 secret-bearing 文件，路径通过 `secret_ref` 记录在 state store。

本变更只迁移 Prismux 已管理的 targets，不迁移工具运行时目录、会话历史、usage cache、backup 文件或 Menubar 设置。目标用户是“本机已有 5 个账号/profile，换新电脑后想一次导入继续用”。

GitHub 调研结论：

- `Loongphy/codex-auth` 已实现 `export`/`import`：导出 stored account auth snapshots 到目录，目录可再次导入；`import --purge` 可以从 snapshot 文件重建 registry。
- `Sls0n/codex-account-switcher` 的核心模型更简单：保存 `~/.codex/auth.json` 的 named snapshots，切换时复制或 symlink 到 active auth path。
- `opencue/authmux` 同样围绕 Codex auth snapshot，额外做 per-terminal session memory；Claude parallel accounts 采用每个 profile 一个 config dir，并用 atomic write/0600 同步 auth 文件。
- `Nemo-Illusionist/claude-code-account-switcher` 的 Claude 方案是每个 account 一个完整 `CLAUDE_CONFIG_DIR`，导入现有 config dir 时会处理 macOS Keychain 路径 re-key；这适合目录隔离，但比 Prismux 当前 snapshot/registry 模型更重。
- `CarsonYong/claude-profile` 用 `CLAUDE_CONFIG_DIR` + `direnv` 做项目级自动切换，证明 Claude profile/config-dir 隔离可行，但没有通用的 Prismux state 迁移格式。

第一性原理结论：跨机迁移要迁移的是 Prismux 的“管理事实”和“secret snapshot bytes”，不是当前 active tool home。新机器导入后再通过现有 `use_target` 写入对应工具的 active path，安全边界更清楚。

## Goals / Non-Goals

**Goals:**

- 一个命令导出当前 Prismux 管理的 accounts/profiles 到单个本地 backup 文件。
- 一个命令在新电脑导入 backup，重建 target metadata 和 snapshot 文件。
- 导入时校验 schema version、manifest checksum、每个 snapshot hash、provider 能力和重复 target。
- `backup import` 默认不切换 active target；`backup restore` 才恢复 backup 记录的 active target。
- backup 能力由 `prismux-app` 统一编排，CLI、Menubar、Desktop 共享同一套行为和 report。
- 不打印 raw auth、API key、OAuth credential 或 snapshot payload。

**Non-Goals:**

- 不做云同步、远程分享、自动上传或设备发现。
- 不在第一版实现加密 backup；导出文件本身包含 secret，必须按私有文件处理。
- 不导出 usage 历史、refresh attempts、session logs、Claude projects、Codex sessions 或 backups。
- 不调用 OpenAI/Anthropic 私有 endpoint 来补全身份；只使用本地已有 metadata。
- 不要求 Menubar UI 第一阶段提供按钮；但 `prismux-app` API 必须先具备，避免 CLI-only 设计。

## Decisions

### 1. Backup 使用单文件 JSON，payload 用 hex 编码

格式示意：

```json
{
  "schema_version": 1,
  "created_at_unix": 1783440000,
  "created_by": "prismux 0.2.2",
  "active_targets": [
    { "provider": "codex", "target_kind": "account", "local_id": "..." }
  ],
  "targets": [
    {
      "provider": "codex",
      "target_kind": "account",
      "display_number": 1,
      "alias": "work",
      "name": null,
      "metadata": { "auth_type": "chatgpt" },
      "snapshot_sha256": "...",
      "snapshot_hex": "7b226f..."
    }
  ]
}
```

选择 JSON + hex 是为了不新增 zip/tar/base64 依赖，同时保留单文件“打包”体验。snapshots 通常是几个 KB，5 个账号/profile 的体积膨胀可以接受。后续如果要迁移大量 Claude config dir，再独立引入压缩 archive。

替代方案：

- 目录 backup：实现最简单，但不满足“一键打包成文件”的心智。
- zip/tar：更标准，但当前 workspace 没有依赖；第一版为小 payload 引入 archive 依赖不划算。
- 加密 archive：安全体验更好，但自研加密不可接受，引入密钥管理会扩大范围。

### 2. 只导出 managed targets，不导出整个 state root

导出逻辑从 `StateStore` 读取 accounts/profiles 和 active_targets，再按 `secret_ref` 读取 snapshot bytes。导入逻辑重新写入当前机器的 plugin snapshot 目录，并通过 `StateStore` upsert accounts/profiles。

这样不会把旧机器的绝对路径、WAL 文件、usage 历史或 backup 垃圾带到新机器，也避免 SQLite 跨版本直接复制。

### 3. 导入先落 snapshot，再 upsert metadata

导入每个 target：

1. 解码 snapshot bytes。
2. 校验 `sha256(snapshot_bytes) == snapshot_sha256`。
3. 写入当前机器 provider snapshot 目录，使用私有权限和原子写入。
4. 调用 provider-specific import hook 或 `StateStore` upsert，保留 alias/name、auth/profile metadata。
5. 遇到相同 provider subject/hash 或相同 snapshot hash 时更新已有 target，不分配新编号。

导入失败不能写入 active tool home；已写入的 managed snapshot 可以保留并在错误中报告，或在事务失败时删除。active 切换只能发生在全部导入成功之后。

### 4. Core / App / Frontend 分层

这个能力不是 CLI 私有逻辑。分层如下：

- `prismux-core`：定义 backup schema、target entry、manifest 编解码、hash 校验、secret-bearing 文件的私有原子写入 helper，以及导入/导出 report 的领域类型。core 不知道 CLI/Menubar，也不直接决定按钮文案。
- provider plugins：负责 provider-specific snapshot 边界。Codex 暴露 account snapshot 读写；Claude 暴露 profile/account snapshot 读写，并保留 Keychain/plaintext credential 的现有安全约束。
- `prismux-app`：唯一的 orchestration 层。它持有 `OPERATION_LOCK`，从 plugins + `StateStore` 收集 targets，调用 core 编解码，执行导入去重，必要时调用现有 `use_target` 恢复 active target，并生成 frontend-safe report DTO。
- `prismux-cli`：只是 `prismux-app` backup API 的文本入口，负责参数解析、确认提示和摘要展示。
- Menubar/Desktop：后续通过同一个 `prismux-app` backup API 打开保存/选择文件面板，并展示同一份 report；不能重新实现 backup 解析、hash 校验或 provider 写入。

这样后续 UI 增加导入/导出按钮时，不会出现 CLI 和 Menubar 行为不一致。

### 5. Active target 恢复必须显式

默认 `prismux backup import file.pmxbundle.json` 只导入 targets，不替换 `~/.codex/auth.json`、Claude Keychain 或 `settings.json`。用户运行 `prismux backup restore file.pmxbundle.json` 时，导入完成后按 manifest 中的 active_targets 调用现有 `use_target`。

这复用现有切换的备份、hash 校验和回滚逻辑，避免 backup import 成为第二套危险写入路径。

### 6. CLI 命令语义

CLI 使用 `backup` 命名空间，而不是顶层 `export` / `import-bundle`：

```sh
prismux backup export --to ~/Desktop/prismux-backup.pmxbundle.json
prismux backup export --provider codex --to ~/Desktop/codex-backup.pmxbundle.json
prismux backup import ~/Desktop/prismux-backup.pmxbundle.json
prismux backup restore ~/Desktop/prismux-backup.pmxbundle.json
```

语义约定：

- `backup export`：创建 credential-bearing backup 文件。
- `backup import`：只把 backup 中的 targets 加入 Prismux 管理池，不激活。
- `backup restore`：导入 backup，并恢复 backup 记录的 active target。
- `--provider <id>`：只导出某个 provider。
- `--to <path>`：输出文件路径；不用 `--output`，因为 `to` 更像“保存到哪里”，适合 CLI 和 UI 文案。
- `--force`：允许覆盖已有 backup 文件。

如果用户传入相对路径，例如 `--to prismux-backup.pmxbundle.json`，CLI SHALL 以当前工作目录解析保存位置，并在成功输出中显示 absolute path，例如：

```text
Exported 5 targets to /Users/alice/project/prismux-backup.pmxbundle.json
Warning: this file contains account credentials. Keep it private.
```

不复用现有 `prismux import`，因为它已经表示 provider/profile 内容导入。`backup import` 表示导入 Prismux 迁移备份，且更清楚地提醒用户这是含凭据的文件。

### 7. Security posture

backup 文件包含 secret。导出必须：

- 用 `0600` 私有权限写文件。
- stdout 只打印 target 数量、provider 列表、resolved absolute output path 和 warning。
- manifest metadata 不包含 raw token；raw bytes 只在 `snapshot_hex` 中出现。

导入必须：

- 不打印 payload。
- 在 Unix 上发现 backup 文件 group/world readable 时给 warning；第一版不强制拒绝，避免从 AirDrop/Downloads 拿到文件无法导入。
- 拒绝 future `schema_version`。
- 拒绝 hash mismatch。

## Risks / Trade-offs

- 明文 backup 泄露风险 -> 导出和导入文案明确提示文件包含账号凭据，导出文件使用私有权限；加密留到独立变更。
- hex 体积膨胀 -> 当前目标是少量 auth/profile snapshot，体积可接受；大规模 config-dir 迁移再引入压缩。
- provider metadata 不完整 -> 只承诺恢复 Prismux 已有 metadata，不主动联网补全账号 email/plan。
- Claude macOS Keychain 语义复杂 -> backup 导入只写 managed snapshot；真正应用账号仍走现有 `use_target` 和 credential backend。
- 编号冲突 -> 导入时优先去重并分配当前机器可用编号；原编号作为 preferred display number，不能保证绝对保留。
