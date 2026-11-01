# 2026-11-01-client-workspace-new-session · 提案

- 状态：`approved`
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：用户 2026-11-01「新建会话要在工作区旁边也有一个新建会话的功能」；`research/dsh-ui-modules/navigation.md` §1.1/§2.1（Workspace 行/目标选择、brand、hover/focus）；`docs/26`（C18）、`docs/19` §2/§11、`docs/27` §2.5–§2.8、`docs/11` §8.6；前置：`2026-11-01-client-side-panels`（Workspace 行/会话操作）、`2026-11-01-client-command-surface`（⌘K 与 `/` 分流）
- 评审：docs/reviews/2026-11-01-client-workspace-new-session.md
- 变更预算：**8 个文件**（见 §8）

## 1. 背景 / 问题

`ui-sessions` 目前只有：侧栏顶部「新建会话」和 header 的 `+`，二者都调用 `actions.createConversation`，只能在 **active workspace** 建会话；Workspace 行只有折叠、选中、`⋯` 操作菜单，**没有“在此工作区新建会话”的 scoped affordance**。用户要求在工作区旁也能新建会话（DSH Workspace 的 scoped new-session 语义：显式选中的 Workspace → 当前会话 Workspace → 最近活跃 Workspace；ZeoKeel 必须保持 C18：无 workspace 不建会话、workspace_id 不可变、无 blank session）。

不做的后果：多工作区用户必须先点 Workspace、再点全局「新建会话」，多一步且容易建到错误 workspace；与 DSH 的每 Workspace 新建入口不一致。

## 2. 目标

- 每个 Workspace 行增加「新建会话」按钮：hover 与键盘 focus 均可见（不能 hover-only），`aria-label` 含 workspace 名；点击在**该 Workspace**创建并选中新会话，不经过 picker。
- 当前 Workspace 行有清晰 active 态；非当前行点击后切换 active workspace 并选中新会话（遵守 C18）。
- 全局「新建会话」/header `+`/⌘K 与 scoped 统一受 streaming guard（流式都禁用）；非流式保持 active workspace 规则；不破坏既有键盘路径。
- 流式中创建规则明确（推荐允许同 workspace；跨 workspace 有确认/不误取消）。
- i18n zh/en；键盘/焦点/axe；C20 交互/直觉文件；docs 回写。

## 3. 非目标

- 不新增 blank session / 不自动建默认会话 / 不把无 workspace 变成可建（C18）。
- 不新增 slot/包；不改 `session.create` 契约（仍 `{workspace_id,title?}`）；不改 `workspace_id` 不可变。
- 不做 Workspace 行新建的 picker（目标由该行显式决定）。
- 不纳入品牌 logo 点击（见 §7 D2 推荐 B）与键盘快捷键（D4 推荐后置）；可后续 spec。
- 不改会话排序/归档/fork/search 行为（side-panels 已实现）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 多 workspace 个人用户 | 在「项目 A」行悬停/聚焦，点 + | 直接在项目 A 新建并选中会话，不切错 |
| 键盘用户 | Tab 到 Workspace 行的 +，Enter | 同鼠标；焦点可见、aria 名称含 workspace |
| 企业员工 | 多个 workspace（本机） | 同个人版；无跨 workspace 泄露 |
| 流式中用户 | 当前会话流式中，在另一 workspace 行点 + | 明确是否切换/取消当前流，不静默丢流 |

## 5. 成功指标

- 每个 Workspace 行有 scoped `+`；hover 与 focus 均可见；点击/Enter 创建到该 workspace 并选中，0 次建到错误 workspace。
- **三入口（scoped/global/⌘K）统一 streaming guard**（同一 `createConversation` 入口 guard）：流式时禁用 + 同一文案「当前正在生成，停止后可新建」；非流式全局/⌘K 仍走 active workspace 规则；无 workspace 时 scoped 按钮不出现/禁用且提示可读。
- 流式中创建不产生静默取消；跨 workspace 有确认或明确提示。
- 有 P0 交互测试 + 证据；axe 0 serious；i18n zh/en。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 行内按钮过密/误点 | 中 | 24px 按钮、hover/focus 才显式、点击目标不覆盖行选中；视觉对比/axe |
| 流式中新建会清空/串当前消息（现有 reducer 无 per-conversation 流状态） | 高 | **streaming 三入口（scoped/global/⌘K）统一 guard/禁用** + 同一文案；停止后恢复；不切换 activeConversationId；补三入口/恢复/不串消息单测+e2e |
| 当前 active workspace 判定与 scoped 目标不一致 | 高 | action 显式传 workspace_id；host `session.create` 校验归属；测试 |
| 键盘 focus 不可见/aria 不含 workspace | 中 | `focus-visible` + 动态 aria-label；axe/e2e |
| 与 side-panels 会话行 `+`/`⋯` 视觉冲突 | 中 | 复用现有行高/图标按钮样式；视觉基线 |
| 无 workspace 时按钮语义 | 低 | 不渲染 Workspace 行即无按钮；空态提示先建 workspace |
| 快捷键/品牌点击范围膨胀 | 低 | 本 spec 不做（D2/D4），另 spec |

