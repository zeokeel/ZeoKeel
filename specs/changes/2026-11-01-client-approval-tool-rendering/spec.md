# 2026-11-01-client-approval-tool-rendering · 行为规格

> 只写「系统必须怎样」，不写实现选型；实现见 `design.md`。
> 评审基线：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（CHANGES-REQUESTED；本版为复评对象）。审批覆盖率口径 = `docs/19` §4.1 的 7/9 完整 + 1/9 部分（非目标见 `proposal.md` §3）。

## 1. 需求

### R1 · 审批请求元数据（工具侧声明，agent-rig 聚合）

- **WHEN** 任一工具即将执行且**策略求值为「需要询问」**（`should_ask`；always-allow 命中路径见 R3）
- **THEN** 运行时发出 `approval_requested` 事件，包含：`approval_id`、`tool`、`args`（**已脱敏**，见 R7）、`reason`、`deadline_ms`（绝对时间戳）、
  `auto`（待决策为 `false`；always-allow 自动批准为 `true`，见 R3），以及可选 `metadata`（`ApprovalMetadata`）：
  `summary`（一句话动作摘要）、`cwd`（绝对工作目录）、`read_paths`/`write_paths`（工具侧声明的路径列表，可空）、
  `network_targets`（`host:port` 或 `<mcp:server-id>`；可空）、`network_undetermined`（无法静态判定时为 `true`）、
  `risk`（`low|medium|high`）、`diff`（写类工具可为 null）。
- **AND** `metadata` 由**工具侧**通过 `Tool::approval_metadata`（默认 `None`）声明；`ApprovalMetadata`/`ApprovalRisk` 与 `ApprovalDecision` 均**定义在 `zk-contract-tool`**，`zk-contract-agent` 以 `pub use` 再导出（事件/旧路径 `zk_contract_agent::ApprovalDecision` 保持兼容）；agent-rig 不读文件、不猜 `path/paths/cwd`、不生成第二份「参数→影响面」知识。
- **AND** 元数据缺失（工具未实现）时事件 `metadata = null`；UI 显示「该工具未声明影响面」并标记 degraded，**不得**伪造 risk/路径/diff。
- **AND** 旧事件消费者未读新字段时仍可工作（全部新字段 `#[serde(default)]`）。

### R2 · 审批卡展示与决策（`docs/19` §4.1 子集）

- **WHEN** UI 收到 `approval_requested`（`auto=false`）
- **THEN** 审批卡展示：动作摘要、工具名 + 完整命令/参数（已脱敏；关键参数不可折叠隐藏）、工作目录（标记是否越出 workspace）、
  读写路径列表 + 网络目标（无法静态判定时明示）、写操作 diff（可展开）、风险级别、剩余时间倒计时（来自 `deadline_ms`）。
- **AND** 决策按钮：允许一次 / 始终允许（本会话） / 拒绝；**默认焦点在「拒绝」**；`Esc` = 拒绝；快捷键不得默认确认。
- **AND** 决策值映射为 `allow_once | allow_session | reject | cancel`；**超时/断线/窗口关闭/显式取消的映射以 `design.md` §2.0 的 S3a–S3c 为唯一表**（超时→`reject+timed_out`；显式取消→`cancel`；断线/窗口关闭→`reject`），本规格不另立第二份映射。
- **AND** 超时由宿主 fail-closed：超时后 UI 显示「已超时拒绝」，迟到点击不生效。
- **AND** 「记住范围选择器（含『仅该路径』scope）」「超时延长一次」「危险操作二次确认」为**非目标**（`proposal.md` §3，引用 `docs/19` §4.1/§4.2）；UI 不得宣称已提供。

### R3 · 「始终允许（本会话）」与审批链单 owner

- **WHEN** 用户选择「始终允许（本会话）」
- **THEN** 该 `(conversation_id, tool_name)` 在本会话内后续调用**不再阻塞等待**，但事件链留下**成对**记录：
  `approval_requested{auto:true}` → `approval_resolved{allow_session}`（UI 直接渲染「已按本会话允许自动批准」，不渲染待决策按钮、不抢焦点）。
