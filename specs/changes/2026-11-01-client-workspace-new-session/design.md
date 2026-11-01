# 2026-11-01-client-workspace-new-session · 设计

> 实现选型与取舍；行为见 `spec.md`。状态 `draft`，按 proposal §7 **已定（Lead 2026-11-01）** 的 A/B/**修订 D3（流式禁用）**/B 设计。

## 0. 决策摘要

| 决策 | 选择 | 理由 |
| --- | --- | --- |
| D1 可见性（已定） | **hover + 键盘 focus 可见；active 行常显** | 满足“不能 hover-only”；侧栏密度/视觉噪声可控；active 行用户最可能新建 |
| D2 品牌点击（已定） | **本 spec 不做** | 用户只要求 workspace 旁；品牌区承载 OEM/窗口拖动/无登录态；DSH 差异记录 |
| D3 流式（已定，2026-11-01 修订） | **scoped/global/⌘K 三入口在 streaming 时统一禁用**；停止后恢复；不引入 per-conversation 流状态 | 现有 reducer 无 per-conversation 流缓冲，新建会清空/串当前消息；禁用最小可行；DSH per-session 流差异记录、另 spec |
| D4 快捷键（已定） | **后置** | ZeoKeel 无快捷键注册系统；避免与输入/⌘K 冲突；另 spec |

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | `@zeokeel/ui-sessions`（Workspace 行按钮）、`@zeokeel/ui-kit`（UiActions 扩展）、`apps/desktop/web`（controller/i18n/App） |
| 契约 | 不改 `session.create`；不新增 IPC/DB/文件；`UiActions.createConversationIn(workspaceId)` additive |
| 路由 | 无 |
| 生成物 | 无（UI action 类型非生成契约） |
| 文档 | `docs/19` §2（侧栏 Workspace 行新建/流式/键盘）；无需 docs/16/26/28（无 slot/契约/数据变化；status 回写由统一任务处理） |
| 兼容 | `createConversation()` 非流式行为不变；**streaming 时三入口统一拒绝**；无 workspace 不建会话（C18）；无 blank session |

## 2. 方案

### 2.1 Workspace 行结构

现有行（`packages/ui/ui-sessions/web/src/index.tsx`）结构：
`折叠 | 文件夹+标题(+默认徽标) | ⋯ 操作`。在标题与 `⋯` 之间插入 **scoped 新建按钮**（复用与 `⋯` 相同的 icon button 类/24px 点击区）：

```tsx
<div role="group" aria-label={`${workspace.title}`} className="group relative flex ...">
  {/* 折叠/标题保持 */}
  <button
    type="button"
    aria-label={t("workspaceNewConversationAria", { workspace: workspace.title })}
    title={t("workspaceNewConversation", { workspace: workspace.title })}
    aria-busy={busyWorkspaceId === workspace.id}
    aria-disabled={streaming || busyWorkspaceId !== null}
    disabled={streaming || busyWorkspaceId !== null || workspaceObviouslyInvalid}
    title={streaming ? t("workspaceNewStreamingDisabled") : t("workspaceNewConversation", { workspace: workspace.title })}
    className={cn(
      "inline-flex h-5 w-5 ... focus-visible:opacity-100",
      workspaceActive ? "opacity-100" : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100",
    )}
    onClick={(event) => {
      event.stopPropagation(); // 不触发整行选中/折叠
      requestCreate(workspace);
    }}
  >
    <IconPlus />
  </button>
  {/* ⋯ 保持 */}
</div>
```

- 可见性：`group-hover` + `group-focus-within` + active 行常显；`focus-visible` 强制可见。**不 hover-only**。
- 点击/Enter/Space → `requestCreate(workspace)`：
  - **`streaming === true` 时所有行直接禁用**（R3）：不打开 Modal、不发 IPC、不切 workspace；tooltip/aria 文案「当前正在生成，停止后可新建」；
  - 非流式：直接 `actions.createConversationIn(workspace.id)`；创建中 `busyWorkspaceId` 防重复；成功后清 busy；
  - 失败：`showToast(friendlyError)`（controller 内已有模式），选择/active 不变。
