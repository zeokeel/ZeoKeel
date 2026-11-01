# 2026-11-01-client-approval-tool-rendering · 提案

- 状态：`approved`
- Owner：`dsh-ref`（2026-11-01 评审修复作者；原作者 `lead`）
- 日期：2026-11-01
- 关联：`docs/reviews/2026-11-01-client-ui-parity-specs.md`（P0-2/P0-3/P1-1/P1-2/P1-4/P1-5/P1-11/P1-12/P1-13/P1-14/P1-16/P2-1/P2-2/P2-9 + T5 决策）；`docs/19-client-ux-flows.md` §4.1/§4.2；`docs/38-agent-runtime.md` §2/§3；`docs/28-ui-module-map-and-skeleton.md` §4.5/§4.6；`specs/changes/2026-11-01-client-config-approval-mcp/spec.md` R2（审批事件链共享表）；`research/client-dsh-parity-2026-11-01.md` §4 P0-1/P0-2/P1-9/P2-5
- 评审：docs/reviews/2026-11-01-client-ui-parity-specs.md
- 变更预算：约 41 个文件 / 900–1300 行（契约 3 + kit 1 + 工具包 6 + agent-rig 2 + 跨包编译面 3 + 会话存储 1 + 宿主 3 + ui-kit 1 + ui-chat 4 + desktop web 5 + e2e 2 + 文档 5 + spec 5）；**超过 10 文件 / 400 行阈值，预算例外见 §8**

## 1. 背景 / 问题

独立评审（`docs/reviews/2026-11-01-client-ui-parity-specs.md`）对上一版 spec 给出 **CHANGES-REQUESTED**（P0=4/P1=16/P2=10；本 spec 相关条目见 §10）。本版按评审与 Lead 决策重写，核心是四类「看起来有、实际是错的/不可实现」的问题：

1. **审批卡安全字段缺失**：现状只有 `tool`/`args`/`reason` + 批准/拒绝（`apps/desktop/web/src/ApprovalPart.tsx` 旧实现；合同事件只有 4 字段，`packages/contracts/agent/src/lib.rs:111-116`）。`ApprovalDecision` 契约已有 `Cancel`（**迁移前**定义在 `packages/contracts/agent/src/lib.rs:165-174`；本 spec 按 P0-13.1 移至 `zk-contract-tool` 并由 agent re-export）但运行期不可达：宿主按字符串 `decision == "approved"` 判定（`apps/desktop/src/ipc.rs:728-740`），前端只发 `approved/rejected`（`apps/desktop/web/src/state/controller.ts:667-679`）。用户无法判断一次操作的真实影响面。
2. **工具行刷新丢失（方向与状态双错）**：`ToolCallFinished` 在工具结束时立即写 `role="tool"` 的 JSON 消息（`apps/desktop/src/events.rs:60-79`），assistant 消息在整轮结束才写（`apps/desktop/src/ipc.rs:1776-1783`、`:1842-1857`），会话按 `created_at_ms ASC, id ASC` 读取（`packages/client/store-session-sqlite/src/lib.rs:651`）⇒ 真实顺序是 `user → tool… → assistant`；上一版「归并到前一条 assistant」会跨轮错挂。并且拒绝/取消路径在发 `ToolCallStarted/Finished` 之前就 `return`（`packages/client/agent-rig/src/lib.rs:345-356`、`:357-359`），工具行根本不落库，`denied/cancelled` 态刷新后不可达。会话 `Message` 也没有 turn 归属键（`packages/contracts/session/src/lib.rs:52-72`）。
3. **`usage` 事件被丢弃**：契约已有 `AgentEvent::Usage`（`packages/contracts/agent/src/lib.rs:140` 区域），`parseAgentEvent` 落 default→null（`apps/desktop/web/src/protocol.ts:669-670`）。
4. **审批元数据、脱敏、资产登记与门禁都缺位**：上一版在 agent-rig 中心化猜测 `path/paths/cwd` 并直接读文件生成 diff（重复工具层知识、绕过工具抽象，C17）；持久化 `args` 会新增敏感数据面却无脱敏设计；新增 `contract.approval` capability 与 `zk capability` 扫描范围不符；改契约却未把生成物门禁写进验收。

这些不是「缺功能」，而是**安全/正确性必须先修**：不修则 Phase 2 的能力面建在错误的事件、turn 归属与安全语义上。

## 2. 目标

