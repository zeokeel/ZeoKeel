# 2026-11-01-client-workspace-new-session · 任务清单

> 每个任务必须能独立提交、独立验证；没有 verify 的任务不算任务。
> 状态：`draft`（proposal §7 D1–D4 **已定（Lead 2026-11-01）**：A/B/**修订 D3 流式禁用**/B；未独立评审/未批准前不得改产品代码，C16/C19）。
> 前置：`2026-11-01-client-side-panels`（Workspace 行/会话操作）已实现；本 spec 只加 scoped 新建，不改其行为。

## 0. 前置：文档口径（C12）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档先行：`docs/19` §2 记录 Workspace 行 scoped 新建（hover/focus/active 可见）、**streaming 三入口（scoped/global/⌘K）统一禁用/停止后恢复**、无 workspace/不可用边界；`KNOWN-LIMITATIONS` 记录 brand 点击/`Mod+N` 后置与单流模型 vs DSH per-session 流差异（P2-3） | `docs/19-client-ux-flows.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py` | ☑ |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | ui-kit：`UiActions.createConversationIn(workspaceId)` additive；旧 `createConversation` 不变；i18n 键（行 aria、streaming 禁用 tooltip「当前正在生成，停止后可新建」） | `packages/ui/ui-kit/web/src/context.tsx`、`apps/desktop/web/src/i18n.tsx` | 0 | `pnpm --dir packages/ui/ui-kit/web typecheck && pnpm --dir packages/ui/ui-kit/web lint`；`bash scripts/i18n-check.sh` | ☑ |
| 2 | ui-sessions Workspace 行：在标题与 `⋯` 间加 scoped `+`；hover/focus-within/active 常显；`aria-label` 含 workspace；点击 `stopPropagation`；busy 防重复；**streaming 时所有行禁用 + tooltip/aria；停止后恢复；无确认 Modal** | `packages/ui/ui-sessions/web/src/index.tsx` | 1 | `pnpm --dir packages/ui/ui-sessions/web typecheck && pnpm --dir packages/ui/ui-sessions/web lint && pnpm --dir packages/ui/ui-sessions/web test:unit` | ☑ |
| 3 | controller/app：`createConversationIn(workspaceId)` 复用 `session.create`/parse/toast；**在 `createConversation`/`createConversationIn` 共用入口加 streaming guard（toast+return，不调用 IPC/不切）**；目标 != active 时先创建成功后 `workspace_selected`+`conversation_created`；App 的侧栏顶部/header `+` 与 ⌘K `new-conversation` 也受同一 guard（按钮 disabled / command disabledReason）；不切换 activeConversationId/不串消息；不可用以 host 错误为准（本地仅空 path/id 禁用） | `apps/desktop/web/src/state/controller.ts`、`apps/desktop/web/src/App.tsx` | 1,2 | `pnpm --dir apps/desktop/web typecheck && pnpm --dir apps/desktop/web lint`；`pnpm --dir apps/desktop/web test:unit` | ☑ |
| 4 | app 单测（追加）：目标 workspace 调用/成功选中/失败不改状态；**streaming 时 scoped/global/⌘K 三入口均被 guard 拒绝（toast/无 IPC）、停止后恢复；新会话不串当前消息**；重复点击只 1 次 | `apps/desktop/web/tests/workspace.test.tsx` | 3 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 5 | 交互契约 + e2e：`SES2-WSNEW-01/02` P0 + `-03` P1；`e2e/interaction/sessions-workspace-new.spec.ts`（新）覆盖 hover/focus 可见、点击/Enter 建到目标、active 常显、无 workspace、**streaming 三入口禁用/停止恢复/不串消息**、失败、axe；证据 `target/evidence/2026-11-01-client-workspace-new-session/interaction/<ID>.json` | `e2e/interaction/sessions-workspace-new.spec.ts`（新） | 2,3 | `pnpm e2e:ui --grep "workspace-new"`（工具已落地，`d0b7741`）；`pnpm e2e:ui:gate` | ☑ |
| 6 | 收尾：`interaction.md`/`intuition.md` 实现开始时一起改 `active`；acceptance 填证据；门禁全量；用户测试未跑标 UNVERIFIED | 本 spec `acceptance.md`（只写本文件） | 1–5 | `python3 scripts/spec-check.py`；`python3 scripts/docs-check.py`；`pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build` | ☑ |