- 行点击仍由标题按钮 `selectWorkspace`；scoped 按钮 `stopPropagation`；不改变行语义。
- 三入口统一 guard：scoped 行按钮、侧栏顶部/header `+`、⌘K `new-conversation` 都依赖 `UiValue.streaming` 与同一 `createConversation*` guard；streaming 时按钮 disabled + 同一 i18n toast/aria，⌘K `CommandDescriptor.disabled/disabledReason`；不允许任何一个入口绕过。
- active 行：`workspaceActive` 时按钮常显；`aria-current`/高亮保持现状。

### 2.2 UiActions / controller

`packages/ui/ui-kit/web/src/context.tsx`（`UiActions`）新增：

```ts
createConversationIn: (workspaceId: string) => void;
```

`apps/desktop/web/src/state/controller.ts` 新增 `createConversationIn(workspaceId)`，复用 `createConversation` 的校验/IPC/parse/toast：

```text
if workspaceId empty / not in state.workspaces -> toast workspaceRequired|not found; return
title = translate(conversationNewTitle)
session.create{workspace_id: workspaceId, title}
  -> parseConversation; invalid -> throw
  -> if workspaceId !== activeWorkspaceId:
       // streaming 时 UI 已禁用，不会走到这里
       dispatch workspace_selected{workspaceId}
       refreshConversations(workspaceId)
  -> dispatch conversation_created{conversation}             // activeConversationId=新会话
  -> toast? 不额外
catch -> toast friendlyError; 不 dispatch
```

- `createConversation()`（全局/header/⌘K 共用）也必须在**入口加 streaming guard**：`state.streaming === true` 时 `showToast(workspaceNewStreamingDisabled)` 并 return；不调用 `session.create`、不改 activeConversationId；scoped 的 `requestCreate` 同样先走该 guard（UI 禁用后仍二次守卫）。两个 action 共用同一 guard helper，不各自实现。
- 旧 `createConversation()` 的非流式 active workspace 行为保持不变；streaming 行为改为统一拒绝（不再声称“全局不变”）。
- 并发：controller 仅检查 `session.create` 响应；UI 的 `busyWorkspaceId` 唯一防重复；streaming 时按钮禁用（无确认流程）。
- **流式中三入口均不允许创建**：UI 禁用 + controller 对 `createConversation`/`createConversationIn` 二次 guard；不切换 activeConversationId、不 cancel 当前 turn、不串消息。
- 无 per-conversation 流状态：不实现 `streamingByConversation`/消息路由；DSH per-session 后台流差异见 proposal D3，后续 spec 处理。
- `App.tsx` 注入 `createConversationIn: controller.createConversationIn`；`UiActions` 其余不变。
- 全局按钮/header `+` 仍调 `createConversation`；⌘K `new-conversation` 仍走全局。

### 2.3 无 Workspace / 不可用

- 无 Workspace 时列表为空，不渲染行/按钮；全局按钮 disabled + `workspaceRequired`（现状）。
- Workspace 不可用：**以 host `session.create` 的错误为准**（可读提示、选择不变）；本地仅对**明显无效**（空 path/空 id）禁用按钮；不新增逐行可用性探测、不静默落到默认 workspace。

### 2.4 标题/滚动/折叠

- 新会话标题沿用 `conversationNewTitle`（i18n），不预填用户输入。
- `conversation_created` 后该 workspace 若折叠：展开（`setCollapsed` 移除 id）并 `scrollIntoView`（`useEffect`/ref 或 `requestAnimationFrame`，遵循 reduced-motion）。
- active workspace 行保持高亮；新会话行选中；不改变其它 workspace 的 `conversationsByWorkspace`。
- 若目标 workspace != active，`workspace_selected` 会触发既有 reducer 清空/切换消息状态；新会话消息为空，符合预期。

### 2.5 键盘/焦点/axe

- 行内顺序：折叠 → 标题 → **新建** → `⋯`；按钮 `aria-label` 含 workspace 名；`title` 同样含名。
- 按钮 focus-visible 可见；Tab 可达；Enter/Space 触发；流式禁用时不可执行；无确认 Modal（流式禁用替代）。
- 行本身仍可点击；按钮点击不冒泡；焦点不会因 hover 显示消失（group-focus-within 保持）。
- axe 0 serious；无新增 raw color（token）。

### 2.6 i18n

新增 zh/en 文案（`apps/desktop/web/src/i18n.tsx`）：
- `workspaceNewConversation`: 「在 {workspace} 新建会话」/“New conversation in {workspace}”
- `workspaceNewConversationAria`: 同
- 复用 `conversationNewTitle`、`workspaceRequired`、现有错误翻译；不新增硬编码。