- **AND** **策略求值与 always-allow 命中只在 agent-rig 求值**；宿主 broker 只负责 IPC 阻塞与超时，**不持有**会话允许集合（进程内、不落盘）。
- **AND** 作用域仅内存、仅当前会话、按工具名；新会话/重启/登出后清空，重新询问。
- **AND** 取消路径的 `turn_cancelled` 由 **agent-rig 统一发出**（审批卡 cancel 与 `chat.cancel` 共用一条路径；宿主不再手发第二次），宿主据其写 assistant `status=cancelled`。
- **AND** 事件序列必须与 `design.md` §2.0《审批事件序列表》一致；该表同时被 `specs/changes/2026-11-01-client-config-approval-mcp/spec.md` R2 引用（同一张表，不复制第二份）。
  具体要求：需要询问的路径必须 `requested → decided` 成对；策略短路（config R2 判定「不需要询问」）时**两者都不发**，不得出现孤立 `approval_resolved`。

### R4 · 工具调用持久化、turn 归属与重放

- **WHEN** 工具调用结束（成功/失败/拒绝/取消）
- **THEN** 运行时发出终态 `tool_call_finished`，包含 `name`、`result`、`args`（已脱敏）、`status ∈ success|failed|denied|cancelled`；
  拒绝与取消路径必须发该终态事件并落库（不得在发事件前提前 return）。
- **AND** 会话存储的 `Message` 增加 `turn_id`（`#[serde(default)]`，旧行读为空串）；工具行（`role="tool"` JSON：`{name,args,status,result}`）与 assistant 行
  使用同一 `turn_id`。
- **WHEN** 会话重新加载
- **THEN** UI 将 `role="tool"` 行解析为工具卡，按 **`turn_id` 归并到同一 turn 的 assistant 消息**（不是「前一条 assistant」）；
  终态显示 `success|failed|denied|cancelled`。
- **AND** **legacy 行**（`turn_id` 为空）回退规则：同一会话中，连续的 tool 行归入其**后一条** assistant 消息（真实持久化顺序 `user → tool* → assistant`）；
  其后没有 assistant 时显示为独立「未归属工具记录」组并标记 degraded；**禁止跨 turn 归并**。
- **AND** 无法解析的历史行走降级：显示可折叠原文并标记「无法解析的工具记录」，不白屏、不丢整条会话。
- **AND** 持久化顺序：`BridgeSink` 对终态 `ToolCallFinished`（`success|failed|denied|cancelled`）**先写会话 DB，再判断 cancelled 短路**；
  cancelled 短路只抑制 **bus 上的非终态事件**（`TurnCancelled` 仍发），因此取消/失败/拒绝的终态工具行不会丢（P1-R3）。
- **AND** **工具执行中取消的终态语义（P1-13.2 已冻结）**：取消置位后，无论发生在 gate 前还是工具 outcome 返回后，agent-rig 都发终态 `tool_call_finished{cancelled}`；
  `result` 原样保留工具已产生的结果文本（**不伪造失败**）；UI 在 `status=cancelled` 且 result 非空时显示「已取消（副作用可能已发生）」提示；取消发生在执行前（无已产生结果）时 result 为空、UI 只显示「已取消」。
- **AND** 工具执行中取消后刷新，工具卡仍为 `cancelled`、包含副作用提示（result 非空时）且不白屏（真实 store e2e）。
- **AND** 刷新后的工具卡内容（名称/脱敏参数/结果/状态/所属 turn）与刷新前一致（golden 断言，含真实 SQLite store 写入顺序与中文/emoji 分片）。

### R5 · 实时用量

- **WHEN** 收到 `AgentEvent::Usage`
- **THEN** UI 将用量写入当前 turn 的助手消息并实时展示（token 计数），终态 `turn_completed.reply.usage` 到达时以完整值覆盖；
  不得写入其他 turn/其他消息。
- **AND** usage 事件在 `turn_cancelled`/`error` 后到达也不污染下一条消息。

### R6 · i18n 门禁

- **WHEN** 运行 `bash scripts/i18n-check.sh`
- **THEN** exit 0：客户端 UI 源码无硬编码中文；启动错误边界文案来自 i18n 字典（bootstrap 期以默认 locale 翻译，`en-US` 有对应词条）。

