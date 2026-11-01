# 2026-11-01-client-command-surface · 行为规格

> 只写“系统必须怎样”，不写实现选型；实现见 `design.md`。
> 状态：`draft`；Batch 1（6 个可用命令 + 3 个 Batch 2 占位）。

## 1. 需求

### R1 · `/` 命令面（DSH 分组/键位/解析）

- **WHEN** 用户在 composer 词首输入 `/`
- **THEN** 打开命令菜单；空查询固定两节、顺序与 DSH 一致：
  - 「添加」= `file`、`goal`、`plan`、`feedback`
  - 「指令」= `compact`、`permission`、`model`、`export`
  - `/skill` 作为独立「技能」组末位（Batch 2 占位）；其它未识别候选不出现
- **AND** `/` 菜单**不再包含** `new-conversation`/`toggle-sidebar`/`open-settings`/`toggle-inspector`/`cancel`/`retry` 等 UI 动作；这些动作只在 ⌘K 命令面板保留且行为不变（两 surface 由同一 catalog 按 `surface` 过滤，唯一执行器）
- **AND** 键入非空查询按命令名/说明本地过滤（大小写不敏感）；`↑/↓` 移动高亮；`Enter`/`Tab` 接受；`Esc`/`Shift+Tab` 关闭回 composer；菜单打开时焦点留在输入框（`aria-activedescendant`）
- **AND** 命令行有类型语义：`action`（`/file`、`/feedback`）、`popupSelect`（`/model`、`/permission`）、`leadingInput`（`/goal`）、`execute`（`/export`）；Batch2 为 `disabled/unimplemented`
- **AND** 选中命令后从草稿移除触发词 `/<query>`；不把命令文本发送为普通消息；正文/其余草稿保留

### R2 · 未知/非法命令与附件边界

- **WHEN** 用户输入 `/not-a-command` 或选择 Batch2 占位
- **THEN** 不发送消息、不执行任何动作；显示 i18n 可读提示「不支持的命令 / 该命令将在后续版本提供」；草稿保留
- **AND** 任何未知/非法命令**不得**静默降级为普通提示词（DSH 语义）；输入不以 `/` 开头或 `/` 不在词首时按普通文本处理
- **AND** 只有 `/file` 可打开附件选择并接收附件；其它命令带附件时拒绝并提示「该命令不支持附件」，附件 chip/草稿原位保留（不静默丢弃）
- **AND** 流式中 Batch1 命令按能力禁用：`/model`/`/permission`/`/file` 只在非流式可用；`/goal`/`/export`/`/feedback` 可在流式只读/打开（导出内容以当前已持久化消息为准）；Batch2 永远禁用

### R3 · `/file`

- **WHEN** 用户选择 `/file`
- **THEN** 打开系统文件选择器（复用 composer 附件流程/allowlist/≤1 MiB/≤5 个/消息），不发送消息、不生成工具行
- **AND** 选择成功后 chip 正常出现；取消/失败保留草稿；流式中禁用并提示

### R4 · `/model`（会话级）

- **WHEN** 用户选择 `/model`
- **THEN** 打开 popupSelect：personal = **当前模型 + 手动输入模型名 + recentModels（localStorage，最多 8，仅成功应用过的模型）+ 推理等级**（`minimal|low|medium|high|xhigh|max`）；**不声称/不硬编码 provider 模型列表**（无模型发现数据源，诚实降级 DSH）；enterprise = **bootstrap 返回的 gatewayModels 列表**（host 缓存、UI 仅展示；不显示上游 key/base_url）；当前会话有效值初始高亮，标注「默认/会话覆盖」
- **AND** 选择只影响**当前会话后续轮次**，不写全局 settings；切会话恢复各会话值；重启按持久化 override 恢复；在 turn 开始前应用、turn 结束恢复全局 settings；失败回滚并提示，不影响已发送消息
- **AND** 应用顺序（P1-2）：desktop 在 turn 开始前获取**全局 config-mutex**，按该 conversation 的 override 应用 model/effort（`configure_model`/`configure_agent`）与 permission（`set_approval_policy` 单点），turn 结束恢复全局 settings 后释放；互斥期内其它 conversation 的 turn **排队**（不并发使用全局配置）；无 override 的 conversation 直接使用全局配置；恢复以 turn 开始时的 global settings snapshot/revision 为准，若期间 settings 已更新则恢复最新全局（不写回旧代际）
- **AND** **`settings.set` 的 apply/publish 必须获取同一 config-mutex**（不能只锁 turn）：turn 持有会话 override 期间用户保存 settings → settings.set 排队到该 turn 结束，不改变当前 turn 的 model/permission override；turn 结束后按 settings snapshot/revision 恢复/应用最新全局；并发/取消/失败均保证不串代际
- **AND** **排队 turn 的取消语义（P2-1 冻结）**：`chat.cancel` 只作用于当前正在运行的 turn；已排队等待 mutex 的 turn 在获得锁后先检查 cancelled，若已取消则**跳过执行、不产生 agent 事件/消息/错误提示**，直接结束并释放锁（不报错、不阻塞后续 turn）
- **AND** personal 用 provider 的 base_url/api_key/model/provider 调 `configure_model`；enterprise 用网关凭证 + host 缓存的 gatewayModels 中的选定值（C7），不引入上游 key；effort 用 `configure_agent(AgentSettings)`
- **AND** 校验：personal 手动模型名 trim 非空、长度≤200、不含控制字符；enterprise 模型必须在 host 缓存列表内；未知 effort/非法模型拒绝并保持旧值；显示当前值/默认；无可用输入时给空态/下一步

