# DSH Web · Conversation / Composer / Tool / Approval / Plan / Goal / Jobs / Subagent / Deliverables 模块研究

> 只读研究：未修改任何代码/规范。证据 = 本机 DSH 包 README + 随包 `.d.ts` 契约 + ZeoKeel `docs/19`、`docs/27`、`research/dsh.md`；并行出现的 `docs/28`、`docs/12`、`docs/16` 与骨架文件在 §4.6 对账。
> 路径简写：`$PKG = /usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai`；
> `records/input/chat-nodes/slots.d.ts` 等均指对应 DSH 包 `lib/types/client/contract/` 下的同名文件；
> `view.d.ts` 指 `dsh-jobs/lib/types/view.d.ts`，`model.d.ts` 指 `dsh-api-job-controller/lib/types/client/model.d.ts`。
> C15：DSH 仅作事实/设计参考，不进入 ZeoKeel 依赖、构建输入、索引仓或数据源。
> 标注：**[未验证]** = README/类型未覆盖、仅从命名或相邻包推断；**[推断]** = 由包面行为可推出但未见实现代码。

## 0. 结论先行

1. **DSH 把"会话装配"和"目标渲染"分成两层**：`ui-conversation` 只做 target-neutral 的 event/view registry、按 Session 绑定、输入状态与 shell；`ui-chat` 才注册 Chat 的 Definition/snapshot/View/渲染器（`$PKG/dsh-client-ui-conversation/README.md:14,31`；`$PKG/dsh-client-ui-chat/README.md:13`）。ZeoKeel 的 `@zeokeel/ui-chat` 目前把这两层合在一个包内（`packages/ui/ui-chat/web/src/index.tsx:6`），这是 M1 的主要差距。
2. **Composer 不是单一输入框，而是"常驻 shell + takeover 链"**：审批通过 `conversation.composer` chain 整体接管；计划 chip、GoalBar、Todo、Queue、统计、附件栏、触发菜单各自占独立 slot（conversation:85,97,120-163；plan:52；goal:46）。
3. **工具呈现是"树 + 按 wire tool name 的 keyed 视图"**：Runtime 拥有 call/result 配对与递归 `subCalls`，UI 只按工具名分发原子卡；未注册工具走通用卡（tool:12,36,68,72）。
4. **审批是临时的 allow-once / reject**，持久权限策略归 Host；超时机制在客户端 README 未描述（[未验证]）；面板只是等待 Host waterfall 的一个可回答请求（approval:11,36；approval `slots.d.ts:48,52,78`）。
5. **DSH 的 plan 不是步骤清单**：`/plan` 进入 plan mode，`exit_plan_mode` 提交 Markdown，右侧栏 review + turnTail 卡片；步骤清单属于独立的 Todo dock（plan:32,36,52；conversation:85）。
6. **Jobs / Subagent / Deliverables 都是边缘插件**：Jobs 在 Session header actions，Subagent 在 header lineage + 资源 tab，Deliverables 在 `conversation.chat.turnTail` + 右栏 review tab（jobs:48；subagent:28,30；deliverables:67）。
7. **对 ZeoKeel**：M1 对话域不应新增 slot（shell/inspector/commands 属 `docs/28` 的独立交互面）；先在 `@zeokeel/ui-chat` 内按 conversation/composer/tool/approval 四块拆目录与数据契约，post-M1 再按"第二消费者/替换需求/T21 keyed slot"逐块升格为包（依据 `docs/27:44,46-62`；`docs/19:61`；`docs/16:60-64`）。

## 1. 模块清单与组件树

### 1.1 包/模块清单