- 审批卡按 `docs/19` §4.1（本版覆盖子集，见 §3/§5）展示：动作摘要、工具名 + 完整命令/参数（**已脱敏**）、工作目录（是否越出 workspace）、
  读写路径列表 + 网络目标、写操作 diff 预览（可展开）、风险级别（low/medium/high）、剩余时间倒计时、决策（允许一次 / 始终允许本会话 / 拒绝；
  默认焦点在拒绝；Esc=拒绝）。
- **审批链单 owner = agent-rig**：策略求值与 always-allow 只在 agent-rig 求值；宿主 broker 只做 IPC 阻塞与超时，不持有会话允许集合；
  always-allow 命中时**成对**发 `approval_requested{auto:true}` + `approval_resolved{allow_session}`（不弹决策、不阻塞），并写进与 config spec R2 共享的事件序列表（design §2.0）。
- **turn 归属冻结**：`Message` 增加 `turn_id`（`#[serde(default)]`）作为工具行归属键；工具行按 `turn_id` 归并到**同一 turn**；
  拒绝/取消路径也发终态 `ToolCallFinished{status: denied|cancelled}` 并落库；补真实 store 写入顺序 golden + 刷新一致性 e2e。
- **审批元数据由工具侧声明**：`Tool::approval_metadata(&args,&ctx)`（默认 `None`）由 `fs.write/fs.edit`、`bash/pwsh`、`mcp_call`、`create_worktree` 等关键工具实现，
  agent-rig 只聚合与发事件；不读文件、不猜路径；diff 由工具侧给（上限 256KB）。
