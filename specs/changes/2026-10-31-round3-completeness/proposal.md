# 2026-10-31-round3-completeness · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web / auditor-rust / fixer-media / fixer-contracts）
- 日期：2026-10-31
- 关联：ADR-064/066/067；docs/38；docs/25；上一轮 acceptance backlog
- 评审：docs/reviews/2026-10-31-round3-verification.md
- 变更预算：配置完形（工具/审批超时）+ 向量持久化 + T22 剩余边界 DTO + 内核收尾 + UI 一致性；按域提交

## 1. 背景 / 问题

核心目标要求"所有项目可配置、整个 agent 功能完美、rig 能力全支持"。上一轮已落 rig 默认、media/embedding、AppSettings.agent、Ajv strict；仍存在：
- 工具调用超时、审批超时仍是 env-only 隐藏项（settings 里不可配置），违反"无硬编码隐藏项"。
- cap-vector 仅进程内索引，重启丢失（rig vector 能力不完整）。
- T22 边界仍有 reader-only DTO：workspace.files（FileEntry）、meta.get（Meta）、chat.send 响应（stream id）、question payload；Ajv 未覆盖。
- 内核残留：desktop schedule 内层 std 线程不可中止、server jobs loop 无 cancellable token、Tauri setup 事件转发未走内核。
- UI 残留：ui-sessions 自建命令面板（C17 重复实现）、admin 页面仍有硬编码中文、inspector changes 数据源缺失。

## 2. 目标

- `AppSettings.tools { tool_timeout_ms, approval_timeout_ms }` 契约/校验/UI/运行时生效；env 仅 fallback。
- cap-vector 可选持久化（目录/文件 0600，启动加载，插入/清空保存，测试覆盖）。
- T22 剩余 DTO 生成 schema/TS 并被桌面 Ajv 使用；负向测试。
- 内核：schedule 线程可中止、jobs 任务可取消、Tauri 事件转发纳入内核任务/关闭顺序。
- UI：删除 ui-sessions 重复命令面板（统一走 ui-commands）、admin i18n 补齐、inspector changes 真实数据或明确禁用。
- 全量门禁 + C19 评审 APPROVED。

## 3. 非目标

- `zk api` 路由真源、`zk lock/verify-lock`、媒体真实 provider、Docker/多平台证据（继续 UNVERIFIED）。
- usage 持久化到 session Message（下一轮）。
- 企业媒体端点（C7 禁止）。

## 4. 成功指标

- 新增设置字段在重启后生效（单测/集成）；工具超时与审批超时可配置且 env fallback 保留。
- T22 schema 61+，桌面新边界全部 Ajv 校验。
- cap-vector 重启后索引恢复（测试）。
- 门禁全绿，e2e（m1/agent/login/admin）绿，review APPROVED。

## 5. 风险

| 风险 | 缓解 |
| --- | --- |
| 线程/任务取消引入回归 | 保留旧路径 fallback，加测试 |
| 向量持久化并发/损坏 | 原子写临时文件+rename、0600、损坏时降级为空并记日志 |
| T22 新 DTO 破坏兼容 | serde default、只增字段、generated diff 审核 |
| UI 面板迁移影响 e2e | 保留 aria/按钮名称，e2e 回归 |
