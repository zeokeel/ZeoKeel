# 2026-11-01-client-command-surface · 任务清单

> 每个任务必须能独立提交、独立验证；没有 verify 的任务不算任务。
> 状态：`draft`（proposal §7 D1–D6 **已定（Lead 2026-11-01，推荐 A）**；未独立评审通过前不得改产品代码，C16/C19）。
> 顺序：前置 composer-surface（已实现）→ **本 spec**；Batch2 另 spec。side-panels 已实现合并不冲突（只共享 `App.tsx`/`i18n.tsx`/`ui-kit`，本 spec 只追加）。

## 0. 前置：文档口径（C12）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档先行：`docs/19` §2/§11 记录 DSH `/` 命令面（两节分组/键位/两 surface 分流/GoalBar/错误与空态）；`docs/26` §5 登记新 IPC/错误码与 override/export/feedback 文件约定；`KNOWN-LIMITATIONS` 记录 D3–D5 差异 | `docs/19-client-ux-flows.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py`；`rg` 对照 contract 命令常量 | ☑ |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约（additive）：`goal.get/set/update`、`session.override_get/set`、`session.export`、`feedback.submit` + DTO/错误码；`auth.configure` 可选 `models[]`（enterprise host 缓存）；`CommandDescriptor` 的 surface/kind 字段在 ui-kit；**不改 ChatSendPayload/session 表** | `packages/contracts/ipc/src/lib.rs` | 0 | `cargo test -p zk-contract-ipc`；`cargo test --workspace export_schema` | ☑ |
| 2 | 生成物：重生成 JSON Schema/TS，确认新命令/错误码出现且无其它漂移 | `shared/contracts/schema/*`、`shared/contracts/ts/*`（生成器写，不手改） | 1 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ☑ |
| 3 | ui-kit：`CommandDescriptor` 扩展（surface/kind/disabledReason）、命令 actions/DTO；**共享 Modal 实现放 `index.tsx`**（feedback/dialog 复用，不新增 `Modal.tsx`、不建第二套 Dialog） | `packages/ui/ui-kit/web/src/{types.ts,context.tsx,index.tsx}` | 1 | `pnpm --dir packages/ui/ui-kit/web typecheck && pnpm --dir packages/ui/ui-kit/web test:unit` | ☑ |
| 3b | **C17 Modal 迁移（P1-5）**：把 `ui-sessions` 私有 Modal 改为 import ui-kit 共享 Modal（删除本地实现），确认现有 workspace remove/rename 等确认行为/Esc/焦点不回归；禁止两套 Dialog | `packages/ui/ui-sessions/web/src/index.tsx` | 3 | `pnpm --dir packages/ui/ui-sessions/web typecheck && pnpm --dir packages/ui/ui-sessions/web lint && pnpm --dir packages/ui/ui-sessions/web test:unit`；e2e 既有迁移确认用例 | ☑ |
| 4 | app：唯一 catalog 增加 DSH slash 行/Batch2 占位（UI 动作 `surface:palette`，命令 `surface:slash`）；`runCommand` 二次守卫未知/占位；实现 `runCommand`/actions（goal/override/export/feedback；personal `/model` 手动输入+recentModels≤8、enterprise 列表来自 host 缓存）；控制器/protocol/i18n | `apps/desktop/web/src/{App.tsx,state/controller.ts,protocol.ts,i18n.tsx}` | 3 | `pnpm -r typecheck && pnpm -r lint`；`pnpm --dir apps/desktop/web test:unit`；`bash scripts/i18n-check.sh` | ☑ |
| 5 | ui-chat 命令菜单：分组/过滤/键位（/、↑↓、Enter/Tab、Esc/Shift+Tab）、aria、disabled/占位、附件边界、未知命令不发送；复用 composer 附件/触发逻辑，不建第二套菜单 | `packages/ui/ui-chat/web/src/AssistantComposer.tsx`、`packages/ui/ui-chat/web/src/CommandMenu.tsx`（新） | 3,4 | `pnpm --dir packages/ui/ui-chat/web typecheck && pnpm --dir packages/ui/ui-chat/web lint`；单测 | ☑ |
| 6 | host：override 文件读写/归属/损坏降级；`session.override_get/set` dispatch；**global config-mutex**：turn 前获取，按 conversation 应用 model/effort（configure_model/configure_agent）+ permission（`set_approval_policy` 单点，不改 gate owner），turn 后按 settings snapshot/revision 恢复，失败/取消回滚；其它 conversation turn 排队；**`SETTINGS_SET` apply/publish 也获取同一 config-mutex（turn 运行中 settings.set 不改变当前 override，结束后按 revision 恢复/应用）**；**排队 turn 获锁前检查 cancelled，已取消则跳过不执行/不报错**；enterprise gatewayModels 缓存/校验；两会话并发/恢复/失败/turn 中 settings.set/排队取消测试 | `apps/desktop/src/ipc.rs`、`apps/desktop/src/command_surface.rs`（新）、`apps/desktop/src/main.rs`、`tools/zk/src/main.rs`（generated facade `set_approval_policy`） | 1,2,3b | `cargo test -p zk-desktop`；`zk compose --check` | ☑ |
| 7 | `/goal`：host IPC + tool-goal driver（同一 `.zeokeel/goal.json`），pause/resume/cancel 映射；set/clear **内联确认**（取消不发 IPC）；`GoalBar.tsx` in composer 上方（无 goal 不渲染；读/暂停/清除） | `apps/desktop/src/ipc.rs`、`apps/desktop/src/command_surface.rs`、`packages/ui/ui-chat/web/src/GoalBar.tsx` | 1,4,5 | `cargo test -p zk-desktop`；`pnpm --dir packages/ui/ui-chat/web test:unit` | ☑ |
| 8 | `/export`：host 生成 Markdown（≤8 MiB/文件名 sanitize/空会话/失败）、返回路径；**serve 下载；Tauri 展示/复制路径，不承诺自动打开（`system.open_url` 仅 http(s)）**；无新服务端 | `apps/desktop/src/ipc.rs`、`apps/desktop/src/command_surface.rs`、`apps/desktop/web/src/protocol.ts` | 1,4 | `cargo test -p zk-desktop`；e2e 文件断言 | ☑ |
| 9 | `/feedback`：FeedbackDialog（分类/描述≤4 KiB/本地保存文案/取消/失败重试）；host 本地 JSON，无网络；企业服务端 UNVERIFIED | `packages/ui/ui-chat/web/src/FeedbackDialog.tsx`（新）、`apps/desktop/src/command_surface.rs` | 1,3,4 | `cargo test -p zk-desktop`；`pnpm --dir packages/ui/ui-chat/web test:unit` | ☑ |
| 10 | 测试全覆盖：UI 单测（分组/键位/未知/占位/附件/model/permission/goal/export/feedback/六态）、app 单测、e2e（/file、/model 手动+recent+切会话+重启+enterprise 列表、/permission、/goal+GoalBar、/export 文件/Tauri 路径、/feedback 文件、axe）；host 两会话并发/turn 中 settings.set/排队取消/Modal 迁移回归 | `packages/ui/ui-chat/web/tests/command-surface.test.tsx`（新）、`apps/desktop/web/tests/command-surface.test.tsx`（新）、`e2e/m1-serve.spec.ts`、`e2e/interaction/command-surface.spec.ts`（新） | 4–9 | `pnpm e2e`；交互门禁（task-33 基线已落地）：`pnpm e2e:ui --grep command` + `python3 scripts/ui-interaction-gate.py` | ☑ |
| 11 | 收尾：文档回写/acceptance 证据/门禁；`interaction.md`+`intuition.md` 实现开始时一起改 `active`；用户测试未跑标 UNVERIFIED | 本 spec `acceptance.md`（只写本文件） | 1–10 | `spec-check`/`docs-check`；`zk compose --check`；`zk capability --check`；交互门禁：`pnpm e2e:ui --grep command` + `python3 scripts/ui-interaction-gate.py`（基线已落地） | ☑ |

