# Menubar v1 UX Checklist

本清单用于 `add-native-menubar-app` v1 手工验收。测试前建议使用临时状态目录，避免读取真实账号文件：

```sh
PRISMUX_STATE_ROOT=/tmp/prismux-menubar-ux-state \
CODEX_HOME=/tmp/prismux-menubar-ux-codex \
scripts/bundle-menubar.sh
```

## 场景

- 首次打开：菜单栏显示稳定 fallback；popover 展示 loading 后进入 ready 或 empty，不阻塞退出。
- last-good stale：后台 refresh 失败时保留上一次账号列表；header/status 明确显示 stale/error 信息。
- 无账号：accounts section 为空态可读；Refresh、Open CLI Help、Settings、Quit 仍可用。
- 多个账号：active 标记唯一；非 active 账号显示 Switch；switch 中重复操作被禁用。
- 长 alias：长 alias/account label 在窄宽度下截断或换行，不遮挡 Switch、quota/status 或 footer。
- switch 成功：后端 success report 返回后才更新 active；菜单栏 title 同步到新 active alias/status。
- switch 失败：保留原 active UI；错误状态可见；再次打开 popover 不丢失 last-good 数据。
- quota missing：quota/status 显示 unavailable 或 stale，不影响账号列表和 switch。
- 窄高度滚动：popover 内容可滚动；header、账号列表、footer 不重叠；键盘 focus 可到达 Refresh、Switch、Settings 和 Quit。
- icon-only mode：菜单栏只显示 icon；popover 内信息不丢失。
- background refresh：高频 timer 不绕过 backend cooldown；失败后进入 backoff，不产生连续 provider refresh。