### R7 · 敏感参数脱敏（展示与落库共用）

- **THEN** 事件与持久化中的 `args` 必须经过**同一份** redaction 后再发出（UI 展示与 `BridgeSink` 落库消费同一份已脱敏载荷，不存在第二条展示路径）。
- **AND** redaction 规则 = `Tool::sensitive_arg_keys()`（**`Tool` trait 默认方法**，返回 `&'static [&'static str]`，默认 `&[]`；不动 `ToolSpec`；递归键名匹配 → 整值掩码）+ 内置敏感模式（至少覆盖 `sk-…`、`Bearer …`、`KEY=…`/`TOKEN=…` 的值）；
  掩码形式 `***`，保留命令其余可读部分。
- **AND** 任何日志/事件/错误文案不得输出未脱敏参数。
- **AND** 无法保证完全覆盖的自由文本边界（如任意格式自造密钥、base64 内嵌密钥）必须写入 `KNOWN-LIMITATIONS`；本规格不宣称「零泄漏」。

### R8 · 审批决策 wire 兼容与迁移

- **WHEN** UI 调用 `approval.decide`
- **THEN** 载荷决策为 typed 值 `allow_once | allow_session | reject | cancel`（canonical wire），宿主按 typed `ApprovalDecidePayload` 解析并分发；
  非法值返回明确错误（不静默当拒绝）。
- **AND** 宿主必须兼容旧客户端值：`approved` → `allow_once`、`rejected` → `reject`（`serde(alias)` 或映射层，二选一但只一处实现）；
  新客户端不得再发送旧值。
- **AND** 生成 JSON Schema/TS 类型为 4 个 canonical 值（alias 仅存在于宿主反序列化兼容层，不进入前端校验 schema）；前端类型/协议/reducer 同步。
- **AND** `cancel` 必须可达：`approval.decide{decision:"cancel"}` → 宿主 `resolve_scoped(Cancel)` + `runtime.cancel_agent()` → agent-rig 置内部 cancelled 并**统一发** `turn_cancelled`；断线/窗口关闭不是 `cancel`（按 S3c 映射为 `reject`）。

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 审批超时（S3b） | fail-closed 拒绝，UI 显示已超时；迟到决策忽略；`approval_resolved{decision:reject, timed_out:true}` → 工具终态 `denied` | i18n `approvalTimedOut` |
| 审批中窗口关闭/断线（S3c） | 视为拒绝（fail-closed）；`approval_resolved{reject, timed_out:false}`；工具终态 `denied`；**不**置 `cancel` | 现有 approval fail-closed 语义 |
| 用户点「取消」（S3a） | 决策 `cancel`；`approval_resolved{cancel}` + agent-rig 统一发 `turn_cancelled`；工具终态 `cancelled`；刷新后仍为 `cancelled` | i18n `approvalCancelled` |
| 工具**执行中**取消（gate 后 outcome 前/后） | 仍发终态 `tool_call_finished{cancelled}`；result 保留已产生文本（不伪造失败）；result 非空时 UI 提示「已取消（副作用可能已发生）」；刷新后同状态 | i18n `toolCancelledSideEffects` |
| always-allow 命中 | 不阻塞、不弹决策；`approval_requested{auto:true}` → `approval_resolved{allow_session}`；UI 渲染已自动批准 | 无 |
| 策略短路（config R2） | 不发任何 `approval_*` 事件，直接执行 | 无 |
| `role="tool"` 行 JSON 损坏 | 降级原文 + 折叠，不白屏 | i18n `toolRecordUnparseable` |
| 工具未声明 `approval_metadata` | 卡片显示「该工具未声明影响面」，其余字段照常；不阻塞 | i18n `approvalMetadataMissing` |
| diff 目标文件超大/不可读 | 回退为「将覆盖 + 摘要」，不阻塞审批 | 无错误码（UI 标注） |
| 参数含敏感值 | 卡片/持久化均掩码；原值仅内存中用于执行 | 无 |
| usage 事件重复/乱序 | 幂等合并，按 turn 归属；终态覆盖 | 无 |
| 旧客户端发 `approved/rejected` | host alias 映射后可决策 | 无（兼容层） |
| 非法决策值 | `approval.decide` 返回错误，等待中的审批保持 pending | `approval/invalid-decision` |

