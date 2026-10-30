# 2026-10-30-workspace-scoped-conversations · 任务

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约：Workspace/WorkspaceStore/Conversation.workspace_id/新 session 方法 | packages/contracts/session | — | `cargo test -p zk-contract-session` | ☑ |
| 2 | store：schema v2 迁移 + WorkspaceStore + workspace-scoped session + 旧 API 兼容 | packages/client/store-session-sqlite | 1 | `cargo test -p zk-store-session-sqlite` | ☑ |
| 3 | agent/tool：TurnContext + per-call workspace root | packages/contracts/tool, agent, client/{tool-fs,agent-loop} | 1 | conformance/负向测试 | ☑ |
| 4 | IPC/bridge：workspace.* + workspace 形参 + context run；删除 auto-create | apps/desktop/src, packages/contracts/ipc | 2,3 | e2e API（6th spec）| ☑ |
| 5 | UI：DSH 式平铺 workspace+会话/默认 `~`/首用/切换取消/高密度 | apps/desktop/web, packages/ui | 4 | Playwright 9 tests/截图/axe | ☑ |
| 6 | 证据：迁移/隔离/首用/目录不可用 e2e + acceptance | target/evidence | 1–5 | 全绿 | ☐ |
