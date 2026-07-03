<div align="center">

<img src="assets/prismux-icon/prismux-mac-icon-1024.png" width="128" alt="Prismux" />

# Prismux

**面向 AI coding tools 的本地账号切换工具。**

[English](README.md) | 简体中文

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg?logo=rust)](rust-toolchain.toml)
[![Platform](https://img.shields.io/badge/macOS-Apple%20Silicon-black.svg?logo=apple)](#支持平台)
[![Status](https://img.shields.io/badge/status-early%20v0.x-blue.svg)](ROADMAP.zh-CN.md)

</div>

Prismux 帮助你管理 AI coding tools 的多个本地账号，查看当前使用的是哪一个，并通过编号或 alias 快速切换，减少重复浏览器登录。它是一个轻量的本地控制面板，服务于你已经在使用的 coding tools。

<div align="center">

<img src="assets/screenshots/prismux-menubar-dashboard.png" width="420" alt="Prismux dashboard" />

</div>

## 导航

- [概览](#概览)
- [名称与图标](#名称与图标)
- [安装](#安装)
- [快速开始](#快速开始)
- [常用命令](#常用命令)
- [支持工具](#支持工具)
- [更多文档](#更多文档)

## 概览

- AI coding tool 账号的本地控制面板。
- 桌面 app：快速查看账号状态并切换账号。
- `prismux` CLI：适合 Terminal 工作流和远程环境。
- 为支持的工具保存本地账号快照。
- 支持账号编号与 alias，切换更快。
- 凭据处理保持克制：Prismux 不打印 raw token，也不把 raw auth payload 写入 registry metadata。

## 名称与图标

**Prismux** 由 **prism** 和 **mux** 组合而来。Prism 表示棱镜：把光拆分成不同路径；mux 来自 multiplexer：从多路输入中选择一路作为当前输出。这正好对应 Prismux 的产品语义：保存多个本地账号，在需要时干净地切换当前使用的账号。

图标致敬 Pink Floyd 经典的棱镜意象：光进入、分离，并变成有秩序也有表达力的形态。Prismux 借用了这个视觉隐喻来表达本地账号切换。

## 安装

### 桌面 App

从 GitHub Releases 下载 app archive：

```text
https://github.com/hiQianFan/prismux/releases
```

当前 macOS release 解压后，把 `Prismux.app` 拖到 `/Applications`，再从 Finder 打开。

### CLI 包

Release 也会提供开箱即用的 CLI 包：

```text
prismux-cli-vX.Y.Z-macos-arm64.tar.gz
```

安装方式：

```sh
tar -xzf prismux-cli-vX.Y.Z-macos-arm64.tar.gz
cd prismux-cli-vX.Y.Z-macos-arm64
./install.sh
```

然后验证：

```sh
prismux --version
prismux status
```

桌面 App 也内置同版本 `prismux` 命令。如果你希望使用 App 内置 helper，可以在 Prismux Settings 中点击 `Enable prismux command`。

手动安装和卸载细节见 [安装指南](docs/INSTALL.zh-CN.md)。

## 快速开始

查看已识别的工具 home 和当前账号状态：

```sh
prismux status
```

通过 Codex 官方登录流程添加账号：

```sh
prismux login codex
```

添加 Claude Code 账号：

```sh
prismux login claude --alias work
```

查看已保存账号：

```sh
prismux list
```

按编号或 alias 切换：

```sh
prismux use codex 2
prismux use claude work
```

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `prismux status` | 查看工具 home 与当前账号状态。 |
| `prismux login codex` | 使用官方登录流程添加 Codex 账号。 |
| `prismux login codex --device-auth` | 在远程机器或无浏览器环境添加 Codex 账号。 |
| `prismux login claude --alias work` | 添加 Claude Code OAuth 账号，并命名为 `work`。 |
| `prismux list` | 查看所有已保存账号。 |
| `prismux list codex` | 只查看 Codex 账号。 |
| `prismux use codex 2` | 将 Codex 切换到 2 号账号。 |
| `prismux use claude work` | 将 Claude Code 切换到 `work` 账号。 |
| `prismux alias codex 2 work` | 设置或更新账号 alias。 |

更多示例见 [CLI 指南](docs/CLI.zh-CN.md)。

## 支持工具

| 工具 | 状态 | 说明 |
| --- | --- | --- |
| Codex | 支持 | 官方 login 包装、device auth、账号列表、alias、switch、profile import、额度展示。 |
| Claude Code | 支持 | OAuth account snapshot、gateway/API profile、macOS Keychain。 |
| Gemini CLI | 计划中 | 尚未实现。 |

## 支持平台

| 平台 | 状态 | 说明 |
| --- | --- | --- |
| macOS Apple Silicon | 支持 | 官方 `Prismux.app` release target。 |
| macOS Intel | 不计划 | 源码构建可能可用；不发布官方 app bundle。 |
| Linux | 计划中 | 源码构建可能可用；暂不发布官方 binary。 |
| Windows | 计划中 | 需要验证 credential、权限和外部 CLI 行为。 |

## 安全

Prismux 会处理本地 credential 文件，因此默认保持保守：

- 不打印 raw token 或 raw auth payload。
- registry 只保存 metadata 和 hash，不保存 raw auth。
- 替换 active credential 前会备份。
- 切换前校验 snapshot hash。

安全细节和漏洞私密报告见 [SECURITY.md](SECURITY.md)。

## 更多文档

- [安装指南](docs/INSTALL.zh-CN.md)
- [CLI 指南](docs/CLI.zh-CN.md)
- [源码构建](docs/BUILD.md)
- [贡献指南](CONTRIBUTING.md)
- [架构](docs/ARCHITECTURE.md)
- [产品范围](docs/PRD.md)
- [路线图](ROADMAP.zh-CN.md)
- [发布指南](docs/RELEASE.zh-CN.md)

## License

MIT
