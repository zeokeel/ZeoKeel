# 2026-11-01-client-approval-tool-rendering · 提案

- 状态：`draft`
- Owner：lead
- 日期：2026-11-01
- 关联：`research/client-dsh-parity-2026-11-01.md` §4 P0-1/P0-2/P1-9/P2-5；`docs/19-client-ux-flows.md` §4.1；`docs/28-ui-module-map-and-skeleton.md` §4.6
- 评审：待独立评审（C19）
- 变更预算：约 20 个文件（契约 3 + 宿主 3 + kit 1 + ui-chat 4 + desktop web 3 + i18n 1 + 测试 4 + 文档 2）；同一目标，不拆

## 1. 背景 / 问题

审计确认 4 个 P0 级「看起来有、实际降级」的问题（对比 DSH 与 `docs/19` §4.1）：

1. **审批卡安全字段缺失**：只有 `tool`/`args`/`reason` + 批准/拒绝，缺动作摘要、工作目录、读写范围、diff 预览、风险级别、剩余时间、
   「始终允许（本会话）」、记住范围、默认焦点在拒绝；契约的 `Cancel` 决策不可达。用户无法判断这次操作的真实影响面。
2. **工具行刷新丢失**：宿主把 `ToolCallFinished` 写为 `role="tool"` 的 JSON 消息（`apps/desktop/src/events.rs:60-76`），
   但 UI 的 `toChatMessage` 把非 user 角色全部当 assistant 原文透传（`apps/desktop/web/src/state/reducer.ts:140-152`），
   刷新/重进会话后出现 `{"name":…,"result":…}` 原始 JSON 气泡，真正的工具卡消失。
3. **`usage` 事件被丢弃**：契约已有 `AgentEvent::Usage`（`packages/contracts/agent/src/lib.rs:140`），
   `parseAgentEvent` 落 default→null（`apps/desktop/web/src/protocol.ts:669-670`），UI 只能等 turn 结束汇总。
4. **i18n 门禁红**：`apps/desktop/web/src/main.tsx:47,49` 硬编码中文，`bash scripts/i18n-check.sh` exit 1，
   与 AGENTS.md §6「门禁绿」不符（CI 会红）。

这些不是「缺功能」，而是**安全/正确性必须先修**，否则后续能力面（Phase 2）建在错误的数据与安全语义上。

## 2. 目标

- 审批卡按 `docs/19` §4.1 展示：动作摘要、工具/完整命令、工作目录（是否越出 workspace）、读写路径列表、写操作 diff 预览（可展开）、
  风险级别（low/medium/high）、剩余时间倒计时、决策（允许一次 / 始终允许本会话 / 拒绝；默认焦点在拒绝；Esc=拒绝）。
- 契约支持结构化审批：`ApprovalEvent` 增加元数据（全部 `#[serde(default)]` 向后兼容）；`ApprovalDecision` 增加 `AllowSession`；
  `ApprovalGate` 支持按会话生效的 always-allow 记忆（不落盘、不跨会话）。
- 工具调用持久化/重放：`ToolCallFinished` 带 `args`；UI 把 `role="tool"` JSON 行解析为工具卡并归并到前一条 assistant 消息；
  刷新后工具卡与状态完整恢复；未知/损坏行降级为可折叠原文，不白屏。
- `usage` 事件接线：类型、解析器、reducer 更新当前助手消息的实时用量。
- i18n 门禁恢复绿：启动错误边界文案进 i18n 字典（bootstrap 期用默认 locale 翻译）。
- 不改变工具执行/审批的 fail-closed 语义；超时仍默认拒绝。

## 3. 非目标

- 系统通知/托盘红点（`docs/19` §4.2 的失焦通知）→ 后续 Phase 3 交付项，不在本 spec。
- 审批跨重启持久化、全局永久允许（`docs/19` §4.1 明确不提供）。
- diff 算法覆盖二进制/超大文件（>256KB 回退为摘要）。
- 企业「弱管控」审批策略（C6：不做企业审批）；只做客户端本地审批。
- 工具卡按 DSH 的 54 种 keyed view 拆分（Phase 2）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 模型调用 `fs.write` 触发审批 | 看到将写入的路径、diff、风险级别、剩余时间，默认焦点在「拒绝」 |
| 个人用户 | 同一会话反复批准同一工具 | 勾选「始终允许（本会话）」后不再弹同工具审批；重开会话恢复询问 |
| 个人用户 | 刷新客户端/重进会话 | 工具卡完整恢复（名称/参数/结果/状态），不再是 JSON 气泡 |
| 审计/安全 | 检查审批超时 | 超时自动拒绝且 UI 显示「已超时拒绝」，不静默放行 |