| DSH 包 | 职责 | 关键 slot / 注册 | 证据 |
| --- | --- | --- | --- |
| `ui-conversation` | target-neutral 装配：事件/视图/Group/UserText registry、Session binding、输入机、shell、图片 URL 缓存 | `main.conversation`、`conversation.session`、`conversation.header`、`conversation.composer`、`conversation.input.*` | conversation:14,31,47,85；conversation `slots.d.ts:295-343` |
| `ui-chat` | Chat 目标：节点 View、滚动/折行、turn rail、过程分组、隐藏行、失败行 | `conversation.chat.flow`、`conversation.chat.node`、`conversation.chat.turnTail`、`conversation.chat.assistant-actions` | chat:13,53,80,93,102；`slots.d.ts:318-396` |
| `ui-tool` | Tool 卡：root/子调用树、keyed 原子视图、内置卡、Inspect | `tool.call.toolview`、`tool.call.images` | tool:12,36,46,72；`slots.d.ts:19-38` |
| `ui-approval` | 审批 takeover + 可选关联 Tool 详情 + 决策回传 | `conversation.composer`（match）、`conversation.approval.detail` | approval:11,21；approval `slots.d.ts:17-23,48-96` |
| `ui-plan` | plan mode chip、提交计划卡、右栏 review | `conversation.input.plan`、`conversation.chat.turnTail`、`conversation.plan-review.actions` | plan:32,36,52,54 |
| `ui-goal` | 目标条（composer-context 第二张卡）与 `/goal` 指令气泡 | `conversation.input.dock`、`conversation.user.text.node`（command） | goal:28,32,46 |
| `ui-jobs` | Session header 的后台任务列表与输出面板 | `conversation.session.header.actions` | jobs:28,32,34,48 |
| `ui-subagent` | 子代理 catalog、续聊/只读 composer、`@` 引用源 | header lineage/actions、`dsh-resource://subagentchat/...` | subagent:28,30,40,44 |
| `ui-deliverables` | 变更文件卡、present 交付卡、inline-code 链接、review tab | `conversation.chat.turnTail`、`sidebar.right.pane.tab`、`chatFileMentions` | deliverables:28,37,49,67 |
| `ui-input-trigger` | `/`、`@` 触发检测、分组菜单、pick 路由（本身不含命令目录与引用源） | `conversation.input.overlay`（list） | input-trigger:12,32,46,57-58 |
| `ui-attachment` | 草稿附件 rail、drop overlay、消息图廊、lightbox | `conversation.images.content` + 4 个图片位置 | attachment:28,32,36,52 |
| `ui-message-feedback` | Like/Dislike、反馈弹窗、`/feedback` 装饰 | `conversation.chat.assistant-actions`（order 10）、`conversation.input.overlay`（order 2） | feedback:28,42 |

> 触发菜单的候选来源不在 `ui-input-trigger`：`/` 来自 `ui-commands`，`@` 文件/会话来自 `ui-reference`，运行中子代理来自 `ui-subagent`（input-trigger:34,57-58,86；subagent:44）。

### 1.2 组件树（按渲染位置）

```text
app shell / ui-layout root
└─ main.conversation                         conversation:85
   ├─ conversation.header (session-maybe)
   │  ├─ leading / corner / utilities         conversation:85；slots.d.ts:237-258
   │  └─ conversation.session.header
   │     ├─ lineage（子代理面包屑）            subagent:28
   │     ├─ actions（Jobs、子代理计数）        jobs:48；subagent:28
   │     └─ view tabs（Chat / Trajectory…）    conversation:73,89
   ├─ conversation.session → conversation.view → Chat
   │  └─ conversation.chat.flow
   │     ├─ 过程分组（Group Definition）        chat:102,110
   │     ├─ conversation.chat.node[kind]        chat `slots.d.ts:332-343`
   │     │  ├─ user / assistant / steering / context
   │     │  ├─ tool-call → ToolCallTree → tool.call.toolview[name] → tool.call.images
   │     │  ├─ turn-process / turn-tail
   │     │  ├─ command / compaction / model-retry / turn-error / turn-max-tokens
   │     │  └─ 扩展节点（plan、deliverables 经 turnTail）
   │     └─ conversation.chat.assistant-actions（copy/branch/feedback）
   └─ composer
      ├─ conversation.input.dock（Todo、Goal、Queue）   conversation:85；goal:28
      ├─ conversation.composer（chain；审批 takeover） conversation:120-163；approval:11
      ├─ conversation.composer.bar（Lexical 编辑器）   conversation:77
      ├─ input.left / right / plan / activity          conversation `slots.d.ts:311-324`
      ├─ input.attachments（附件 rail + drop）          conversation `slots.d.ts:333`；attachment:28
      ├─ input.overlay（触发菜单、反馈弹窗）            input-trigger:46；feedback:42
      └─ composer.dock（统计 activity/usage）           chat:71
```

## 2. 交互流程

### 2.1 一次 Turn 的生命周期

