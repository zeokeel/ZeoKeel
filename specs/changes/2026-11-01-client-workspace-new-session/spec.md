# 2026-11-01-client-workspace-new-session · 行为规格

> 只写“系统必须怎样”，不写实现选型；实现见 `design.md`。
> 状态：`draft`（proposal §7 D1–D4 **已定（Lead 2026-11-01）**，待独立评审）。

## 1. 需求

### R1 · Workspace 行 scoped「新建会话」

- **WHEN** 侧栏有 ≥1 个 Workspace，用户 hover 或键盘 focus 到任一 Workspace 行
- **THEN** 该行显示「新建会话」按钮（**不 hover-only**：`focus-visible` 可见；active 行常显；鼠标 hover 其它行时可见）；按钮 `aria-label` 含 workspace 名（`在 <workspace> 新建会话`，i18n zh/en）
- **AND** 点击/Enter/Space 在**该 Workspace**调用 `session.create{workspace_id}`，成功后选中该 Workspace + 新会话；不经过 Workspace picker；不改变其它 Workspace 的会话
- **AND** 点击行内按钮不触发整行选中/折叠（事件不冒泡到行切换）；按钮有独立 focus ring；行仍可点击选中
- **AND** 创建中该按钮 `aria-busy`/disabled 防重复；创建失败显示可读错误（沿用 toast/行内），不选中错误 workspace、不创建半成品；失败后可重试
- **AND** 流式中按钮禁用（R3）；无 Workspace 时不渲染 Workspace 行/按钮；不允许 blank session；无 workspace 的建会话请求仍返回 `session/workspace-required`（C18）

### R2 · active 行与全局入口（三入口统一 streaming guard）

- **WHEN** Workspace 行处于 active 状态
- **THEN** 该行 scoped 按钮常显（无需 hover）；行 active 样式/Aria 不变
- **AND** 三个新建入口 **行为统一**：scoped 行按钮、侧栏顶部「新建会话」/header `+`、⌘K `new-conversation`
  - 都走**同一个 `createConversation` 入口 guard**（不各自实现 streaming 判断）；
  - 非流式：scoped 用该行 workspace；全局/⌘K 用 active workspace；active Workspace 为空时禁用/提示 `workspaceRequired`；
  - **streaming=true 时三入口全部禁用**（按钮 `disabled`/`aria-disabled`），并显示同一 toast/aria 文案「当前正在生成，停止后可新建」；⌘K 项 disabled 且可见原因；不建会话、不切换、不串消息。
- **AND** 新建后列表插入目标 Workspace 下并选中；滚动/展开该 Workspace（若原折叠）；active 行高亮

### R3 · 流式中规则（Lead 2026-11-01 修订）

- **WHEN** 当前会话正在流式（`streaming=true`，任意 Workspace）
- **THEN** **三入口**（所有 Workspace 行 scoped `+`、侧栏顶部/header `+`、⌘K `new-conversation`）全部禁用，使用同一文案「当前正在生成，停止后可新建」（按钮 tooltip/aria、⌘K disabledReason、入口 guard toast）；点击/Enter 不创建、不发送、不切换 workspace
- **AND** **停止/取消当前流后**三入口恢复可用；恢复后按 R1/R2 正常创建
- **AND** 流式期间**不**切换 `activeConversationId`、不清空/不追加当前消息、不取消当前 turn；不引入 per-conversation 流状态（ZeoKeel 当前单流；DSH 有 per-session 后台流，差异记录，另 spec 再实现）
- **AND** 删除原有「同 workspace 允许 / 跨 workspace 确认 Modal」路径；无任何流式新建确认弹窗；**不再声称“全局新建行为不变”**——全局/⌘K 也受同一 streaming guard

### R4 · 失败/边界

- **WHEN** `session.create` 失败（workspace 不可用/磁盘/权限/契约异常）
- **THEN** 显示可读错误（不直出内部码）；当前选择/会话不变；按钮恢复可用；不创建空白/错误归属会话
- **AND** 目标 Workspace 不可用：**以 host `session.create` 返回错误为准**（可读提示、选择不变）；本地**仅对明显无效状态**（无 path/无 id）禁用按钮，不新增逐行可用性探测；绝不静默建到默认 Workspace
- **AND** 连续快速点击：只创建 1 个会话（busy/禁用）；响应乱序不选中旧结果
- **AND** 新建标题沿用现有 `conversationNewTitle` i18n；不预填用户输入；不改 `session.create` 契约

