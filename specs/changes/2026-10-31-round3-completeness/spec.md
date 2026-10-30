# 2026-10-31-round3-completeness · 规格

- 状态：`approved`

## 1. 配置

- `zk-contract-config` 新增 `ToolSettings { tool_timeout_ms: u64 = 120000 (clamp 1000..600000), approval_timeout_ms: u64 = 300000 (clamp 1000..1800000) }`；`AppSettings.tools` serde default。
- 运行时优先级：显式 settings > `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS` env > 默认；设置保存后立即生效（审批 broker 更新超时；agent-rig 每轮读取）。
- 设置 UI 暴露两个字段，范围校验，持久化；错误回传。

## 2. 向量持久化

- `cap-vector` 可选持久化：`configure` 时若配置 dir（env `ZEOKEEL_VECTOR_DIR` 或显式），加载 `vector-index.json`；insert/insert_many/clear 原子保存；文件 0600、目录 0700；损坏/不可读时降级为空并暴露 `unavailable_reason`/日志；测试跨实例恢复。

## 3. T22 边界 DTO

- 新增契约 DTO 并导出 schema/TS：`FileEntry`（workspace.files）、`MetaInfo`（meta.get）、`ChatSendResult`（chat.send 响应）、`QuestionDef`/`QuestionAsked` payload（typed questions）。
- 桌面 Ajv 校验这些边界；未知/错类型拒绝并 warn；负向测试。

## 4. 内核与 UI

- desktop schedule executor：可中止 stop flag + JoinHandle，内核 shutdown 时 join；server jobs loop 接受 CancellationToken（或 atomic stop）由内核 abort 后停止；Tauri 事件转发由内核 task owner 持有。
- ui-sessions 删除本地命令面板，侧栏按钮调用 app action 打开壳 ⌘K；admin 页面文案 i18n；inspector changes 数据源实现（workspace.changes IPC）或明确禁用并文档化。

## 5. 验收

见 acceptance.md。