1. **输入**：草稿由 Session 级 Lexical 编辑器持有，输入机只持有提交平面；`InputState.phase ∈ plain|adjudicating|claimed|submitting`（conversation:77,79；`input.d.ts:261-268`）。`/`、`@` 在光标处触发菜单，claim 后成为原子 chip，chip 在则 claim 在（conversation:81；input-trigger:32）。
2. **提交**：每次尝试先冻结发生时间、手势、投递意图、Session model/run-state 快照，再做异步命令裁决；普通消息通知只发一次（conversation:103）。默认发送是乐观的：同一事务清空草稿/occurrence/undo，然后以 detached attempt 执行（conversation:105）。
3. **落位**：`sendSession` 先登记 submission echo（`session.beginSubmission`），再由 Session 按 delivery mode 与运行态决定去 transcript、QueueDock 还是 pending-steering 面（conversation:105）。
4. **流式**：Assistant 增量按帧批量发布；本地 echo 与 steer echo 在权威记录到达时原子消失（chat:13,108）。Chat 隐藏 system prompt、普通 context 注入、`permission` 命令行，但不改 recorded event（chat:53）。
5. **工具**：`tool/call` 启用 call 呈现；参数块完成本身不启动执行。行先 `preparing`（参数原地增长），派发后 `start`，结果落为 `tool-result`，同一 callId 贯穿三阶段（tool:32,72,77；`records.d.ts:284-295`）。
6. **审批（若命中）**：pending interaction 被发布到 `ctx.uiSession`，审批 chain 在常驻 composer 之上接管；可渲染关联 Tool 详情（approval:11；approval `slots.d.ts:17-23`）。
7. **计划更新（若在 plan mode）**：`exit_plan_mode` 产生提交计划；Turn 结束时计划卡进入最终 artifact 区，并自动在右栏打开一次（plan:36,54）。
8. **完成**：`turn/end` 后渲染 completed-turn footer；只在窗口含 `turn/start` 且所有 attempt 的 usage 完整时显示 usage 行（chat:67,82）。变更文件卡位于 closing message 正文与 action footer 之间（deliverables:28）。
9. **失败/中断**：终态失败渲染红点行；quota 用中性文案；输出 token 上限用琥珀点；恢复期被关闭的 turn 渲染 "stopped" 且无 code chip（chat:60）。

### 2.2 取消 / 重试 / 编辑 / 队列 / steer

| 动作 | DSH 机制 | 证据 |
| --- | --- | --- |
| 取消当前 turn | Stop 按钮；Chat/Composer 聚焦时 **500ms 内两次 Esc**（可配 `stopSequenceMs`），与 Stop 使用同一 scoped cancel | conversation:115 |
| 取消后的按钮 | 运行中主按钮可为 Stop；草稿可操作时同一座位切 Send；turn 结束后 Stop 变禁用 Send | conversation:109 |
| 重试 | 同一 Step 的重试复用节点 key；`model-retry` 节点状态 `scheduled|started|cancelled` | chat:55；`records.d.ts:113-127` |
| 编辑草稿 | 发送失败把草稿恢复（空或仍是上次自动恢复时），保留其后输入；命令提交保持 frozen submitting | conversation:105,107 |
| 编辑已入队消息 | QueueDock 行 Edit 用 `InlineEditor` 暴露字面发送文本，换行保留；另有 Remove、Steer | conversation:97 |
| 队列 | `next-turn` 投影（含冷恢复消息），排除仍由本地 transcript submission 拥有的 requestId；echo 显示 "Sending…" 且按钮禁用 | conversation:97,107 |
| steer | busy 时按设置投递到 pending-steering 面；Inbox 顺序跨客户端，admission 后本地 echo 抑制陈旧行 | conversation:97,105；chat:89 |
| 编辑已发送消息（消息级） | **[未发现]** README/契约只覆盖草稿恢复与队列编辑，未描述消息级 edit/branch-to-edit | conversation:97,105；chat:82 |

### 2.3 工具行的展开/折叠

- 每个 Tool 行持有独立 `useDisclosure`；中间渲染器只转发不订阅；外层 Turn 真正隐藏 process member 时座位重置 disclosure，但组件 key 与 Hook 引用不变；显示模式切换保留展开状态（tool:74；chat:122）。
- 已完成 Turn 按 Work details（Compact/Standard/Detailed/Verbose）折叠过程组，但**不隐藏最终答案**；Verbose 保留时长/状态头且直接展示历史过程行（chat:93）。
- 原子卡只有一种呈现：没有"选中后全高详情"的第二视图；文件路径走 `openFile`，`inspect` 打开 trajectory，不可用时卡片不显示 Inspect（tool:82）。
- 工具根保留递归子调用；PTC dispatch 用 `parentCallId`，生产环境 `run_code` 被排除在 PTC program bindings 外，因此通常只有一层 dispatch（tool:72,135）。

### 2.4 审批的选择与超时

