# Menubar v1 Acceptance Review

本 review 对应 `openspec/changes/add-native-menubar-app` 的 v1 gate。

## 结论

- 未引入 TokenBar fork：Swift target、bundle audit、privacy check 均不包含 TokenBar DTO、scanner/pricing/quota/cache、bundle ID、UserDefaults key 或品牌资源。
- 未引入账号 CRUD：Menubar v1 只提供账号池查看、active 状态、Refresh、Switch、Open CLI Help、Settings、Quit；login/import/alias/remove 仍由 CLI 承担。
- 未引入自动切换：switch 必须由用户显式点击触发，后端只接受 provider + stable local ID，并重新解析目标。
- 未引入 token stats 面板：Menubar v1 只展示账号池、active 状态和 quota/status，不展示本地 token 汇总或 account attribution。
- 未引入完整 analytics dashboard：v1 不提供历史趋势、成本分析、模型明细 dashboard 或第二份 usage cache。

## 验证

- `cargo fmt --all`
- `cargo test`
- `cargo clippy --all-targets --all-features`
- `scripts/bundle-menubar.sh`
- `openspec validate add-native-menubar-app --strict`
