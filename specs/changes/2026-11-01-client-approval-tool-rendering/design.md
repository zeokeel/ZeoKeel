# 2026-11-01-client-approval-tool-rendering · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。
> 评审基线：`docs/reviews/2026-11-01-client-ui-parity-specs.md`；本版按 P0-2/P0-3/P1-4/P1-5/P1-11/P1-12/P1-13/P1-14/P1-16/P1-1/P1-2/P2-1/P2-2/P2-9 修复。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/agent-rig`、`zeokeel/ui-chat`、`zeokeel/ui-kit`、`zeokeel/store-session-sqlite`、`zeokeel/tool-fs-write`、`zeokeel/tool-fs-edit`、`zeokeel/tool-bash`、`zeokeel/tool-pwsh`、`zeokeel/tool-worktree`、`zeokeel/connector-mcp`、`zeokeel/desktop`；`crates/zk-kit-tools`；**编译面**：`zeokeel/agent-loop`（src+tests）与 `zeokeel/tool-session-query`（tests，P1-R2）；不新增包 |
| 契约 | **`zk-contract-tool`（底层）**：定义 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision`（+ `AllowSession`/旧值 alias），`Tool::sensitive_arg_keys()`（trait 默认方法，不动 `ToolSpec`）、`Tool::approval_metadata`（默认 `None`）、`ApprovalGate::decide_scoped`（默认实现）；**`zk-contract-agent`**：`pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk};` 保持旧路径兼容，`ApprovalRequested` 加 `auto/deadline_ms/metadata`、`ToolCallFinished` 加 `args/status`、`ToolCallStatus`；**`zk-contract-session`**：`Message.turn_id`。依赖方向仅 agent → tool（已有），**禁止 tool → agent**（P0-13.1，无环）；跨包编译面：`packages/client/agent-loop`（src+tests）与 `packages/client/tool-session-query/tests`（P1-R2） |
| 路由 | 无新增路由；`approval.decide` 载荷改 typed 解析（`ApprovalDecidePayload` 已存在，`packages/contracts/ipc/src/lib.rs:250-258`） |
| 迁移 | **SQLite v5**：`messages` 加 `turn_id TEXT NOT NULL DEFAULT ''`（仅加列，幂等）；旧会话行 `turn_id=''` 走 legacy 归并 |
| 生成物 | schema/TS（C10/C11）由 `cargo test --workspace export_schema` + `bash scripts/schema-check.sh` 生成/校验 |
| edition/profile/channel | 无差异（personal/enterprise 同 UI；企业不审批由 C6 决定，不在此改） |
| 文档 | `docs/19` §4.1/§4.2 状态、`docs/38` §2/§3 事件与决策枚举、`docs/28` 工具/审批口径、**`docs/24` 契约依赖方向（agent→tool 单向、类型归属 tool）**、`KNOWN-LIMITATIONS` 脱敏/legacy 边界、parity 报告 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 审批等待/超时 | `apps/desktop/src/approval.rs` `ApprovalBroker`（`decide` 阻塞 + `resolve`）；`zk_kit_approval::set_global_gate` | 扩展（typed 决策 + 超时事件；**不**加会话允许集合） | 桌面宿主 |
| 审批事件 | `AgentEvent::ApprovalRequested`/`Resolved`（`packages/contracts/agent/src/lib.rs:111-121`） | 扩展字段 | agent-rig / 契约 |
| 工具元数据 | 无（旧设计在 agent-rig 猜参数） | 新增 `Tool::approval_metadata`（工具侧单一实现） | 契约 + 各工具包 |
| 脱敏 | `zk-kit-tools`（`safe_path`/`read_text`/`truncate`，`crates/zk-kit-tools/src/lib.rs`） | 新增单一 redaction 函数 | kit |
| 工具卡渲染 | `ui-chat/ApprovalPart.tsx`、`ToolCallPart.tsx` | 重写/扩展 | ui-chat |
| 会话写入 | `BridgeSink`（`apps/desktop/src/events.rs:60-79`） | 扩展字段 + turn 归属 | 桌面宿主 |
| 事件解析 | `apps/desktop/web/src/protocol.ts` `parseAgentEvent` | 扩展 | desktop web |
| 启动 i18n | `apps/desktop/web/src/i18n.tsx` `translate(locale,key)` | 复用 | desktop web |
| 审批 e2e | `scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`（真实 personal rig + 假上游；`playwright.agent.config.ts`） | 扩展 | e2e |