- **选择**：聚焦审批详情区后 Enter=allow once、Escape=reject；Reject 按钮聚焦时 Enter 保留按钮原生 reject；输入控件/IME 保留自己的按键（approval:21）。
- **决策值**：`allowed-once | rejected`；一个 request 被撤回或替换后不能再次作答，旧失败也不能解锁新请求（approval:21；approval `slots.d.ts:48,78`）。
- **详情**：可选 `conversation.approval.detail` 渲染关联 Tool call；`PendingApproval` 暴露 `key`、`callId`、`reason`、`displayReason`、`answerable`、`answer`、`delegate`、`abort`（approval `slots.d.ts:52-96`）。
- **超时/记住选择**：DSH 客户端 README 明确"只支持 allow-once 与 reject；持久权限策略归 Host 侧审批包"，**客户端 README 未描述超时、记住范围、风险级别 UI**（approval:36；超时是否以 withdrawal 投影表现 [未验证]）。ZeoKeel `docs/19:103-105` 要求"剩余时间、超时默认拒绝、延长一次；允许一次/始终允许（本会话）/拒绝；记住范围"，这是 ZeoKeel 自有需求，不能从 DSH 客户端推得。
- Auto denial 优先于专用视图，且不暴露原始参数（tool:86）。

### 2.5 计划卡状态

- **chip**：plan mode 生效时显示蓝色 "Plan" 按钮（hover/focus 换圈叉），执行 `/plan off`；否则座位为空；placeholder 切为 plan 任务提示（plan:32）。
- **提交**：`exit_plan_mode` 的参数在 Turn 内解析为 `SubmittedPlan { markdown, title, callId }`；每个提交在最终 artifact 区出现 file-delivery 卡（plan:36；plan `plan.d.ts:10`）。
- **打开**：pending plan 每次提交在当前浏览器会话自动打开一次；关闭后跨 review remount 保持关闭；历史卡只在点击时打开；reload 从 Session 历史恢复文本（plan:36）。
- **review 结果**：README 提到 approval / rejection / dismissal 后卡片仍可打开；reload 丢失未落日志的 review 文本；过期预览引导回 pending review（plan:36,38）。
- **失败**：准入失败（`matched:false`、业务错误、传输故障）显示 inline error，chip 保持到投影确认退出（plan:42）。
- **数据**：`SubmittedPlan` 仅 Markdown/title/callId；**没有步骤数组**。步骤清单在 Todo（`TodoItem`/TodoPanel）中，与 plan 分离（plan:52-54；conversation:85；`records.d.ts` 导入 `TodoItem`）。

### 2.6 输入触发、附件、反馈（与上面模块的接缝）

- **触发**：菜单在输入 overlay；mousedown 选取，`aria-activedescendant` 高亮；Tab 结算高亮（`drill:true` 则下钻），Escape/Shift+Tab 只退出不结算；被 dismiss 的 token 同 query 不再自弹（input-trigger:32）。
- **附件**：草稿 rail 按 pick 顺序、64px 高、图 64 方图/文件 240×64 卡；上传用 spinner + 字节进度，失败可重试；消息图按数量 240 或 64 方图；lightbox Escape/遮罩/关闭，焦点归还（attachment:32,36）。
- **反馈**：Like/Dislike 都在 finalized assistant 的 action row，二者都打开弹窗；已记录评分点击即撤回；interruption 冻结的 partial 无 messageId 因而无反馈控件；无跨 tab 推送（feedback:28,32,44,79）。

## 3. 状态与数据（README/类型可证）

### 3.1 Conversation Node

| kind | 关键载荷 | 说明/证据 |
| --- | --- | --- |
| `user` | seq/time/content/source | 最终用户消息（records:46） |
| `assistant` | messageId?/turn/step/blocks/usage/timing/interrupted? | blocks = text/reasoning/image/tool-call/other（records:27-44,64） |
| `steering` | messageId/seq/time/content | 运行中从 next-step inbox 准入的人类消息（records:89） |
| `context` | producer/form/content | system/context 注入；Chat 多数隐藏（records:100；chat:53） |
| `model-retry` | retryState | scheduled/started/cancelled（records:113） |
| `turn-error` | message/code | 终态失败；code 可为 ACCOUNT_SIGNED_OUT/INTERRUPTED（records:129） |
| `turn-max-tokens` | turn/step | 输出 token 上限（records:147） |
| `tool-result` | callId/name/args/call/content/isError/error/subCalls | 与 call head 配对；`subCalls` 递归（records:157） |
| `compaction` | summary/shadowed counts | 压缩检查点标记（records:194） |
| `unknown` | type/data | 未认领 surface 事件的回落（records:215） |
| `command` | commandId/name/args/outcome | 由 command/run+done 折叠（records:232） |