## 7. 已定决策（Lead 2026-11-01；原待拍板）

- [x] **D1 可见性**（**已定（Lead 2026-11-01）：采用 A**）：hover + 键盘 focus 均可见（**不 hover-only**）；当前 active Workspace 行常显，其它行 hover/focus 出现；触摸/无 hover 设备通过 focus/行选中后常显。
- [x] **D2 品牌 logo 点击**（**已定（Lead 2026-11-01）：采用 B，本 spec 不做**）：DSH 在 Web/Windows 上点击 expanded brand 等同 New Session；ZeoKeel 品牌区承载 OEM/窗口拖拽/无登录态，且用户本次只要求工作区旁；保留品牌为身份标识，后续如要 1:1 另 spec（差异记录）。
- [x] **D3 流式中**（**已定（Lead 2026-11-01）：采用修订 A——流式期间禁用**）：**不引入 per-conversation 流状态**；当前会话正在生成（`streaming=true`）时，**scoped 行按钮、侧栏顶部/header `+`、⌘K `new-conversation` 三入口统一禁用**（同一 guard + 同一 tooltip/aria/disabledReason「当前正在生成，停止后可新建」）；停止/取消后恢复；不切换 activeConversationId、不建会话、不取消/不串当前消息。删除原「同 workspace 允许、跨 workspace 确认 Modal」路径。**与 DSH 偏差（诚实记录）**：DSH 有 per-session 后台流，ZeoKeel 当前为单流模型；per-conversation 流状态另 spec（需 reducer/controller 状态改造，超出本 spec 预算）。
- [x] **D4 快捷键**（**已定（Lead 2026-11-01）：采用 B，后置**）：DSH 有 `Mod+N`/Web `Mod+Alt+N`；ZeoKeel 暂无快捷键注册系统，本 spec 只做按钮/键盘 focus；快捷键另 spec（含与 `/`、⌘K、输入焦点冲突）。

> D1–D4 已由 Lead 于 2026-11-01 按推荐定稿；状态仍保持 `draft`，待独立评审。

## 8. 变更预算与拆分

- 预计改动文件：**9**（实现 5 + 测试 2 + 文档 2；不含本 spec 目录/生成物）。**Lead 2026-11-01 书面批准 8 文件/约 643 行例外**；P2-3 追加 `KNOWN-LIMITATIONS` 回写 1 文件后总 9，仍 ≤10 拆分阈值：
  - `packages/ui/ui-sessions/web/src/index.tsx`（行内 + 按钮/active 态/streaming 禁用）
  - `packages/ui/ui-kit/web/src/context.tsx`（`createConversationIn(workspaceId)` action）
  - `apps/desktop/web/src/App.tsx`（注入 action；现有 createConversation 不变）
  - `apps/desktop/web/src/state/controller.ts`（按 workspace 建会话并选中；streaming 时 UI 禁用、无 cancel/确认分支）
  - `apps/desktop/web/src/i18n.tsx`（`在 {workspace} 新建会话`、streaming 禁用 tooltip 文案 zh/en）
  - `apps/desktop/web/tests/workspace.test.tsx`（追加 action/scoped 行为单测）
  - `e2e/interaction/sessions-workspace-new.spec.ts`（新；P0 ID）
  - `docs/19`（侧栏 Workspace 行 scoped new-session/键盘/流式禁用规则）
  - `KNOWN-LIMITATIONS.md`（brand 点击/`Mod+N` 后置、单流模型与 DSH per-session 流差异）
- 不新增契约/迁移/包/slot；生成物不变；docs/28 状态由统一回写任务处理。
- 是否拆分：否（单一目标；9 文件 ≤10；Lead 已书面批准行数例外）。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "createConversation|newConversation|Workspace|workspaces" packages/ui/ui-sessions apps/desktop/web`；`research/dsh-ui-modules/navigation.md` §1.1/§2.1/§2.7。
- 可复用：
  - `ui-sessions` 的 Workspace 行/折叠/`⋯` 菜单/active 样式（`packages/ui/ui-sessions/web/src/index.tsx`）。
  - 全局新建会话：`actions.createConversation` + `session.create`（`apps/desktop/web/src/state/controller.ts:342`），本 spec 只加 scoped 版本并复用同一 IPC/校验。
  - 行内 icon button 样式/aria（`⋯` 按钮、`focus-visible:opacity-100`）。
  - i18n/Toast（side-panels 既有模式）；本 spec 无确认 Modal/Dialog（流式禁用替代）。
- 本次新增可复用能力：无新 capability；`createConversationIn(workspaceId)` 是 UI action 扩展。
- 为什么不能复用/扩展：`createConversation` 固定 active workspace，无法表达 scoped 目标；新增 optional workspaceId 参数（保持旧调用行为）是同一实现的最小扩展，不建第二套建会话路径。

## 10. 文档与决策

- D1–D4 已由 Lead 2026-11-01 定稿（D3 修订为流式禁用）；状态保持 draft，待独立评审后进入实现（C16/C19）。
- 实现后回写 `docs/19`/`KNOWN-LIMITATIONS`；不需 ADR（无契约/数据/安全边界变化）。
