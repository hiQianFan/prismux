# 设计

## 目标模型

将每个平台建模为一个账号池。

```text
PlatformPool
  platform: codex
  active_number: 1
  previous_active_number: 2
  accounts:
    - number: 1
      alias: optional
      account_label: optional
      plan_label: optional
      auth_hash: sha256(auth snapshot)
      snapshot_path: auth snapshot path
      imported_at_unix: timestamp
      last_activated_at_unix: optional timestamp
      availability: unknown for now
```

编号是平台内 selector，不是 provider identity。`codex #1` 和 `claude #1` 没有身份上的关联。

## Registry 格式

使用轻量的 `registry.omx` 格式，避免一开始就引入额外存储依赖。第一版账号池 registry 直接采用目标字段：

```text
schema_version 1
active_number 1
previous_active_number 2
next_number 3
account 1 <alias-or-empty> <account-or-empty> <plan-or-empty> <auth_hash> <snapshot_path> <imported_at> <last_activated_at>
```

## Login Flow

`omx login codex [--device-auth] [--alias <alias>] [--use]`：

1. 从显式测试路径、`$CODEX_HOME`、`~/.codex` 解析 Codex home。
2. 创建临时 login home，用于承载本次 Codex 官方登录流程。
3. 以临时 login home 作为 `CODEX_HOME` 调用 Codex 官方命令：
   - 默认运行 `codex login`；
   - 如果用户传入 `--device-auth`，运行 `codex login --device-auth`。
4. `--device-auth` 只改变官方 Codex login 的参数，不改变 OpenMux 后续账号池记录方式。
5. 登录流程的 stdin/stdout/stderr 继承当前终端，让用户完成官方登录交互；设备授权模式下，用户可以在另一台有浏览器的设备上打开链接并输入一次性 code。
6. 登录成功后，读取临时 login home 下的 `auth.json`，但不打印内容。
7. 计算 auth bytes 的 SHA-256 hash。
8. 加载 Codex registry。
9. 如果已存在相同 hash 的账号：
   - 更新 snapshot 和时间戳；
   - 如果用户提供了 `--alias`，则更新 alias；
   - 保留原有编号。
10. 如果没有匹配账号：
   - 分配 `next_number`；
   - 写入私有权限的 snapshot 文件；
   - 追加 account metadata；
   - 递增 `next_number`。
11. 默认情况下，不改变当前 active Codex auth。
12. 如果用户传入 `--use`，将新登录账号切换为 active。
13. 清理临时 login home。
14. 原子保存 registry，并输出账号编号和 imported/updated 状态。

## Save Flow

`omx save codex [--alias <alias>]` 是保存当前 active auth 的恢复/高级路径，不是普通用户添加账号的主路径。

1. 从显式测试路径、`$CODEX_HOME`、`~/.codex` 解析 Codex home。
2. 读取 `<codex-home>/auth.json`，但不打印内容。
3. 拒绝缺失或空 auth 文件。
4. 计算 auth bytes 的 SHA-256 hash。
5. 加载 Codex registry。
6. 如果已存在相同 hash 的账号：
   - 更新 snapshot 和时间戳；
   - 如果用户提供了 alias，则更新 alias；
   - 保留原有编号。
7. 如果没有匹配账号：
   - 分配 `next_number`；
   - 写入私有权限的 snapshot 文件；
   - 追加 account metadata；
   - 递增 `next_number`。
8. 原子保存 registry。
9. 输出账号编号，并说明是 imported 还是 updated。

未来可以扩展：

```sh
omx save codex --file ~/backup/auth.json
omx save codex --dir ~/backup/codex-auth
```

## Env Import Flow

`omx import codex "<TOML-or-KV>"` 用于外部中转站、API key 或 provider/profile 配置导入。配置内容放在命令最后，OpenMux 应该优先识别 Codex TOML 片段和官方变量名，例如：

```sh
omx import codex --name apikey-fun "
model_provider = \"codex\"
model = \"gpt-5.5\"

[model_providers.codex]
name = \"codex\"
base_url = \"https://api.apikey.fun\"
wire_api = \"responses\"
requires_openai_auth = true
"
```

或：

```sh
omx import codex "
OPENAI_API_KEY=sk-xxx
OPENAI_BASE_URL=https://api.example.com/v1
OPENAI_MODEL=gpt-5
"
```

Codex plugin 将导入内容写入 `<codex-home>/<profile>.config.toml`。TOML 片段原样保存；OpenAI-compatible KV 转换为 `[model_providers.<id>]` 配置，并只保存 `env_key` 名称，不保存 raw API key。

## List Flow

`omx list` 保持克制，只做全平台总览：

```text
Platform   Accounts   Active   Available
Codex      2          #1       unknown
```

这个输出应该来自 core/plugin 的状态模型，而不是字符串拼接后的再解析。容量在 provider-specific capacity source 实现前统一展示为 `unknown`。

`omx list codex` 展示平台明细：

```text
Codex

Available: unknown
Active: #1

#   Active   Alias   Account   Plan      Available
1   *        -       unknown   unknown   unknown
2            work    unknown   unknown   unknown
```

## Selector Resolution

`omx use codex <selector>` 按以下顺序解析 selector：

1. 精确匹配平台内编号。
2. 精确匹配 alias。
3. 未来再支持：previous account (`-`)、`next`、account metadata、fuzzy matching、`best`。

必须拒绝全数字 alias，避免和数字 selector 产生歧义。

## Switch Flow

切换继续沿用现有安全写入行为：

1. 将 selector 解析为唯一 stored account。
2. 读取目标 snapshot。
3. 如果 active auth 存在且内容不同，先备份。
4. 将目标 bytes 原子写入 Codex `auth.json`。
5. 更新 active 和 previous account number。
6. 输出 restart guidance。

## 从参考项目吸收的决策

- 将 row/number selection 作为一等路径，因为 `codex-auth` 和 `cc-account-switcher` 都成功使用了按编号切换。
- alias 保持为可选 metadata，和 `codex-auth` 的思路一致。
- 使用 content hashing 做重复检测和 active detection 的基础，借鉴 `caam`。
- `login` 使用临时 `CODEX_HOME` 完成官方登录，再导入结果，借鉴 `codex-auth`，避免添加账号时直接污染当前 active Codex auth。
- 切换时不动 settings，并提示用户可能需要 restart，借鉴 `cc-account-switcher`。
- raw auth 不进入 registry metadata；snapshot 是 auth-bearing file，需要私有权限保护。

## 风险

- provider 刷新 token 后，同一账号的 content hash 可能变化；在更强 account matching 实现前，hash-only 重复检测可能仍会产生重复账号。
- stable number 和删除账号后的编号行为需要在实现 remove 前做产品决策。
- 容量 overview 当前仍是 `unknown`；UI 不能暗示已经支持 capacity 获取。