Union：`ConversationNode`（records:256）。Chat 另有 view-only kind：`chat` 的 `turn-process`、`turn-tail`、`manual-compaction`、`retry` 等（chat-nodes.d.ts:30-100）。**[推断]** 具体注册 key 见 `ui-chat` bundle 的 `key:` 列表，README 未逐项列出。

### 3.2 Tool 调用状态

- **阶段**：`preparing`（参数未齐，不可展开）→ `start`（参数齐、无结果）→ `tool-result`（有结果）；`RunningToolCall = preparing|start`；`ToolCallBlock = RunningToolCall|ToolResultNode`（records:284-295）。
- **错误**：不在独立 status 字段，`ToolResultNode.isError` + `error{name,code,reason}`；退出码/信号在结果内容里，UI 侧渲染中性胶囊 + 红点（tool:30）。
- **UI 侧**：`tool.call.toolview` 按 wire name keyed；owner props 含 `callId/name/phase/block/cwd/home/loadImage/openFile/inspect`；未注册走通用卡（tool:36,46）。
- **PTC**：子调用带 `parentCallId`，根调用没有；视图可递归（tool:46,72）。**[未验证]** 非 PTC 的深层嵌套在生产是否出现。

### 3.3 Approval

`PendingApproval`：`sessionId`、`kind:'approval'`、`key`、`toolName`、`callId?`、`reason?`、`displayReason?{en,...}`、`result: Promise<ApprovalDecision>`、`answerable`、`answer(outcome)`、`delegate()`、`abort(reason)`（approval `slots.d.ts:52-96`）。请求经 `SessionPendingInteractionMap.approval` 暴露（同文件:9）。超时、策略、记住范围：**Host 侧，客户端不可见 [未验证]**。

### 3.4 Plan

- `SubmittedPlan { markdown, title, callId }`；`PlanAddress { session, callId }`（plan `plan.d.ts:10,14`）。
- chip 的投影语义：`effective = pending ? !active : active`，是 Host 折叠值而非客户端乐观（plan:52）。
- 动词：`exitPlanMode()` 执行 `/plan off` 并把准入失败映射成 inline error（plan `index.d.ts:16`）。
- review 动作由 question 插件拥有，提供 request key、完整文本、可选 invocation 身份（plan:54）；**[未验证]** review 按钮的 approve/reject 具体 RPC。
- 无步骤数组；无 per-step 状态。

### 3.5 Goal

- `GoalSnapshot { id, revision, objective, phase, blockedReason?, maxGoalRounds }`；`GoalPhase = active|paused|blocked|complete`；`GoalActivation = armed|disarmed`（goal `types.d.ts:38,47-58`）。
- `GoalBar` props：`goal: GoalSnapshot|null|undefined`（undefined=能力缺失/加载中，null=未设置）、`activation?`、四个 mutation verbs（goal `GoalBar.d.ts:13-19`）。
- 动词 `onEdit/onPause/onResume/onClear`，CAS 以投影 ref 为陈旧保护，单飞；失败 inline（goal:46；goal `slots.d.ts:45-56`）。
- 渲染：loading/absent/complete/cleared 不渲染；active+armed 给 pause，active-disarmed/paused 给 resume（goal:28,36）。
- `/goal` 运行投影为 `Command input` 气泡，不创建 user/message 或 model turn（goal:32,46）。

### 3.6 Jobs

- `JobStatus = running|stopping|completed|killed|failed`；恰好一个终态（`$PKG/dsh-jobs/lib/types/view.d.ts:15`）。
- `JobView { id, kind, label, owner?, status, progress?, detail?, startedAt, finishedAt?, output{total,earliest,spillPaths?} }`（view:54-90）。
- 客户端 roster：`JobsSnapshot { rows: Record<sessionId, JobView[]>, observed: Record<jobId, ObservedJob> }`；`ObservedJob { text, gapBefore, streaming, error? }`（`dsh-api-job-controller/lib/types/client/model.d.ts:13-33`）。
- 交互：两段式 Stop（第一次 arm，3 秒内第二次 `ctx.jobs.kill`）；kill 不占用模型 notice，agent 仍收到完成通知；settled 段可客户端清除；展开才开 observation stream（jobs:32,34,38）。