- 搜索证据：`rg "ApprovalGate|approval.decide|role.*tool|parseAgentEvent|approval_metadata" packages apps`；`./target/debug/zk find approval` → `kit.approval`；`grep -n 'zk-contract-tool' packages/contracts/agent/Cargo.toml`（agent→tool 已存在）。
- **契约类型归属/依赖方向（P0-13.1）**：审批类型放 `zk-contract-tool`（更底层）；agent 只 re-export 并把事件字段指向 tool 类型；**禁止**在 tool 的 trait 签名里引用 agent 类型（否则 Cargo 环）。新增证据：`cargo tree -p zk-contract-tool` 无 `zk-contract-agent`。
- **不新增 capability id**：复用 `contract.agent`（`shared/contracts/capability-index.json:11`）与 `contract.tool`（`:71`）；`ui-chat` 工具行解析器是包内实现，不登记 INDEX；
  `./target/debug/zk capability --check` 必须 exit 0 且 INDEX 条数不变（评审 P1-4）。
- 为什么不能复用/扩展：旧 `ApprovalGate::decide -> bool` 无法表达 `allow_session/cancel`（加带默认实现的 `decide_scoped`）；
  UI 没有工具行解析层（新增 `ui-chat/toolRows.ts`）；agent-rig 无工具侧元数据通道（新增 `Tool::approval_metadata` 默认方法）。

## 2. 方案

### 2.0 审批事件序列表（**权威表**；与 config spec R2 共享，不复制）

> 本表是 `specs/changes/2026-11-01-client-approval-tool-rendering` 与 `specs/changes/2026-11-01-client-config-approval-mcp` R2 的**同一张表**。
> config spec R2 的「`requested → decided` 成对或缺省、不得孤立 resolved」由本表行 S1–S3c/S5 满足；**已与 config 作者对齐（Lead 2026-11-01）**：config R2 改为引用本表，引用由 config 作者在复评前落实（本 spec 不改 config）。
> 求值顺序（单 owner=agent-rig）：**① 策略求值（config `should_ask`；approval 先于 config 时用默认实现 `should_ask = requires_approval`）** → ② 不需要询问 → S5；
> 需要询问 → **③ always-allow 命中？** → S4；未命中 → ④ 发请求并阻塞 → S1/S2/S3a/S3b/S3c。
> **不新增 `approval_skipped` 事件**（Lead 2026-11-01）：策略短路按 S5 完全不发审批事件；always-allow 命中按 S4 发成对 `auto:true` 事件。

| # | 场景 | 事件序列（agent-rig 发出；宿主 broker 只阻塞/超时） | 说明 |
| --- | --- | --- | --- |
| S1 | ask + 用户允许一次 | `approval_requested{auto:false}` → `approval_resolved{allow_once}` → `tool_call_started` → `tool_call_finished{success}` | 成对；工具行落库 |
| S2 | ask + 用户拒绝 | `approval_requested{auto:false}` → `approval_resolved{reject}` → `tool_call_finished{denied}` | 成对；**无** `tool_call_started`；终态行落库 |
| S3a | ask + 用户**显式取消**（审批卡 cancel） | `approval_requested{auto:false}` → `approval_resolved{cancel,timed_out:false}` → `tool_call_finished{cancelled}` → `turn_cancelled`（agent-rig 统一发） | 成对；cancel 单一路径见 §2.5.3 |
| S3b | ask + **超时** | `approval_requested{auto:false}` → `approval_resolved{reject,timed_out:true}` → `tool_call_finished{denied}`（**无** `turn_cancelled`） | 成对；fail-closed，turn 继续处理拒绝 |
| S3c | ask + **断线/窗口关闭/chat.cancel 等待者消失** | `approval_requested{auto:false}` → `approval_resolved{reject,timed_out:false}` → `tool_call_finished{denied}`；若因 `chat.cancel` 取消 turn，则另由 agent-rig 发 `turn_cancelled` | 成对；fail-closed；不把断线当显式 cancel |
| S4 | always-allow 命中（未经询问） | `approval_requested{auto:true}` → `approval_resolved{allow_session}` → `tool_call_started` → `tool_call_finished{success}` | **成对**、不阻塞、不弹决策；UI 渲染「已自动批准」；工具行落库 |
| S5 | 策略求值 = 不需要询问（config R2） | （无任何 `approval_*`） → `tool_call_started` → `tool_call_finished{success}` | 与 config R2「不产生 ApprovalRequested/孤立 Resolved」一致 |