## 2. 写作用域

- 本变更只写：`packages/contracts/ipc/src/lib.rs`（+生成物由生成器写）、`packages/ui/ui-kit/web/src/{types.ts,context.tsx,index.tsx}`、`packages/ui/ui-sessions/web/src/index.tsx`、
  `packages/ui/ui-chat/web/src/{AssistantComposer.tsx,CommandMenu.tsx,GoalBar.tsx,FeedbackDialog.tsx}`、`packages/ui/ui-chat/web/tests/command-surface.test.tsx`（新）、
  `apps/desktop/src/{ipc.rs,command_surface.rs,main.rs}`、`tools/zk/src/main.rs`、`apps/desktop/web/src/{App.tsx,state/controller.ts,protocol.ts,i18n.tsx}`、
  `apps/desktop/web/tests/command-surface.test.tsx`（新）、`e2e/m1-serve.spec.ts`、`e2e/interaction/command-surface.spec.ts`（新）、
  `docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md`、本 spec 目录。
- 明确不写：`packages/ui/ui-commands/**`（⌘K 组件不改；只共享 catalog）、`packages/ui/ui-settings/**`、`packages/contracts/{agent,session,config}`、
  `packages/client/**`（tool-goal 等不改行为）、server/admin、生成物（生成器写）、Batch2 命令实现。
