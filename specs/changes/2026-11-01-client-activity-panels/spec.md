# 2026-11-01-client-activity-panels · 行为规格

> 只写"系统必须怎样"，不写实现选型；实现见 `design.md`。
> 状态：`draft`（proposal §7 D1–D4 拍板 + 独立评审通过前不得改产品代码，C16/C19）。

## 1. 需求

### R1 · 右栏 activity tab（order 40）

- **WHEN** 用户打开右栏第 4 个 tab「活动」（activity；order 40，位于 files/changes/terminal 之后）
- **THEN** 展示聚合快照：jobs、subagents/agent-team、goal、todo、schedules、present deliverables；各分区独立空/加载/错误态
- **AND** 快照来源：`activity.snapshot{workspace_id, conversation_id}`；tab 打开时加载，随后 **2s 轮询** + 手动刷新按钮 + 动作后刷新；
  关闭 tab/切换会话停止轮询；旧请求结果丢弃（按会话/序列号防串）
- **AND** tab 不新增 slot/包（在 `ui-inspector` 内，docs/16 记录 order 40）；会话级 tab 状态沿用 Phase 2b 持久化；六态不白屏

### R2 · Jobs（列表/输出/停止）

- **WHEN** activity tab 打开且当前 workspace 存在后台 jobs
- **THEN** 列出 id/命令/cwd/状态（running/completed/failed:N）/启动时间/输出摘要（stdout+stderr 截断，≤4 KiB/条）；
  可按状态/命令过滤（可选简单过滤）；可展开查看该 job 输出；可停止运行中的 job
- **AND** 只展示 **cwd 位于当前 workspace 根内** 的 jobs（canonicalize 后按 **路径组件** `Path::starts_with` 判断，不能用字符串前缀放过 `/workspace-evil`；其它 workspace/无 cwd 不混入）；停止/查看未知 job 返回可读错误；
  无 jobs 显示空态「暂无后台任务」；jobs 为进程内状态，重启后为空（不伪造历史）
- **AND** 停止为显式用户动作（不是 agent 工具调用）；**停止前必须内联二次确认**（显示 job id/命令/「将终止进程」影响，默认焦点「取消」，确认后才发 `activity.action{job_kill}`；取消/ Esc 不变）；确认 UI 复用 ui-kit Button/既有确认模式，不新增第二套 Dialog（C17）；
  失败（已结束/未知）不崩溃并提示
- **AND** 确认区 `role=alertdialog` + 可读 `aria-label`（「确认停止任务」），`aria-busy` 在 action 中；键盘可达确认/取消

### R3 · Subagent / agent-team

- **WHEN** 当前会话有 subagent 记录或最近一次 `agent_team` 运行
- **THEN** 列出当前会话（parent = 当前 conversation）的 subagent：id/depth/status(running/done/failed/cancelled)/prompt（截断）；
  运行中的可「中断」；已结束的显示结果不可用/历史（不提供续聊/打开子会话）
- **AND** 展示**最近一次** `agent_team` 工具行结果（objective/status/workers[id/task/status]/completed/failed/cancelled/total，只读）；
  不改团队状态模型（不新增 roster/任务板持久化）
- **AND** 中断前**内联二次确认**（显示 subagent id/状态/「将中断该子代理」影响，默认焦点「取消」，确认后才发 action；取消/Esc 不变）；确认区 `role=alertdialog` + aria-label（「确认中断子代理」），复用 ui-kit 确认原语，不新增第二套 Dialog（C17）
- **AND** 中断未知/已结束 id 返回可读错误（例如「子代理不存在或已结束」），UI 刷新状态；空白显示对应空态

### R4 · Goal 状态

- **WHEN** 当前会话存在 `goal` 工具记录
- **THEN** 显示最近一次 goal 结果（objective/status/rounds/max_rounds/reason，来自已持久化工具行）；状态枚举与 goal 工具一致
  （active/achieved/complete/cancelled/blocked，未知值原样显示为文本并标记 unknown）
- **AND** 无 goal 显示空态；goal 数据只读（set/run/cancel 仍由 agent 的 goal 工具执行；本 spec 不新增写入面）；
  刷新/重进会话后仍可见（来自持久化行）；损坏/不可解析行跳过并提示「历史记录不可解析」

### R5 · Todo 清单

- **WHEN** 当前会话存在 `todo` 工具记录
- **THEN** 按工具语义 replay 出清单：`list` 结果替换状态、`add` 追加、`update` 改状态（保留文本）、`remove` 删除；
  展示 text/status（pending/in_progress/completed）；未知 status 原样显示并标记