- **取消/超时/拒绝映射（唯一表）**：本表 S3a/S3b/S3c 即唯一映射——**超时 → `reject + timed_out:true`**；**用户显式取消 → `cancel`**；**断线/窗口关闭 → `reject + timed_out:false`**；用户拒绝 → `reject`（S2）。spec/R8 与 UI 只引用本表，不另立第二份表述（P2-N2）。
- 所有 `approval_*` 事件都带 `approval_id`；`approval_requested` 带 `auto`、`deadline_ms`、`metadata`；`approval_resolved` 带 `decision`、`timed_out`（默认 false）。
- 迟到决策：broker 找不到 waiter（已超时/已被取消）→ `approval/pending-not-found`，UI 忽略；事件不重复发。
- 每 turn 结束时 `turn_completed`/`turn_cancelled` 仍是终态事件（沿用 `docs/38` §3 顺序）。

### 2.1 关键流程

```text
tool requires_approval
  → agent-rig: should_ask(policy, spec)            [策略，单 owner]
      ├─ false → S5（不发审批事件，直接执行）
      └─ true
          → always_allow 命中？                    [会话记忆，单 owner]
              ├─ true  → S4（成对 auto 事件，不阻塞）
              └─ false → redact args → tool.approval_metadata → 发 requested{metadata,deadline}
                          → broker.wait(deadline) [主机只阻塞/超时]
                          → 决策（typed）→ 发 resolved → 允许执行 / denied / cancelled
tool end（含 denied/cancelled）
  → 发 tool_call_finished{args(redacted), status} → BridgeSink 写 role=tool 行（带 turn_id）
turn end
  → ipc 用 BridgeSink 记录的 turn_id 写 assistant 行（带 turn_id）
reload
  → messages_in 读回（含 turn_id）→ reducer 解析 role=tool → toolRows 按 turn_id 归并（legacy 回退见 §2.5）
```

### 2.2 契约与类型（`zk-contract-agent` / `zk-contract-tool` / `zk-contract-session`）

```text
// zk-contract-tool（定义层；不依赖 zk-contract-agent）
pub struct ApprovalMetadata {
  summary: String, cwd: String,
  read_paths: Vec<String>, write_paths: Vec<String>,
  network_targets: Vec<String>,            // "host:port" 或 "<mcp:server-id>"
  network_undetermined: bool,              // 无法静态判定（shell/自由文本）
  risk: ApprovalRisk,                      // low|medium|high（closed enum）
  diff: Option<String>,                    // 工具侧生成，<=256KB；超限/失败 None
}
pub enum ApprovalRisk { Low, Medium, High }
pub enum ApprovalDecision { AllowOnce, AllowSession, Reject, Cancel } // snake_case；allow_once alias "approved"；reject alias "rejected"
trait Tool {
  fn spec(&self) -> ToolSpec;
  fn sensitive_arg_keys(&self) -> &'static [&'static str] { &[] }  // 默认无敏感键；需要时工具覆写
  fn approval_metadata(&self, args: &serde_json::Value, context: &TurnContext)
      -> Option<ApprovalMetadata> { None }        // 默认不声明；工具侧单一实现
  ...
}
trait ApprovalGate {
  fn decide(&self, id, tool, args, reason) -> bool;                 // 旧方法保持（第三方实现不破）
  fn decide_scoped(&self, id, tool, args, reason) -> ApprovalDecision { // 新方法默认转调
    if self.decide(...) { AllowOnce } else { Reject }
  }
}

// zk-contract-agent（已依赖 tool；仅 re-export + 事件）
pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk}; // 保持 zk_contract_agent::* 旧路径
AgentEvent::ApprovalRequested { approval_id, tool, args, reason,
  #[serde(default)] auto: bool,
  #[serde(default)] deadline_ms: u64,
  #[serde(default)] metadata: Option<zk_contract_tool::ApprovalMetadata> }
AgentEvent::ApprovalResolved { approval_id, decision: zk_contract_tool::ApprovalDecision,
  #[serde(default)] timed_out: bool }
AgentEvent::ToolCallFinished { name, result,
  #[serde(default)] args: String,
  #[serde(default)] status: ToolCallStatus }   // ToolCallStatus 留 agent：success|failed|denied|cancelled，默认 success

// zk-contract-session
Message { ..., #[serde(default)] turn_id: String }  // 旧行空串
```

- `ApprovalMetadata` 的每个字段都 `#[serde(default)]`（`risk` 默认 `medium`、布尔默认 `false`、字符串/列表默认空、`diff` 默认 `None`）；
  `ApprovalRisk` 为 closed enum（low/medium/high，不带 unknown）；`ToolCallStatus` 默认 `success`（留在 agent）。这些字段只由本地工具/agent-rig 产出，不作为外部输入校验面。
