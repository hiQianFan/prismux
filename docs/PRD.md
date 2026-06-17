# OpenMux PRD

## 产品定位

OpenMux 是 AI coding tools 的本地账号池管理器。

用户不是因为想管理 auth 文件才使用 OpenMux。用户真正想解决的是：自己在 Codex、Claude Code、Gemini CLI 等工具里有多个账号，希望知道每个平台的账号池大概还能不能用，并能快速切换账号，不必重复走浏览器登录流程。

核心产品原则：

> OpenMux 管理的是平台账号池。默认视图是全平台总览；聚焦某个平台时再展示账号明细；切换账号应该先支持编号选择，而不是要求用户一开始就给账号取名。

## 用户心智

用户通常按这个顺序思考：

1. 这台机器上有哪些 AI coding 平台被 OpenMux 识别或绑定了？
2. 每个平台下面有几个账号？
3. 每个平台当前 active 的账号是谁？
4. 每个平台账号池大概还有多少可用额度？
5. 如果某个平台快不够用了，我应该切到该平台里的哪个账号？
6. 用久之后，我要不要给这些账号取更好记的 alias？

因此，OpenMux 不应该把 alias 命名放进首次使用流程。alias 是整理账号用的，不是导入账号的门槛。

## 主要用户路线

### 首次使用

用户想添加一个 Codex 账号。OpenMux 应该帮助用户进入 Codex 官方登录流程，并在登录成功后自动把账号加入 Codex 账号池。

```sh
omx status
omx login codex
```

期望行为：

- `status` 确认 OpenMux 是否能找到工具 home、config file、auth file 和 OpenMux 自己的 state。
- `login codex` 调用 Codex 官方登录流程，并在登录成功后自动保存 auth snapshot。
- 用户不需要提供 alias。
- OpenMux 给该账号分配平台内编号。
- 如果 OpenMux 检测到该 auth state 已经导入过，则更新已有账号，而不是创建重复账号。

示例输出：

```text
Imported Codex account #1
```

### 添加更多账号

用户想添加第二个账号时，仍然使用 OpenMux 入口，而不是先手动运行 `codex login` 再回来 `omx save`：

```sh
omx login codex
```

期望行为：

- OpenMux 创建临时 login 环境，让 Codex 官方登录流程在该环境中完成。
- 登录成功后，OpenMux 自动读取登录结果并保存到账号池。
- 默认情况下，添加账号不应破坏当前 active 账号。
- 如果可以识别这是已存在账号，则更新已有记录。
- 如果暂时无法识别真实 account metadata，则至少通过 content hash 做重复检测。
- 如果确实是新账号，则分配下一个平台内编号。
- Codex account 和 plan 可以从官方 `auth.json` 中的 `id_token` claims 安全提取；不得展示 access token、refresh token 或 raw JWT。

远程服务器、SSH、容器或无桌面环境中，用户可以使用：

```sh
omx login codex --device-auth
```

`--device-auth` 表示使用 Codex 的设备授权登录模式：终端显示登录链接和/或一次性 code，用户在任意有浏览器的设备上完成授权，终端中的登录流程继续完成。OpenMux 只负责透传这个登录模式，并在登录完成后自动记录账号。

这不是 OpenMux 的另一套账号体系，也不是用户日常添加账号时必须理解的概念。它只是把 Codex 官方的 `codex login --device-auth` 包进 OpenMux 的主入口里，让没有本机浏览器的环境也能完成同样的账号添加流程。登录成功后，账号仍然获得平台内编号，仍然进入 Codex 账号池，仍然可以通过 `omx list codex` 和 `omx use codex <number>` 管理。

如果用户希望登录完成后立刻切换到新账号，可以显式使用：

```sh
omx login codex --use
```

### 全局总览

用户运行：

```sh
omx list
```

这不是 raw account dump，而是全平台账号池总览。

它应该回答：

