## ADDED Requirements

### Requirement: Menubar 使用 OpenMux-owned versioned contract
Swift Menubar SHALL 只通过 OpenMux `staticlib` C ABI 的单一 `omx_menubar_call(request_json)` 和 versioned JSON envelope 调用 dashboard、accounts、switch 与 refresh；它 MUST NOT 依赖 TokenBar/tokscale report schema 或 OpenMux SQLite 表结构。

#### Scenario: Swift 加载 dashboard
- **WHEN** Swift 以 `{"schema_version":1,"op":"dashboard","payload":{}}` 调用 C ABI
- **THEN** backend SHALL 返回带 `schema_version`、`ok`、`data/error` 的 OpenMux envelope
- **AND** payload SHALL 使用 OpenMux-owned DTO

### Requirement: C ABI 必须定义安全内存所有权
每个返回字符串 SHALL 由 Rust 分配并由唯一公开 free 函数释放；Swift wrapper SHALL 在成功、decode failure 和 cancellation 路径恰好释放一次。

#### Scenario: Swift 无法解码 response
- **WHEN** response JSON 不符合当前 Swift decoder contract
- **THEN** Swift wrapper SHALL 返回 contract error
- **AND** SHALL 仍调用 free 且不得 double-free

### Requirement: panic 与错误不得跨 ABI 泄漏
Rust entry point SHALL 捕获 panic 并转换为安全 error envelope；error MUST NOT 包含 token、raw auth、API key、完整 provider response、raw log 或未脱敏私有路径。

#### Scenario: application service panic
- **WHEN** C ABI 内部调用发生 panic
- **THEN** entry point SHALL 返回稳定 internal error code 或安全失败结果
- **AND** panic SHALL NOT unwind 跨过 C ABI

### Requirement: Swift 不得拥有数据引擎
Swift process MUST NOT 直接扫描 provider logs、计算 pricing、去重 usage event、执行 SQLite SQL、读取或写入 auth payload，且 MUST NOT 调用 provider quota endpoint。

#### Scenario: 用户手动刷新
- **WHEN** 用户在 Menubar 点击 Refresh
- **THEN** Swift SHALL 只调用 OpenMux refresh contract
- **AND** scan、quota fetch、persistence 和 retry decision SHALL 由 Rust application/plugin 层完成

### Requirement: CLI 与 Menubar 共享 application service
CLI 与 Menubar 对同一 state、window 和 filters 的查询 SHALL 通过同一 OpenMux application/query implementation 产生一致 totals、account state 和 status 语义。

#### Scenario: CLI 与 Menubar 查询 today usage
- **WHEN** 两者针对同一 state root 和本地时间窗口查询 today usage
- **THEN** token totals 与 group ranking SHALL 一致
- **AND** Menubar SHALL NOT 维护独立 cache database 改写结果

### Requirement: contract 采用 additive-first 演进
同一 major `schema_version` 内新增字段 SHALL 保持旧 Swift decoder 可忽略；删除、重命名或改变字段语义 MUST 提升 major schema version 并提供迁移期。

#### Scenario: backend 增加 optional diagnostic field
- **WHEN** Rust 在现有 envelope 中增加 optional 字段
- **THEN** 旧版 Menubar SHALL 能继续解码既有字段
- **AND** backend SHALL NOT 改变现有字段含义

### Requirement: FFI runtime 遵守路径 override
Menubar backend SHALL 与 CLI 一样遵守 `OMUX_STATE_ROOT`、`CODEX_HOME` 和 provider-specific 已定义 override，以支持隔离测试和用户显式配置。

#### Scenario: 集成测试使用临时目录
- **WHEN** 测试以临时 `OMUX_STATE_ROOT` 和 `CODEX_HOME` 初始化 runtime
- **THEN** dashboard、refresh 和 switch SHALL 只访问临时状态
- **AND** SHALL NOT 读取或修改用户真实 auth/state