- **契约类型归属（P0-13.1）**：`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 **`zk-contract-tool`**（底层）；`zk-contract-agent` 用 `pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk};` 保持 `zk_contract_agent::*` 路径兼容；
  `AgentEvent::ApprovalRequested.metadata` 使用 tool 类型。依赖方向只有 **agent → tool**（已有），**禁止 tool → agent**（无环，C4 单向缝）。
- **脱敏单一实现**：`Tool::sensitive_arg_keys()`（`Tool` trait 默认方法，返回 `&[]`；不动 `ToolSpec`）+ 一份 redaction（展示与落库共用），补「含 token 的 bash 调用 → 卡片/DB 掩码」负向测试，
  不可脱敏边界写 `KNOWN-LIMITATIONS`。
- **决策 wire 迁移冻结**：typed `ApprovalDecision`（`allow_once/allow_session/reject/cancel`），保留 `approved/rejected` 旧值 alias 兼容；
  前端/生成 schema 同步；4 值 + 旧值 + 非法值单测。`Cancel` 因此可达。
- **复用而非新增 capability**：复用 `contract.agent`/`contract.tool`（INDEX 已存在，`shared/contracts/capability-index.json:11,71`），删除 `@capability contract.approval`，不新增 INDEX 条目；`zk capability --check` 进验收。
- **生成物门禁**：`cargo test --workspace export_schema && git diff --exit-code` + `bash scripts/schema-check.sh` 进 tasks/acceptance（C10/C11）。
- **e2e 用真实 personal rig**：`scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`（不用 `pnpm e2e` 的 mock profile）。
- 不改变工具执行/审批的 fail-closed 语义；超时/断线仍默认拒绝。

## 3. 非目标

- **`docs/19` §4.1「记住范围」选择器**：本 spec 只实现「始终允许（本会话）」（维度 = 会话 + 工具名，决策 A）；**「仅该路径」scope 与显式记住范围选择器推迟 Phase 2/3**。依据：评审 T5 选项②与 Lead 决策。
- **`docs/19` §4.1「超时……可『延长一次』」**：本 spec 只做「剩余时间 + 超时默认拒绝」；延长一次推迟 Phase 2/3。
- **`docs/19` §4.2「危险操作（删除、写入工作区外、shell 网络）二次确认」**：推迟 Phase 2/3。
- **`docs/19` §4.2「审批挂起时失焦 → 系统通知 + 托盘红点；超时前 60s 再通知」**：Phase 3 交付项。
- 审批跨重启持久化、全局永久允许（`docs/19` §4.1 明确不提供）。
- **网络目标的静态判定边界**：只对参数内可静态确定的工具（URL/主机、MCP server id）展示域名/端口；shell 等自由文本命令无法静态判定时显示
  「网络目标无法静态判定」并置 `network_undetermined=true`（诚实边界，见 `KNOWN-LIMITATIONS`），不做流量级拦截。
- diff 算法覆盖二进制/超大文件（>256KB 回退为摘要）。
- 企业「弱管控」审批策略（C6：不做企业审批）；只做客户端本地审批。
- 工具卡按 DSH 的 54 种 keyed view 拆分（Phase 2）；历史 `role="tool"` 行升级为结构化 `tool_calls`（Phase 2 评估）。

> 以上非目标与 §5 成功指标共同取代上一版「§4.1 100%」的不实声明（评审 P1-12）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 模型调用 `fs.write` 触发审批 | 看到将写入的路径、diff、风险、剩余时间，默认焦点在「拒绝」；参数不含明文密钥 |
| 个人用户 | 同一会话反复批准同一工具 | 选择「始终允许（本会话）」后同工具不再弹窗，但事件链留下成对 `approval_requested{auto:true} → approval_resolved{allow_session}` 审计痕迹；重开会话恢复询问 |
| 个人用户 | 拒绝/取消一次工具调用 | 工具卡仍以「已拒绝/已取消」终态出现；刷新后仍是同一状态，不变成 JSON 气泡 |
| 个人用户 | 刷新客户端/重进会话 | 工具卡完整恢复（名称/脱敏参数/结果/状态），且归属正确的 turn（不跨轮错挂） |
| 审计/安全 | 检查审批超时 | 超时自动拒绝且 UI 显示「已超时拒绝」，不静默放行；迟到决策无效 |
| 安全评审 | shell 命令含 token | 审批卡与会话 DB 都显示掩码；`KNOWN-LIMITATIONS` 诚实列出无法保证的边界 |

## 5. 成功指标

1. **审批卡覆盖率（取代「§4.1 100%」）**：`docs/19` §4.1 共 9 行字段 = **7/9 完整覆盖**（动作摘要、工具/命令、工作目录、读写范围含网络目标、diff 预览、风险级别、决策 3 选项 + 默认焦点拒绝）
   **+ 1/9 部分覆盖**（超时：剩余时间 + 超时默认拒绝已覆盖；「可延长一次」列非目标）+ **1/9 非目标**（记住范围）；`§4.2` 的二次确认/通知列为非目标（见 §3）。
2. **审批事件链 golden**：4 类场景（ask→allow_once / ask→reject / always-allow 命中 / 策略短路）事件序列与 design §2.0 表一致；与 `config spec` R2 引用同一张表（P0-2）。
3. **turn 归属与终态**：真实 SQLite store 写入顺序 golden（`user → tool* → assistant`，含中文/emoji 分片）按 `turn_id` 归并；拒绝/取消工具的终态行落库并在刷新后显示
   `success|failed|denied|cancelled`；损坏行降级不白屏（P0-3）。
4. **决策 wire**：4 个 canonical 值 + `approved/rejected` 旧值 alias + 非法值拒绝，单测覆盖；前端/协议/生成 schema 一致（P1-11）。
5. **脱敏**：含 token 的 `bash` 调用 → 审批卡掩码（e2e）+ 会话 DB 掩码（`zk-desktop` 单测，`BridgeSink` 消费同一份已脱敏载荷）；`Tool::sensitive_arg_keys()` 机制有独立单测；边界写入 `KNOWN-LIMITATIONS`（P1-13）。
6. **能力与生成物/契约 API**：`./target/debug/zk capability --check` exit 0 且 INDEX 无新增条目（复用 `contract.agent`/`contract.tool`）；
   `cargo test --workspace export_schema && git diff --exit-code` exit 0；`bash scripts/schema-check.sh` exit 0（P1-4/P1-5）；
   `cargo public-api diff`（`docs/04` §10 要求；**本机未安装 → 记 UNVERIFIED，不冒充已跑**）与「`cargo tree -p zk-contract-tool` 输出无 `zk-contract-agent`」无环证据（P0-13.1/P2-N16）。
7. **e2e**：`bash scripts/agent-e2e.sh`（真实 personal 组合 + 假上游）绿，用例覆盖：卡字段/默认焦点拒绝/Esc/超时拒绝/显式取消/**工具执行中取消→刷新 cancelled**/始终允许（auto 事件对）/刷新一致/
   损坏行/usage 实时/脱敏/turn 顺序（P1-14/P1-R3）。
8. **门禁**：`bash scripts/i18n-check.sh` exit 0；`pnpm -r typecheck/lint/test:unit`、相关 `cargo test` 绿；**`cargo check --workspace --all-targets` exit 0**（P1-R2 编译面）；`python3 scripts/spec-check.py`、`docs-check.py`、`review-check.py` 绿（复评时）。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| `Message.turn_id` 需要 SQLite v5 迁移；旧/新版本双向读 | 高 | 仅加列 + `serde(default)` + 迁移测试；旧行读为空 → legacy 归并路径；旧二进制读 v5 DB 时列不参与其固定 SELECT（迁移按 `has_column` 幂等） |
| 工具行归并/legacy 回退方向错 | 高 | 真实 store 顺序 golden（中文/emoji）+ 刷新 e2e；legacy 回退规则只挂「下一条同会话 assistant」，不跨轮 |
| `Tool::approval_metadata` 默认 None → 部分工具卡无影响面 | 中 | 关键工具（fs.write/fs.edit/bash/pwsh/mcp_call/create_worktree）必须实现（任务验收）；其余显示「该工具未声明影响面」并标 degraded，不伪造 |
| 脱敏无法覆盖自由文本中的所有密钥格式 | 中 | 单一 redaction（key 名单 + 常见 token 正则）；边界写入 `KNOWN-LIMITATIONS`；负向测试至少覆盖 `sk-…`/`Bearer …`/`KEY=…` |
| `ApprovalGate` 增加 typed 决策方法 | 中 | 新方法带默认实现（转调旧 bool），第三方实现不破；broker 覆盖 |
| 事件字段扩展破坏 mock/e2e 传输 | 中 | 全部 `#[serde(default)]`；mock transport 补字段；e2e 全量 |
| 策略求值与 always-allow 的先后顺序 | 中 | 单 owner（agent-rig）+ design §2.0 表冻结；strict 下 always-allow 仍优先（用户显式选择）——Lead 2026-11-01 已定（§7 c） |
| 生成物漂移 | 中 | export_schema + schema-check 进任务与验收 |

