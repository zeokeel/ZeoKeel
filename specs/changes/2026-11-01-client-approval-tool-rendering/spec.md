# 2026-11-01-client-approval-tool-rendering · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。

## 1. 需求

### R1 · 审批请求元数据

- **WHEN** 任一 `requires_approval` 工具即将执行
- **THEN** 运行时发出 `approval_requested` 事件，至少包含：`approval_id`、`tool`、`args`、`reason`、
  `summary`（一句话动作摘要）、`cwd`（绝对工作目录）、`read_paths`/`write_paths`（从参数提取的路径列表，可能为空）、
  `risk`（`low|medium|high`）、`deadline_ms`（绝对时间戳）、`diff`（写类工具可为 null）。
- **AND** 旧事件消费者未读新字段时仍可工作（全部字段 `#[serde(default)]`）。
- **AND** 不得把密钥/token 放进 `args`/`diff` 或写日志。

### R2 · 审批卡展示与决策

- **WHEN** UI 收到 `approval_requested`
- **THEN** 审批卡展示 R1 全部字段：动作摘要、工具名 + 完整命令/参数（关键参数不可折叠隐藏）、工作目录（标记是否越出 workspace）、
  读写路径列表、写操作 diff（可展开）、风险级别、剩余时间倒计时（来自 `deadline_ms`）。
- **AND** 决策按钮：允许一次 / 始终允许（本会话）/ 拒绝；默认焦点在「拒绝」；Esc = 拒绝；快捷键不得默认确认。
- **AND** 决策值映射为 `allow_once | allow_session | reject | cancel`；`cancel` 由用户显式取消触发。
- **AND** 超时由宿主决定并 fail-closed：超时后 UI 显示「已超时拒绝」，迟到的点击不生效。

### R3 · 「始终允许（本会话）」作用域

- **WHEN** 用户选择「始终允许（本会话）」
- **THEN** 该 `(conversation_id, tool_name)` 在本会话内后续调用不再弹审批，事件链仍发出 `approval_resolved{allow_session}`。
- **AND** 作用域仅内存、仅当前会话；新会话/重启/登出后清空，重新询问。

### R4 · 工具调用持久化与重放

- **WHEN** 工具调用结束
- **THEN** 会话存储保留可重放记录：`{name, args, result, status}`（现有 `role="tool"` JSON 行即可，字段补齐 `args`）。
- **WHEN** 会话重新加载
- **THEN** UI 将 `role="tool"` 行解析为工具卡，按时间顺序归并到其所属 assistant 轮次；状态显示
  `pending|running|success|failed|cancelled`。
- **AND** 无法解析的历史行走降级：显示可折叠原文并标记「无法解析的工具记录」，不白屏、不丢整条会话。
- **AND** 刷新后的工具卡内容（名称/参数/结果/状态）与刷新前一致（golden 断言）。

### R5 · 实时用量

- **WHEN** 收到 `AgentEvent::Usage`
- **THEN** UI 将用量写入当前助手消息并实时展示（token 计数），终态 `turn_completed.reply.usage` 到达时以完整值覆盖。
- **AND** usage 事件在 `turn_cancelled`/`error` 后到达也不污染下一条消息。

### R6 · i18n 门禁

- **WHEN** 运行 `bash scripts/i18n-check.sh`
- **THEN** exit 0：客户端 UI 源码无硬编码中文；启动错误边界文案来自 i18n 字典（bootstrap 期以默认 locale 翻译，`en-US` 有对应词条）。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 审批超时 | fail-closed 拒绝，UI 显示已超时；迟到决策忽略 | 现有 approval 超时语义 |
| 审批中窗口关闭/断线 | 视为拒绝（宿主 fail-closed） | 事件 `approval_resolved{reject}` |
| `role="tool"` 行 JSON 损坏 | 降级原文 + 折叠，不白屏 | i18n `toolRecordUnparseable` |
| diff 目标文件超大/不可读 | 回退为「将覆盖 + 摘要」，不阻塞审批 | 无错误码（UI 标注） |
| usage 事件重复/乱序 | 幂等合并，终态覆盖 | 无 |

## 3. 权限与可见性

- 仅本地用户可决策；企业弱管控下服务端不下发审批策略（C6），审批仍在本机。
- 审批事件只进本地事件流；不写审计（企业服务端不为员工行为兜底）。
- 敏感参数（如 env/token）在展示前脱敏（沿用工具参数现状，不新增泄漏面）。

## 4. 数据与兼容

- `AgentEvent::ApprovalRequested` / `ToolCallFinished` 增加字段，全部 `#[serde(default)]`；旧数据/旧消费者可解析。
- `ApprovalDecision` 增加 `allow_session`（`#[serde(rename_all="snake_case")]`），旧值 `allow_once|reject|cancel` 不变。
- `ApprovalGate` 增加带默认实现的方法（旧实现保持 bool 语义），不破坏第三方实现。
- 会话 `role="tool"` JSON 行字段增加 `args`/`status`；旧行缺字段时按默认解析并降级展示。

## 5. 验收断言（可执行）

- [ ] 契约序列化：新旧 JSON 双向兼容（Rust 单测）。
- [ ] 审批卡 e2e：diff 展示、默认焦点拒绝、Esc 拒绝、超时拒绝、始终允许本会话后同工具不再弹窗。
- [ ] 刷新恢复 e2e：工具卡名称/参数/结果/状态与刷新前一致；损坏行走降级。
- [ ] usage 实时断言：流式过程中 token 计数递增，终态一致。
- [ ] `bash scripts/i18n-check.sh` exit 0；`pnpm -r typecheck/lint/test:unit`、`cargo test` 相关包绿。

## 6. 未决问题

- [ ] 「始终允许」按工具名 vs 参数指纹（见 proposal §7 A）。
- [ ] 历史工具行是否升级为结构化 `tool_calls`（见 proposal §7 C）。
