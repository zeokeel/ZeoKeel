# 2026-10-31-round3-completeness · 任务

| # | 任务 | 验证 | 状态 |
| --- | --- | --- | --- |
| 1 | ToolSettings 契约 + 运行时应用（agent-rig/desktop broker）+ 测试 | cargo tests；settings 生效断言 | pending |
| 2 | 设置 UI 工具/审批超时 | pnpm gates + vitest | pending |
| 3 | cap-vector 持久化 + 测试 | cap-vector tests（跨实例恢复/损坏降级） | pending |
| 4 | T22 FileEntry/MetaInfo/ChatSendResult/Question DTO + schema/TS + Ajv | export_schema/schema-check；desktop vitest | pending |
| 5 | 内核：schedule 可中止、jobs cancellable、Tauri 转发 | cargo test；SIGTERM/重启冒烟 | pending |
| 6 | UI：统一命令面板、admin i18n、inspector changes | pnpm gates；e2e | pending |
| 7 | 全量门禁 + e2e + C19 评审 | 全部命令 exit 0；review APPROVED | pending |