## 7. 需要用户拍板 / 已拍板记录（2026-11-01 评审修复）

已定（Lead 决策，本版按此重写）：

- [x] **T5 → 选项②**：审批卡覆盖 `docs/19` §4.1 子集（7/9 完整 + 1/9 部分）；「记住范围选择器（含『仅该路径』scope）」「超时延长一次」「§4.2 危险操作二次确认」推迟 Phase 2/3，写入 §3 与 acceptance 剩余风险。
- [x] **P0-2**：审批链单 owner = agent-rig（策略 + always-allow 只在这里求值）；broker 只 IPC 阻塞、不持有会话允许集合；always-allow 命中成对发
      `approval_requested{auto:true}` + `approval_resolved{allow_session}`（design §2.0 事件序列表，config spec R2 引用同一张表）。
- [x] **P0-3**：`Message.turn_id`（`#[serde(default)]`）为归属键；工具行按 turn 归并；拒绝/取消也发终态 `ToolCallFinished{status}` 并落库。
- [x] **P1-11**：typed `ApprovalDecision` 4 值 + `approved/rejected` 旧值 alias；前端/生成 schema 同步。
- [x] **P1-13**：`Tool::sensitive_arg_keys()`（trait 默认方法，不动 `ToolSpec`）+ 单一 redaction（展示/落库共用）+ 负向测试 + `KNOWN-LIMITATIONS` 边界。
- [x] **P1-16**：审批元数据改由 `Tool` trait 侧 `approval_metadata` 声明；agent-rig 只聚合；不读文件/不猜路径；diff 由工具侧给（≤256KB）。
- [x] **P1-4**：删除 `contract.approval`，复用 `contract.agent`/`contract.tool`，不新增 INDEX 条目；`zk capability --check` 进验收。
- [x] **P1-1/P1-2**：tasks 增加「跨 spec 顺序（settings-ia → approval → config → mcp-config）」与「决策落文档」前置任务（`docs/19` §4.1/§4.2、`docs/38` §2/§3）。
- [x] **原 A/B/C 保留**：A「始终允许」按 `(conversation_id, tool_name)`、进程内、不落盘、新会话清空；B 剩余时间：agent-rig 计算 `deadline_ms`、broker 负责等待/超时、
      UI 只倒计时（broker 不持有允许集合）；C 历史 `role="tool"` JSON 行格式保留，字段补 `args`/`status`，turn 归属放 `Message.turn_id`（不入 JSON）。