- **依赖方向（P0-13.1）**：`zk-contract-tool` 不依赖 `zk-contract-agent`；agent 已依赖 tool（`packages/contracts/agent/Cargo.toml`），因此类型定义在 tool、agent `pub use` 不产生环。
  `ApprovalDecidePayload`（`packages/contracts/ipc/src/lib.rs`）继续写 `zk_contract_agent::ApprovalDecision` 亦可编译（re-export 兼容），生成 schema 不变。

### 2.3 决策 wire 迁移（P1-11）

- 宿主 `approval.decide` 分支（当前 `apps/desktop/src/ipc.rs:728-740` 用 `decision == "approved"`）改为：
  `serde_json::from_value::<ApprovalDecidePayload>(payload)` → `broker.resolve_scoped(approval_id, decision)`；
  非法值 → `CommandResponse::err("approval/invalid-decision")`；未知 `approval_id` → `approval/pending-not-found`（保持）。
- `ApprovalDecision` 的 `serde(alias)` 只服务宿主兼容旧客户端；前端 TS/生成 schema 只接受 4 个 canonical 值。
- 前端 `controller.ts` 的 `decideApproval(approvalId, decision)` 改为 4 值 union；`reducer`/`ApprovalView.decision` 同步为
  `pending|allow_once|allow_session|reject|cancel`（展示层把旧 `approved/rejected` 映射到 `allow_once/reject`）。
- `Cancel` 可达路径（单一路径）：审批卡「取消」按钮 → `approval.decide{cancel}` → 宿主 `resolve_scoped(Cancel)` + `runtime.cancel_agent()` → agent-rig 置 cancelled 并统一发 `turn_cancelled`；**断线/窗口关闭不是 cancel**（按 S3c 映射 reject）。
- 单测：4 canonical + 2 legacy alias + 非法值 + 旧 `approved` 经 alias 得 `allow_once`。

### 2.4 审批元数据：工具侧声明（P1-16）

- `Tool::approval_metadata(&args, &ctx)` 在工具包内实现，agent-rig 只做 `metadata = tool.approval_metadata(...)` 聚合；**不读文件、不猜路径**。
- 必须实现的关键工具（任务验收）：
  | 工具 | read_paths | write_paths | network_targets | risk | diff |
  | --- | --- | --- | --- | --- | --- |
  | `fs.write` | — | `[target]`（相对化） | — | medium | 工具侧生成 unified diff（读旧文件 ≤256KB；新建文件摘要） |
  | `fs.edit` | — | `[target]` | — | medium | 同上（old/new 片段） |
  | `bash` / `pwsh` | — | — | `network_undetermined=true` | high | — |
  | `mcp_call` | — | — | `["<mcp:{server}>"]` | high | — |
  | `create_worktree` | `[path or workspace]` | `[path]` | — | high | — |
  | `web.fetch` / `web.search`（可选） | — | — | `[host[:port]]`（从 URL 解析） | low/medium | — |
- 未实现 `approval_metadata` 的工具：`metadata=None`；UI 显示「该工具未声明影响面」+ degraded 标记（不阻塞、不伪造）——Lead 2026-11-01 已定。
- `cwd` = `context.workspace_root`；`summary` 由工具侧给（`"<verb> <target>"`）；路径相对化由工具侧用 `zk_kit_tools` 的 workspace 语义完成，agent-rig 不再做路径推断。

### 2.5 turn 归属、持久化与重放（P0-3 / P1-R3）

#### 2.5.1 契约与存储

- `Message.turn_id`（`#[serde(default)]`）。SQLite v5：`ALTER TABLE messages ADD COLUMN turn_id TEXT NOT NULL DEFAULT ''`
  （`has_column` 幂等；`schema_version` 升 5；`append_message`/`messages_in` 增列；旧行读回空串）。旧二进制对新 DB 按其固定列 SELECT，不读新列。
- 编译面（P1-R2 选项①）：`ToolSpec` 不变；`Tool::sensitive_arg_keys()` 为默认方法；新增字段仍需同步
  `packages/client/agent-loop/src/lib.rs`（两处 `ToolCallFinished` 字面量）、`packages/client/agent-loop/tests/*`、
  `packages/client/tool-session-query/tests/tool.rs`（`Message {` 字面量）。

#### 2.5.2 宿主写入顺序与 assistant status（P1-R3①②）