- 并行/重叠：与 side-panels 已实现的 `App.tsx`/`i18n.tsx`/ui-kit 只**追加** DSH 命令行/actions；先 rebase 后改，冲突以本 spec 的 append-only 为原则。
- 文件预算：**24**（不含 spec 目录与生成物；Lead 2026-11-01 书面批准 24 文件/约 728 行例外），与 proposal §8 一致；若实现超 28 文件，先拆 `/export` 子 spec。

## 3. 提交顺序

1. 任务 0（文档）→ 2. 契约/生成物（1–2）→ 3. ui-kit（3）→ 4. app catalog/actions（4）→ 5. 命令菜单（5）→
6. host override（6）→ 7. goal/GoalBar（7）→ 8. export（8）→ 9. feedback（9）→ 10. 测试（10）→ 11. 收尾（11）。

## 4. 验收前检查

- [ ] `intuition.md`/`intuition-walkthrough.md` 与 `interaction.md` 已写；**实现开始时一起改 `active`**，P0 映射全覆盖，P0 摩擦无 OPEN。
- [ ] `python3 scripts/spec-check.py`（draft warning 可接受）；`python3 scripts/docs-check.py`。
- [ ] `cargo test -p zk-contract-ipc -p zk-desktop`；`cargo test --workspace export_schema`（基线后幂等）；`bash scripts/schema-check.sh`。
- [ ] `zk compose --edition personal --profile default --channel official --check`；`zk capability --check`（无新增）。
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh`。
- [ ] `pnpm e2e`（m1-serve 追加：/file、/model、/permission、/goal、/export、/feedback、未知/占位）exit 0。
- [ ] 交互门禁（task-33 基线已落地 `package.json` `e2e:ui` + `scripts/ui-interaction-gate.py`；实现时运行）：`pnpm e2e:ui --grep command`、`python3 scripts/ui-interaction-gate.py`。
- [ ] 无手改 `@generated`；无运行期加载（C2）；无第二套菜单/执行器/Dialog（C17）；无新 capability；C18 归属不变。
- [ ] PR 只包含本 spec 范围；acceptance 证据已填；用户测试未跑标 **UNVERIFIED**。

## 5. 跨 spec / 共享文件

- 依赖 composer-surface 的 `/` 菜单、附件、引用、模型只读标签；本 spec **替换**其命令目录内容（UI 动作移到 ⌘K），不复制实现。
- 共享：`App.tsx`、`i18n.tsx`、`ui-kit` 由 side-panels 先合并；本 spec 在其后 append，避免并行改同一文件。
- Batch2（`/compact` `/plan` `/skill`）另 spec；本 spec 只放禁用占位与 `command/unimplemented` 错误。

## 6. 实现备注（2026-11-01，impl-settings）

- 契约：新增 `goal.get/set/update`、`session.override_get/set`、`session.export`、`feedback.submit` + DTO（`GoalView/GoalResult`、`ConversationOverride*`、`SessionExportResult`、`FeedbackSubmitResult`）；`auth.configure` 加 `models[]` 缓存；agent 契约加 `set_approval_preset`（默认 no-op），agent-rig 映射到 `config::ApprovalPolicy::should_ask`（单点）；`export_schema` 121 schemas。
- host `command_surface.rs`：override 文件（原子写/损坏降级/归属校验/空串清除）、config-mutex（turn 前 apply、turn 后按 snapshot restore；settings.set 共用；排队 turn 获锁后复检 cancelled 跳过）、goal 经 `zk_composed::zk_tool_goal::driver`、`/export` Markdown + `/download`（serve，cookie + exports 白名单）、`/feedback` 本地 JSON。
- UI：`ui-kit` 唯一 `Modal`（`ui-sessions` 私有 Modal 已删除迁移）、composer `/` 菜单改为 slash surface（DSH 分组/键位/unknown/unimplemented/附件边界/leadingInput `/goal <objective>`）、Model/Permission/Goal/Feedback dialogs、GoalBar。
- 偏差/说明：`/goal` 的 leadingInput 在提交时拦截（菜单在空格后关闭，spec 语义仍满足：命令文本绝不发送）；`/export` Tauri 路径复制、serve `/download` 下载；排队取消语义已实现并由单测覆盖（长时并发压测 UNVERIFIED）；C20 基线需人工确认，用户测试 UNVERIFIED。