### R5 · 可访问性/一致性

- **AND** 按钮有可读名称、tooltip（含 workspace 名）；键盘 Tab 顺序在折叠/标题/`⋯` 之间合理；Esc 不误关侧栏
- **AND** axe 0 serious；focus ring 可见；`prefers-reduced-motion` 下无额外动画；i18n zh/en 无硬编码
- **AND** 不新增 slot/包/DB/契约；复用 ui-sessions 行样式/按钮/i18n；不建第二套建会话路径
- **AND** 与 DSH 的差异：本 spec 不做 brand 点击与 `Mod+N`（proposal D2/D4 推荐后置）；Workspace 行的目标选择为显式 scoped（符合 DSH §2.1 的显式 Workspace 优先级）

## 2. 边界与错误

| 场景 | 期望行为 | 错误码/提示 |
| --- | --- | --- |
| 无 Workspace | 不渲染按钮；全局新建禁用/提示 | `workspace/not-found` / `session/workspace-required` |
| Workspace 不可用/已移除 | 按钮禁用/提示；不建到默认 | `workspace/path-unavailable` / `workspace/not-found` |
| `session.create` 失败 | toast/行内可读错误；选择不变 | `session/*`（既有封闭集合） |
| 流式中任意入口 | scoped/global/⌘K 三入口统一禁用 + toast/aria；停止后恢复 | 无新错误码；UI 文案 |
| 快速重复点击 | 只建 1 个；busy 禁止重复 | — |
| 目标与 active 不同 | 创建后选中目标 Workspace | — |
| 键盘/读屏 | 名称含 workspace、focus 可见、按钮不被 `⋯`/行吞 | — |

## 3. 权限与可见性

- personal/enterprise 同一行为；只操作本机 workspace；无跨用户/跨 workspace 数据；服务端无新策略（C6/C18）。
- 企业版无登录/无 workspace 时仍禁止建会话；登录后按本机 workspace。

## 4. 数据与兼容

- 无新增表/字段/文件/契约；仍 `session.create{workspace_id,title?}`；`workspace_id` 创建后不可变。
- 旧客户端不需要新 action；新 UI 的 scoped 按钮是 additive。
- 回滚：移除按钮/回退 commit；已建会话仍归属正确 workspace，行为不变。
- 幂等/并发：按钮 busy 防重复；SC 无幂等键，重复请求由 UI 抑制；host 仍按现有校验。

## 5. 验收断言（可执行）

- [ ] 每个 Workspace 行有 scoped `+`；hover/focus 可见、active 行常显；`aria-label` 含 workspace 名；`SES2-WSNEW-01`/`-02` e2e 通过。
- [ ] 点击/Enter/Space 在目标 workspace 建会话并选中；不经过 picker；不误建到 active workspace（多 workspace 断言）。
- [ ] 行内按钮不触发整行选中/折叠；按钮独立 focus；Tab 顺序合理；axe 0 serious。
- [ ] 全局按钮/header `+`/⌘K `new-conversation` 与 scoped 一样受 streaming guard；非流式仍走 active workspace 规则；无 workspace 时不可建；C18 错误码不变。
- [ ] 流式中三入口（scoped/global/⌘K）统一禁用 + 同一文案；停止后恢复；不建会话/不切换/不串消息；无确认 Modal。
- [ ] 失败/不可用/快速重复：可读错误、选择不变、只建 1 个、可重试。
- [ ] i18n zh/en 无硬编码；docs/19 回写；无新增契约/迁移/slot；旧数据兼容。

## 6. 已定决策（Lead 2026-11-01）

- [x] **D1**：按钮 hover+focus 可见（不 hover-only）、active 行常显。
- [x] **D2**：本 spec 不做品牌点击（DSH 差异记录；后续另 spec）。
- [x] **D3**（2026-11-01 修订）：流式期间**所有 Workspace 行 `+` 禁用**（tooltip/aria「当前正在生成，停止后可新建」）；停止后恢复；不建/不切/不串消息；不引入 per-conversation 流状态（DSH per-session 流差异另 spec）。
- [x] **D4**：快捷键后置（另 spec）。
- 状态保持 `draft`，待独立评审；其余实现细节见 design/tasks。