## 3. 权限与可见性

- 仅本地用户可决策；企业弱管控下服务端不下发审批策略（C6），审批仍在本机。
- 审批事件只进本地事件流；不写审计（企业服务端不为员工行为兜底）。
- 敏感参数按 R7 脱敏；展示、持久化、日志三处共用同一份已脱敏载荷。

## 4. 数据与兼容

- **契约类型归属（P0-13.1）**：`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 `zk-contract-tool`；`zk-contract-agent` 用 `pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk};` 保持旧路径兼容；
  依赖方向只有 agent → tool（已有），**禁止 tool → agent**（无环，C4）。`ToolCallStatus` 留在 `zk-contract-agent`。
- `AgentEvent::ApprovalRequested` 增加 `auto: bool`、`deadline_ms: u64`、`metadata: Option<zk_contract_tool::ApprovalMetadata>`（全部 `#[serde(default)]`）。
- `AgentEvent::ToolCallFinished` 增加 `args: String`、`status: ToolCallStatus`（默认 `success`，旧行/旧事件缺字段按 success 解析）。
- `ApprovalDecision`（定义在 `zk-contract-tool`）增加 `AllowSession`（snake_case），旧值 `allow_once|reject|cancel` 不变；宿主反序列化兼容 `approved`/`rejected` alias；`zk_contract_agent::ApprovalDecision` 路径经 re-export 保持可用。
- `ApprovalGate` 增加带默认实现的 `decide_scoped(...) -> ApprovalDecision`（默认转调旧 `decide -> bool`），不破坏第三方实现。
- `zk_contract_session::Message` 增加 `turn_id: String`（`#[serde(default)]`，默认空串）；SQLite 会话库升 v5 仅加列，旧行 `turn_id=''`；
  迁移按 `has_column` 幂等，旧二进制对新 DB 仍按其固定列 SELECT 工作（不读新列）。
- `role="tool"` JSON 行增加 `args`/`status`（缺省：args 空串、status success）；`turn` 归属放 `Message.turn_id`（不改 JSON 行结构，零迁移）。
- `Tool` trait 增加 `sensitive_arg_keys(&self) -> &'static [&'static str]`（**默认 `&[]`**）与 `approval_metadata(&self, args, ctx)`（默认 `None`）；**`ToolSpec` 不变**（避免 30 个工具包扫改，P1-R2 选项①）。

## 5. 验收断言（可执行）

- [ ] 契约序列化：新字段/新枚举默认值兼容；`ApprovalDecision` 4 值 + `approved/rejected` alias + 非法值单测（Rust）。
- [ ] 审批事件链 golden：ask→allow_once / ask→reject / always-allow auto / 策略短路 4 场景，断言成对或全缺（记录型 `EventSink` + fake tool）。
- [ ] 工具终态：denied/cancelled 也发 `ToolCallFinished{status}` 并落库；cancelled 时 BridgeSink 先落库再抑制 bus；真实 SQLite store 写入顺序 golden（`user→tool*→assistant`，中文/emoji 分片）→ 按 `turn_id` 归并断言。
- [ ] 刷新恢复 e2e：工具卡名称/脱敏参数/结果/状态/归属 turn 与刷新前一致；损坏行走降级；legacy 行回退规则（挂后一条 assistant）。
- [ ] 审批卡 e2e（真实 personal rig）：字段齐全、默认焦点拒绝、Esc 拒绝、超时拒绝、始终允许后同工具不再弹决策且事件成对、**工具执行中取消→刷新后工具卡 `cancelled` 且不白屏**。
- [ ] 脱敏负向：含 `sk-…`/`Bearer …`/`KEY=…` 的 bash 调用 → 审批卡与会话 DB 均掩码；`sensitive_arg_keys` 独立单测。
- [ ] usage 实时断言：流式过程中 token 计数递增且归属当前 turn，终态一致。
- [ ] 生成物/编译面：`cargo test --workspace export_schema && git diff --exit-code` exit 0；`bash scripts/schema-check.sh` exit 0；`./target/debug/zk capability --check` exit 0（无 INDEX 新增）；`cargo check --workspace --all-targets` exit 0（覆盖 agent-loop/tool-session-query 字面量编译面）。
- [ ] **契约依赖/API（P0-13.1/P2-N16）**：`cargo tree -p zk-contract-tool` 输出中**无** `zk-contract-agent`（无环）；`cargo public-api diff` 附证据或记 UNVERIFIED（本机未安装）；`docs/04` §10 的两侧反序列化对拍样本已补。
- [ ] **执行中取消（P1-13.2）**：慢工具执行中取消 → 单测断言发 `ToolCallFinished{cancelled}` 且 result 保留已产生文本；e2e 断言刷新后卡为 `cancelled` 且包含「副作用可能已发生」提示。
- [ ] `bash scripts/i18n-check.sh` exit 0；`bash scripts/agent-e2e.sh` exit 0；`pnpm -r typecheck/lint/test:unit`、相关 `cargo test` 绿。