- `BridgeSink` 对**终态** `ToolCallFinished`（`success|failed|denied|cancelled`）**先写会话 DB，再判断 cancelled 短路**：
  当前实现（`apps/desktop/src/events.rs:60-79`）在 `if cancelled && !TurnCancelled { return; }`（`:62`）之后才落库，
  `chat.cancel` 置 `job.cancelled`（`apps/desktop/src/ipc.rs:1745-1752`）后所有事件被丢弃——取消路径的终态行会丢。
  新顺序：① 若 `ToolCallFinished` → 用 `current_turn_id` 写 `role="tool"` 行（content=`{name,args,status,result}`）；② 再决定 bus：
  `TurnCancelled` 始终发；cancelled 时抑制**非终态**事件（避免迟到 delta/tool-start 污染 UI）。
- `BridgeSink` 增加 `turn_cancelled: AtomicBool`（收到 `TurnCancelled` 置 true）。`ipc.rs` 的 assistant 行 status **不再**直接用 `job.cancelled`：
  改为 `if bridge.turn_cancelled() { "cancelled" } else { "done" }`（覆盖审批 cancel 与 `chat.cancel` 两条路径；P1-R3②）。
- `run_chat` 持有 `Arc<BridgeSink>`（当前 `Arc::new` 临时值，改为 `let bridge = Arc::new(...)` 后 `sink = bridge.clone()`），
  结束后用 `bridge.turn_id()` 写 assistant/错误消息（`assistant_message(..., turn_id)`）。

#### 2.5.3 approval-cancel 单一路径（P1-R3②）

0. agent-rig 工具闭包在进入 gate 前已能构造终态：拒绝路径（`agent-rig/src/lib.rs:345-356`）与取消路径（`:357-359`）在 return 前**必须**发
   `tool_call_finished{args: redacted, status: denied|cancelled, result: ...}`；成功/失败路径补 `args` 与 `status: success|failed`（P0-3 回归修复）。
0b. **执行中取消（P1-13.2 已冻结）**：工具 outcome 返回后（`agent-rig/src/lib.rs:397/422` 的 Ok/Err 分支）必须**再检查** `cancelled_for_tool`：
   若已置位 → 发 `tool_call_finished{status: cancelled, result: <工具已产生的文本原样>}`（**不伪造失败**），返回给模型的 `ToolOutput` 文本仍为原结果（诚实副作用）；
   若工具尚未执行（gate 前取消）→ `status: cancelled`、`result: ""`。UI 规则见 §2.5.4；单测覆盖「慢工具 + 执行中取消」。
1. UI `approval.decide{decision:"cancel"}` → 宿主 typed 解析 → `broker.resolve_scoped(approval_id, Cancel)`（解除等待）。
2. 宿主**同一步**调用 `state.runtime.cancel_agent()`（= 置 agent-rig 内部 `cancelled`，现有 `RigAgent::cancel`），**不**在该分支手发 `turn_cancelled`。
3. agent-rig 在 gate 返回 `Cancel` 时：发 `tool_call_finished{cancelled}`，置内部 cancelled，返回；`run_stream_in_shared*` 的既有分支
   （`agent-rig/src/lib.rs:878-890`）**统一发** `turn_cancelled`（且只发一次）。
4. `chat.cancel` 同样走「释放等待 + 置 cancelled + `cancel_agent()`」：先让 broker 把所有 pending waiter 以 `Reject` 释放
   （否则 agent-rig 会阻塞到审批超时），再置 cancelled；并删除其手发 `turn_cancelled`（`ipc.rs:1873-1879` 的 `emit_kind(... "turn_cancelled")`），
   保证单一 emitter（P1-R3②）。返回值/错误文案不变。
5. 断线/窗口关闭：broker 等待通道断开（`recv_timeout` 的发送端被 drop）→ 视为 `Reject`（S3c），不置 cancelled、不发 `turn_cancelled`；
   应用退出同理走 broker `release_all(Reject)`。

#### 2.5.4 UI 解析（`ui-chat/toolRows.ts`）

- 新数据：按 `Message.turn_id` 分组；tool 行归入同 turn 的 assistant（或该 turn 的错误消息）；无 assistant 时渲染独立「未归属工具记录」组。
- legacy（`turn_id==''`）：同一会话内连续 tool 行 → 归入**后一条** assistant；其后无 assistant → 独立组（degraded）；边界记 `KNOWN-LIMITATIONS`（Lead 2026-11-01 已定）。
- 损坏 JSON：降级为可折叠原文 + `toolRecordUnparseable`；未知字段忽略。
- 排序：按 `created_at_ms, id`；同一 assistant 下 toolCalls 按该顺序。
- 状态：持久化行只含终态 `success|failed|denied|cancelled`；`pending|running` 仅存在于流式内存态（`tool_call_started` 之后、`tool_call_finished` 之前）；
  旧数据缺 status → `success` 并标 legacy degraded。