### 2.7 与 DSH 的差异

- DSH：brand click=New Session（Web/Windows）、`Mod+N`；每个 Workspace 行/目标优先级见 §2.1。
- 本 spec：Workspace 行 scoped 按钮 + 三入口统一 streaming guard；brand 点击与快捷键按 D2/D4 后置；目标显式=该行 workspace（符合 DSH“显式选中的 Workspace”优先级）；**流式期间三入口禁用（D3 修订）**——DSH 有 per-session 后台流，ZeoKeel 当前单流模型，后续 per-conversation 流另 spec。

## 3. 备选与取舍

- 只有 hover 显示：违反任务“不能 hover-only/键盘 focus”，拒绝。
- 全部常显：密度高，active 行常显+hover/focus 已满足可发现性；拒绝。
- 新增 Picker：多一步、与 scoped 语义冲突；拒绝。
- 流式允许/确认方案：现有 reducer 无 per-conversation 流缓冲，同 workspace 新建会清空/串消息；在 8/9 文件预算内无法安全实现，故 Lead 2026-11-01 修订为流式禁用，per-session 流另 spec。
- 品牌点击/快捷键：成本/副作用大，后置。

## 4. 安全与隐私

- 无新增数据/IPC/密钥；只在当前会话/workspace 内；`session.create` 既有归属校验；不跨 workspace 读。
- 流式期间禁用新建，避免清空/串当前消息；停止后可建（无需确认 Modal）。
- 不新增日志；i18n 无用户数据。

## 5. 生成物与门禁

- 无契约/生成物变更；`zk compose --check` 不受影响。
- `pnpm -r typecheck/lint/test:unit/build`；`pnpm e2e`；`pnpm e2e:ui --grep "workspace-new"`；`pnpm e2e:ui:gate`（工具已落地，`d0b7741`）。
- `spec-check`/`docs-check`。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T1 UI 单测 | 行按钮 render/aria/active 常显/hover-focus class；点击 stopPropagation；busy 防重复；**streaming 时 scoped/global/⌘K 全禁用/不触发 action/同一 aria 文案；停止后恢复**；失败 toast | `pnpm --dir packages/ui/ui-sessions/web test:unit`（若包测试）；`pnpm --dir apps/desktop/web test:unit` |
| T1 app 单测 | `createConversationIn` 目标 workspace 调用/成功 dispatch/失败不改状态；**streaming 时 scoped/global/⌘K 三入口均被 guard 拒绝（toast/无 IPC）**；停止后恢复；新建不串当前消息 | `apps/desktop/web/tests/workspace.test.tsx` |
| T3 e2e | 多 workspace：hover/focus 显示；点击建到目标并选中；键盘 Enter；**streaming 三入口（scoped/global/⌘K）禁用 + 同一文案；停止后恢复；不串消息**；失败可见错误；axe | `e2e/m1-serve.spec.ts` 或交互 `e2e/interaction/sessions-workspace-new.spec.ts` + 证据 |
| 手测 | hover/focus/active、Tab 顺序、窄窗/折叠、reduced-motion | 截图/axe |

## 7. 文档更新

- [ ] `docs/19` §2：侧栏 Workspace 行 scoped 新建、hover/focus/active 可见性、**流式期间禁用所有行 + 停止后恢复**、无 workspace 边界；DSH per-session 流差异。
- [ ] 无需 docs/16/26/28（无 slot/契约/数据变化）；状态由统一回写处理。
- [ ] `KNOWN-LIMITATIONS`：brand 点击/`Mod+N` 后置、**流式期间禁用 Workspace 新建（单流模型；DSH per-session 流差异）**。

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 误点/行点击冲突 | 中 | 中 | stopPropagation/24px/focus-visible；单测 | UI/e2e |
| 流式新建清空/串消息 | 中 | 高 | **streaming 时三入口统一 guard/禁用** + 同一 tooltip/aria/disabledReason；停止后恢复；不切 activeConversationId；单测/e2e | 单测/e2e |
| 多 workspace active 状态错 | 中 | 高 | 显式 workspace_id；dispatch workspace_selected 后 created；失败不切 | app 单测 |
| hover-only 无障碍回归 | 中 | 中 | focus-within/focus-visible；axe | e2e |
| 与 side-panels `⋯`/行样式冲突 | 中 | 低 | 复用现有样式；视觉基线 | 手测 |
