# 2026-11-01-client-activity-panels · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `ui-inspector`（activity tab order 40）、`ui-chat`（turnTail）、`@zeokeel/ui-kit`（共享 DTO/UiActions）；不新增包/slot |
| 契约 | `zk-contract-ipc`：新增 `activity.snapshot`/`activity.action` + payload/result DTO（additive）；不新增 capability；`docs/26` §5 登记命令/错误码 |
| 路由 | 无 HTTP 路由；本地 IPC 新增 2 命令（`serve`/Tauri 共用 dispatch） |
| 迁移 | **无数据库迁移**；不新增持久化状态（goal 用既有 `goal.get`；todo/present 派生自 tool 行；schedule 复用 `.zeokeel/schedules.json`；plan 由 plan-mode `.zeokeel/plans.json`） |
| 生成物 | `shared/contracts/{schema,ts}`、`crates/zk-composed/{Cargo.toml,src/lib.rs}` 由生成器写；本 spec 只改生成器源 `tools/zk/src/main.rs`（生成 `zk-composed` facade 暴露 `zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule`），**不手改 `@generated` 模板/产物** |
| edition/profile/channel | 无 edition 分支；personal/enterprise 同面 |
| 文档 | `docs/16`（右栏 tab order 40）、`docs/26` §5（activity 命令/错误码）、`KNOWN-LIMITATIONS.md`（进程内/派生/goal 只读边界）；`docs/28` 的 activity tab 预留已在 Phase 2b 记录 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| jobs 列表/输出/停止 | `kit.jobs`：`zk_kit_jobs::{list,output,kill}`（`crates/zk-kit-jobs/src/lib.rs:119-175`） | 复用 | kit |
| subagent roster/status/interrupt | `kit.subagent`：`zk_kit_subagent::{list,runner}` + `SubagentRunner::{status,result,interrupt}`（`crates/zk-kit-subagent/src/lib.rs:28-90,123-208`） | 复用 | kit |
| agent-team 最近运行结果 | 工具行结果 JSON（`packages/client/tool-agent-team/src/lib.rs:195-379`） | 复用（只读展示） | 工具行 |
| goal 状态 | `goal.get`（`client-command-surface` IPC；同一 `.zeokeel/goal.json`） | 复用（只读；不派生工具行、不新增写面） | command-surface host |
| todo 清单 | `todo` 工具 args/result（`packages/client/tool-todo/src/lib.rs`：list/add/update/remove 语义） | 复用（按工具语义 replay，不新增状态机） | 工具行 |
| present 交付物 | `present` 工具行结果 JSON（`packages/client/tool-present/src/lib.rs:41`） | 复用 | 工具行 |
| schedule 列表/更新/删除 | `zk_tool_schedule::tool()` 的公开 `Tool::call_json`（list/update/delete，含 cron/interval/文件形状校验，`packages/client/tool-schedule/src/lib.rs:76-160`） | 复用（host 经 `zk-composed` 生成 facade 调用；前端不 import；不建第二套解析） | 工具包/generated facade |
| 右栏 tab/预览/状态 | Phase 2b `ui-inspector`（tab order 40 预留、file_read 预览、六态/持久化） | 复用/扩展 | ui-inspector |
| turnTail 容器/消息部件 | Phase 2b `ui-chat/AssistantThread.tsx`（消息部件/`data-tool` 数据） | 复用（同文件内新增 turnTail） | ui-chat |
| i18n/a11y | `useUi().t`、`SlotErrorBoundary`、AxeBuilder | 复用 | ui-kit/app |

- 搜索证据：`rg -n "job_list|job_kill|subagent|agent_team|goal|todo|schedule|present|turnTail" packages apps crates docs`；
  `./target/debug/zk find kit.jobs` → `crates/zk-kit-jobs/src/lib.rs`；`zk find kit.subagent` → `crates/zk-kit-subagent/src/lib.rs`；
  capability 索引 `kit.jobs`（`shared/contracts/capability-index.json:107`）、`kit.subagent`、`contract.ipc`（`:43`）、`kit.tools`（`:131`）。
- 为什么不能复用/扩展：这些能力只有工具调用/进程内 registry，没有 UI 读面；但都能从**现有 kit API / 已持久化工具行 / tool 包公开 API** 得到，
  因此不新增存储或第二套状态机（C17）。不修改 `tool-todo`/`tool-agent-team`/`tool-goal` 行为（D2-A）。