- `cancelled` 展示（P1-13.2）：`result` 非空 → 显示「已取消（副作用可能已发生）」+ 原结果文本（只读展示，不当作成功）；`result` 为空 → 仅显示「已取消」。
  该提示走 i18n `toolCancelledSideEffects`，刷新后由持久化 status/result 复原。

#### 2.5.5 golden 与 e2e

- 真实 `SqliteSessionStore`（临时 DB）写 `user → tool(s) → assistant`，断言 `messages_in` 原始顺序；再用 `toolRows` 归并断言
  toolCalls 全挂在 assistant 上、`turn_id` 正确（中文/emoji 内容分片场景）。
- **工具执行中取消 → 刷新后工具卡 `cancelled`**：`scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts` 新增用例（慢工具执行中触发取消，刷新后断言卡状态 `cancelled`、**包含「副作用可能已发生」提示（result 非空时）**、无白屏、DB 行存在）。

### 2.6 脱敏（P1-13）

- 单一实现：`crates/zk-kit-tools` 新增 `pub fn redact_tool_args(sensitive_keys: &[&str], args: &serde_json::Value) -> serde_json::Value`：
  1. 递归遍历对象：键名 ∈ `sensitive_keys`（大小写不敏感，来自 `tool.sensitive_arg_keys()`）→ 值置 `"***"`；
  2. 对所有字符串值应用内置模式（至少：`sk-[A-Za-z0-9_-]{8,}`；`Bearer\s+\S{8,}`；`(?i)(api[_-]?key|token|secret|password)\s*[=:]\s*\S+`）；
  3. 保留对象结构/其余可读文本（bash command 只掩码 secret 子串）。
- 调用点唯一：agent-rig 在发 `approval_requested` 与 `tool_call_finished` **之前**调用一次；UI 与 `BridgeSink` 都消费事件里的已脱敏 `args`（不存在第二条展示路径）。
- 日志/错误文案不得打印未脱敏 args；敏感键由工具包覆写 `Tool::sensitive_arg_keys()` 声明（shipped 工具默认空；`mcp_call.arguments` 走递归键名+正则；机制有合成 `Tool` 实现单测，不扫改 30 个工具包）。
- 边界（`KNOWN-LIMITATIONS`）：任意格式自造密钥、base64/多层编码内嵌密钥、密钥不在 key/value 模式内（如纯随机串）无法保证；不宣称零泄漏。

### 2.7 i18n

- `apps/desktop/web/src/main.tsx:47,49` 的启动错误边界文案改为 `translate("zh-CN", "bootErrorTitle")` / `"bootErrorHint"`（bootstrap 期默认 locale）；
  `i18n.tsx` 增加 zh/en 词条；新增审批卡文案键（`approvalTimedOut`/`approvalCancelled`/`approvalMetadataMissing`/`approvalAutoApproved`/`approvalRemaining` 等）。
- `bash scripts/i18n-check.sh` exit 0（任务 #11）。

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 事件字段 + JSON 行解析 + `Message.turn_id`（本设计） | 零会话迁移（仅加列/可空字段）、兼容旧数据 | 解析层需容错；legacy 回退规则 | 采用 |
| B. 新增结构化 `tool_calls` 表/字段 | 类型安全 | 契约+迁移面大 | 放弃（Phase 2 可评估） |
| C. 审批元数据由 agent-rig 按 args 猜测并读文件生成 diff（旧设计） | 不改工具包 | 第二份工具知识（C17）、绕过工具抽象、安全/编码分叉 | **放弃**（P1-16） |
| D. always-allow 命中时「都不发事件」 | 严格符合「不再弹」 | 缺审计痕迹；与 UI「已自动批准」不可见 | 放弃；采用 S4 成对 `auto:true`，UI 不阻塞 |
| E. broker 维护会话允许集合并求值 | 宿主统一 | 双实现/清理时机不一致（P0-2） | **放弃**；单 owner=agent-rig，broker 只阻塞 |
| F. 审批类型留在 agent、tool 依赖 agent（旧稿） | agent 内聚 | **Cargo 依赖环（agent→tool 已有，tool→agent 新增）→ 不可编译** | **放弃**；采用 P0-13.1：类型定义在 tool、agent re-export |
| G. 执行中取消不改变已完成工具状态（旧稿） | 实现简单 | 与「取消后刷新 cancelled」验收冲突，且掩盖已发生副作用 | **放弃**；采用 P1-13.2：cancelled + 保留结果 + 副作用提示 |