- 有哪些平台？
- 每个平台有几个账号？
- 每个平台当前 active 的账号编号/alias 是什么？
- 每个平台账号池的总体可用概览是什么？
- 每个平台账号池关键窗口的聚合剩余额度是什么？例如 Codex 在全局视图展示账号池内 `5h` 平均剩余额度。
- 每个平台是否有账号进入 limited/exhausted 等需要关注的状态？全局视图只展示收敛后的状态文案。

全局总览应该保持克制。不要在这里展示每个账号的 reset time、详细 usage source、诊断 warning、plan、weekly window 或单账号明细。那些内容属于 `omx list <platform>` 或 `omx doctor <platform>`。`omx list` 的 `Overall` 和 `5h` 都是平台账号池级别的聚合视角，不是 active 单个账号的额度；当前 active 账号只作为“正在选择谁”的上下文展示。

示例：

```text
Overview
╭──────────┬─────────┬───────┬─────────┬─────┬───────────╮
│ Platform ┆ Active  ┆ Accts ┆ Overall ┆ 5h  ┆ Status    │
╞══════════╪═════════╪═══════╪═════════╪═════╪═══════════╡
│ Codex    ┆ #2 work ┆ 3     ┆ 63%     ┆ 16% ┆ 1 limited │
│ Claude   ┆ #1      ┆ 2     ┆ -       ┆ -   ┆ ok        │
│ Gemini   ┆ #1 main ┆ 1     ┆ 92%     ┆ -   ┆ ok        │
╰──────────┴─────────┴───────┴─────────┴─────┴───────────╯
```

### 平台明细

用户运行：

```sh
omx list codex
```

这个视图聚焦某一个平台账号池。

它应该回答：

- 当前 active 账号是谁？
- 每个账号的编号、alias、account、plan、关键 usage window 和状态是什么？
- 每个账号关键 usage window 的 reset time 是什么？这些时间跟在对应窗口百分比后面展示，不单独占用 `Refresh` 列。

示例：

```text
Codex accounts: 3 total, active #2 work

*   #   Alias      Account              Plan      5h                    Weekly                Status
-   1   personal   qianfan@gmail.com    Plus      92% (06-16 18:42:10)  68% (06-20 08:00:00)  -
*   2   work       qianfan@company.com  Team      16% (06-16 18:42:11)  77% (06-20 08:00:00)  low
-   3   -          unknown              unknown   -                     -                     timeout
```

### 切换账号

默认切换路径使用平台内编号：

```sh
omx use codex 2
```

期望行为：

- 替换 active auth 前先备份当前 auth state。
- 恢复目标账号的 auth state。
- 将目标账号标记为该平台 active。
- 告诉用户目标工具如果正在运行，可能需要 restart 才能生效。

示例：

```text
Using Codex account #2
Restart Codex if it is already running.
```

当账号数据更丰富时，可以支持更多 selector：

```sh
omx use codex work
omx use codex next
omx use codex -
```

selector 含义：

- `2`：在只有 account 的平台中按账号编号切换；在同时有 accounts 和 profiles 的平台中按 `omx list <platform>` 当前展示编号切换。
- `work`：按用户设置的 alias 切换。
- `next`：切到该平台下一个可用账号。
- `-`：切回上一个 active 账号。

`best` 是有价值的未来 selector，但当前不进入核心切换路线。它需要先明确 capacity 语义，再决定按最高百分比、最长可用时长还是 reset 时间选择。

### 查看当前状态

用户可以在两个层级查看当前状态：

```sh
omx current
omx current codex
```

- `omx current` 展示所有已连接平台当前 active 的账号。
- `omx current codex` 展示 Codex 当前 active 账号，包括编号、alias、account、plan 和 capacity when known。

### 命名和整理

alias 是可选的。只有当用户想让列表更好读时才需要设置。

```sh
omx alias codex 1 personal
omx alias codex 2 work
```

规则：

- login/save/import 不要求 alias。
- 账号编号仍然是默认 selector。
- alias 是显示和选择的便利层，不是 provider account。
- 如果能检测出 account 和 plan，应该和 alias 分开展示。
- 全数字 alias 应该被拒绝，避免和编号 selector 冲突。