- [x] **Lead 决策（2026-11-01，复评前；原「作者假设」全部接受）**：
  - (a) shell/自由文本命令的网络目标无法静态判定 → `network_undetermined=true` + 卡片显示「网络目标无法静态判定」（不做流量拦截）；
  - (b) 无 `approval_metadata` 的工具（含第三方）卡片显示「该工具未声明影响面」+ degraded，不伪造 risk/paths/diff；
  - (c) `strict` 策略下用户已选的「始终允许本会话」仍优先（不重复询问）；
  - (d) legacy 空 `turn_id` 工具行回退「后一条同会话 assistant」，边界记入 `KNOWN-LIMITATIONS`；
  - (e) **不新增 `approval_skipped` 事件**：策略短路按 config R2「不发任何审批事件」（S5）；always-allow 走 S4 成对 `auto:true` 事件；
  - (f) 与 config 作者已对齐：config spec R2 改为引用本 spec `design.md` §2.0《审批事件序列表》S1–S5（引用落实由 config 作者在复评前完成；本 spec 不改 config）。
- [x] **复评第三轮 Lead 决策（2026-11-01，task-11）**：
  - **P1-R2 → 选项①**：`sensitive_arg_keys` 改为 `Tool` trait 默认方法（`fn sensitive_arg_keys(&self) -> &'static [&'static str] { &[] }`），不动 `ToolSpec`（避免 30 个工具文件扫改）；
    `Message.turn_id`/`ToolCallFinished` 的编译面仍把 `packages/client/agent-loop`（src+tests）与 `packages/client/tool-session-query/tests` 加入 tasks 写作用域与预算；
    acceptance 增加 workspace 级 `cargo check --workspace --all-targets` 门禁（P1-5 补充）。
  - **P1-R3**：`BridgeSink` 对终态 `ToolCallFinished`（含 `cancelled/failed/denied`）**先落库再判断 cancelled 短路**（bus 仅抑制非终态事件；`TurnCancelled` 仍可发）；
    approval-cancel 单一路径 = 宿主 `broker.resolve_scoped(Cancel)` + `runtime.cancel_agent()`，由 **agent-rig 统一发 `TurnCancelled`**（宿主不再手发第二次），assistant 行 status 由 BridgeSink 观察到的 `TurnCancelled` 决定；
    补「工具执行中取消 → 刷新后工具卡 `cancelled`」真实 store e2e。
  - **P2-N2**：cancel/reject 唯一映射（design §2.0 S3a–S3c）：超时 → `reject + timed_out:true`；用户显式取消 → `cancel`；断线/窗口关闭 → `reject`；
    不再保留「fallback cancel」等第二份表述。
  - **P2-N3**：`docs/28` 分开写「工具终态枚举（success|failed|denied|cancelled；pending|running 仅内存）」与「组件六态（空/加载/错误/离线/流式中/已取消，AGENTS §7）」，二者不是同一概念。
  - **P2-N10**：行号 `packages/contracts/agent/src/lib.rs:165-174` 已修正（P2-1 同类残余）。