- 新增可复用能力：无新 capability/INDEX 条目；仅 `contract.ipc` 既有契约新增命令。

## 2. 方案

右栏新增 activity tab（order 40）+ 聊天 turnTail。host 提供两个 IPC：只读聚合 `activity.snapshot` 与显式动作 `activity.action`；
snapshot 由 kit registry（jobs/subagents）、既有持久化 tool 行（todo/present/team）、schedule 工具 list 组装；**goal 不进入 snapshot，由独立 `goal.get` 读取**；
action 复用 kit kill/interrupt 与 schedule 工具 update/delete；UI 只在打开 tab 时轮询/刷新，不新增事件通道。

### 2.1 关键流程

```text
打开 activity tab → activity.snapshot{workspace_id, conversation_id}
  → jobs: kit.jobs::list()（cwd 在当前 workspace 根内）
  → subagents: kit.subagent::list()（parent == conversation）+ runner.status
  → teams/todos/present: 扫描 messages_in 的 role=tool 行（name/args/result/turn_id）
  → goal: 调既有 `goal.get`（单 owner；不派生工具行）
  → schedules: zk_tool_schedule::tool().call_json(ctx, {"action":"list"})
  → 渲染分区；2s 轮询 + 手动刷新
动作：
  job kill → activity.action{action:"job_kill", id}
  subagent interrupt → {action:"subagent_interrupt", id}
  schedule toggle/edit/delete → {action:"schedule_update"|"schedule_delete", id, updates}
  → 动作后重新 snapshot，并以 host 当前状态校验（不新增 version 字段）；目标已被其它动作结束/未知 → 可读错误并刷新
  → 失败返回可读错误（保留旧快照）；不乐观改状态
  → job 停止/subagent 中断：action 前内联二次确认（对象名/影响；默认焦点取消；role=alertdialog），确认后才发 action
turnTail：assistant 轮结束 → 扫描本轮 present → 交付卡；**plan 由 plan-mode PlanCard 负责，不扫描计划标题**
```

### 2.2 数据结构（新增）

```text
# zk-contract-ipc
commands::ACTIVITY_SNAPSHOT = "activity.snapshot"
commands::ACTIVITY_ACTION   = "activity.action"

ActivitySnapshotPayload { workspace_id, conversation_id }
ActivitySnapshot {
  jobs: Vec<ActivityJob>,
  subagents: Vec<ActivitySubagent>,
  teams: Vec<ActivityTeamRun>,
  // 2026-11-01 修订：goal 不进 snapshot；UI 独立调 goal.get

  todos: Vec<ActivityTodo>,
  schedules: Vec<ActivitySchedule>,
  presentations: Vec<ActivityPresentation>,
  truncated: bool,
}
ActivityJob { id, command, cwd, status, started_ms, output_excerpt }
ActivitySubagent { id, parent, depth, status, prompt }
ActivityTeamRun { turn_id, message_id, status, workers: Vec<ActivityTeamWorker> }
ActivityTeamWorker { id, task, status, result_excerpt: Option<String> }
ActivityTodo { id, text, status }
ActivitySchedule { id, prompt, cron, enabled, interval_seconds, next_run_ms: Option<u64> }
ActivityPresentation { path, description, turn_id, message_id }

ActivityActionPayload {
  workspace_id, conversation_id,
  action: "job_kill" | "subagent_interrupt" | "schedule_update" | "schedule_delete",
  id: String,
  #[serde(default)] updates: serde_json::Value,  # schedule_update: {enabled?, interval_seconds?, prompt?}
}
ActivityActionResult { ok: true }   # 失败走 CommandResponse.err
```

限制：jobs 摘要 ≤4 KiB/条、≤50 条；subagents ≤64（registry 上限）；todos/schedules/presentations ≤100；teams 只取最近 3 次；
字符串 prompt/description/result_excerpt 截断（≤512 字符）；`truncated=true` 当任一分区裁剪。

### 2.2.1 生成器 facade（C4/C11，P2-11/P2-12）

