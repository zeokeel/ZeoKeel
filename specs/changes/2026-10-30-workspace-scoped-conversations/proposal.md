# 2026-10-30-workspace-scoped-conversations · 提案

- 状态：`approved`
- Owner：lead
- 日期：2026-10-30
- 关联：docs/26、docs/24 §7、ADR-058、C18、D66、T4/T5
- 评审：docs/reviews/2026-10-30-workspace-and-process.md
- 变更预算：合同 3 文件 + store 2 文件 + 测试；分 T1–T5 提交

## 1. 背景 / 问题

当前 `Conversation` 无 `workspace_id`，`SessionStore::create_conversation(title)` 无工作区；`chat.send` 无 conversation 时自动建会话。
这违反 C18/docs/26（所有对话必须归属工作区；无工作区不建会话），且 agent cwd/工具根无法绑定。

## 2. 目标

- 契约：新增 `Workspace`/`WorkspaceStore`；`Conversation.workspace_id`；workspace-scoped session 方法。
- 存储：SQLite schema v2（workspaces + conversations.workspace_id + messages 重建），旧库迁移/默认 Workspace/回填；删除/重分配。
- Agent/tool：`TurnContext`（workspace root/conversation）注入，工具默认根 = workspace（T2）。
- IPC/UI：`workspace.*`、`session.*`/`chat.*` 带 workspace；移除 auto-create；workspace switcher/首用（T4，依赖 UI 设计评审）。
- 证据：Rust 迁移/负向测试 + Playwright 两工作区隔离/首用/不可用目录。

## 3. 非目标

- 归档/置顶/排序、文件浏览/监听、跨工作区 ungrouped（post-M1）。
- 企业版服务端工作区（客户端概念）。

## 4. 成功指标

- `cargo test -p zk-store-session-sqlite -p zk-contract-session` 绿；v1→v2 迁移消息不丢。
- 无 workspace 创建会话失败；跨 workspace messages/chat mismatch。
- UI 首用/切换/重启归属 e2e + axe/截图（设计评审通过后）。

## 5. 需要用户拍板

- [x] 用户 2026-10-30 指令：所有对话必须在工作区内进行（参考 DSH）。
- [x] docs/26 经独立评审 APPROVED（2026-10-30 第四轮）。
- [ ] UI 高密度设计方向待用户确认（T4 阻塞条件）。