### 3.7 Subagent

- `SubagentCatalogEntry { id, createdAt, mode: one-shot|external|continuable|unknown, label? }`（`$PKG/dsh-subagent/lib/types/projection-types.d.ts:8-23`）。
- `SubagentTimingProjection { settledMs, active?{since,through}, lastTurnCompleted? }`（同文件:25-40）。
- 续聊：continuable 且父在线 → 普通 composer，Send 入子 FIFO，独立 Stop 走 `subagents/interruptByParent`；父不可用且未运行 → 只读 composer 解释恢复路径；运行中则输入/Send 禁用但 Stop 可用（subagent:40）。one-shot 永远只读（subagent:12,70）。
- 地址：`dsh-resource://subagentchat/session/<child>?parent=<parent>&mode=<mode>`，右栏 tab 固定 Chat、省略主 header（subagent:30）。
- `@` 源：零 RPC 的 running children，插入字面 `@label `，不解析为续聊地址（subagent:44）。

### 3.8 Deliverables

- `ChangesSummary = Pick<WorkspaceChangesSummary,'turn'|'files'|'total'|'added'|'deleted'>`，经认证路由读取（deliverables `changes.d.ts:22`）。
- Turn 数据：`produced[]`（成功 write/edit/可变 str_replace_editor）、`presented?[]`、`changes?{seq}`（`turn-deliverables.d.ts:6-28`）。
- 卡：单文件 "Edited"+basename；多文件 header 显示完整数、增删行；四行后折叠；binary/too large；hover 500ms 预览；行→review tab（deliverables:45）。
- review tab：按 Session+announcement seq+turn 寻址，文件选择器、unified/side-by-side、换行、打开原文件/原生打开；5000 行截断（deliverables:49）。
- inline-code 链接：精确路径/唯一参数别名/唯一 basename；歧义保持惰性（deliverables:57）。
- Coding Tools 关闭时卡与 summary 立即消失，但 deliveries、inline links、记录的 workspace changes 仍在（deliverables:43）。

### 3.9 附件与反馈

- 附件引用由 Session 授权、图片 URL 缓存按 Session 绑定并在释放时 revoke（conversation:14,65）。
- 图片位置四选一：`conversation.chat.user.images`、`conversation.chat.message.images`、`conversation.trajectory.images`、`tool.call.images`，共享 `conversation.images.content` Factory（attachment:52）。
- 反馈：每 Session 一个 MessageFeedbackController + 一个 DialogController；list 首次 hover/focus 才读；mutation 串行并带 version；`version-conflict` 用回复中的权威 item 对账（feedback:44）。评分/分类/备注是 log-only Session 事件，不进模型上下文（feedback:12）。

## 4. 给 ZeoKeel 的模块拆分建议

### 4.1 先守住的约束

- shell 只认 registry 的 slot/component，不 import 业务包（`docs/27:44`）。
- UI 包之间只依赖 `@zeokeel/ui-kit`；共享视觉/交互进 ui-kit，禁止复制（`docs/27:46-62`；C17）。
- `@assistant-ui/*` 只允许出现在 `ui-chat`（`docs/27:61`）。
- **Slot 口径存在文档冲突**：`docs/19:61`/`docs/16:60-64` 仍写 M1 只有 `ui.sessions`/`ui.chat`/`ui.settings`；`docs/28:37-42` 已新增 `ui.shell`/`ui.inspector`/`ui.commands`。按 C12 先裁决再扩槽；新增 slot 必须同改 `tools/zk` 的 slot_binding。
- 审批完整流程 post-M1、M1 是占位卡（`docs/19:34`）；但 `docs/12:76` 把审批列入 M1 工具链，口径需统一。

### 4.2 DSH 模块 → ZeoKeel 目标形态与优先级

