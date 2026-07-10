## ADDED Requirements

### Requirement: Portable backup 导出

Prismux SHALL 支持将已管理的 account/profile targets 导出为一个本地 portable backup 文件。

#### Scenario: 导出全部 targets

- **GIVEN** Prismux 已管理 3 个 Codex accounts 和 2 个 Claude profiles
- **WHEN** 用户运行 `prismux backup export --to targets.pmxbundle.json`
- **THEN** Prismux 创建 `targets.pmxbundle.json`
- **AND** backup 包含 5 个 targets 的 metadata 和 snapshot payload
- **AND** backup 包含 schema version、创建时间和 active target metadata
- **AND** stdout 只显示导出数量、provider 摘要和 resolved absolute output path
- **AND** stdout 不输出 raw token、API key、OAuth credential 或 snapshot payload。

#### Scenario: 相对输出路径解析

- **GIVEN** 当前工作目录是 `/Users/alice/project`
- **WHEN** 用户运行 `prismux backup export --to prismux-backup.pmxbundle.json`
- **THEN** Prismux 将 backup 写入 `/Users/alice/project/prismux-backup.pmxbundle.json`
- **AND** stdout 明确显示该 absolute path
- **AND** stdout 提示该文件包含账号凭据，应保存到用户信任的位置。

#### Scenario: 按平台导出

- **GIVEN** Prismux 已管理 Codex 和 Claude targets
- **WHEN** 用户运行 `prismux backup export --provider codex --to codex.pmxbundle.json`
- **THEN** backup 只包含 Codex targets
- **AND** 不包含 Claude account/profile snapshot。

#### Scenario: 无可导出 targets

- **GIVEN** Prismux 没有已管理 targets
- **WHEN** 用户运行 `prismux backup export --to empty.pmxbundle.json`
- **THEN** Prismux 返回明确错误
- **AND** 不创建包含空 secret payload 的 backup。

### Requirement: Backup 文件安全写入

Prismux SHALL 将导出的 backup 当作 secret-bearing 文件处理。

#### Scenario: 导出文件使用私有权限

- **WHEN** 用户运行 `prismux backup export --to targets.pmxbundle.json`
- **THEN** Prismux 使用原子写入创建 backup
- **AND** 在支持 POSIX permissions 的平台上将文件权限设置为 `0600`。

#### Scenario: 拒绝覆盖已有文件

- **GIVEN** `targets.pmxbundle.json` 已存在
- **WHEN** 用户运行 `prismux backup export --to targets.pmxbundle.json`
- **THEN** Prismux 拒绝覆盖
- **AND** 提示用户选择新路径或传入显式覆盖选项。

### Requirement: Portable backup 导入

Prismux SHALL 支持从 portable backup 导入 account/profile targets 到当前机器的 Prismux state root。

#### Scenario: 导入 backup

- **GIVEN** `targets.pmxbundle.json` 包含 5 个有效 targets
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 校验 backup schema version
- **AND** 校验每个 target snapshot hash
- **AND** 将 snapshot 写入当前机器对应 provider 的 Prismux snapshot 目录
- **AND** 将 target metadata 写入当前机器 state store
- **AND** 输出导入、新增、更新和跳过数量
- **AND** 不修改 Codex active auth、Claude credential backend 或 Claude settings。

#### Scenario: restore 显式恢复 active target

- **GIVEN** backup 记录 Codex active account
- **WHEN** 用户运行 `prismux backup restore targets.pmxbundle.json`
- **THEN** Prismux 先完成全部 target 导入
- **AND** 使用现有 target activation 流程恢复 backup 记录的 active target
- **AND** activation 仍执行写入前备份、hash 校验和失败回滚。

#### Scenario: 导入 future schema

- **GIVEN** backup 的 `schema_version` 大于当前 Prismux 支持的版本
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 拒绝导入
- **AND** 不写入任何 snapshot 或 state store 记录。

#### Scenario: snapshot hash mismatch

- **GIVEN** backup 中某个 target 的 `snapshot_sha256` 与 payload bytes 不匹配
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 拒绝导入该 backup
- **AND** 不应用 active target
- **AND** 不打印损坏 payload。

### Requirement: 导入去重

Prismux SHALL 在导入 backup 时避免为相同 target 创建重复记录。

#### Scenario: 相同 snapshot 已存在

- **GIVEN** 当前机器已存在一个 Codex account
- **AND** backup 中包含相同 snapshot hash 的 Codex account
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 更新已有 account metadata
- **AND** 不分配新的 Codex account 编号。

#### Scenario: 相同 provider subject 已存在

- **GIVEN** 当前机器已存在一个具有相同 provider subject hash 的 account
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 更新已有 account
- **AND** 保留当前机器已有 display number。

#### Scenario: profile name 冲突但内容不同

- **GIVEN** 当前机器已有 Claude profile `work`
- **AND** backup 中也包含名为 `work` 但 snapshot hash 不同的 Claude profile
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 不静默覆盖当前 profile
- **AND** 返回冲突错误或为导入 profile 分配明确的新名称。

### Requirement: Backup 内容边界

Prismux SHALL 只导出 Prismux 管理的 target metadata 和 snapshots。

#### Scenario: 不导出运行时历史

- **GIVEN** Codex home 包含 sessions
- **AND** Claude home 包含 projects、todos 或 history
- **WHEN** 用户运行 `prismux backup export --to targets.pmxbundle.json`
- **THEN** backup 不包含 Codex sessions
- **AND** backup 不包含 Claude projects、todos 或 history
- **AND** backup 不包含 Prismux usage history、refresh attempts 或 backup files。

#### Scenario: 不导出未管理 active auth

- **GIVEN** 当前 Codex `auth.json` 存在
- **AND** 该 auth 尚未通过 Prismux save/login/import 进入 managed account pool
- **WHEN** 用户运行 `prismux backup export --to targets.pmxbundle.json`
- **THEN** backup 不包含该 unmanaged active auth
- **AND** Prismux 提示用户可先运行 `prismux save codex`。

### Requirement: Backup import/export 兼容 provider 能力

Prismux SHALL 只导入当前构建支持的 provider target 类型。

#### Scenario: 当前构建不支持 backup 中的 provider

- **GIVEN** backup 包含 provider `gemini`
- **AND** 当前 Prismux 构建尚未支持 Gemini plugin
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 跳过或拒绝该 provider target，并在摘要中说明原因
- **AND** 不影响同一 backup 中可支持的 Codex 或 Claude targets。

#### Scenario: 当前 provider 不支持该 target kind

- **GIVEN** backup 包含某 provider 的 `profile` target
- **AND** 当前 provider plugin 不支持 profiles
- **WHEN** 用户运行 `prismux backup import targets.pmxbundle.json`
- **THEN** Prismux 拒绝导入该 target
- **AND** 摘要包含 unsupported target kind。
