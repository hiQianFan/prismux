# Capability: Account Pool Management

## ADDED Requirements

### Requirement: 平台账号池

OpenMux SHALL 将导入的账号建模为 platform-specific account pools。

#### Scenario: 平台内编号

- GIVEN Codex 已导入两个账号
- AND Claude 已导入一个账号
- WHEN 用户查看平台明细
- THEN Codex 账号可以通过 `codex #1` 和 `codex #2` 选择
- AND Claude 账号可以通过 `claude #1` 选择
- AND 不同平台上的相同编号不表示相同 provider identity。

### Requirement: 平台登录并自动记录

OpenMux SHALL 允许用户通过 `omx login <platform>` 进入平台官方登录流程，并在登录成功后自动将账号加入对应平台账号池。

#### Scenario: 首次 login 添加账号

- GIVEN Codex CLI 可用
- AND 还没有导入过 Codex 账号
- WHEN 用户运行 `omx login codex`
- THEN OpenMux 运行 Codex 官方登录流程
- AND 登录成功后将登录结果保存为 Codex account `#1`
- AND 命令输出包含导入账号编号
- AND 不打印 raw auth payload。

#### Scenario: device auth login

- GIVEN 用户处在远程或无浏览器环境
- WHEN 用户运行 `omx login codex --device-auth`
- THEN OpenMux 使用 Codex 的 device auth 登录模式
- AND 登录成功后自动将账号加入 Codex 账号池
- AND 账号编号、snapshot 保存、重复检测和后续切换行为与普通 `omx login codex` 保持一致。

#### Scenario: login 时可选 alias

- GIVEN Codex CLI 可用
- WHEN 用户运行 `omx login codex --alias work`
- THEN OpenMux 登录并记录 Codex 账号
- AND 将 `work` 保存为该账号的 optional alias metadata。

#### Scenario: login 后立即切换

- GIVEN Codex CLI 可用
- WHEN 用户运行 `omx login codex --use`
- THEN OpenMux 登录并记录 Codex 账号
- AND 将该账号切换为 active Codex account。

### Requirement: 重复检测

OpenMux SHALL 在导入的 auth 与已导入 auth snapshot 匹配时避免创建新账号。

#### Scenario: 基于 content hash 的重复导入

- GIVEN Codex account `#1` 已经从某个 auth file 导入
- AND 当前 active Codex auth file 具有相同 content hash
- WHEN 用户运行 `omx login codex` 或 `omx save codex`
- THEN OpenMux 更新 account `#1`
- AND 不创建 account `#2`。

### Requirement: 保存当前 active auth

OpenMux SHALL 保留 `omx save <platform>`，用于将已经存在的 active auth state 保存到 OpenMux。

#### Scenario: 保存当前 active auth

- GIVEN 当前 Codex active auth file 存在
- WHEN 用户运行 `omx save codex`
- THEN OpenMux 将当前 active auth 保存或更新到 Codex 账号池
- AND 不打印 raw auth payload。

### Requirement: 外部配置导入

OpenMux SHOULD 使用 `omx import <platform> "<TOML-or-KV>"` 从外部导入中转站、API key 或 provider/profile 配置。

#### Scenario: 导入 Codex TOML profile

- GIVEN 用户从中转站复制了 Codex TOML 配置片段
- WHEN 用户运行 `omx import codex --name apikey-fun "<TOML>"`
- THEN OpenMux 将配置写入 Codex profile config file
- AND 不覆盖用户现有 Codex `config.toml`。

#### Scenario: 导入 Codex OpenAI-compatible KV

- GIVEN 用户从中转站复制了 `OPENAI_API_KEY`、`OPENAI_BASE_URL` 和可选 `OPENAI_MODEL`
- WHEN 用户运行 `omx import codex "<KV>"`
- THEN OpenMux 识别官方变量名
- AND 生成 Codex 可用的 provider/profile 配置
- AND 不将 raw API key 写入 registry metadata。

### Requirement: 全局总览

OpenMux SHALL 将 `omx list` 渲染为克制的 all-platform account-pool overview。

#### Scenario: 全局 list

- GIVEN Codex 已导入两个账号
- WHEN 用户运行 `omx list`
- THEN 输出包含一行 Codex
- AND 该行包含 account count、active account number 和 availability overview
- AND 输出不包含每个账号的 reset time 或诊断细节。

### Requirement: 平台明细

OpenMux SHALL 将 `omx list <platform>` 渲染为 platform-specific account detail。

#### Scenario: Codex 明细 list

- GIVEN Codex 已导入两个账号
- WHEN 用户运行 `omx list codex`
- THEN 输出包含每个 Codex account 的一行
- AND 每行包含 account number、active marker、alias when present、account when known、plan when known 和 availability state。

### Requirement: Selector-based Switching

OpenMux SHALL 使用 platform-local number selectors 切换账号。

#### Scenario: 按 number 切换

- GIVEN Codex account `#2` 存在
- WHEN 用户运行 `omx use codex 2`
- THEN OpenMux 将 account `#2` 恢复到 active Codex auth path
- AND 将 account `#2` 记录为 active。

#### Scenario: 按 alias 切换

- GIVEN Codex account `#2` 的 alias 是 `work`
- WHEN 用户运行 `omx use codex work`
- THEN OpenMux 将 account `#2` 恢复到 active Codex auth path
- AND 将 account `#2` 记录为 active。

### Requirement: Alias Metadata

OpenMux SHALL 将 alias 视为 optional account metadata，而不是必需的 account identity。

#### Scenario: 设置 alias

- GIVEN Codex account `#2` 存在
- WHEN 用户运行 `omx alias codex 2 work`
- THEN OpenMux 将 `work` 保存为 account `#2` 的 alias
- AND account `#2` 仍然可以通过 number 选择。

#### Scenario: 拒绝数字 alias

- GIVEN Codex account `#2` 存在
- WHEN 用户运行 `omx alias codex 2 123`
- THEN OpenMux 拒绝该 alias
- BECAUSE numeric selectors 保留给 platform-local account numbers。

### Requirement: 安全 auth 切换

OpenMux SHALL 在账号切换时保护 active auth state。

#### Scenario: 替换前创建备份

- GIVEN Codex active auth file 存在
- AND 目标 account snapshot 内容不同
- WHEN 用户切换到目标账号
- THEN OpenMux 先备份之前的 active auth
- AND 将目标 auth snapshot 原子写入 active Codex auth path。

### Requirement: Unknown Availability

OpenMux SHALL 在没有 capacity source 时将 capacity 表示为 `unknown`。

#### Scenario: capacity unavailable

- GIVEN 尚未实现 Codex capacity source
- WHEN 用户运行 `omx list`
- THEN Codex availability overview 是 `unknown`
- AND OpenMux 不将 unknown 当成 zero。