## 2. 写作用域

- 本变更只写：`packages/ui/ui-sessions/web/src/index.tsx`、`packages/ui/ui-kit/web/src/context.tsx`、
  `apps/desktop/web/src/{App.tsx,state/controller.ts,i18n.tsx}`、`apps/desktop/web/tests/workspace.test.tsx`、
  `e2e/interaction/sessions-workspace-new.spec.ts`（新）、`docs/19`、`KNOWN-LIMITATIONS.md`、本 spec 目录。
- 明确不写：`session.create`/contracts/DB/迁移/生成物；`ui-sessions` 的会话行 `⋯`/搜索/归档行为；`ui-kit` 其它组件；
  brand 点击/快捷键（proposal D2/D4 后置）；Batch2 命令。
- 并行/重叠：side-panels 刚实现 `ui-sessions/index.tsx`、`apps/desktop/web`；本 spec 只在其后**追加**按钮/action，需先 rebase，冲突以 append-only 为原则。
- 文件预算：**9**（实现 5 + 测试 2 + 文档 2；不含 spec 目录与生成物）。**Lead 2026-11-01 书面批准 8 文件/约 643 行例外**；P2-3 追加 `KNOWN-LIMITATIONS` 后总 9，≤10 拆分阈值，不做拆分。

## 3. 提交顺序

1. 任务 0（文档）→ 2. ui-kit action/i18n（1）→ 3. ui-sessions 行（2）→ 4. controller/app（3）→ 5. app 单测（4）→
6. 交互/e2e（5）→ 7. 收尾（6）。

## 4. 验收前检查

- [ ] `intuition.md`/`intuition-walkthrough.md` 与 `interaction.md` 已写；**实现开始时一起改 `active`**；P0 全覆盖；P0 摩擦无 OPEN。
- [ ] `python3 scripts/spec-check.py`（draft warning 可接受）；`python3 scripts/docs-check.py`。
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh`。
- [ ] `pnpm e2e`（既有 m1 用例不回归）；`pnpm e2e:ui --grep "workspace-new"`；`pnpm e2e:ui:gate`（工具已落地，`d0b7741`）。
- [ ] 无契约/迁移/生成物变化；`zk compose --check`；`zk capability --check`（无新增）。
- [ ] 无第三套建会话路径（只有 `createConversation`/`createConversationIn` 共用实现）；C18 无 workspace 不建会话。
- [ ] PR 只包含本 spec 范围；用户测试未跑标 **UNVERIFIED**。

## 5. 跨 spec / 共享文件

- 与 side-panels：共享 `ui-sessions/index.tsx`、`App.tsx`、`controller.ts`、`i18n.tsx`；side 已合并后本 spec 追加，不并行。
- 与 command-surface：⌘K `new-conversation` 仍走全局 active 规则，不受 scoped 按钮影响；不改变 `/` 命令面。
- 与 DSH 差异：brand 点击/`Mod+N` 后置（D2/D4）；本 spec 只做 Workspace 行 scoped 新建。

## 6. 实现备注（2026-11-01，impl-settings）

- `UiActions.createConversation` 扩展为 `(workspaceId?) => Promise<boolean>`；`AppController.createConversation` 成为三入口唯一 guard：streaming 时 toast「当前正在生成，停止后可新建」并返回 false；workspace 解析（scoped=行 workspace，全局/⌘K=active）；目标 workspace 不存在/为空时 `workspaceRequired`；成功后 `conversation_created` + （目标非 active 时）`conversation_activated` 选中目标 workspace/新会话；失败 toast、选择不变。
- ui-sessions：Workspace 行 scoped `+`（hover/focus-visible 可见、active 常显、`aria-label` 含名、独立 focus、`stopPropagation`、busy `aria-busy/disabled`）；侧栏顶部/header `+` 在 streaming 或 no-workspace 时禁用并带同一 title；空 workspace 不渲染行（既有行为）。
- ⌘K：`new-conversation` 目录项 streaming 时 `disabled+disabledReason`（同一文案），执行经同一 guard（`runCommand`）。
- 计划/实现偏差：streaming 的 e2e 断言用页面内 MutationObserver 原子捕获（fake upstream turn 极短，避免竞态），⌘K disabledReason 由 `command-catalog.test.ts` 单测覆盖；视觉基线需人工确认（仅截图证据）。