- **生成物归属澄清（P2-11）**：`tools/zk/templates/app_desktop.Cargo.toml` 是**生成器输入源**（其首行 `@generated` 为历史遗留注释，不代表它是本 spec 的生成物）；本 spec **不改该模板**，只改生成器源 `tools/zk/src/main.rs` 并让 `zk compose` 生成 `zk-composed`/`apps/desktop/Cargo.toml`；二者仍是 @generated，禁止手改，`--check` 必须绿。
- 前端不 import tool；host 不直接新增 `zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule` 依赖到 `apps/desktop`：`tools/zk/src/main.rs`
  生成 `zk-composed` 的对应依赖与 `pub use`（`zk_kit_jobs`/`zk_kit_subagent`/`zk_tool_schedule`），host 通过 `zk_composed::*` 调用。
- 只改生成器源，`@generated` 模板/产物由 `zk compose` 写；**`tools/zk/templates/app_desktop.Cargo.toml` 保持不动（不是本 spec 写作用域）**；`zk compose --check` 绿。

### 2.3 派生规则（host 内 helper，单测覆盖）

- **tool 行解析**：`content` JSON `{name,args,status,result}`；其中 `args` 是 **JSON 字符串**（`ToolCallFinished.args: String`，P2-10）→
  `PersistedToolRow { name, args: Option<Value>（对 args 字符串做 serde_json::from_str 二次解析）, result, turn_id }`；
  legacy 行可能无 args/status；任一层 JSON 解析失败跳过并计入 `truncated`/提示（不 panic）。
- **goal（2026-11-01 修订）**：activity 不派生 goal；UI 直接调既有 `goal.get{workspace_id,conversation_id}`（command-surface），展示 objective/status/rounds/max_rounds/reason/last_result；无 goal 空态；损坏降级；不写、不产生工具行。
- **todo**：按消息顺序 replay `name=="todo"` 且 `status=="success"` 的行（args 用二次解析后的对象）：
  `list` → result JSON 数组替换；`add` → args.text + 从 result `added <id>` 提取 id，status=pending；
  `update` → args.id/status（不存在则忽略）；`remove` → args.id 删除。无 args 的 legacy 行跳过。
- **presentations**：`name=="present"` 且成功 → result JSON `{presented,description}`；按 turn 分组，去重 path；保留 turn_id/message_id。
- **teams**：`name=="agent_team"` 且成功 → result JSON `{status,workers[],...}`；取最近 3 次；workers 映射 id/task/status/result 前 512 字符；
  解析失败跳过。
- **jobs**：`zk_kit_jobs::list()` 返回数组 JSON；过滤 `cwd` canonicalize 后以当前 workspace canonical root 为前缀，**用 `Path::starts_with`（组件级）
  而非字符串前缀**（避免 `/workspace-evil`）；`output_excerpt` =
  stdout + stderr 截断（≤4 KiB）。
- **subagents**：`zk_kit_subagent::list()` 过滤 `parent == conversation_id`；status 直接用 registry；prompt 截断。
- **schedules**：调用 `zk_tool_schedule::tool().call_json(&ctx, {"action":"list"})`（ctx 由 workspace root + conversation + workspace id 构造），
  结果 JSON 数组解析为 `ActivitySchedule`（缺字段用默认；id/prompt 必须存在，否则跳过）。
- **action**：`job_kill` → `zk_kit_jobs::kill(id)`；`subagent_interrupt` → `zk_kit_subagent::runner().interrupt(id)`；
  `schedule_update/delete` → 先 `list` 预检 id 存在（不存在 → `activity/schedule-invalid`，P1-3），`updates` 白名单（enabled/interval_seconds/prompt）
  且 **host 预校验 prompt ≤32 KiB**（tool-schedule 无长度校验，P2-13），再调 `call_json`（`{action:"update"/"delete", id, ...}`）；
  若 delete 返回 `"deleted 0"` 同样映射 `activity/schedule-invalid`；所有错误映射为 `activity/*` 封闭字符串（§2.4）。

### 2.4 错误码（CommandResponse.error 字符串，docs/16/KNOWN 记录）

`activity/job-not-found`、`activity/job-finished`、`activity/subagent-not-found`、`activity/schedule-unreadable`、
`activity/schedule-invalid`、`activity/derived-unavailable`（非阻塞提示）、`activity/truncated`、`session/workspace-mismatch`、
`session/not-found`（既有）。

### 2.5 UI：activity tab

- Phase 2b `ui-inspector` 的 `TABS` 增加 `activity`（顺序 files 10/changes 20/terminal 30/activity 40；docs/16 固化）；
  面板内联在 `index.tsx`（不新增组件文件）：分区标题 + 内容 + 分区级状态；tab 切换沿用会话级持久化。