| DSH 模块 | ZeoKeel 目标 | M1 做法 | post-M1 触发 |
| --- | --- | --- | --- |
| conversation + chat | `@zeokeel/ui-chat`（单 slot `ui.chat`） | 包内拆 `conversation/` 与 `chat/`；保持 `chat.thread` 导出 | T21 keyed/chain slot 成熟后再谈独立包 |
| composer（含 queue/steer/trigger） | `ui-composer`（新 slot 候选 `ui.composer`） | 先做 `composer/` 目录 + 纯 reducer/attempt 类型 | 需要独立替换输入区或第二消费面时 |
| tool | `ui-tools`（`ui.toolcall`，见 `docs/19:47`） | 包内 `tool/` + 按 wire name 的卡片注册表 | 工具卡超过 3 个团队并行维护时拆包 |
| approval | `ui-approval`（`docs/19:48`） | 占位卡 + 完整 DTO（决策/超时/范围留字段） | M2 完整审批流程 |
| plan | `ui-plan` | 现状 docs/28 要步骤卡（与 DSH 不同，见 §4.6）；DSH 接缝 = plan mode + 文档审阅 | 产品确认计划模型后 |
| goal | `ui-goal` | 不做 | 长任务/自动续跑进入范围后 |
| jobs | `ui-jobs` | 不做 | 后台/bash 长任务进入范围后 |
| subagent | `ui-subagent` | 不做 | 子代理能力进入范围后 |
| deliverables | `ui-deliverables` | 可选：先做"本轮改动文件"轻量卡 | git summary / review tab 需求出现后 |
| input-trigger | `ui-input-trigger` | 只做静态命令列表 + 字面 `@` 插入（`docs/28:59`）；不做 DSH source/adjudication 流水线 | 需要可扩展 source/drill/drilldown 时 |
| attachment | `ui-attachment` | 只做入口/占位（`docs/28:59`）；上传状态机与六态后置 | 草稿 rail/lightbox 需求出现后 |
| message-feedback | `ui-message-feedback` | 不做 | 反馈后端落地后 |

### 4.3 M1 先做哪些（不改 slot，只改包内结构）

1. **`conversation/`**：消息 node 判别联合（user/assistant/tool/error/command 最小集）、Turn 生命周期、流式 partial、六态渲染（`docs/27:37`）。对应 DSH 的 `ui-chat` 节点层，不引入 group/折叠/rail。
2. **`composer/`**：草稿、附件 id、`phase`（plain/submitting）、发送尝试快照、失败恢复；**M1 不做** queue/steer（无运行中 inbox 时不需要），但把 `deliveryMode` 字段留进 attempt 类型。
3. **`tool/`**：`ToolCardRegistry`（key = wire tool name，未注册回落通用卡）+ `ToolCallView` 判别联合（preparing/start/result/error）；卡片只用 ui-kit primitives。
4. **`approval/`**：M1 占位卡，但数据模型按 `docs/19:91-105` 预留 `risk/scope/timeout/decision`；决策值至少 `allow-once/reject`，另两个（session/tool/path remember）标 post-M1，不与 DSH 冲突。
5. **共享 DTO 上移 `ui-kit`**：`MessagePart`、`ToolCallView`、`ApprovalView`、`MessageStatus`、`TurnSummary`；避免 ui-chat 内部循环依赖，也避免未来拆包时复制类型（C17）。现有 `ui-kit/types.ts:73-100` 的 `ToolCallView/ApprovalView/ChatMessage` 是起点，但需补 phase 与错误结构。

### 4.4 与现有 `@zeokeel/ui-chat` 的关系

- **对话域 M1 不新建包**：`@zeokeel/ui-chat` 仍是 `ui.chat` 的唯一实现，导出保持 `components = { "chat.thread": ChatView }`（`packages/ui/ui-chat/web/src/index.tsx:68-70`），`runtime.ts` 继续做 assistant-ui ExternalStore 适配（`packages/ui/ui-chat/web/src/runtime.ts:1-5`）。
- **只做"包内模块化"**：新增目录不新增 slot、不改 `zk.toml`、不改 registry；目录边界用 ESLint/import 约束（M1 可先靠 review，T21 再上 lint-boundaries，`docs/16:105`）。
- **不照搬 DSH 的声明合并**：DSH 用 `SlotMap`/`ChatNodeDataMap` 声明合并扩展；ZeoKeel M1 用显式判别联合 + ui-kit guard（`docs/16:84`，C10 M1 过渡例外 ADR-059）。拆包时再评估 keyed slot。
- **不要把 approval/plan 塞进 chat 组件内部**：即使 M1 同包，也走"composer 之上的独立组件 + props"，为未来 `conversation.composer` 式 takeover 留位。

### 4.5 后置顺序与判据

1. M2：`ui-approval`（安全关键，`docs/19:91-112` 的字段/超时/通知必须先文档评审）。
2. M2+：`ui-deliverables`（先只做 changed-files；review tab 依赖文件对比能力）。
3. post-M1：`ui-plan` 或 `ui-goal` 二选一先做（取决于产品是"先审计划"还是"先跑长目标"）。
4. 最后：`ui-jobs`/`ui-subagent`（都依赖后台任务/子代理运行时）、`ui-input-trigger`、`ui-message-feedback`。
5. 每拆一个包都需：新 slot 或 keyed contribution + `tools/zk` 绑定 + `docs/16/19/27` 更新 + 六态与 axe 证据（`docs/27:94-95`；`docs/19:126`）。

