# 2026-11-01-client-compact-command · 行为规格

> 只写系统必须怎样；实现见 design。状态 draft（D1/D2/D6 已定 Lead 2026-11-01）。

## 1. 需求

### R1 · 命令真实化
- **WHEN** 用户输入 `/` → 选择 `/compact`
- **THEN** 菜单「指令」组显示 compact 为真实命令（不再是「未实现」）；选择后移除 `/query`、焦点回输入框、命令不发送为普通消息；流式中禁用。
- **AND** 通过 Batch1 唯一 catalog/runCommand；未知命令仍 `command/unknown`；移除 `command/unimplemented` 对 compact。

### R2 · 压缩
- **WHEN** 非流式、有历史，执行 `/compact`
- **THEN** host 读取当前消息视图，保留最近 6 轮/12 条（取先到），对更早 user/assistant 文本调用专用 host summarizer（personal provider / enterprise gateway；不产生 agent turn/tool 行/审批），生成摘要。
- **AND** 摘要以 `role=system` marker 消息持久化（id `compaction:<uuid>`，content JSON：summary/through_message_id/created/input_tokens/output_tokens/keep_recent），**原始消息不删**；**`SessionStore::messages_in` 保持原始语义不变**；新增 `session.compacted_view`（raw+marker 构造「marker+最近 N 轮」）仅供 agent replay/分隔 UI/摘要注入；`session_search`/fork/审计读 raw（P1-1）。
- **AND** 成功后 UI 用 compacted view 显示「较早消息已压缩」+ 摘要/压缩条数/usage；`session.compaction_get`/`session.compacted_view` 重启/刷新恢复；下一轮 agent 使用 compacted view；搜索/fork/审计仍见原始消息。
- **AND** 输入 ≤12k tokens、摘要 ≤2k tokens；超限拒绝、不写 marker、不改历史；失败（provider/网关/DB/超时/取消）同样不写；无新消息（最新 marker 边界=最后消息）返回 no-op；同一边界不重复写。
- **AND** 只作用于当前 workspace/conversation；不跨 workspace；不丢原始归档；不自动压缩。

## 2. 边界与错误
| 场景 | 期望 | 错误码 |
| --- | --- | --- |
| 流式中 compact | 禁用/提示 | `compact/busy` |
| 超 12k/摘要超 2k/无 provider | 拒绝，不改历史 | `compact/too-large`、`compact/unavailable` |
| provider/DB/超时失败 | 保留原历史，可重试 | `compact/failed` |
| 无新消息 | no-op 提示 | — |
| 跨 workspace/不存在会话 | 拒绝 | 既有 `session/*` |
| 离线/已取消 | 六态可读 | 既有 offline/cancelled |

## 3. 权限与可见性
- personal 本地 provider；enterprise 仅网关（不持上游 key，C7）；无企业策略/服务端端点；只当前 workspace/conversation。

## 4. 数据与兼容
- 无 DB migration；marker 复用 messages/append_message；`messages_in` 原始语义保留；compacted view 仅 replay/分隔 UI/摘要；search/fork/audit 读 raw；无 marker=当前行为；旧客户端不调用新 IPC。
- 回滚：删 marker（或忽略）后回原历史；旧会话不受影响。

## 5. 验收断言
- [ ] 菜单真实、无 unimplemented；Batch1 不回归。
- [ ] 成功：marker+最近 6 轮、原消息保留、UI 分隔/usage、下一轮摘要生效。
- [ ] 流式/超限/失败/取消不改历史；重复 no-op；错误码/六态/i18n/axe。
- [ ] `messages_in` 原始语义回归；compacted view 仅 replay/分隔 UI；压缩后 search/fork/审计仍见原始消息；host replay 注入摘要。
- [ ] P0 交互测试+证据；用户测试 UNVERIFIED。

## 6. 未决问题
- [x] D1/D2/D6 已定；仅实现细节在 design/tasks。
