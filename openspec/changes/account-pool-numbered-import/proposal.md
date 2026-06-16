# 变更：account-pool-numbered-import

## 摘要

构建 OpenMux 的平台账号池能力。用户应该可以通过 OpenMux 进入平台官方登录流程，登录成功后自动加入账号池；不取名也能添加账号，看到克制的全平台总览，进一步查看某个平台的账号明细，并用平台内编号切换账号。

## 动机

真实用户想添加 Codex、Claude Code 或 Gemini CLI 账号时，不应该先手动运行平台 login，再回到 OpenMux save。OpenMux 的核心心智应该是账号池入口：帮助用户进入平台官方登录流程，并在登录成功后自动记录账号。

参考现有账号切换工具后，有几个成熟做法值得吸收：

- `codex-auth` 支持显示行号选择、alias 作为 metadata、切回上一个账号，以及替换 `auth.json` 前先备份。
- `cc-account-switcher` 使用账号编号快速切换，并明确告诉用户只切换 authentication，不动 settings。
- `caam` 把 auth 文件视为 bearer credential 快照，并用 content hashing 判断当前 active profile，避免隐藏状态和实际 auth 文件脱节。

OpenMux 应该吸收这些经验，同时保持自己的产品模型：先提供小而清楚的全局账号池 overview，再提供平台 detail；alias 只在用户想整理账号时才出现。

## 范围

- 支持 `omx login <platform>` 作为普通用户添加账号的主路径。
- 支持 `omx login codex --device-auth`，用于远程/无浏览器环境。
- 支持 `omx login <platform> --alias <alias>` 和 `omx login <platform> --use`。
- 保留 `omx save <platform>` 作为保存当前 active auth 的恢复/高级路径。
- 将 `omx import <platform> "<KV>"` 保留给外部中转站、API key 或 provider/profile 配置导入。
- 为每个平台内的账号分配稳定编号。
- 支持 `omx use <platform> <number>` 作为默认切换路线。
- 保留 alias 作为可选 metadata 和 selector。
- 将 `omx list` 改为克制的全平台账号池 overview。
- 增加 `omx list <platform>` 展示某个平台的账号池明细。
- 为导入的 auth snapshot 增加 content-hash 重复检测。
- 保留 auth-bearing 文件的备份、原子写入和私有权限行为。

## 非目标

- 本变更不实现额度/usage 获取。
- 本变更不实现 `best` selector；它只作为未来功能点保留。
- 本变更不增加 GUI、daemon、watcher 或并发隔离 profile。
- 本变更不调用未文档化的 provider API。

## 用户体验

首次添加账号：

```sh
omx login codex
```

```text
Imported Codex account #1
```

远程或无浏览器环境：

```sh
omx login codex --device-auth
```

`--device-auth` 透传 Codex 的设备授权登录模式。用户在另一台有浏览器的设备上完成授权，OpenMux 在登录成功后自动保存账号。

它不是独立的 OpenMux 认证模式，也不是普通本地登录的必选项。它只决定 OpenMux 调用官方 Codex 登录时使用 `codex login --device-auth`，后续仍按同一套账号池逻辑分配编号、保存 snapshot、做重复检测，并允许用户用 `omx use codex <number>` 切换。

登录后立即切换：

```sh
omx login codex --use
```

全局总览：

```sh
omx list
```

```text
Platform   Accounts   Active   Available
Codex      2          #1       unknown
```

平台明细：

```sh
omx list codex
```

```text
Codex

Available: unknown
Active: #1

#   Active   Alias   Account   Plan      Available
1   *        -       unknown   unknown   unknown
2            -       unknown   unknown   unknown
```

按编号切换：

```sh
omx use codex 2
```

```text
Using Codex account #2
Restart Codex if it is already running.
```

可选 alias：

```sh
omx login codex --alias work
omx use codex work
```

## 开放问题

- 删除账号后，平台内编号应该留下空洞，还是为了列表清爽而重新压紧？
- `omx login codex` 默认是否应该保持当前 active 不变，还是登录完成后自动切换到新账号？
- 第一版 registry metadata 应该保存完整 hash，还是只保存短 fingerprint 加 snapshot path？
