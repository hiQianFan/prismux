# Roadmap

[English](ROADMAP.md)

## v0.1: macOS 公开版本

- Codex account login、save、list、alias、switch 和 profile import。
- Claude Code profile 导入/切换，以及 OAuth account snapshot 导入/切换。
- macOS Apple Silicon 和 Intel `OpenMux.app` full bundle release，内置同版本 `omx` helper。
- `cargo install --git` 开发者安装路径。
- README 和用户文档双语。
- 仓库清理、CI、issue/PR templates 和安全政策。

## v0.1 Hardening

- 改进诊断和恢复建议。
- 增加真实使用场景的 smoke test 覆盖。
- 根据首个公开版本反馈加固 release 自动化。
- 评估 `cargo deny` 等依赖/许可证检查。

## v0.2: Linux 验证

- 验证 Codex 和 Claude 在 Linux 上的行为。
- 验证 credential 文件权限和外部 CLI 行为。
- 稳定后加入 Linux official binary。

## v0.3: Windows 验证

- 验证 Windows 上的路径发现、文件替换、进程查找和 credential 存储行为。
- 判断是否需要额外 Windows ACL/private-permission 处理。
- 稳定后加入 Windows official binary。

## 后续

- macOS release 稳定后维护 Homebrew tap。
- crate 命名和公开 API 边界稳定后考虑 crates.io。
- 如果 app-bundled helper 之外有真实需求，再发布 standalone CLI tarball。
- Gemini CLI plugin。
- v0.1 checksum 之外的 artifact signing/provenance。
- 更多 provider/profile 导入格式。

## 非目标

- OpenMux 不是 API gateway、model router 或 provider marketplace。
- v0.1 不包含 daemon、watcher、动态 plugin loading、Sparkle 自动更新或 Developer ID notarization。
- OpenMux 不调用 provider 私有 API 来补全账号 metadata。