### R5 · `/permission`（会话级审批策略）

- **WHEN** 用户选择 `/permission`
- **THEN** popupSelect 显示三档 DSH 预设的**审批语义映射**（不是 sandbox）：
  - 「只读（保守）」→ `Strict`（所有工具调用都询问）
  - 「工作区写（需确认）」→ `AutoReadOnly`（只读工具静默，变更工具询问）
  - 「完全权限」→ `Ask`（仅按工具 `requires_approval` 询问；**不能绕过**此弱不变量）
  - 当前值为初始高亮；显示「仅审批策略，不限制工具/网络/文件」说明
- **AND** 只对当前会话后续 turn 生效；切会话恢复；重启按 override 恢复；失败回滚；不改全局 settings；不新增 sandbox
- **AND** 应用与 model 共用同一 **全局 config-mutex**：turn 前获取、把会话 policy 注入 agent 侧 `set_approval_policy` 单点（沿用 approval spec 的唯一 policy source；**不包裹/不替换 approval gate owner**、不改 `ApprovalPolicy::should_ask`），turn 结束恢复全局后释放；其它 conversation turn 排队
- **AND** UI 明确标注与 DSH 的差异（DSH 预设含沙箱；我们只有审批），不得宣称“完全权限=沙箱绕过”或“可限制员工操作”（C6）

### R6 · `/goal` 与 GoalBar

- **WHEN** 用户输入 `/goal <objective>` / `/goal` / `/goal clear` / 选择 GoalBar 的暂停/恢复
- **THEN** `/goal <objective>` 设置当前会话目标（trim、非空、≤4096 bytes；max_rounds 用默认/可选），先经**内联确认**（显示 objective 摘要）；确认后写 `.zeokeel/goal.json`（与 tool-goal 同一存储/结构）并刷新 GoalBar
- **AND** `/goal` 只查看当前状态（objective/status/rounds/max_rounds/reason），不提交消息；`/goal clear` 经内联确认后 cancel（status=cancelled，保留记录）；暂停=status `blocked`+reason「paused by user」，恢复=status `active`（复用 tool-goal 既有状态，不新增状态机）
- **AND** GoalBar 在 composer 上方（DSH 位置）显示目标摘要/状态/轮次/暂停/清除；无 goal 不渲染；goal 已 achieved/cancelled 显示只读结果；流式中不阻塞输入
- **AND** 只操作当前 workspace/conversation；不产生 agent 工具行；不修改 tool-goal 行为；host 失败返回可读错误并保留原 goal

### R7 · `/export`

- **WHEN** 用户选择 `/export`
- **THEN** 导出当前会话（Batch1 推荐 Markdown）：正文 + 工具行摘要 + usage；文件名 `<title>-<yyyyMMdd-HHmm>.md`；由 host 写当前 workspace `.zeokeel/exports/` 并返回相对路径/字节数
- **AND** 前端显示成功或可读失败；**serve 提供下载；Tauri 无 opener → 写文件后展示/复制路径，`system.open_url` 仅允许 http(s)、不承诺自动打开文件**；不新增服务端；不跨 workspace；不写日志
- **AND** 空会话/无消息允许导出（生成空结构+说明）；超过 8 MiB 拒绝并提示；磁盘/权限失败可读；流式中导出以当前已持久化消息为准并提示

### R8 · `/feedback`

- **WHEN** 用户选择 `/feedback`
- **THEN** 打开反馈弹窗：分类（bug/idea/other）+ 可选描述（≤4 KiB）；确认提交后写本地 `.zeokeel/feedback/<conversation>-<timestamp>.json`（含分类/描述/app 版本/edition/平台/时间，不含密钥/附件内容）
- **AND** UI 明示「本地保存」；企业版服务端端点不存在时不得显示已提交/已发送，标 UNVERIFIED；取消/失败保留表单可重试；不写日志
- **AND** 离线可用（本地写）；失败可读；不跨 workspace；不触发网络请求（Batch1）

### R9 · 通用约束

