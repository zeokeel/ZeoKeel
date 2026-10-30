# 2026-10-30-workspace-scoped-conversations · 任务

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约：Workspace/WorkspaceStore/Conversation.workspace_id/新 session 方法 | packages/contracts/session | — | `cargo test -p zk-contract-session` | ☑ |
| 2 | store：schema v2 迁移 + WorkspaceStore + workspace-scoped session + 旧 API 兼容 | packages/client/store-session-sqlite | 1 | `cargo test -p zk-store-session-sqlite` | ☑ |
| 3 | agent/tool：TurnContext + per-call workspace root | packages/contracts/tool, agent, client/{tool-fs,agent-loop} | 1 | conformance/负向测试 | ☑ |
| 4 | IPC/bridge：workspace.* + workspace 形参 + context run；删除 auto-create/UI 接线待 T5 | apps/desktop/src, packages/contracts/ipc | 2,3 | e2e API（6th spec）| ◐ |
| 5 | UI：平铺 workspace 列表/默认 `~`/首用/切换取消（**阻塞：高密度设计待用户确认**） | apps/desktop/web, packages/ui | 4 | Playwright/截图/axe/DOM 断言 | ☐ |
| 6 | 证据：迁移/隔离/首用/目录不可用 e2e + acceptance | target/evidence | 1–5 | 全绿 | ☐ |
