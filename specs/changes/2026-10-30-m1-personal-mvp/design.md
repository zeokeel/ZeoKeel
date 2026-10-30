# 2026-10-30-m1-personal-mvp · 技术设计

## 影响面

| 类型 | 变更 |
| --- | --- |
| 契约 | agent 增加流式事件/EventSink；新增 ipc/session/config 契约 |
| 包 | agent-loop 流式；新增 store-session-sqlite、store-settings-json |
| apps | desktop：Tauri 命令 + serve 载体；web：React UI |
| 生成物 | zk-composed Runtime 增 sessions/settings/run_stream |
| 文档 | 无新主题（沿用 05/19/24/25） |

## 1.5 复用检查

| 点 | 现有 | 结论 | 归属 |
| --- | --- | --- | --- |
| 事件信封 | `zk-contract-ipc` | 复用 | 契约 |
| 会话 DTO | `zk-contract-session` | 复用 | 契约 |
| 设置 DTO | `zk-contract-config` | 复用 | 契约 |
| 流式事件 | agent 契约扩展 | 扩展 | 契约 |
| UI 基础组件 | Radix/自有 | 新建 | apps/desktop/web |

## 方案

- core：agent-loop 实现 run_stream（文本分片 + tool 事件）；store-session-sqlite 用 rusqlite WAL；
  store-settings-json 写 `$ZEOKEEL_HOME/settings.json`（权限 600）。
- bridge：Tauri 单命令 `ipc_call` + `serve`（axum）复用同一 dispatch；事件用 Tauri event / SSE。
- UI：React 19 + Vite + TS；transport 抽象 `invoke` vs `fetch/EventSource`；状态 reducer；settings/session 视图。
- harness：Playwright 启动 serve 载体（headless Chromium），模拟新建会话/发送/工具/切换/刷新；截图 + axe。

## 测试

- Rust：core 包单测 + composed 集成测试 + bridge dispatch 单测。
- e2e：`pnpm --dir apps/desktop/web test:e2e`（Playwright）。
- 证据：target/evidence/m1/{playwright-report,screenshots,axe.json}。