- **AND** 命令菜单/弹窗/GoalBar/反馈弹窗覆盖六态：空/加载/错误/离线/流式中/已取消；i18n zh/en；axe 0 serious；焦点可见并回 composer
- **AND** 新 IPC 全部走 Rust DTO + generated schema；前端运行时校验（C10）；生成物由生成器写（C11）
- **AND** 不新增 capability；复用 ui-kit/现有 composer/attachment/approval；不建第二套 Dialog/menu/执行器（C17）
- **AND** 不改 workspace 归属/默认 workspace/agent cwd（C18）；命令只作用于当前 workspace 的当前会话

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 未知 `/xxx` | 拒绝，不发送，草稿保留 | `command/unknown` |
| Batch2 占位 | disabled，不执行，提示 Batch2 | `command/unimplemented` |
| 命令带附件但非 /file | 拒绝，附件/草稿保留 | `command/attachments-unsupported` |
| `/model` personal 空模型/非法字符/超长；enterprise 模型不在 host 缓存列表；未知 effort | 拒绝，旧值不变 | `session/override-invalid` |
| `/permission` 未知策略 | 拒绝 | `session/override-invalid` |
| `/goal` 空/超 4096 bytes/无 goal clear | 拒绝/可读提示 | `goal/invalid`、`goal/not-found` |
| `/export` 超 8 MiB/写失败/无会话 | 拒绝/失败提示 | `export/too-large`、`export/failed`、`session/not-found` |
| `/feedback` 描述超 4 KiB/写失败 | 行内错误/失败提示 | `feedback/invalid`、`feedback/failed` |
| 离线/流式/已取消 | 按 R1/R2 禁用或读态；不白屏 | 既有 offline/streaming 文案 |
| 跨 workspace/不存在会话 | 拒绝，不串数据 | `session/workspace-mismatch`、`session/not-found` |

- 错误码为封闭字符串，登记 `docs/26` §5；i18n 映射，不直出内部码。

## 3. 权限与可见性

- personal：全部 Batch1 命令可用；`/model` 当前模型+手动输入+recentModels+effort（无 provider 发现）；`/feedback` 本地；`/export` workspace 内。
- enterprise：`/model` 只列 bootstrap/host 缓存的 gatewayModels（不持有上游 key，C7）；`/permission` 仍审批策略；`/feedback` 本地并标服务端 UNVERIFIED；不引入企业管控新策略（C6）。
- 服务端不下发命令策略；命令能力由客户端/版本决定；无跨用户/跨 workspace 数据。

## 4. 数据与兼容

- 新增 workspace 元数据 `.zeokeel/conversation-overrides.json`（无 DB migration，workspace 由文件位置隐含）：按 conversation 存模型/effort/审批策略；原子写、损坏降级为全局默认并提示；不含 key。
- `.zeokeel/goal.json` 保持 tool-goal 结构；`.zeokeel/exports/` 生成 Markdown；`.zeokeel/feedback/` 生成 JSON；均为 workspace 内文件，不进 git（workspace 自身规则）。
- `recentModels` 仅存 localStorage（模型名，最多 8；跨 workspace 全局列表，不上传/不进 settings）。
- 兼容：旧会话无 override 用全局默认；旧客户端不发新命令；IPC additive；`CommandDescriptor` 新增可选字段 `surface`/`kind`/`disabledReason` 不破坏 ⌘K。
- 回滚：删除新文件/回退 commit；全局 settings/会话/消息不变；已生成导出/反馈文件保留（孤儿，记录 KNOWN）。
- 幂等：override 覆盖写；goal set 重置 round；export 每次新文件；feedback 每次新文件。

## 5. 验收断言（可执行）

- [ ] `/` 空查询分组 = 添加(file,goal,plan,feedback) / 指令(compact,permission,model,export) / 技能(skill)；6 个 Batch1 可用，3 个 Batch2 disabled 且 `command/unimplemented`；未知命令不发送。
- [ ] `/` 中无 UI 动作；⌘K 仍有 new-conversation/toggle-sidebar/open-settings/toggle-inspector/cancel/retry（enterprise +3）且行为不变。
- [ ] `/file` 打开选择器、成功/取消/失败、附件限制复用；其它命令带附件拒绝。
- [ ] `/model` personal/enterprise 列表与当前值、会话级生效/切回/重启恢复/失败回滚；不写全局 settings。
- [ ] `/permission` 三档映射与当前值、会话级恢复、失败回滚；UI 标注非沙箱/不限制员工。
- [ ] `/goal` set/view/clear/pause/resume + GoalBar + 同一 `.zeokeel/goal.json`；不产生工具行；非法/超限拒绝。
- [ ] `/export` 成功（内容/文件名/上限/空会话）与失败；workspace 内；Tauri/serve 都有下载/路径提示。
- [ ] `/feedback` 弹窗/分类/描述上限/本地保存/取消/失败；不假成功；无网络请求。
- [ ] 六态/键盘/焦点/axe/i18n；错误码登记 `docs/26`；`zk compose --check`/schema-check 绿；旧数据/旧命令兼容。
- [ ] P0/P1 交互 ID 有 e2e 标签与证据（`e2e/interaction/command-surface.spec.ts`）；用户测试未跑标 UNVERIFIED。

## 6. 未决问题

- [ ] D1–D6 见 proposal §7；默认按推荐 A；未拍板前 draft。
- [ ] `/export` Batch1 Markdown vs DSH ZIP：推荐 Markdown，ZIP 记 KNOWN/Batch2。
- [ ] `/goal` 暂停用 `blocked+reason` 的 UI 映射是否可接受（复用 tool-goal 状态，不新增状态机）。
