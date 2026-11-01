# 2026-11-01-client-approval-tool-rendering · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/agent-rig`、`zeokeel/ui-chat`、`zeokeel/ui-kit`；桌面宿主；不新增包 |
| 契约 | `zk-contract-agent`：`ApprovalRequested`/`ToolCallFinished` 增加字段、`ApprovalDecision::AllowSession`；`zk-contract-tool`：`ApprovalGate::decide_scoped`（默认实现转调 `decide`）；`zk-contract-ipc`：`ApprovalDecidePayload` 复用枚举，无需新命令 |
| 路由 | 无新增路由；`approval.decide` 载荷枚举扩展 |
| 迁移 | 无数据库迁移；会话 `role="tool"` 行字段增加（旧行默认解析） |
| 生成物 | schema/TS（C10/C11）由 `cargo test export_schema` + 脚本生成 |
| edition/profile/channel | 无差异（personal/enterprise 同 UI） |
| 文档 | `docs/19` §4 状态、`docs/28` 工具/审批口径、parity 报告回写 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 审批等待/超时 | `apps/desktop/src/approval.rs` `ApprovalBroker`；`zk_kit_approval::set_global_gate` | 扩展（枚举决策 + 会话记忆） | 桌面宿主 |
| 审批事件 | `AgentEvent::ApprovalRequested`/`Resolved`；`hooks.rs` | 扩展字段 | agent-rig |
| 工具卡渲染 | `ui-chat/ToolCallPart.tsx`（现有折叠行）、`ApprovalPart.tsx` | 复用/扩展 | ui-chat |
| 会话写入 | `BridgeSink`（`apps/desktop/src/events.rs:60-76`） | 扩展字段 | 桌面宿主 |
| 事件解析 | `apps/desktop/web/src/protocol.ts` `parseAgentEvent` | 扩展 | desktop web |
| 启动 i18n | `apps/desktop/web/src/i18n.tsx` `translate(locale,key)` | 复用 | desktop web |

- 搜索证据：`rg "ApprovalGate|approval.decide|role.*tool|parseAgentEvent" packages apps`。
- 为什么不能复用：bool gate 无法表达 allow_session/cancel；UI 没有工具行解析层。
- 新增可复用能力：`@capability contract.approval`（枚举决策 + 元数据 DTO）；`ui-chat` 工具行解析器（后续可提升为 keyed view）。

## 2. 方案

1. **契约**（`zk-contract-agent`）：
   ```text
   ApprovalRequested { approval_id, tool, args, reason,
     summary, cwd, read_paths, write_paths, risk: ApprovalRisk, deadline_ms,
     diff: Option<String> }
   ToolCallFinished { name, args, result, status }
   ApprovalDecision { AllowOnce, AllowSession, Reject, Cancel }
   ```
   全部 `#[serde(default)]`；`ApprovalRisk` 为 closed enum（low/medium/high）。
2. **agent-rig**：在工具闭包内（已有 `TurnContext` + `ToolSpec` + args）构造元数据：
   - `summary`：`"<verb> <target>"`（tool 名 + 关键参数单行）；
   - `cwd`：`context.workspace_root`；
   - `read_paths`/`write_paths`：从 args 的 `path`/`paths`/`cwd` 提取并相对化判断；
   - `risk`：按工具族映射（只读→low；write/edit→medium；shell/pwsh/terminal/worktree→high）；
   - `deadline_ms`：`now + approval_timeout_ms`；
   - `diff`：仅 `fs.write`/`fs.edit`，读取目标文件（≤256KB）生成最小 unified diff；失败为 `None`。
   - `always-allow`：`Mutex<HashSet<(String,String)>>`（conversation_id, tool）放 runtime；命中则直接 `AllowSession` 事件、不阻塞。
3. **宿主 broker**：`decide` 返回 `ApprovalDecision`；`resolve(id, decision)`；等待队列 + 超时返回 `Reject` 并 emit `approval_resolved{reject, timed_out:true}`（事件加 `timed_out` 字段默认 false）；
   broker 维护 `HashSet<(conversation, tool)>` 会话允许集合（进程内、随登出/新会话清理）。