### 保存当前状态

`save` 不是普通用户添加账号的主路径。普通用户添加账号应该使用 `login`。`save` 的职责是把已经存在的本机 active auth 状态保存到 OpenMux。

典型场景：

- 用户已经通过 Codex App、VS Code extension 或手动 `codex login` 改变了当前 active auth。
- OpenMux registry 丢失，但当前工具 auth 文件仍然存在。
- 用户未来希望从指定 auth file 或备份目录恢复账号。

命令形态：

```sh
omx save codex
omx save codex --alias work
```

`save` 语义是“我已经在 Codex App、VS Code extension 或手动 `codex login` 中登录好了，请 OpenMux 保存当前状态”。未来如果需要从指定 auth file 或备份目录恢复，应优先扩展为：

```sh
omx save codex --file ~/backup/auth.json
omx save codex --dir ~/backup/codex-auth
```

### 外部配置导入

`import` 用于从外部导入中转站、API key 或 provider/profile 配置。用户常见输入是中转站网页复制的一段 Codex TOML 或 KV，OpenMux 应该接受整段内容放在命令最后，自动识别并转换为目标工具支持的配置。

Codex TOML 输入示例：

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

Codex/OpenAI-compatible 输入示例：

```sh
omx import codex "
OPENAI_API_KEY=sk-xxx
OPENAI_BASE_URL=https://api.example.com/v1
OPENAI_MODEL=gpt-5
"
```

Claude Code 输入示例：

```sh
omx import claude "
ANTHROPIC_API_KEY=sk-ant-xxx
ANTHROPIC_BASE_URL=https://api.example.com
ANTHROPIC_MODEL=claude-sonnet-4-5
"
```

无参数时，`omx import <platform>` 可以读取 stdin；剪贴板或交互粘贴是后续增强。OpenMux 应该优先使用官方变量名和官方配置入口，不要求用户手写 Codex `config.toml` 或 Claude `settings.json`。

Profile 命名规则应尽量贴近中转站身份：显式 `--name` 优先；未提供时优先从 `base_url` host 推断，例如 `https://api.apikey.fun/v1` 生成 `api-apikey-fun`；再回退到 `model_provider` 或 provider id。

Claude Code 需要额外区分 profile 和 OAuth account：

- `omx import claude` 导入中转/API profile，只写 Claude Code user `settings.json` 的 `env`。
- `omx login claude --alias work` 调用官方 Claude Code 登录流程，并在登录成功后自动导入 OAuth account snapshot。
- `omx import claude` 在没有配置内容时从本机已有官方 Claude Code 登录产物导入 OAuth account snapshot。
- `omx use claude <selector>` 在 account/profile 中自动推断，唯一命中 profile 时只切换 profile，唯一命中 account 时只恢复 OAuth credential snapshot 和 `oauthAccount` metadata。
- 如果 selector 同时命中 account 和 profile，OpenMux 必须报歧义错误，不静默选择。
- 当前实现支持 macOS Keychain backend 和 plaintext `.credentials.json` backend；Keychain 读写必须通过独立 backend，禁止在日志、错误或命令行参数中暴露 payload。

### 诊断和恢复

用户运行：

```sh
omx doctor
omx doctor codex
```

doctor 应该回答：

- OpenMux 能否找到各工具 home？
- 能否找到 active auth file？
- OpenMux 自己的 state 是否可读写？
- registry 是否可读？
- stored auth snapshots 是否存在？
- auth-bearing 文件权限是否足够安全？
- 当前 active auth 是否能匹配到已知账号？

## 功能需求

### 平台 Target Catalog