- 数据/动作经 `UiValue.actions`（app 注入；见 §2.7）：`activitySnapshot(workspaceId, conversationId)`、`activityAction(payload)`；
  组件内 `useEffect` 2s interval（仅 tab 打开）+ 手动刷新 + 动作后刷新；`AbortController`/序号丢弃旧响应；卸载清 interval。
- jobs：行（状态点/命令/cwd/摘要）+ 展开输出 + 停止按钮（**仅 running 可用**，点击先出现**内联确认区**（job id/命令/影响，默认焦点取消，role=alertdialog），确认后 `aria-busy` + 就地状态，未知/已结束显示可读错误，不重复点击）；subagents：行 + 中断（同样仅 running + 内联确认 + busy）；team：最近运行卡片；
  goal：独立读取 `goal.get` 的 objective/status/rounds；todo：checkbox 样式只读列表（不写）；schedule：行 + enable toggle + 内联编辑（prompt/interval）+ 删除（二次确认，
  复用 ui-kit 按钮 + 内联确认，不新增 Modal 组件）；presentations：path/description + 「在右栏查看」。
- 六态：各分区空/加载/错误；轮询失败保留旧数据 + stale 标记；离线仍尝试本地 IPC，失败显示错误。

### 2.6 UI：turnTail（仅 present）

- 在 `packages/ui/ui-chat/web/src/AssistantThread.tsx` 的 `AssistantMessage` 中，于 `MessagePrimitive.Parts` 之后渲染 turnTail：
  - 扫描本轮/最近 `present` 工具结果 → 交付卡（文件名/描述/path 去重；点击 `openInspectorPreview` 打开右栏 files）。
  - **不识别/不折叠 plan**；plan 已由 `2026-11-01-client-plan-mode` 的 `PlanCard` 接管（同位置由 plan-mode 负责，activity 不重复）。
  - 无 present 不渲染；损坏结果可读降级；仅完成/取消/错误消息展示策略沿用；i18n。

### 2.7 app 集成

- `UiActions` 新增：`activitySnapshot(workspaceId, conversationId): Promise<ActivitySnapshot>`、`activityAction(payload): Promise<void>`、
  `openInspectorPreview(path: string): void`；实现直接在 `App.tsx`（已有 transport 调用模式，不新增 controller 文件）。
- `App.tsx` 增加 `previewRequest` 状态 + 传给 `AppShell`/inspector；`inspectorTab` 类型含 `"activity"`；`protocol.ts` 增加
  `parseActivitySnapshot`/`parseActivityActionResult`（Ajv/generated schema 校验）；i18n 键（分区标题/动作/错误/空态/交付）。
- 轮询由 ui-inspector 负责（不污染 app reducer）；app 只提供 IPC action。

### 2.8 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 两个聚合 IPC（snapshot/action）+ 现有 kit/tool 行/tool-schedule 复用 + 右栏 tab + turnTail | 文件/契约小、无新状态机、复用最大 | snapshot 需 host 派生/聚合；轮询而非事件 | **采用** |
| B. 每能力独立 IPC + 面板各自持久化 | 边界清晰 | 命令多、契约/文件膨胀、重复状态 | 放弃（预算/ C17） |
| C. 改 tool-todo/tool-agent-team 增加持久化/查询 API | 数据更准 | 修改工具行为/状态机、超出本 spec | 放弃（D2-A；后续可另立） |
| D. activity 用事件推送（SSE/新事件类型） | 实时 | 新事件通道/契约/连接管理 | 放弃（D4-A；轮询足够 MVP） |
| E. plan mode（Plan 工具/审批/执行状态） | 对齐 DSH | 已由 `client-plan-mode` 实现；activity 不再重复 | 移交（2026-11-01） |

### 2.9 安全与隐私

- 只读快照 + 显式用户动作；所有数据在当前 workspace/会话范围内（jobs cwd 前缀校验、subagents parent 过滤、schedule 只写当前 workspace）。
- 不新增网络/权限/审批；job kill/subagent interrupt 为用户显式操作（非 agent 工具）；schedule 写复用工具既有校验/原子写。
- 输出摘要/提示词截断，避免大 payload；不记录密钥（jobs 输出可能含用户数据，仅本地展示，不写日志）。
- 无新增持久化/审计；错误为可读字符串，不泄露路径之外信息（activity 只返回 workspace 相对/命令本身）。
- C4/C2：前端 `apps/desktop/web` 不 import tool/kit 实现，所有数据经 host IPC；host（组合根）通过 `zk-composed` 生成 facade 调用 kit/tool，
  不新增 app 直接依赖；personal/enterprise 构建均包含所需包（生成期解析，不运行期加载）。

