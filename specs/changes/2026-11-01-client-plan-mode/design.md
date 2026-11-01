# 2026-11-01-client-plan-mode · 设计

> 状态 draft；D3/D4/D6 已定。共享设计：三子 spec 串行 compact→plan-mode→skill-commands；共享文件先写者写；错误码 `plan/*` 登记 docs/26 §5；菜单占位由 Batch1 catalog 改真实。

## 0. 决策
| 决策 | 选择 |
| --- | --- |
| D3 | plan 禁 mutating/允许只读；exit_plan_mode 唯一提交；本地审核；无 sandbox/企业策略 |
| D4 | 右栏 plan tab order 50 + turnTail；`.zeokeel/plans.json` |
| D6 | 不自动执行计划/不自动压缩 |

## 1. 影响
- 新包 `packages/client/tool-plan` 四件套 `exit_plan_mode`；agent-rig plan filter；`tools/zk/src/main.rs` generated facade `set_plan_mode`；host ipc.rs plan 状态/审核；ui-chat PlanCard；ui-inspector plan tab；ui-kit context DTO/actions；App/controller/i18n；e2e。
- 无 DB migration；不改 tool contract/enterprise；docs/19/26/KNOWN。

## 2. 方案
### 2.1 tool-plan 包
- zk.toml role `tool.plan` contract `zk-contract-tool`, editions personal/enterprise；Cargo；src；conformance；README。
- `exit_plan_mode` spec: `{plan: string}` (Markdown ≤64 KiB), mutating false/requires_approval false；call_json 校验 plan 非空/UTF-8/≤64 KiB，读 `.zeokeel/plans.json` 当前 mode（host 在 turn 前写 mode=active），写 status pending_review+plan+through_turn，返回 `{status,plan_path}`；非 active → `plan/not-active`。
- 工具只写计划文件，不执行计划/不改 mode off。

### 2.2 agent-rig plan mode
- 新增 `set_plan_mode(Option<Arc<dyn PlanModeSource>>)`（与 set_approval_policy 同模式）；`dynamic_tools` 构造时按 **显式白名单/黑名单** 过滤，而不是只用 `ToolSpec.mutating`：
  - **允许**：只读工具（非 delegation）+ `exit_plan_mode`；
  - **禁止**：所有 `mutating=true` 工具 + **所有 delegation 工具**（`subagent`、`subagent_start`、`subagent_fork`、`agent_team`、`workflow`、`ralph`；即使 spec.mutating=false）+ schedule/MCP 执行类；
  - **delegation 分类派生（P2-1）**：在 `PlanModePolicy` 中集中派生：`tool name == subagent` / `starts_with("subagent_")` / `{agent_team,workflow,ralph}` → delegation；若注册表/生成 facade 能提供 role/capability（`tool.subagent*`/`tool.workflow`/`tool.ralph`/`tool.agent_team`），优先按 role 集合派生；**静态清单兜底**（当前六个名字）+ 枚举测试，防止后续新 delegation 漏网。
  - 禁止工具不执行、返回可读 `plan/mutating-blocked`/`plan/delegation-blocked`；
  - **不把 plan mode 传播到 child agent**（本 spec 不做，记后续）：由于 child 继承全部 tools，必须在父层阻断 delegation，防止绕过。
- 生成 `zk-composed` facade；host 在 turn 前按 `.zeokeel/plans.json` 设置/清除；不修改 `ApprovalPolicy`/gate owner。
- 单测：active 时 mutating 0 次/只读可用/exit 可调用；**逐个 delegation 工具（subagent/subagent_start/subagent_fork/agent_team/workflow/ralph）被阻断且 child mutating 0**；off 恢复全部；不传播 plan 到 child 记录 KNOWN/后续。

### 2.3 host/IPC
- `plan.get {workspace_id,conversation_id}` → `{mode,status,plan,feedback,through_turn}`（无→off/null）。
- `plan.update {workspace_id,conversation_id,action:"approve"|"reject"|"continue"|"off",feedback?}`：approve→status approved+mode off（不执行）；reject→status rejected+feedback、mode active；continue→plan mode active；off→mode off（取消未提交草稿）；状态原子写 `.zeokeel/plans.json`，校验归属/≤64 KiB/幂等。
- `/plan` toggle：mode active/off；`/plan off` 同 update off。
- host 在 turn 前注入 plan mode；失败清空 mode 并提示；不新增企业端点。

### 2.4 UI
- Batch1 catalog：`slash-plan` 从 unimplemented → `leadingInput`；`/plan`/`/plan off` 解析；runCommand 调 actions。
- `PlanCard`（turnTail）：status/plan Markdown/批准/拒绝/继续/反馈；pending 显示动作；approved/rejected 只读；无计划不渲染。
- `ui-inspector`：TABS 增加 plan order 50；面板显示计划/状态/反馈/审核按钮；复用 side-panels 宽度/折叠/localStorage；流式中只读。
- i18n zh/en；焦点回计划卡/composer；axe；六态。

## 3. 备选
完全禁工具（无法调研）；提示词级禁止（不安全）；复用 activity tab（混排）；自动执行计划（不安全）——均不采用。

## 4. 安全
-plan 本地审核，不接企业策略/sandbox（C6）；不自动执行；mutating 由工具过滤拒绝；计划文件 workspace 内，不含 key；C18 归属；C10 schema/C11 生成物/C17 单实现。

## 5. 测试
| 层 | 用例 | 命令 |
| --- | --- | --- |
| tool | exit_plan_mode 参数/非 active/超限/pending | cargo test -p zk-tool-plan |
| agent-rig | active mutating 0/只读/exit；逐 delegation 工具阻断/child 0；off 恢复 | cargo test -p zk-agent-rig |
| host | plan.get/update approve/reject/continue/off/恢复/归属/损坏 | cargo test -p zk-desktop |
| UI | 命令/plan tab/turnTail/六态/键盘 | pnpm ui-chat/app test:unit |
| e2e | 进入/只读/mutating 拒绝/exit/review/刷新/axe | pnpm e2e；e2e/interaction/command-plan.spec.ts |
| 门禁 | compose/validate/schema/i18n/gate | tasks §4 |

## 6. 文档
docs/19 plan mode/审核；docs/26 IPC/plan 文件/错误码；KNOWN 无 sandbox/不自动执行。