4. **i18n**：`main.tsx` 用 `translate("zh-CN", "bootErrorTitle")` 等；`i18n.tsx` 增加 3 个键；`i18n-check.sh` 通过。
5. **UI**：`ApprovalPart` 重写为安全卡（分组：动作/影响面/风险/超时/决策）；`toolRows.ts` 提供
   `parseToolRows(messages): { assistantId, toolCalls }[]` 与 `mergeToolRows`；`reducer` 在 `session.messages` 载入后归并；
   实时事件仍走现有流式 state（不重复渲染）。
6. **usage**：`ui-kit/types.ts` 增加 `{type:"usage"; usage}`；`protocol.ts` 解析；`reducer` 写当前 streaming message 的 usage。

### 2.1 关键流程

```text
tool requires_approval → agent-rig 构造元数据 → emit approval_requested
  → broker 等待 → UI 展示/倒计时 → approval.decide(decision)
  → broker resolve → agent-rig continue/reject → emit approval_resolved → 工具执行或拒绝
tool finished → emit tool_call_finished{name,args,result,status} → BridgeSink 写会话行
reload session → reducer 载入消息 → toolRows.parse 归并工具卡
```

### 2.2 数据结构

```text
ApprovalRisk = "low" | "medium" | "high"
ApprovalDecision = "allow_once" | "allow_session" | "reject" | "cancel"
SessionToolRow (role="tool" content JSON) = { name, args?, result, status? }
```

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 扩展事件字段 + JSON 行解析（本设计） | 零迁移、兼容旧数据 | 解析层需容错 | 采用 |
| B. 新增结构化 `tool_calls` 表/字段 | 类型安全 | 契约+迁移面大 | 放弃（Phase 2 可评估） |
| C. 审批元数据由宿主补 | 不改 agent-rig | 宿主拿不到 workspace/diff 上下文 | 放弃 |

## 4. 安全与隐私

- 审批 fail-closed：超时/断线/解析失败一律拒绝；迟到决策无效。
- `always-allow` 仅进程内、按会话+工具名；不落盘、不跨会话；登出/切换会话清理。
- diff 只读目标文件、限 256KB、不写日志；审批卡不展示 env/token（沿用参数脱敏）。
- 企业弱管控：不做服务端审批策略（C6）。

## 5. 发布与回滚

- 无灰度；随客户端发布。
- 回滚：代码回退；事件/会话字段 `serde(default)`，新版写旧版可读。
- 客户端兼容：企业/个人同构建；旧会话数据（无 args/status）降级展示。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | schema 生成/`--check` | `zk compose ... --check && cargo test -p zk-contract-agent export_schema` |
| T1 单包 | 契约兼容、agent-rig 元数据、broker 决策/超时、i18n | `cargo test -p zk-contract-agent -p zk-agent-rig -p zk-desktop`；`bash scripts/i18n-check.sh` |
| T2/T3 | e2e 审批/刷新/用量 | `pnpm e2e` |
| T4/T5 | enterprise 构建 + 截图 | `zk compose --edition enterprise ... && pnpm -r build` |

## 7. 文档更新

- [ ] `docs/19` §4 审批卡状态（M1 后置项转已实现）
- [ ] `docs/28` 工具/审批口径
- [ ] `research/client-dsh-parity-2026-11-01.md` 缺口状态
- [ ] `docs/README.md` 主题登记：否

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| AgentEvent 扩展破坏 e2e/mock 传输 | 中 | 高 | serde default + mock 传输补字段 | e2e/mock 全量 |
| broker 枚举改动影响现有审批测试 | 中 | 中 | 保留 bool 适配层 | `cargo test -p zk-desktop` |
| diff 计算开销/权限 | 中 | 低 | 限 256KB + 失败回退 | 单测 |
| 工具行归并顺序错误 | 中 | 中 | golden trace（中文/emoji 分片） | ui-chat 单测 + e2e |