### 2.10 发布与回滚

- 无灰度/强制更新；IPC additive、无 DB 迁移；旧客户端不调用新命令。
- 回滚：回退 commit；schedule 文件由既有工具继续读写；goal 由 goal.get 读取（文件不变）；todo/present 工具行不变；无新持久化需清理；activity tab 从 UI 消失（旧版本忽略）。

### 2.11 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | compose/schema/capability | `zk compose --edition personal --profile default --channel official --check`；`bash scripts/schema-check.sh`；`zk capability --check` |
| T1 契约 | DTO schema/默认值 | `cargo test -p zk-contract-ipc`；`cargo test --workspace export_schema` |
| T1 host | snapshot 派生（todo replay/present/team/损坏降级/args 二次解析）、**goal.get 只读（不进 snapshot、不派生工具行）**、jobs 组件级过滤、schedule 预检/prompt 上限/delete `deleted 0` 映射、边界 | `cargo test -p zk-desktop` |
| T2 UI | tab/分区/轮询/动作；turnTail 仅 present；goal 分区读 goal.get | `pnpm -r typecheck && pnpm -r lint`；e2e/手工 |
| T3 e2e | `e2e/interaction/activity.spec.ts`（@interaction ID）+ `e2e/serve-m1.sh` 剧本（job/subagent/todo/goal/schedule/present；goal 用 goal.get）；`pnpm e2e:ui --grep activity`（工具已落地） | 交互门禁；host 单测并行 |
| T4 门禁 | typecheck/lint/i18n/build | `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh` |

## 3. 备选与取舍（细节）

- 轮询 vs 事件：MVP 选轮询（D4-A）；activity tab 关闭/切会话即停，不占后台。
- todo replay vs 工具持久化：选 replay（D2-A）；legacy/损坏行提示，不修改工具状态模型。
- schedule 调用方式：host 直接构造 `Tool::call_json` 请求（list/update/delete），复用工具侧校验/原子写；不做第二份 JSON 读写。
- 团队面板：只显示最近 agent_team 结果（工具行），不做任务板/续聊（非目标）。

## 4. 文档更新

- [ ] `docs/16` §2.2/§2.4：右栏 tab order 增加 `activity 40`（files 10/changes 20/terminal 30）；不新增 slot。
- [ ] `KNOWN-LIMITATIONS.md`：jobs/subagent 进程内、重启丢失；todo 由工具行推导；goal 只读 goal.get；schedule 无新建/历史；**plan 由 plan-mode 负责（activity 不展示）**；activity 轮询 2s/无事件推送；输出摘要截断。
- [ ] `docs/28`：activity tab/turnTail 的预留已在 Phase 2b（side-panels）记录；本 spec 不重复改（如需最终状态由后续统一回写）。
- [ ] ADR/D：无新增；**D3 plan 展示已被 `client-plan-mode` 取代**（2026-11-01 修订记录）。

## 5. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 工具行 legacy/损坏导致派生缺失 | 中 | 低 | 跳过+提示；单测；KNOWN | host 单测 |
| 全局 jobs/subagent 状态与会话可见范围不一致 | 中 | 中 | cwd/parent 过滤；空态说明；KNOWN | host 单测 + e2e |
| schedule 写坏文件 | 低 | 高 | 复用 tool 校验/原子写；未知 id/非法值拒绝；e2e 文件断言 | host 单测 + e2e |
| 轮询负载/竞态 | 低 | 低 | 2s/仅打开 tab/关闭即停 + AbortController/序号丢弃旧响应；快照有界 | e2e/手工 |
| 与 side-panels 共享文件竞写 | 中 | 中 | 严格顺序（side-panels 合并后开工）；共享文件 owner 见 tasks §5 | 评审 |
| app 直接依赖 tool 实现（C4） | 低 | 中 | 已改方案：app 不 import tool/kit；host（组合根）经 `zk-composed` 生成 facade 调用 `zk-tool-schedule` 公开 `Tool::call_json`，数据只走 host IPC（见 §2.4/§2.6）；无 app→tool 直接依赖 | 评审 |