- [x] **复评第四轮 Lead 决策（2026-11-01，task-14）**：
  - **P0-13.1 → 选项①**：把 `ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义**移到 `zk-contract-tool`**；`zk-contract-agent` 用 `pub use` 保持 `zk_contract_agent::*` 路径兼容（agent 已依赖 tool，**无环**）；
    `ToolCallStatus` 留在 agent；`AgentEvent::ApprovalRequested.metadata` 用 tool 类型；tasks 写作用域加 `packages/contracts/tool` 与 agent re-export 改动；`docs/24` 契约依赖方向同步；
    acceptance 增 `cargo public-api diff`（未装 → UNVERIFIED）与 `cargo tree -p zk-contract-tool` 无反向依赖证据。
  - **P1-13.2 → 选项①（推荐）**：冻结「执行中取消」终态——取消置位后，无论发生在 gate 前还是工具 outcome 返回后，agent-rig 都发终态 `ToolCallFinished{status:cancelled}`；
    `result` 原样保留工具已产生的结果文本（不伪造失败），UI 在 `cancelled` 时显示「已取消（副作用可能已发生）」；若取消发生在执行前（无已产生结果）result 为空、UI 只显示「已取消」；
    补慢工具 e2e：刷新后卡片 `cancelled` 且包含该提示。
  - **P2-N16**：acceptance 增加 `cargo public-api diff`（工具未安装 → 记 UNVERIFIED，不冒充）+ 契约反序列化对拍样本要求（`docs/04` §10）。
- 状态保持 `draft` 直至独立复评通过（C19）。

## 8. 变更预算与拆分

- 预计改动文件：契约 3（agent/tool/session）+ kit 1（`zk-kit-tools` 单一 redaction）+ 工具包 6（tool-fs-write/tool-fs-edit/tool-bash/tool-pwsh/tool-worktree/connector-mcp）+
  agent-rig 2 + `store-session-sqlite` 1 + 宿主 3（approval.rs/events.rs/ipc.rs）+ ui-kit 1 + ui-chat 4 + desktop web 5（protocol/reducer/controller/i18n/main）+
  e2e 2（`e2e/agent-rig.spec.ts`、`scripts/agent-e2e.sh`）+ 文档 6（docs/19、docs/38、docs/28、docs/24、KNOWN-LIMITATIONS、parity research）+ 本 spec 5 ≈ **39 文件 / 900–1200 行**；
  另加 **跨包编译面**（P1-R2）：`packages/client/agent-loop/src/lib.rs` + `agent-loop/tests/*` + `packages/client/tool-session-query/tests/*` = **+3 文件 ≈ 42 文件**（`ToolSpec` 仍不动，故不触发 30 个工具包扫改）；P0-13.1 的契约类型移动只改 `contracts/tool` + `contracts/agent` re-export（已在契约 3 文件内），不新增文件。
- **拆分判断：不拆**，但按模板与评审 §4 记录**预算例外**：本变更只有一个不可分割的目标——「审批事件链 + 工具行重放的正确性与安全性」；
  拆开会导致「事件字段先合、turn 归属后合」的中间态让刷新正确性继续错（P0-3 依赖 P0-2 的事件字段）。若 Lead 要求落到 10 文件阈值，可按 tasks 的
  Phase 1a（事件链/卡/脱敏）→ Phase 1b（turn_id/工具行重放/存储 v5）→ Phase 1c（wire 迁移/e2e/文档）切成三个可独立合并的 spec；
  本版不主动拆；**Lead 批准例外（2026-11-01；第三轮 ~41 → 第四轮 ~42 文件，含 docs/24 依赖方向）**：同意按 ~42 文件 / 900–1300 行预算执行（理由：P0 安全/正确性同一目标 + 用户已批 Phase 1 结构 + P1-R2 选①后仅 3 个编译面文件 + P0-13.1 仅类型归属调整），与评审 §4「目标单一可保留」建议一致；若后续范围扩大再按 Phase 1a/1b/1c 拆分。
- i18n 门禁修复（R6）是否属同一目标：**属必须的合并前置**（门禁红会阻断任一 PR），保留在本 spec 但单独成任务（tasks #11），成本约 1 文件 / 20 行。

## 9. 复用与已有实现（C17）

- 已搜索：`rg "ApprovalPart|ApprovalDecision|ToolCallFinished|parseAgentEvent|approval_metadata" packages apps`；`./target/debug/zk find approval` → `kit.approval`（1 命中）；
  `shared/contracts/capability-index.json` 已有 `contract.agent`（:11）、`contract.tool`（:71）、`kit.approval`（:91）。
- 可复用：`@zeokeel/ui-kit`（Button/Chip/icons）、`ui-chat` 现有 `ApprovalPart`/`ToolCallPart`、`zk-kit-approval` 全局 gate 槽、
  `BridgeSink` 会话写入器、`zk-kit-tools`（`safe_path`/`truncate`/`read_text`，redaction 放这里）、现有 `scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts`。
- **本次不新增 capability id**：复用 `contract.agent`/`contract.tool`（既有 INDEX 条目），不新增 `docs/generated/CAPABILITY-INDEX.md` 条目；
  `zk capability --check` 作为验收门禁（评审 P1-4）。
- 为什么不能复用/扩展：旧 `ApprovalGate::decide -> bool` 无法表达 `allow_session`/`cancel`（加带默认实现的 `decide_scoped`）；
  UI 没有工具行解析层（新增 `ui-chat/toolRows.ts`）；agent-rig 无工具侧元数据通道（新增 `Tool::approval_metadata` 默认方法）。

## 10. 评审条目处置（复评对照）

| 评审条目 | 处置 | 本版落点 |
| --- | --- | --- |
| P0-2 审批链双实现/互斥约束 | 修：单 owner + 事件序列表 | proposal §2/§7、spec R3、design §2.0、tasks #5/#7 |
| P0-3 工具行方向/状态不可达 | 修：`turn_id` + 终态事件 + store v5 + golden/e2e | proposal §2、spec R4、design §2.4、tasks #1/#5/#6/#7/#12 |
| P1-4 capability 登记错误 | 修：删除 `contract.approval`，复用既有 id | proposal §9、design §1.5、tasks #14、acceptance §1 |
| P1-5 缺生成物门禁 | 修：export_schema + schema-check 进 tasks/acceptance | tasks #1/#2/#14、acceptance §1 |
| P1-11 wire 漂移 | 修：typed 4 值 + 旧值 alias + 单测 | spec R8、design §2.3、tasks #1/#7/#9/#10 |
| P1-12 §4.1 100% 不实 | 修：7/9 完整 + 1/9 部分 + 非目标 | proposal §3/§5、spec R2、acceptance §5 |
| P1-13 无脱敏 | 修：`sensitive_arg_keys` + 单一 redaction + 负向测试 + 边界 | spec R7、design §4、tasks #3/#5/#7/#12/#13 |
| P1-14 e2e 跑错 profile | 修：`scripts/agent-e2e.sh` + `e2e/agent-rig.spec.ts` | tasks #12、acceptance §1 |
| P1-16 元数据由 agent-rig 猜/读文件 | 修：`Tool::approval_metadata` 工具侧声明 | spec R1、design §2.2、tasks #4/#5 |
| P1-1/P1-2 跨 spec 顺序/文档先行 | 修：tasks §5 跨 spec 顺序 + #0 决策落文档前置 | tasks §0/§5 |
| P2-1 行号漂移 | 修：全文引用改为实测行号 | 本版全文 |
| P2-2 两个 `## 6` | 修：复用证据改 §7 | acceptance |
| P2-3 config design 自相矛盾 | **不属本 spec**（config 作者范围；本次写作用域仅本目录） | 移交 config 作者 |
| P2-9 证据路径未定义 | 修：预填 `target/evidence/2026-11-01-client-approval-tool-rendering/` | acceptance §1/§2 |
| **P1-R2** 跨包字段编译面 | 修：`sensitive_arg_keys` 改 `Tool` trait 默认方法（不动 `ToolSpec`）；agent-loop/tool-session-query 进写作用域；workspace 级 check 门禁 | proposal §7、spec R7/§4、design §2.2/§2.6、tasks #1b/#14、acceptance §1 |
| **P1-R3** 取消路径被 BridgeSink 抑制 | 修：终态先落库再判 cancelled；agent-rig 统一 `TurnCancelled`；assistant status 由 BridgeSink 决定；补 store e2e | design §2.5.2–§2.5.4、spec R4、tasks #5/#7/#12、acceptance §2/§5 |
| **P2-N2** cancel/reject 措辞不一 | 修：唯一映射 S3a–S3c（超时 reject+timed_out；显式 cancel；断线 reject） | design §2.0、spec R2/R8/§2 |
| **P2-N3** docs/28 概念混用 | 修：拆分「工具终态枚举」与「组件六态」 | design §7、tasks #13、acceptance §3 |
| **P2-N10** 行号残余漂移 | 修：`agent/src/lib.rs:165-174` | proposal §1 |
| **P0-13.1** 跨契约依赖环 | 修：审批类型定义移到 `zk-contract-tool`，agent `pub use` 保持路径兼容；无环；docs/24 依赖方向；public-api 证据 | proposal §7、spec R1/§4、design §1/§2.2、tasks #0/#1/#14、acceptance §1/§8 |
| **P1-13.2** 执行中取消终态 | 修：取消置位后 gate 前/outcome 后都发 `ToolCallFinished{cancelled}`；result 保留已产生文本 + UI 副作用提示；慢工具 e2e | design §2.5.3/§2.5.5、spec R4/§2、tasks #5/#9/#11/#12、acceptance §2 |
| **P2-N16** public-api 证据缺位 | 修：acceptance 增 `cargo public-api diff`（未装 → UNVERIFIED）+ 反序列化对拍样本 | acceptance §1/§5、tasks #14 |

> 状态保持 `draft` 直至独立评审通过（C19）；复评通过前不得改产品代码（C16）。
