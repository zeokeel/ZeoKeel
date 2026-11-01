# 2026-11-01-client-activity-panels · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`done`（2026-11-01 实现完成；门禁/证据见 `acceptance.md` §1/§1.5/§8）。
> 顺序：Phase 2a `client-composer-surface` → Phase 2b `client-side-panels` → **本 spec**（共享文件在其合并后排队）。

## 0. 前置：文档/决策口径（C12）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档先行：`docs/16` §2.2/§2.4 记录右栏 tab order（files 10/changes 20/terminal 30/**activity 40**）且不新增 slot；**`docs/26` §5 登记 `activity.snapshot`/`activity.action` 命令与 `activity/*` 错误码**（P1-2）；`KNOWN-LIMITATIONS` 预登记（jobs/subagent 进程内、todo 工具行推导、goal 只读 goal.get、2s 轮询、输出截断；**plan 由 client-plan-mode 负责**）；`docs/28` 的 activity 预留已由 side-panels 记录 | `docs/16-ui-packages.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py`；`rg` 对照 contract 命令常量 | ✅ |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约（additive）：`commands::ACTIVITY_SNAPSHOT`/`ACTIVITY_ACTION`；payloads `ActivitySnapshotPayload`/`ActivityActionPayload`；results `ActivitySnapshot` + `ActivityJob/Subagent/TeamRun/TeamWorker/Todo/Schedule/Presentation`/`ActivityActionResult`（**goal 不进 snapshot**）；不新增 capability | `packages/contracts/ipc/src/lib.rs` | 0 | `cargo test -p zk-contract-ipc` | ✅ |
| 2 | 生成物：重生成 JSON Schema/TS（`export_schema` + `scripts/schema-ts.py`），确认 activity DTO 出现且无其它漂移 | `shared/contracts/schema/*`、`shared/contracts/ts/*`（生成器写） | 1 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ✅ |
| 3 | 组合/生成器 facade：`tools/zk/src/main.rs` 生成 `zk-composed` facade 暴露 `zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule`（app 只依赖 `zk-composed`，不直接依赖 tool；**不手改 @generated 模板/产物**）；compose `--check` 绿 | `tools/zk/src/main.rs` | 0 | `zk compose --edition personal --profile default --channel official --check`；`cargo check -p zk-desktop` | ✅ |
| 4 | host：`activity.snapshot`（jobs **canonicalize + `Path::starts_with`** 过滤、subagents parent 过滤、teams/todos/presentations 工具行派生且 **`args` 字符串二次 JSON 解析**；**goal 读既有 `goal.get`（不派生工具行、不进 snapshot）**、schedules 经生成 facade 复用 `zk_tool_schedule::tool().call_json`）；`activity.action`（job_kill/subagent_interrupt/schedule_update/schedule_delete + 错误码；**动作后重新 snapshot + host 当前状态校验（不新增 version 字段），未知/已结束 job/subagent 映射可读错误**；schedule 先 `list` 预检、prompt≤32 KiB、`deleted 0`→`activity/schedule-invalid`）；派生/边界/损坏行/args 二次解析单测（`ipc.rs` 内） | `apps/desktop/src/ipc.rs` | 1,3 | `cargo test -p zk-desktop` | ✅ |
| 5 | ui-kit：Activity DTO 镜像、`UiActions.activitySnapshot/activityAction/openInspectorPreview`、`previewRequest` 类型（与 side-panels inspector props 对齐） | `packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-kit/web/src/context.tsx` | 1 | `pnpm --dir packages/ui/ui-kit/web typecheck && pnpm --dir packages/ui/ui-kit/web test:unit` | ✅ |
| 6 | ui-inspector：`TABS` 增加 `activity`（order 40）+ 面板（jobs/subagents/team/todo/schedule/present 分区 + **goal 分区单独调 `goal.get`**；空/加载/错误/stale；2s 轮询仅打开时、手动刷新、动作后刷新；**关闭/切走/切会话 clearInterval + 忽略旧 response/generation**；停止/中断仅 running + **内联二次确认（对象名/影响，默认焦点取消，role=alertdialog）** + busy/错误可读；schedule 删除内联确认）；复用 side-panels 的宽度/持久化/六态/i18n | `packages/ui/ui-inspector/web/src/index.tsx` | 5 | `pnpm --dir packages/ui/ui-inspector/web typecheck && pnpm --dir packages/ui/ui-inspector/web lint` | ✅ |
| 7 | ui-chat turnTail：**仅 present 交付卡**（本轮去重、点击 `openInspectorPreview`）；**不展示/折叠 plan**（plan 由 client-plan-mode PlanCard 负责）；仅完成/取消/错误消息；i18n 键 | `packages/ui/ui-chat/web/src/AssistantThread.tsx` | 5 | `pnpm --dir packages/ui/ui-chat/web typecheck && pnpm --dir packages/ui/ui-chat/web lint` | ✅ |
| 8 | desktop-web：`App.tsx` 注入 activity actions + `previewRequest`（打开右栏 files 预览）+ `inspectorTab` 含 `"activity"`；`protocol.ts` 解析 snapshot/action（Ajv/generated schema）；`i18n.tsx` 键（zh/en） | `apps/desktop/web/src/App.tsx`、`apps/desktop/web/src/protocol.ts`、`apps/desktop/web/src/i18n.tsx` | 2,4,5,6,7 | `pnpm -r typecheck && pnpm -r lint`；`bash scripts/i18n-check.sh` | ✅ |
| 9 | 交互契约 + e2e（真实 serve+SQLite）：`interaction.md`（planned，P0/P1 ID）在实现开始时改 `active`；**新增确认交互测试**：`job-stop-confirm`（确认前无 action/取消不变，确认后停止）与 `subagent-interrupt-confirm`（同）挂到 `ACT2-JOB-01`/`ACT2-SUBAGENT-01`；`e2e/serve-m1.sh` 增加 activity 剧本（后台 job/subagent/todo/goal/schedule/present；goal 用 goal.get）；`e2e/interaction/activity.spec.ts`（新）按 `interaction.md` 的 P0/P1 ID 写 @interaction 标签测试（tab/轮询/goal.get/job 停止/subagent 中断/schedule 编辑删除/turnTail present/axe）；证据 `target/evidence/2026-11-01-client-activity-panels/interaction/<ID>.json` | `e2e/serve-m1.sh`、`e2e/interaction/activity.spec.ts`（新） | 4,8 | `pnpm e2e:ui --grep activity`（工具已落地）；`pnpm e2e:ui:gate` | ✅ |
| 10 | 收尾：文档回写（0 已完成项补证据）、acceptance 填证据、门禁全量；**交互门禁 `pnpm e2e:ui`/`ui-interaction-gate.py` 依赖 `ui-interaction-verification` 落地（当前 UNVERIFIED，不得声称已绿）** | `docs/16-ui-packages.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md`、本 spec `acceptance.md` | 1–9 | `python3 scripts/spec-check.py`；`python3 scripts/docs-check.py`；`zk compose --check`；`zk capability --check`；待工具落地：交互门禁命令 | ✅ |

