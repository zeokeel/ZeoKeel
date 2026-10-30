# 2026-10-30-m1-personal-mvp · 提案

- 状态：`approved`
- Owner：lead + 多 teammate
- 日期：2026-10-30
- 关联：docs/12 M1、docs/05/19/24、spec 2026-10-30-m0-scaffold-minimal-demo
- 变更预算：按子任务分片提交

## 1. 背景 / 问题

M0 骨架已可运行（zk compose/Tauri/axum/mock demo）。用户要求按文档继续施工直到全部功能可用，
M1 是个人版客户端 MVP：真实可用的聊天、流式 agent、工具、会话持久化、设置、本地 provider、可测试 UI。

## 2. 目标（本增量）

- 流式 agent 事件（TurnStarted/TextDelta/ToolStarted/ToolFinished/Completed/Error/Cancelled）。
- 会话持久化（SQLite WAL）+ 设置持久化（JSON；key 仅本地）。
- React 19 + Vite UI：会话列表、聊天、工具卡、设置、空/加载/错误/离线/流式/取消六态；i18n 中英；键盘可用。
- Tauri IPC + HTTP serve 载体（同一套命令/事件），供 Playwright 模拟真人操作。
- Playwright e2e：发消息→流式→读文件工具→刷新后会话仍在→取消→设置保存→a11y/截图。
- 保留 C17：能力索引/禁止目录/import 边界不退化。

## 3. 非目标（本增量外）

- 真实 OpenAI/Ollama 网络调用（下个增量；先 mock + 接口）。
- 审批交互、shell/todo 工具、安装包/更新（下一增量）。
- 企业版服务端（并行 M2 任务）。

## 4. 成功指标

- `cargo test --workspace` 绿；`zk validate/compose --check` 绿。
- Playwright 全绿 + 截图归档；axe 无 serious/critical。
- 人类操作路径（新建会话→发送“读取 demo-data/hello.txt”→看到工具结果→切换会话→重启后恢复）可重复。

## 5. 需要用户拍板

- [x] 用户 2026-10-30 指令：按文档继续实现，直到所有功能可用。