- OpenMux 将 account 和 profile 都视为平台下可切换的 target。
- account/profile 的底层 registry、snapshot 和 apply 逻辑必须按平台能力分离；CLI 层只做 target catalog 聚合、编号展示和 selector 分发。
- `omx list <platform>` 是数字 selector 的唯一来源。只要平台同时有 accounts 和 profiles，列表必须按 accounts 在前、profiles 在后的顺序生成连续展示编号。
- 展示编号是当前列表编号，不是持久 ID；新增/删除 target 后可以动态变化。底层 account/profile 持久编号不得因为展示编号变化而重排。
- profile 没有持久编号时，仍必须获得当前列表展示编号，并可通过该编号执行 `omx use <platform> <number>`。
- 非数字 selector 按 account alias 和 profile name 精确匹配；同时命中时必须报歧义错误，不静默偏向 account 或 profile。
- 聚合平台同一时间只能有一个 active target。切换 account 后 profile 不再显示 active；切换 profile 后 account 不再显示 active。
- 每个账号拥有平台内持久编号，这是 account-only 平台或底层 account plugin 的默认 selector。
- 每个账号可以有 alias。
- 每个账号可以有检测到的 account metadata，例如 email、account id、team 或 provider-specific account context；plan 单独展示。Codex 第一版从 `id_token` claims 提取 email 和 ChatGPT plan，并在 account 不可用时回退到 account id。
- 每个账号可以有 usage/capacity metadata。

账号持久编号是平台内位置，例如 `codex account #1`、`claude account #1`。它们不代表 provider identity。展示编号是 CLI 当前列表的选择编号，例如 `codex profiles` 可以在 3 个 accounts 后显示为 `#4`。

例如：

```sh
omx use codex 2
omx use codex 4
```

如果 `omx list codex` 中 `#2` 是账号，则切换到该账号；如果 `#4` 是 profile，则切换到该 profile。

### Login / Save / Import

- `omx login <platform>` 是普通用户添加账号的主路径。
- `omx login claude` 不是 OpenMux 自研 OAuth flow；它包装官方 Claude Code CLI 登录。官方登录成功会改写真实 Claude credential，因此 OpenMux 导入 snapshot 后必须登记该 account 为 active，并清除同平台 profile active marker。
- `omx login <platform> --device-auth` 支持远程/无浏览器环境的设备授权登录模式；它只是选择 provider 官方登录方式，不改变账号池记录、编号、重复检测和切换语义。
- `omx login <platform> --alias <alias>` 可以在登录成功后顺手设置 alias。
- `omx login <platform> --use` 可以在登录成功后立刻切换到新账号。
- 对 Claude Code，官方登录流程本身会激活新 credential，`--use` 只是显式表达用户意图，不改变最终 active 结果。
- `omx save <platform>` 是恢复/高级路径，用于保存当前已经存在的 active account。
- `omx save <platform> --file <path>` 和 `omx save <platform> --dir <path>` 是未来恢复/迁移能力。
- `omx import <platform> "<TOML-or-KV>"` 用于导入外部中转站或 provider/profile 配置，配置内容放在命令最后。
- `omx import claude [--name <name>]` 在没有外部 KV/TOML 内容时用于导入本机已有 Claude Code OAuth account snapshot。
- `save` 和 `import` 都不得打印 raw auth content 或 raw API key。
- login/save 都应尽可能避免重复账号。
- login/save 创建新账号时分配下一个平台内编号。
- login/save 应存储导入时间、auth hash、snapshot path，以及可用时的 account/plan metadata。

### List

- `omx list` 展示全平台 overview。
- `omx list <platform>` 展示平台 detail。
- 同时有 accounts 和 profiles 的平台必须分组展示，但使用同一组连续编号。
- 空 account/profile section 仍展示空表格，避免在同一平台下出现结构跳变。
- 默认输出优先使用 human-readable table。
- 全局 overview 只展示紧凑可用概览；缺失的 usage 字段展示 `-`。
- 更细的 reset、freshness、source 和诊断信息应放到平台 detail 或 doctor。

### Use

