## ADDED Requirements

### Requirement: 展示可切换账号与 active 状态
Menubar SHALL 从 OpenMux account report 展示 provider、稳定 local ID 对应的 alias、active 标记、plan、quota 摘要和 status；Swift SHALL NOT 自行扫描 auth 目录推断账号。

#### Scenario: 加载账号列表
- **WHEN** backend 返回多个已管理账号
- **THEN** Menubar SHALL 明确标记唯一 active account
- **AND** SHALL 将不可用或 stale 账号与健康账号区分展示

### Requirement: 切换必须由用户显式触发
Menubar SHALL 仅在用户选择具体账号后发起 switch command，第一版 MUST NOT 根据 quota、usage 或排序结果自动切换账号。

#### Scenario: 用户选择非 active 账号
- **WHEN** 用户点击一个非 active account 的切换操作
- **THEN** Menubar SHALL 使用 platform 与稳定 local ID 发起一次显式 switch
- **AND** SHALL 在后端确认成功前保持原 active 标记

### Requirement: 切换复用 OpenMux 安全后端
Switch command SHALL 由 OpenMux application service 重新解析目标并调用 provider plugin；备份、atomic auth replacement、私有权限和 registry active 更新 MUST NOT 在 Swift 中重新实现。

#### Scenario: 目标账号在点击后已被移除
- **WHEN** backend 执行 switch 时无法再解析目标 local ID
- **THEN** switch SHALL 失败且不得替换 active auth
- **AND** Menubar SHALL 显示安全的 target-not-found 状态并重新加载账号列表

#### Scenario: auth replacement 失败
- **WHEN** provider plugin 无法完成备份或 atomic replacement
- **THEN** backend SHALL 返回失败并保留原 active account
- **AND** Menubar SHALL NOT 乐观标记目标为 active

### Requirement: 切换操作必须 single-flight
同一 Menubar runtime SHALL 防止并发 switch，并在 switch 期间禁用重复操作；refresh 与 switch 的协调 SHALL 由 backend 保证不会产生相互覆盖的 auth 写入。

#### Scenario: 用户连续点击两个账号
- **WHEN** 第一个 switch 尚未完成时用户再次触发 switch
- **THEN** 第二个操作 SHALL 被拒绝或排队为明确的新操作
- **AND** SHALL NOT 同时执行两个 auth replacement

### Requirement: 切换完成后返回权威状态
成功的 switch response SHALL 包含 backend 确认的 active account 和 freshness/status；Menubar SHALL 以该 response 或随后一致性查询更新 UI。

#### Scenario: 切换成功
- **WHEN** provider plugin 完成目标账号切换并提交 active registry
- **THEN** Menubar SHALL 将 backend 返回的账号标记为 active
- **AND** SHALL 触发受调度约束的 quota/dashboard 更新