## 2. 写作用域

- 本变更只写：`packages/contracts/ipc/src/lib.rs`（+生成物由生成器写）、`tools/zk/src/main.rs`（生成 facade；**不手改 @generated 模板/产物**）、
  `apps/desktop/src/ipc.rs`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、`packages/ui/ui-inspector/web/src/index.tsx`、
  `packages/ui/ui-chat/web/src/AssistantThread.tsx`、`apps/desktop/web/src/{App.tsx,protocol.ts,i18n.tsx}`、
  `e2e/serve-m1.sh`、`e2e/interaction/activity.spec.ts`（新）、`docs/16-ui-packages.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md`、本 spec 目录。
- 明确不写：`tool-todo`/`tool-goal`/`tool-agent-team`/`tool-schedule` 行为（复用现有；schedule 只调用公开 API）、
  `packages/client/agent-rig/**`、server/admin、生成物（生成器写）、side-panels/composer spec 目录。
- 并行/重叠：1→2→3 串行；4 依赖 1/3；5 依赖 1；6/7 依赖 5；8 依赖 2/4/5/6/7；9 依赖 4/8。
  `apps/desktop/web/src/{App.tsx,protocol.ts,i18n.tsx}`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、
  `packages/ui/ui-inspector/web/src/index.tsx` **只允许本 spec 写**（composer/side-panels 已先合并，见 §5）。
