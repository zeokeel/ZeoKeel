# 2026-10-30-m1-personal-mvp · 任务清单

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约扩展（agent 流式 / ipc / session / config） | packages/contracts/ | — | `cargo test -p zk-contract-*` | ☑ |
| 2 | core：agent-loop 流式 + session/settings store | packages/client/ | 1 | `cargo test -p zk-agent-loop -p zk-store-session-sqlite -p zk-store-settings-json` | ☐ |
| 3 | composition：Runtime 增 sessions/settings/run_stream | tools/zk, crates/zk-composed | 2 | `zk compose --check` | ☐ |
| 4 | bridge：Tauri 命令 + serve 载体 | apps/desktop/src | 3 | `cargo check -p zk-desktop` + curl `/ipc` | ☐ |
| 5 | UI：React 聊天/会话/设置/六态/i18n | apps/desktop/web | 1 | vitest + build | ☐ |
| 6 | e2e：Playwright 真人操作/截图/axe | e2e/、playwright.config.ts | 4,5 | `pnpm test:e2e` | ☐ |
| 7 | 对抗性验收 + 修复循环 | 只读/报告 | 6 | verifier 报告 0 P0/P1 | ☐ |