- `omx use <platform> <selector>` 切换某个平台的 active account 或 profile。
- 对于同时具备 accounts 和 profiles 的平台，数字 selector 按 `omx list <platform>` 当前展示编号解析；展示编号由 accounts 在前、profiles 在后连续生成，不等同于底层 registry 持久编号。
- 非数字 selector 支持 account alias 和 profile name；唯一命中时自动执行对应切换。
- 非数字 selector 同时命中 account 和 profile 时必须报歧义错误，要求用户改成唯一 alias/profile name。
- 聚合平台在用户心智上同一时间只能有一个 active target；切换 account 时 profile 不再显示为 active，切换 profile 时 account 不再显示为 active。
- 后续可以支持 `next`、previous account、identity、`best` 或 fuzzy matching。
- switch 必须在替换前备份当前 auth state。
- switch 必须在平台支持时使用 atomic writes。
- Claude profile switch 和 Claude OAuth account switch 在 CLI 上使用统一 `claude` 入口，但底层能力、registry、snapshot apply 和回滚逻辑必须分离。

### Capacity

capacity 是产品模型的一部分，但当前全局总览要保持克制。

每个平台账号池最终应该能表达：

- 总体可用概览；
- 每个账号的多个 usage limit，例如 Codex 的 `5h` 和 `weekly`；
- 每个 limit 的 used percent、remaining percent 和 reset time；
- freshness；
- data source；
- 不含 token/raw auth 的诊断状态，例如 `auth`、`timeout`、`network` 或 provider HTTP 状态码。

聚合规则必须明确：

- 全局 overview 显示平台账号池级别的聚合 usage：`Overall` 优先对所有已知 `UsageLimit.remaining_percent` 做平均；Codex 等有关键窗口的平台可以额外展示账号池内 `5h` 平均剩余额度；这些列不是 active 单个账号的额度。
- 平台 detail 可以按 provider 展示关键窗口，例如 Codex 展示 `5h`、`Weekly` 和 `Status`；usage window 没有数据时展示 `-`，原因放在 `Status`。`Status` 默认展示 `-`，低额度展示 `low`，用完/不可用展示 `limited`，请求/解析异常展示安全诊断 code，不重复窗口百分比。
- 如果只有旧的 `availability` summary 可用，可以回退展示已知 summary 的平均值。
- 如果部分账号已知、部分未知，overview 只聚合已知 usage；缺失原因放在平台 detail 的 `Status`。
- 如果没有任何账号 capacity data，usage 字段应展示 `-`。
- UI 不应将缺失数据当成 zero。
- Codex 当前没有稳定公开的本地额度文件或普通 CLI 子命令可直接读取百分比。OpenMux 可以参考 Codex 官方源码中使用的 `backend-api/wham/usage` 做 best-effort usage 查询；该能力不是稳定公开 API，必须短超时、失败回退 `unknown`，且不得打印 bearer token。
- Claude/Gemini 后续不应强行套用 Codex 的 `5h`/`weekly` 列；它们应该填充统一 `UsageLimit` 模型，并由各自 plugin 决定 detail table 的 provider-specific 列。

### Safety

- 不打印 raw auth payload。
- 不把 auth payload 存进 registry metadata。
- auth snapshot 和 account metadata 分离。
- 切换账号前备份 active auth。
- auth-bearing 文件优先使用私有权限。
- 除非明确作为产品决策接受，否则不调用私有或未文档化 provider API。

## 非目标

- OpenMux 不是 model router、API gateway 或 provider marketplace。
- OpenMux 不替代 provider login flow。
- OpenMux 不需要在 CLI 心智稳定前做 GUI。
- OpenMux 不要求用户导入账号前先给账号取名。
- OpenMux 不把 alias、account 和 plan 混为一谈。

## 开放问题

- 删除账号后，平台内编号应该保留空洞，还是为了列表清爽而重新压紧？
- `omx login codex` 默认是否应该保持当前 active 不变，还是登录完成后自动切换到新账号？
- 重复检测第一步使用 content hash 是否足够，何时引入更强 account matching？
- `next` 应该按导入顺序、最近使用顺序，还是 capacity 状态选择？
