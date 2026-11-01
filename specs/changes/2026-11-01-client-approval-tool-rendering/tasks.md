# 2026-11-01-client-approval-tool-rendering · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 评审修复版（作者 `dsh-ref`）：按 `docs/reviews/2026-11-01-client-ui-parity-specs.md` 的 P0-2/P0-3/P1-1/P1-2/P1-4/P1-5/P1-11/P1-12/P1-13/P1-14/P1-16/P2-1/P2-2/P2-9 重排。

## 0. 前置任务（C12：决策先落文档，先于产品代码）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | **决策落文档**：① `docs/19` §4.1/§4.2 记录本 spec 覆盖 7/9+1/9 与三项非目标（记住范围选择器/超时延长一次/§4.2 二次确认）、通知列 Phase 3；② `docs/38` §2/§3 记录 `ApprovalDecision` 4 值+旧值 alias、`ToolCallStatus`、S1–S5 事件序列表并指向本 spec design §2.0；③ **`docs/24` 契约依赖方向（P0-13.1）**：`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision` 定义在 `zk-contract-tool`、`zk-contract-agent` re-export；依赖仅 agent→tool，禁止 tool→agent | `docs/19-client-ux-flows.md`、`docs/38-agent-runtime.md`、`docs/24-composition-and-runtime-spec.md` | 复评通过 | `python3 scripts/docs-check.py` | ☑ |
| 0b | **跨 spec 对齐（已对齐，Lead 2026-11-01；引用落实由 config 作者在复评前完成，本 spec 不写 config）**：`2026-11-01-client-config-approval-mcp` R2 改为引用本 spec design §2.0 事件序列表 S1–S5；config 合并时把策略求值接入 agent-rig 的 `should_ask` 单点（替换默认实现） | `specs/changes/2026-11-01-client-config-approval-mcp/**`（**非本 spec 写作用域**，登记） | 0 | `python3 scripts/spec-check.py`；复评记录注明 | ☐（非本 spec 执行；config 作者在 config 合并前落实 R2 引用） |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约（**P0-13.1 类型归属**）：`ApprovalMetadata`/`ApprovalRisk`/`ApprovalDecision`（+`AllowSession`/旧值 alias）**定义在 `zk-contract-tool`**；`zk-contract-agent` 用 `pub use zk_contract_tool::{ApprovalDecision, ApprovalMetadata, ApprovalRisk};` 保持旧路径，`ApprovalRequested{auto,deadline_ms,metadata}`/`ToolCallFinished{args,status,ToolCallStatus}` 事件字段用 tool 类型；`Tool::sensitive_arg_keys()`（trait 默认方法，不改 `ToolSpec`）、`Tool::approval_metadata`（默认 None）、`ApprovalGate::decide_scoped`（默认实现）；`Message.turn_id`；serde 兼容测试 | `packages/contracts/agent/src/lib.rs`、`packages/contracts/agent/tests/*`、`packages/contracts/tool/src/lib.rs`、`packages/contracts/tool/tests/*`、`packages/contracts/session/src/lib.rs`、`packages/contracts/session/tests/*` | — | `cargo test -p zk-contract-agent -p zk-contract-tool -p zk-contract-session`；`cargo tree -p zk-contract-tool` 无 `zk-contract-agent`（无环） | ☑ |
| 1b | **跨包编译面同步（P1-R2）**：`ToolCallFinished{args,status}` 的两处字面量（agent-loop）+ `Message.turn_id` 字面量（tool-session-query tests）；`ToolSpec` 不改 | `packages/client/agent-loop/src/lib.rs`、`packages/client/agent-loop/tests/*`、`packages/client/tool-session-query/tests/*` | 1 | `cargo check -p zk-agent-loop -p zk-tool-session-query`；`cargo check --workspace --all-targets` | ☑ |
| 2 | 生成物同步：重生成 JSON Schema/TS（`export_schema`）并校验 SCHEMAS/Ajv；确认 `ApprovalDecision` 生成枚举为 4 canonical 值（alias 不进 schema） | `shared/contracts/schema/*`、`shared/contracts/ts/*`（**生成器写，不手改**） | 1 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ☑ |
| 3 | 单一 redaction：`zk-kit-tools::redact_tool_args(sensitive_keys: &[&str], args: &Value)`（键名递归掩码 + `sk-`/`Bearer`/`KEY=`/`TOKEN=`/`password` 模式，保留结构与可读文本）；单测（含合成 `Tool` 实现的 `sensitive_arg_keys()` 覆写） | `crates/zk-kit-tools/src/lib.rs`（或新 `redaction.rs`）、`crates/zk-kit-tools/tests/*` | 1 | `cargo test -p zk-kit-tools` | ☑ |
| 4 | 工具侧审批元数据（`approval_metadata`）+ 需要时覆写 `Tool::sensitive_arg_keys()`（shipped 工具默认 `&[]`）：`fs.write`/`fs.edit`（write_paths + diff≤256KB）、`bash`/`pwsh`（summary/cwd/`network_undetermined`/risk=high）、`mcp_call`（`<mcp:server>`/risk=high）、`create_worktree`（路径/risk=high）；各包单测（workspace 外路径、无 path 工具、diff 超限回退） | `packages/client/tool-fs-write/**`、`tool-fs-edit/**`、`tool-bash/**`、`tool-pwsh/**`、`tool-worktree/**`、`connector-mcp/**` | 1,3 | `cargo test -p zk-tool-fs-write -p zk-tool-fs-edit -p zk-tool-bash -p zk-tool-pwsh -p zk-tool-worktree -p zk-connector-mcp` | ☑ |
| 5 | agent-rig：① 策略单点 `should_ask`（默认 = `requires_approval`，为 config 预留替换点）；② always-allow 单 owner（进程内 `(conversation,tool)`，不落盘）；③ 聚合 `tool.approval_metadata` + 调 redaction + 发 `approval_requested{auto,deadline_ms,metadata}`；④ always-allow 命中发 S4 成对事件（auto:true，不阻塞）；⑤ denied/cancelled 也发 `tool_call_finished{args,status}`；⑥ gate 返回 `Cancel` 时置内部 cancelled（由既有分支统一发一次 `turn_cancelled`）；⑦ **执行中取消（P1-13.2）**：工具 outcome 返回后检查 `cancelled_for_tool`，已置位则发 `tool_call_finished{status:cancelled, result:<已产生文本原样>}`（未执行则 result 空），返回给模型的文本保持原结果；⑧ 事件链 golden（S1–S5 含 S3a–S3c，记录型 EventSink） | `packages/client/agent-rig/src/lib.rs`、`packages/client/agent-rig/src/hooks.rs`、`packages/client/agent-rig/tests/*` | 1,3,4 | `cargo test -p zk-agent-rig` | ☑ |
| 6 | 会话存储：`Message.turn_id` 落库（SQLite v5 仅加列、幂等迁移、旧行空串、`messages_in` 读回）；真实 store 写入顺序 golden（`user→tool*→assistant`，中文/emoji 分片） | `packages/client/store-session-sqlite/src/lib.rs`、`packages/client/store-session-sqlite/tests/*` | 1 | `cargo test -p zk-store-session-sqlite` | ☑ |
| 7 | 宿主：① `BridgeSink` 记录 `TurnStarted.turn_id`，tool 行 content 写 `{name,args,status,result}` 且 `Message.turn_id` 正确；② **终态 `ToolCallFinished` 先落库再判断 cancelled 短路**（bus 仅抑制非终态；`TurnCancelled` 仍发），记 `turn_cancelled` 标志；③ `ipc.rs` 用 `Arc<BridgeSink>` 的 turn_id 写 assistant/错误消息，assistant status 由 `bridge.turn_cancelled()` 决定；④ `ApprovalBroker` 改 typed 决策（`decide_scoped`/`resolve`）+ 超时/断开 fallback（reject）、**不持有**会话允许集合；⑤ `approval.decide` typed 解析（4 值 + legacy alias + 非法值错误）；`approval.decide{cancel}` = resolve(Cancel)+`runtime.cancel_agent()` 单一路径；`chat.cancel` = broker 释放全部 pending waiter(Reject)+置 cancelled，删除 `chat.cancel` 手发 `turn_cancelled` | `apps/desktop/src/approval.rs`、`apps/desktop/src/events.rs`、`apps/desktop/src/ipc.rs`、`apps/desktop/tests/*` | 1,5,6 | `cargo test -p zk-desktop` | ☑ |
| 8 | ui-kit 类型镜像（C10/ADR-059 M1 过渡）：`Message.turn_id`、`ApprovalMetadata`、`ToolCallFinished.args/status`、`ApprovalDecision` 4 值、`ApprovalView` 状态字段；与生成 schema 对齐 | `packages/ui/ui-kit/web/src/types.ts` | 1 | `pnpm --dir packages/ui/ui-kit test:unit` | ☑ |
| 9 | ui-chat：① `ApprovalPart` 安全卡（字段分组/倒计时/默认焦点拒绝/Esc/三决策/auto 自动批准态/degraded 无元数据）；② 新增 `toolRows.ts`：按 `turn_id` 归并（新数据）+ legacy 回退（挂后一条 assistant/独立组）+ 损坏行降级 + 终态四态；③ `ToolCallPart` 展示 args/status 与 **cancelled+副作用提示**（result 非空 → i18n `toolCancelledSideEffects`「已取消（副作用可能已发生）」；result 空 → 仅「已取消」）；④ 单测 | `packages/ui/ui-chat/web/src/ApprovalPart.tsx`、`ToolCallPart.tsx`、`toolRows.ts`（新）、`AssistantThread.tsx`、`packages/ui/ui-chat/tests/*` | 8 | `pnpm --dir packages/ui/ui-chat test:unit` | ☑ |
| 10 | desktop web：`protocol.ts` 解析 approval 新字段/tool 终态字段/usage；`reducer.ts` 工具行归并 + approval 四值状态 + usage 按 turn 归属；`controller.ts` decide 四值（旧值展示映射） | `apps/desktop/web/src/protocol.ts`、`apps/desktop/web/src/state/reducer.ts`、`apps/desktop/web/src/state/controller.ts` | 8,9 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 11 | i18n 门禁：启动错误边界文案进字典（zh/en）、bootstrap 用默认 locale；新增审批/工具卡文案键（含 `toolCancelledSideEffects`「已取消（副作用可能已发生）」） | `apps/desktop/web/src/main.tsx`、`apps/desktop/web/src/i18n.tsx`、i18n 资源 | — | `bash scripts/i18n-check.sh` | ☑ |
| 12 | e2e（真实 personal rig）：扩展 `scripts/agent-e2e.sh` 的假上游工具剧本 + `e2e/agent-rig.spec.ts`：卡字段、默认焦点拒绝、Esc 拒绝、超时拒绝、始终允许（auto 成对事件/同工具不再弹决策）、刷新一致（turn 归属）、损坏行降级、usage 实时、含 token 的 bash 脱敏（**卡片掩码**；DB 掩码由 #7 的 `BridgeSink` 单测断言）、**工具执行中取消→刷新后工具卡 `cancelled`、包含「副作用可能已发生」提示（result 非空时）且不白屏（真实 store）** | `scripts/agent-e2e.sh`、`e2e/agent-rig.spec.ts` | 5,7,9,10 | `bash scripts/agent-e2e.sh`（必要时 `ZEOKEEL_AGENT_GREP=...` 单跑） | ☑ |
| 13 | 文档回写：`docs/19` §4 实现状态、`docs/38` §2/§3 状态、`docs/28` §4.6 **分开写**「工具终态枚举（success|failed|denied|cancelled；pending|running 仅内存）」与「组件六态（AGENTS §7，空/加载/错误/离线/流式中/已取消）」+ 决策枚举、`KNOWN-LIMITATIONS`（脱敏/legacy/shell 网络目标边界）、`research/client-dsh-parity-2026-11-01.md` 缺口状态 | `docs/19-client-ux-flows.md`、`docs/38-agent-runtime.md`、`docs/28-ui-module-map-and-skeleton.md`、`KNOWN-LIMITATIONS.md`、research 报告 | 1–12 | `python3 scripts/docs-check.py` | ☑ |
| 14 | 收尾：acceptance 证据、生成物/能力/组合 `--check`、workspace 编译面、复评材料 | `specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md` | 1–13 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh`；`./target/debug/zk capability --check`；**`cargo check --workspace --all-targets`**；**`cargo tree -p zk-contract-tool`（无 agent 反向依赖）**；**`cargo public-api diff`（未安装 → 记 UNVERIFIED，不冒充）** + `docs/04` §10 两侧反序列化对拍样本；`./target/debug/zk compose --edition personal --profile default --channel official --check`；`python3 scripts/spec-check.py && python3 scripts/docs-check.py && python3 scripts/review-check.py` | ☑ |

## 2. 写作用域

- 本变更只写：`packages/contracts/{agent,tool,session}`、`crates/zk-kit-tools`、`packages/client/{agent-rig,agent-loop, tool-fs-write,tool-fs-edit,tool-bash,tool-pwsh,tool-worktree,connector-mcp,store-session-sqlite}`、`packages/client/tool-session-query/tests/*`（跨包编译面，P1-R2）、
  `apps/desktop/src/{approval.rs,events.rs,ipc.rs}`、`packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-chat/web/src/**`、`apps/desktop/web/src/**`、`scripts/agent-e2e.sh`、`e2e/agent-rig.spec.ts`、文档（§0+§13，含 `docs/24` 依赖方向）、本 spec。
- 明确不写：`packages/ui/ui-settings*`（属 settings-ia）、`specs/changes/2026-11-01-client-config-approval-mcp/**`（§0b 由 config 作者执行）、server/admin、生成物（由生成器写）。
- 并行/重叠：
  - 任务 1 先行（2–10 依赖）；任务 3/4 可并行；任务 5/6/7 分别在不同包，可并行但 7 依赖 5/6 的字段；
  - 任务 8 后 9/10 可并行；任务 11 独立；任务 12 依赖 5/7/9/10；任务 13/14 收尾。
  - 与其它 spec 的重叠文件 owner：本 spec 实现期间 `apps/desktop/src/ipc.rs`、`packages/client/agent-rig/src/lib.rs`、`packages/ui/ui-kit/web/src/types.ts`、
    `apps/desktop/web/src/i18n.tsx`、`docs/19` **只允许本 spec 写**（config 后置排队，见 §5）。

## 3. 提交顺序

1. 任务 0（决策落文档）→ 2. 任务 1（契约）→ 3. 任务 2（生成物）→ 4. 任务 3/4（kit/工具）→ 5. 任务 5（agent-rig）→
6. 任务 6/7（存储/宿主）→ 7. 任务 8（ui-kit）→ 8. 任务 9/10/11（ui-chat/desktop web/i18n）→ 9. 任务 12（e2e）→ 10. 任务 13/14（文档/证据）。
（同一文件若被两任务触及，按上面顺序串行。）

## 4. 验收前检查

- [x] `python3 scripts/spec-check.py`（本 spec 仍 `draft` → 只有 draft warning）
- [x] `python3 scripts/docs-check.py`
- [x] 变更涉及的 T0/T1 命令全绿（见 design §6）
- [x] `cargo test --workspace export_schema && git diff --exit-code`
- [x] `bash scripts/schema-check.sh`
- [x] `./target/debug/zk capability --check`（INDEX 无新增条目）
- [x] `cargo check --workspace --all-targets` exit 0（覆盖 agent-loop/tool-session-query 字面量编译面，P1-R2）
- [x] `cargo tree -p zk-contract-tool` 输出无 `zk-contract-agent`（P0-13.1 无环）
- [x] `cargo public-api diff`：附证据或记 UNVERIFIED（本机未安装；不冒充已跑，P2-N16）
- [x] `bash scripts/i18n-check.sh` exit 0
- [x] `bash scripts/agent-e2e.sh` exit 0（真实 personal rig；`pnpm e2e` 不覆盖审批，不使用）
- [x] 无手改 `@generated`；无新增 capability 文案（复用 `contract.agent`/`contract.tool`）
- [x] 复用检查已做（`zk find`/能力索引，见 design §1.5）
- [x] 跨 spec 顺序已按 §5 执行（settings-ia 先于本 spec；config 后置）
- [x] PR 只包含本 spec 范围；`acceptance.md` 已填证据路径 `target/evidence/2026-11-01-client-approval-tool-rendering/`

## 5. 跨 spec 合并顺序与共享契约（P1-1/P1-2）

- **顺序（Lead 决策）**：`2026-11-01-client-settings-ia` → **本 spec** → `2026-11-01-client-config-approval-mcp` → `2026-11-01-client-mcp-config`（若 config 拆分）。
- **共享事件序列表**：唯一真源 = 本 spec `design.md` §2.0（S1–S5，其中 S3 拆为 S3a–S3c）。config spec R2 的「requested→decided 成对或缺省、不得孤立 resolved」与 S1–S3c/S5 对齐；
  **已与 config 作者对齐（Lead 2026-11-01）**：R2 改为引用本表，引用由 config 作者在复评前落实（任务 0b；本 spec 不改 config）。
- **策略求值对接**：本 spec 在 agent-rig 提供单点 `should_ask`（默认 `requires_approval`）；config 合并时注入 `ApprovalPolicy::should_ask`（不改变本表事件序列）。
- **写作用域重叠**：`apps/desktop/src/ipc.rs`、`packages/client/agent-rig/src/lib.rs`、`packages/ui/ui-kit/web/src/types.ts`、`apps/desktop/web/src/i18n.tsx`、`docs/19` 由本 spec 独占；
  config 以任务后置排队（在 config tasks 中登记），不得并行改同一文件。
- **文档先行**：任务 0 必须先于产品代码合并；复评时核对 `docs/19`/`docs/38` 已记录本 spec 的覆盖口径与非目标（C12/C19）。