## 4. 安全与隐私

- fail-closed：超时/断线/解析失败一律拒绝；迟到决策无效；`approval/pending-not-found` 不重开请求。
- always-allow 仅进程内、按 `(conversation_id, tool_name)`；不落盘、不跨会话；登出/切换会话清理（agent-rig 运行时）。
- 脱敏见 §2.6；展示/持久化/日志共用一份已脱敏载荷；未知边界写 `KNOWN-LIMITATIONS`。
- 企业弱管控：不做服务端审批策略（C6）；不新增网络拦截/流量检查。
- `approval_metadata` 不读文件（diff 由工具侧在自身抽象内生成）；agent-rig 不持有路径推断逻辑。

## 5. 发布与回滚

- 无灰度；随客户端发布。
- 回滚：代码回退；事件/会话字段 `serde(default)`，新版写旧版可读；SQLite v5 仅加列（旧二进制忽略新列，不破坏读取）。
- 客户端兼容：企业/个人同构建；旧会话数据（无 `turn_id`/`args`/`status`）走 legacy 归并 + degraded 标记。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合/生成物/编译面 | schema 生成/`--check`、INDEX 不变、跨包字面量可编译 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh`；`./target/debug/zk capability --check`；**`cargo check --workspace --all-targets`** |
| T1 契约 | 新字段默认值；`ApprovalDecision` 4 值 + alias + 非法值；`ApprovalMetadata` 序列化；**类型位置**（tool 定义、agent re-export 旧路径可用） | `cargo test -p zk-contract-agent -p zk-contract-tool -p zk-contract-session`；`cargo tree -p zk-contract-tool`（无 agent） |
| T1 kit | redaction：键名掩码、`sk-`/`Bearer`/`KEY=` 正则、保留结构 | `cargo test -p zk-kit-tools` |
| T1 工具 | `approval_metadata`：fs.write/fs.edit/bash/pwsh/mcp_call/create_worktree；路径/网络/risk/diff≤256KB；workspace 外路径 | `cargo test -p zk-tool-fs-write -p zk-tool-fs-edit -p zk-tool-bash -p zk-tool-pwsh -p zk-tool-worktree -p zk-connector-mcp` |
| T1 运行时 | 事件链 golden（S1–S5，含 S3a–S3c 映射）、always-allow 单 owner、denied/cancelled 终态、deadline、cancel 统一 TurnCancelled、**执行中取消（慢工具）→ cancelled 且 result 保留** | `cargo test -p zk-agent-rig` |
| T1 存储 | SQLite v5 迁移（v4→v5、旧行空 turn_id）、写入顺序 golden | `cargo test -p zk-store-session-sqlite` |
| T1 宿主 | broker typed 决策/超时、`approval.decide` 4 值+alias+非法、BridgeSink 终态先落库/cancelled 抑制 bus、turn_id/args/status、assistant status 来源 | `cargo test -p zk-desktop` |
| T1 前端 | ui-kit 类型、toolRows（新/legacy/损坏）、ApprovalPart 渲染/焦点 | `pnpm --dir packages/ui/ui-kit test:unit`；`pnpm --dir packages/ui/ui-chat test:unit`；`pnpm --dir apps/desktop/web test:unit` |
| T2/T3 e2e | 审批卡字段/默认焦点拒绝/Esc/超时/显式取消/始终允许(auto 事件对)/刷新一致/损坏行/usage/脱敏/turn 顺序/**工具执行中取消→刷新 cancelled + 副作用提示** | `bash scripts/agent-e2e.sh`（真实 personal rig + `e2e/agent-rig.spec.ts`） |
| T4/T5 | enterprise 构建 + 截图/axe | `zk compose --edition enterprise --profile default --channel official && pnpm -r build`；证据存 `target/evidence/2026-11-01-client-approval-tool-rendering/` |
| 门禁 | i18n/TS/Rust/契约 API | `bash scripts/i18n-check.sh`；`pnpm -r typecheck/lint/test:unit`；`cargo test` 相关包；`cargo check --workspace --all-targets`；`cargo public-api diff`（未装 → UNVERIFIED） |

## 7. 文档更新

- [ ] `docs/19` §4.1/§4.2：记录本 spec 覆盖 7/9+1/9 与三项非目标（记住范围/延长一次/二次确认）、通知 Phase 3
- [ ] `docs/24` §2.1/契约依赖说明：agent→tool 单向；`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 tool、agent re-export；禁止 tool→agent（P0-13.1）
- [ ] `docs/38` §2/§3：`ApprovalDecision` 4 值 + 旧值 alias；`ToolCallStatus`；S1–S5 事件序列表引用本 spec design §2.0
- [ ] `docs/28` §4.6 **工具/审批 DTO 口径**：①「工具终态枚举」= `success|failed|denied|cancelled`（持久化行）+ `pending|running`（仅流式内存态）；②审批决策枚举 4 值 + 旧值 alias。**不要**把工具终态与 AGENTS §7 的「组件六态」（空/加载/错误/离线/流式中/已取消）混写（P2-N3）
- [ ] `KNOWN-LIMITATIONS.md`：脱敏边界 + legacy 归并边界 + shell 网络目标不可静态判定
- [ ] `research/client-dsh-parity-2026-11-01.md` 缺口状态回写
- [ ] `docs/README.md` 主题登记：否

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| SQLite v5 迁移/双版本读写 | 低 | 高 | 仅加列 + 幂等 `has_column` + 旧行默认空 | 迁移单测 + e2e 刷新 |
| 工具行归并/legacy 方向 | 中 | 高 | turn_id 优先 + legacy 只挂后一条 assistant + golden | store golden + e2e |
| 工具侧元数据实现不一致 | 中 | 中 | 关键工具任务验收 + 未实现时诚实降级 | 工具包单测 + e2e 卡字段 |
| 脱敏不完整 | 中 | 中 | 单一实现 + 正则覆盖常见格式 + `KNOWN-LIMITATIONS` | 负向单测 + DB 断言 |
| typed gate 改动影响第三方实现 | 低 | 中 | `decide_scoped` 带默认实现转调旧 bool | 契约测试 |
| 事件扩展破坏 mock 传输 | 中 | 高 | serde default + mock 补字段 | e2e/mock 全量 |
| 生成物漂移 | 中 | 中 | export_schema + schema-check 进门禁 | acceptance §1 |
| P1-R2 跨包字段编译面（agent-loop/tool-session-query 字面量） | 中 | 高 | 仅 `ToolSpec` 不动；agent-loop(src+tests)/tool-session-query tests 进写作用域；workspace check 门禁 | `cargo check --workspace --all-targets` |
| P1-R3 取消路径终态行丢失 | 中 | 高 | BridgeSink 终态先落库再抑制 bus；TurnCancelled 单一 emitter；取消刷新 e2e | `zk-desktop` 单测 + `scripts/agent-e2e.sh` |
| P0-13.1 契约依赖环 | 低 | 高 | 类型定义移 tool、agent re-export；`cargo tree -p zk-contract-tool` 无 agent；workspace check | `cargo check --workspace --all-targets` + cargo tree |
| P1-13.2 执行中取消语义歧义 | 中 | 中 | 冻结「cancelled + result 原样 + 副作用提示」；result 空/非空区分执行前/后；单测+e2e | `zk-agent-rig` 慢工具单测 + agent-e2e |
| public-api 工具不可用 | 低 | 中 | acceptance 记 UNVERIFIED（不冒充）；以 `cargo tree`/workspace check 补无环证据 | acceptance §1 |

## 9. 跨 spec 对接（P1-1/P1-2）

- **顺序**：`settings-ia` → **本 spec（approval）** → `config` → `mcp-config`（Lead 决策；本 spec tasks §5 记录）。
  本 spec 对 config 的唯一依赖是「策略求值接口」：本 spec 用默认 `should_ask = requires_approval`（= 现状），config 合并时替换为 `ApprovalPolicy::should_ask` 并更新本表引用。
- **共享事件序列表**：本 design §2.0 是唯一真源；**已与 config 作者对齐（Lead 2026-11-01）**，config spec R2 改为引用本表（引用由 config 作者在复评前落实；本 spec 不修改 config）。
- **共享文件 owner**：`apps/desktop/src/ipc.rs`、`packages/client/agent-rig/src/lib.rs`、`ui-kit/web/src/types.ts`、`i18n.tsx` 在本 spec 实现期间
  只允许本 spec 写；config 后续任务以「后置排队」方式改（tasks §5）。
- **决策落文档前置**（tasks #0）：`docs/19` §4.1/§4.2 口径、`docs/38` §2/§3 事件/决策枚举、`docs/24` 契约依赖方向必须先于产品代码合并（C12）。