## 5. 成功指标

- 审批卡字段覆盖 `docs/19` §4.1 表格 100%（除 §4.2 通知，明确列为剩余风险）。
- `cargo test -p zk-contract-agent -p zk-desktop -p zk-agent-rig` 新增/修改用例绿；`ApprovalDecision` 序列化兼容旧值。
- e2e：写入审批展示 diff + 默认焦点拒绝 + 超时拒绝 + 始终允许本会话；刷新后工具卡恢复；usage 实时递增。
- `bash scripts/i18n-check.sh` exit 0；`pnpm -r typecheck/lint/test:unit/build` 绿。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| `ApprovalGate` 从 bool 改为枚举会动 `zk-kit-approval`/host broker | 高 | 契约层新增带默认实现的方法，旧实现保持可用；broker 单测覆盖 |
| diff 预览读文件可能碰到权限/大文件 | 中 | >256KB/读失败回退「摘要 + 将覆盖」；只读不写 |
| 工具行 JSON 解析需兼容历史数据 | 中 | serde_json 解析失败降级原文；e2e 用历史会话数据 |
| 审批元数据来自 agent-rig，需拿到 TurnContext/workspace 根 | 中 | 在 agent-rig 工具闭包内构造（已有 context）；单测断言 cwd/paths |
| 决策「始终允许」作用域误用 | 高 | 只按 `(conversation_id, tool_name)` 记忆；不落盘；登出/新会话清空 |

## 7. 需要用户拍板

- [ ] **A. 「始终允许本会话」按工具名还是按「工具 + 参数指纹」？** 推荐按工具名（同 DSH/常见 CLI 习惯），风险是同一工具不同路径被放行；
      备选按参数指纹（更安全但会重复弹窗）。
- [ ] **B. 剩余时间展示**：审批超时由宿主 broker 计时（UI 只倒计时显示，超时以事件通知）——推荐；备选 UI 本地计时（可能不一致）。
- [ ] **C. 历史 `role="tool"` 数据**：保留现有 JSON 行格式（只改 UI 解析）还是新增结构化 `tool_calls` 字段并迁移？
      推荐保留 JSON 行（向后兼容、零迁移），结构化字段留到 Phase 2。

> 未拍板前状态保持 `draft`，不进入实现。

## 8. 变更预算与拆分

- 预计改动文件：契约 3（agent/session/ipc）+ 宿主 3（approval.rs/events.rs/ipc.rs）+ agent-rig 2 + ui-kit 1 + ui-chat 4 +
  desktop web 3（protocol/reducer/controller）+ i18n 1 + 测试 4 + 文档 2 ≈ 23。
- 拆分：不拆；四项都是同一「聊天渲染/审批正确性」主题。

## 9. 复用与已有实现（C17）

- 已搜索：`rg "ApprovalPart|ApprovalDecision|ToolCallFinished|parseAgentEvent" packages apps`；`docs/generated/CAPABILITY-INDEX.md`。
- 可复用：`@zeokeel/ui-kit`（Button/Chip/icons）、`ui-chat` 现有 `ApprovalPart`/`ToolCallPart`、`zk-kit-approval` 全局 gate 槽、
  `BridgeSink` 会话写入器、e2e 现有审批用例。
- 本次新增可复用能力：`zk-contract-agent::ApprovalDecision` 扩展与 `ApprovalGate::decide_with_scope`（带默认实现）；
  工具行 JSON 视图解析器放 `ui-chat`（后续 Phase 2 可提升为 keyed tool view）。
- 为什么不能复用/扩展：现有 bool gate 无法表达「始终允许本会话/取消」；工具行只按 assistant 原文渲染，缺解析层。