- **AND** 只读（不在面板内增删改）；按工具行顺序稳定排序；legacy 行（无 args）或损坏 JSON 跳过并提示「由会话工具记录推导，可能不完整」；
  刷新/重进会话后仍可见；不新增第二套 todo 存储/状态机

### R6 · Schedule 管理（当前 workspace）

- **WHEN** activity tab 打开或用户操作 schedule 分区
- **THEN** 列出当前 workspace `.zeokeel/schedules.json` 的任务：id/prompt（截断）/cron/enabled/interval_seconds/next_run；
  支持：暂停/启用（toggle enabled）、编辑 prompt/interval_seconds、删除（二次确认）
- **AND** 所有写操作复用 `zk-tool-schedule` 的公开 Tool 调用（list/update/delete；cron/interval/文件形状校验在原实现内），
  host **不新增第二套文件解析/校验**；前端 app 只经 host IPC，不 import tool 实现（C4/C2）；
  **delete 先经 host `list` 预检 id 存在**（或解析 tool 输出 `deleted 0`），不存在 → `activity/schedule-invalid`；文件损坏/不存在显示空态或可读错误；不提供新建
- **AND** 写操作只作用于当前 workspace；成功后刷新列表；非法 id/interval(<60)/**prompt >32 KiB（host 预校验，tool-schedule 本身无长度校验）** 拒绝并提示；
  delete 未知 id 映射 `activity/schedule-invalid`（P1-3）；操作失败不破坏原文件（tool 的原子写语义）

### R7 · turnTail（present 交付物 + 计划展示）

- **WHEN** 一条 assistant 轮结束（status done/cancelled/error 均按现有消息状态渲染）且本轮存在 `present` 工具调用或可识别计划段落
- **THEN** 在消息尾部显示 turnTail 卡：
  - **交付物**：本轮 `present` 结果解析为 path/description（去重、按调用顺序），显示文件名/描述；点击「在右栏查看」打开右栏 files 预览
    （复用 Phase 2b `workspace.file_read`/预览；文件不存在显示可读错误，不白屏）；
  - **计划展示**：若本轮 assistant 文本含 Markdown 标题行（`#`~`######` 后接 `plan`/`计划`，大小写不敏感；中文后允许空格/冒号/行尾，
    host/UI 正则按 `^(#{1,6}\s*)?(计划|Plan)(?=\s|$|[：:])` 实现，不能用 `\b`），折叠展示该标题段落（Markdown 渲染，复用现有 MarkdownText）；
    无匹配/解析失败不显示计划卡
- **AND** 不实现 plan mode/计划批准/执行状态机/计划工具；不新增消息类型（turnTail 由现有消息内容与工具行派生）；
  无 present/计划时不显示空卡；刷新/重进会话后仍可恢复（由持久化工具行/文本派生）

### R8 · 质量与边界

- **WHEN** 任意分区在空/加载/错误/离线/流式中/已取消下
- **THEN** 分区级状态可见（不整栏白屏），可重试/刷新；轮询失败保留上次成功数据并显示 stale 提示（不反复弹错）
- **AND** 所有动作有 i18n 文案（zh/en）；tab/动作/删除确认键盘可达，axe 0 serious/critical；
  不引入运行期加载/新 slot/新状态存储（C2/C17）；不改变 agent/审批/权限语义

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 会话不属于 workspace / 无活动会话 | 快照拒绝/空态，不跨 workspace | `session/workspace-mismatch`、`session/not-found` |
| jobs 未知 id / 已结束停止 | 可读错误，刷新列表 | `activity/job-not-found`、`activity/job-finished` |
| subagent interrupt 未知/已结束 | 可读错误（「不存在或已结束」），不崩溃 | `activity/subagent-not-found` |
| schedule 文件损坏/不存在 | 空态或降级提示，不写坏文件 | `activity/schedule-unreadable` |
| schedule update/delete 未知 id / interval<60 / prompt>32 KiB / 非法 cron | 拒绝并提示（id/interval/cron 复用工具校验；prompt 上限 host 预校验） | `activity/schedule-invalid` |
| todo/goal/present 工具行损坏或 legacy 无 args | 跳过并提示「部分历史记录不可解析」 | `activity/derived-unavailable`（提示非阻塞） |
| 轮询期间会话切换/关闭 tab | 丢弃旧结果，不更新到新会话；停止轮询 | — |
| activity 快照输出过大 | 每条输出摘要≤4 KiB；条数有界（jobs≤50、subagents≤64、todo/schedules≤100） | `activity/truncated`（标记） |
| present 文件不存在/预览失败 | 卡仍显示，点击报可读错误 | `workspace/path-not-found`（既有） |
| 离线 | 本地 IPC 可能仍可用；不可用时分区错误态+重试；不白屏 | 既有离线提示 |
| 计划标题误判 | 仅严格标题匹配，匹配不到不显示；不猜测 | — |

## 3. 权限与可见性

- 角色：个人用户/企业员工均可查看/操作本会话/workspace 的 activity；无管理员/审计员专属；无服务端策略。
- 可见性：jobs 按 cwd 限定当前 workspace；subagents 按 parent conversation；goal/todo/present 按当前会话。
- 服务端是否下发策略：否（弱管控 C6）；所有动作为本地显式用户操作，不新增审批/网关语义。

## 4. 数据与兼容

- 新增 IPC（additive）：`activity.snapshot`、`activity.action`（payload/result 见 design §2.2）；不改 `chat.send`/agent/审批契约。
- 新增读面：jobs/subagent 进程内 registry；goal/todo/present 从既有持久化 `role="tool"` 行派生；schedule 复用 `.zeokeel/schedules.json`
  既有文件形状（工具所有权不变，无 schema 迁移、无数据库变更）。
- 兼容性：旧客户端不调用新命令；新面板不改消息内容/事件；legacy 工具行（无 args）降级提示；旧数据无迁移。
- 幂等：snapshot 只读；job kill/subagent interrupt 非幂等（已结束错误）；schedule toggle 幂等（目标状态）、delete 幂等（不存在返回错误或成功？design 定：不存在=可读错误）。
- 回滚：回退 commit；无数据迁移；schedule 文件由既有工具写，回滚后仍可被工具/automation 读取；无持久化新状态需清理。

## 5. 验收断言（可执行）

- [ ] 契约/host：`activity.snapshot` 字段/边界；jobs 过滤与 kill；subagent roster/interrupt；goal/todo/present 工具行派生（含损坏/legacy）；
      schedule list/update/delete 经 tool-schedule 校验；`cargo test -p zk-contract-ipc -p zk-desktop`。
- [ ] 派生单测：todo `list/add/update/remove` replay、goal 最近完整结果、present 去重、团队 JSON 解析失败降级；输出/条数截断。
- [ ] UI：activity tab（order 40）渲染各分区/空态/错误/轮询停止；jobs 输出/停止（**停止先内联确认，取消不发 action**）；subagent 中断（同样确认）；schedule toggle/edit/delete 确认；
      turnTail present 卡点击打开右栏预览；计划段落折叠/无匹配不显示（单测/手工）。
- [ ] e2e（`e2e/interaction/activity.spec.ts` + `e2e/serve-m1.sh` 剧本科；`pnpm e2e:ui` 依赖 `ui-interaction-verification`，当前 UNVERIFIED）：activity tab 打开、todo/goal/schedule/present 剧本、
      schedule 暂停/删除后文件断言、present 卡→右栏预览、job 后台任务停止、subagent interrupt；axe 0 serious。
- [ ] 门禁：`cargo test --workspace export_schema` + `git diff` 幂等 + `bash scripts/schema-check.sh`；`zk compose --check`；`zk capability --check`
      （无新增条目）；`pnpm -r typecheck/lint/test:unit/build`；`bash scripts/i18n-check.sh`。
- [ ] 兼容/回滚：无 activity 时各区空态；工具行损坏降级不白屏；回滚后 schedule/工具行无影响。

## 6. 已定决策

- [x] **D1 schedule 读写路径与数据源**：`activity.snapshot` 读聚合、`activity.action` 写动作；host 经 `zk-composed` 生成 facade 复用 `zk-tool-schedule` 公开 `Tool::call_json`（不新建第二套解析/校验；app 不直接依赖 tool）。
- [x] **D2 todo/agent-team 口径**：todo 从已持久化工具行 replay；agent-team 只展示最近 `agent_team` 工具行结果；不改工具状态模型。
- [x] **D3 plan 展示**：仅识别 assistant Markdown 计划标题段落（中文感知正则）折叠展示；不实现 plan mode/审批/状态机。
- [x] **D4 刷新策略**：activity tab 打开时 2s 轮询 + 手动刷新 + 动作后刷新，关闭即停；无新事件通道。
- 已定（Lead 2026-11-01）：activity 位置 = 右栏 tab order 40 + chat turnTail；面板数据复用现有 tool/kit，不新增第二套状态机。