### 4.6 与 docs/28 / 当前骨架的对齐与差异（只读观察）

- **对齐**：`docs/28:10-12` 定 chat/tool/approval/plan/goal 在 M1 合并为 `ui-chat`；composer 并入（`docs/28:12`）；`ui.toolcall`/`ui.approval`/`ui.plan` 只预留不建包（`docs/28:46`）；jobs/subagent/deliverables 后置（`docs/28:14-15,29`）。与 §4.3/4.4 一致。
- **差异 1（计划卡）**：`docs/28:58` 与 `apps/desktop/web/src/ux/modules/Conversation.tsx:25-39` 用 `PlanStep{label,status:done|running|pending}` 步骤卡；DSH 的 plan 只有 `SubmittedPlan{markdown,title,callId}`（plan `plan.d.ts:10`），步骤属于 Todo。两者不能共用同一数据源。
- **差异 2（审批位置）**：DSH 是 composer takeover（approval:11；conversation:120-163）；骨架放在 message 内（`Conversation.tsx:41-56`，`MessageView.approval`）。takeover 需要 pending interaction + 键盘锁 + 焦点归还；inline 需要消息携带审批。契约不同，不能只改样式。
- **差异 3（工具状态）**：骨架 `running|done|error`（`apps/desktop/web/src/ux/types.ts:15-23`）；DSH 有 `preparing/start/result`，并有 cancelled/interrupted 语义（records:284-295；tool:30）。建议 M1 至少补 `preparing`、`cancelled`，否则流式参数与取消态无法表达。
- **差异 4（决策命名）**：骨架 `approved|rejected`（`types.ts:29-35`），DSH `allowed-once|rejected`（approval `slots.d.ts:48`）。后者为未来"始终允许"留出命名空间；现在用 `approved` 会把 once/always 混为一谈。
- **差异 5（消息节点）**：骨架只有 user/assistant + tools/plan/approval（`types.ts:36-44`）；DSH 还有 steering/context/command/turn-error/turn-max-tokens/compaction（records:89-256）。M1 可裁剪，但 reducer 应按 `kind` 判别并预留未知 kind 的忽略路径。
- **差异 6（composer）**：骨架 props 仅 value/streaming/disabled（`apps/desktop/web/src/ux/modules/Composer.tsx:5-12`）；DSH 有 attempt 快照、delivery mode、echo 与失败恢复（conversation:103-107）。M1 可不做 queue/steer，但 props 宜预留 `deliveryMode`/`attachments`。

## 5. 待确认问题

1. **Plan 数据源**：`docs/28:58` 与骨架要"步骤状态 + 进度 + 自动继续"，DSH 只有 plan-mode 的 Markdown 审阅（plan:36；plan `plan.d.ts:10`）。M1 计划卡绑定哪个契约？若采用步骤，是否与 Todo 的 `TodoItem` 合并，避免两套步骤状态？
2. **审批形态与策略**：DSH 是 composer takeover + `allowed-once/rejected`（approval:11,36）；骨架是消息内 inline + `approved/rejected`（`Conversation.tsx:41-56`）。M1 选哪种？`docs/19:103-105` 的超时/延长/始终允许是否进 M1（会影响"弱管控"表述与审计口径）？
3. **状态词汇**：M1 是否补 `preparing`/`cancelled` 工具态，并把审批决策改为 `allowed-once`（为 always 预留）？这决定 `ui-kit` DTO、reducer 与 golden traces，后改代价高。
4. **后置模块入口**：`docs/28:14-15,29` 说 jobs/subagent/deliverables 不建包；M1 是"完全隐藏"还是"灰显入口 + 空态"？灰显会引入不可用控件，需与 `docs/19` 的六态/可达性口径一致。
5. **M1 composer 契约与 slot 冻结**：是否现在预留 `deliveryMode`/`attachments`/`queue` 字段（即使不实现），以及 `ui.toolcall`/`ui.approval`/`ui.plan` 何时从"预留"变"创建"？`docs/28:37-42` 新增的 `ui.shell`/`ui.inspector`/`ui.commands` 与 `docs/19:61`/`docs/16` 的三 slot 口径冲突，需先裁决；再与 T21 的 keyed/chain slot 设计一起冻结，避免二次破坏性改型。
