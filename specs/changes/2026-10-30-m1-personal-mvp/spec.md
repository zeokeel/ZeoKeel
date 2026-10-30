# 2026-10-30-m1-personal-mvp · 行为规格

## R1 流式聊天

- WHEN 用户发送文本 THEN UI 立即显示用户消息，助手气泡流式增长，最终落库。
- WHEN 输入“读取 demo-data/hello.txt” THEN 依次出现工具开始/结束事件，最终文本含文件正文。
- WHEN 用户点取消 THEN 收到 TurnCancelled，已生成内容保留并标记取消。

## R2 会话

- WHEN 新建/切换会话 THEN 消息按会话隔离；重启应用后从 SQLite 恢复。
- WHEN 会话列表为空 THEN 显示空态引导（不是白屏）。

## R3 设置

- WHEN 修改 provider/主题/语言 THEN 保存到本地 settings 文件并在下次启动生效；非法 URL/空模型给出字段错误。
- 企业版相关字段在个人版不可见（本增量仅个人版）。

## R4 传输

- Tauri：`ipc_call` 命令 + 事件通道；serve 载体：`POST /ipc` + SSE `/events`。
- UI 只依赖 `zk-contract-ipc` 的 CommandRequest/CommandResponse/UiEvent 形状。

## R5 质量

- 六态：空/加载/错误/离线/流式中/已取消，每个可见页面覆盖。
- a11y：键盘可完成发送/切换会话/保存设置；焦点可见；axe 无 serious/critical。
- i18n：zh-CN/en-US 文案不硬编码在组件逻辑里。