- 文件预算：**15**（不含本 spec 目录与生成物），与 proposal §8 一致（Lead 例外：14 + docs/26）。

## 3. 提交顺序

1. 任务 0（文档口径）→ 2. 任务 1（契约）→ 3. 任务 2（生成物）→ 4. 任务 3（组合依赖）→ 5. 任务 4（host）→
6. 任务 5（ui-kit）→ 7. 任务 6/7（activity/turnTail）→ 8. 任务 8（app）→ 9. 任务 9（e2e）→ 10. 任务 10（证据/文档/门禁）。

## 4. 验收前检查

- [x] `intuition.md`/`intuition-walkthrough.md`（active）已评审：P0 全覆盖、无 OPEN P0；实现开始时随 `interaction.md` 改 `active`
- [x] `python3 scripts/spec-check.py`（draft warning 可接受）；`python3 scripts/docs-check.py`
- [x] `cargo test -p zk-contract-ipc -p zk-desktop`；`cargo test --workspace export_schema`（新增 schema 作基线，提交后验幂等）；`bash scripts/schema-check.sh`
- [x] `zk compose --edition personal --profile default --channel official --check`；`zk capability --check`（无新增条目）
- [x] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh`
- [ ] **待工具落地**：`pnpm e2e:ui --grep activity`（依赖 `2026-11-01-ui-interaction-verification`；当前 UNVERIFIED）；e2e 证据 `target/evidence/2026-11-01-client-activity-panels/interaction/<ID>.json`
- [x] 无手改 `@generated`；无新增运行期加载（C2）；无第二套 jobs/subagent/schedule/todo/goal 状态机（C17）；**plan 不展示（client-plan-mode 负责）**
- [ ] *（依赖工具落地，当前 UNVERIFIED）* `pnpm e2e:ui --grep activity`；证据 `target/evidence/2026-11-01-client-activity-panels/interaction/`
- [x] `pnpm e2e:ui:gate`（工具已落地；本 spec `interaction.md` 实现时由 `planned` 改 `active`）
- [x] `cargo check --workspace --all-targets`；PR 只包含本 spec 范围；`acceptance.md` 已填证据路径

## 5. 跨 spec 顺序与共享文件

- 顺序：Phase 1 全部 → Phase 2a `client-composer-surface` → Phase 2b `client-side-panels` → **本 spec**。
- 共享文件（本 spec 独占，必须在前两者合并后改）：`packages/contracts/ipc/src/lib.rs`、`tools/zk/src/main.rs`（在 composer/side 的 kit facade 后追加 jobs/subagent/tool-schedule re-export）、`apps/desktop/src/ipc.rs`、
  `apps/desktop/web/src/{App.tsx,protocol.ts,i18n.tsx}`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、
  `packages/ui/ui-inspector/web/src/index.tsx`、`packages/ui/ui-chat/web/src/AssistantThread.tsx`、`docs/16`。
- 依赖 side-panels 的成果：右栏 tab 状态持久化/宽度/预览组件/file_read、`openInspectorPreview` 所需的 `previewRequest` prop 对齐（side-panels design §2.3）。