## 6. 已定决策与未决问题

**无未决问题**（Lead 2026-11-01 复评前决策，记录于 `proposal.md` §7；本 spec 仍为 `draft` 直至复评通过）：

1. **网络目标边界**：shell/自由文本命令无法静态判定 → 卡片显示「网络目标无法静态判定」并置 `network_undetermined=true`（不做流量拦截）。
2. **元数据缺失**：未实现 `approval_metadata` 的工具（含第三方）卡片显示「该工具未声明影响面」+ degraded，不伪造 risk/paths/diff。
3. **策略与 always-allow 先后**：`strict` 策略下用户已选「始终允许本会话」仍优先，不重复询问。
4. **legacy 归并**：空 `turn_id` 工具行归入「后一条同会话 assistant」（依据真实写入顺序 `user → tool* → assistant`），不做跨轮全局搜索；边界记 `KNOWN-LIMITATIONS`。
5. **不新增 `approval_skipped` 事件**：策略短路按 S5 不发事件（config R2）；always-allow 走 S4 成对 `auto:true` 事件。
6. **config 对齐**：config spec R2 改为引用本 spec `design.md` §2.0 事件序列表（S1–S5），引用落实由 config 作者在复评前完成。
7. **P1-R2（Lead 2026-11-01 选项①）**：`sensitive_arg_keys` 为 `Tool` trait 默认方法，`ToolSpec` 不变；`agent-loop`（src+tests）与 `tool-session-query/tests` 纳入写作用域；workspace 级 `cargo check` 进验收。
8. **P1-R3（Lead 2026-11-01）**：BridgeSink 终态先落库再判断 cancelled（bus 仅抑制非终态，TurnCancelled 可发）；approval-cancel 单一路径 = resolve(Cancel)+cancel_agent，agent-rig 统一 `TurnCancelled`；assistant status 由观察到的 TurnCancelled 决定。
9. **P2-N2/N3/N10（Lead 2026-11-01）**：cancel/reject 唯一映射见 design §2.0 S3a–S3c；docs/28 拆分「工具终态枚举」与「组件六态」；行号 `agent/src/lib.rs:165-174` 已修。
10. **P0-13.1（Lead 2026-11-01 选项①）**：`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义移到 `zk-contract-tool`；`zk-contract-agent` `pub use` 保持路径兼容；无环；docs/24 依赖方向同步；acceptance 加 `cargo public-api diff`（未装记 UNVERIFIED）与 `cargo tree` 无环证据。
11. **P1-13.2（Lead 2026-11-01 选项①）**：执行中取消终态冻结——取消置位后不论 gate 前/outcome 后都发 `ToolCallFinished{cancelled}`；result 原样保留已产生文本（不伪造失败）；result 非空时 UI 提示「已取消（副作用可能已发生）」；慢工具单测+e2e。
12. **P2-N16（Lead 2026-11-01）**：acceptance 增 `cargo public-api diff`（工具未装 → UNVERIFIED）+ 契约反序列化对拍样本（`docs/04` §10）。
