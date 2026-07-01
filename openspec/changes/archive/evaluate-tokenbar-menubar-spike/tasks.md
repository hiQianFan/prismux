## 1. Spike 基线与合规

- [ ] 1.1 固定 TokenBar upstream commit、仓库 URL、license、copyright/NOTICE 和第三方资源清单。
- [ ] 1.2 建立隔离 fork 或临时工作区，确认不进入 OpenMux workspace、Cargo members、CI 或默认 build。
- [ ] 1.3 记录 TokenBar macOS 发布链路：签名、更新、Homebrew/Sparkle 相关约束；本 spike 不实现这些链路。

## 2. Coupling Audit

- [ ] 2.1 梳理 Overview 相关 SwiftUI views、view models、state store 和 Rust/FFI 入口。
- [ ] 2.2 标记 TokenBar 自有 scanner、pricing/model mapping、usage cache、aggregation、quota fetcher 和 account/quota 定义。
- [ ] 2.3 列出可复用 UI/shell 与必须删除或禁用的数据引擎代码。

## 3. Contract Spike

- [ ] 3.1 准备 OpenMux `omx usage --json --no-scan` 样例或 mock `UsageReport` JSON，不包含 auth payload、token、API key、raw log 或未脱敏路径。
- [ ] 3.2 将一个 Overview screen 的数据入口替换为 OpenMux JSON/mock contract。
- [ ] 3.3 禁用 TokenBar 自有 scanner/aggregator，确认 Overview totals 不由 TokenBar 数据引擎计算。
- [ ] 3.4 记录启动延迟、刷新延迟、CLI 子进程开销、错误降级和 stale data 表达。

## 4. Decision

- [ ] 4.1 量化 fork delta、必须删除代码、保留代码和预计 upstream sync 成本。
- [ ] 4.2 根据采用门槛和停止线输出 go/no-go 结论。
- [ ] 4.3 若 go，创建或更新独立 Menubar implementation change；若 no-go，保存 UX mapping 和不 fork 依据。
