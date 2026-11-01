# 2026-11-01-client-composer-surface · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`draft`（proposal §7 D1–D6 **已拍板**：用户 2026-11-01 批准推荐项；独立评审通过前不得改产品代码，C16/C19）。

## 0. 前置：文档口径（C12）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0 | 文档先行：`docs/19` §2 记录 composer 输入行新增 `/`、`@`、附件、模型只读标签、usage 指示的 Phase 2a 范围、「不新增 `ui.composer` slot」及附件/引用块上限；**`docs/26` §5 登记 `workspace.attachment_save` 命令与 `attachment/*` 错误码**（P1-2）；`KNOWN-LIMITATIONS` 记录附件/引用块边界与缩略图 `data:` 约束 | `docs/19-client-ux-flows.md`、`docs/26-workspace-and-conversations.md`、`KNOWN-LIMITATIONS.md` | — | `python3 scripts/docs-check.py`；`rg` 对照 contract 命令常量 | ☑ |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约（additive）：`commands::WORKSPACE_ATTACHMENT_SAVE` + `AttachmentSavePayload`/`AttachmentSaveResult`（`workspace_id/conversation_id/name/mime/data_base64` → `path/name/bytes/mime`）；限制常量放 host，不新增 capability | `packages/contracts/ipc/src/lib.rs` | 0 | `cargo test -p zk-contract-ipc`；`cargo test --workspace export_schema` | ☑ |
| 2 | 生成物：重生成 JSON Schema/TS（`export_schema` + `scripts/schema-ts.py`），确认 `workspace.attachment_save` schema 出现且无其它漂移 | `shared/contracts/schema/*`、`shared/contracts/ts/*`（生成器写，不手改） | 1 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ☑ |
| 3 | kit + 生成器 facade（P0-1/C17/C4）：新增 `zk_kit_tools::safe_join_under(root, parts)` 单一 helper（逐组件 no-follow、canonicalize 父目录、`starts_with(root)`、拒绝绝对/`..`；单测含父目录 symlink/`..`/绝对/workspace 外）；`tools/zk/src/main.rs` 生成 `zk-composed` facade（暴露 kit 能力，`apps/desktop` 不直接依赖 kit/tool）；**不手改 `@generated` 模板/生成物** | `crates/zk-kit-tools/src/lib.rs`、`tools/zk/src/main.rs` | 0 | `cargo test -p zk-kit-tools`；`zk compose --edition personal --profile default --channel official --check` | ☑ |
| 3b | host：`workspace.attachment_save` dispatch（allowlist、≤1 MiB、base64 解码、magic/空文件、文件名 sanitize、conversation 归属）；路径**必须**经 `safe_join_under`（经 `zk_composed` facade）逐组件校验后原子写 `.zeokeel/attachments/<conv>/<uuid>-<name>`；host 单测覆盖成功/超限边界/类型/非法 base64/父目录 symlink 逃逸/归属 | `apps/desktop/src/ipc.rs` | 1,3 | `cargo test -p zk-desktop` | ☑ |
| 4 | ui-kit：新增 `CommandDescriptor`/`DraftRef`/`AttachmentSaveResult` 等共享类型；`UiActions` 增 `runCommand`/`searchWorkspaceFiles`/`saveAttachment`/`openModelSettings`（`openSettings(paneId?)` 深链）；`UiValue` 增 `commands`/`modelLabel` | `packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-kit/web/src/context.tsx` | 1 | `pnpm --dir packages/ui/ui-kit/web typecheck && pnpm --dir packages/ui/ui-kit/web test:unit` | ☑ |
| 5 | app 单一命令目录：`App.tsx` 现有 `commands` 增加 `cancel`/`retry`（disabled 规则），作为 ⌘K 与 composer `/` 的唯一 catalog；实现 `UiActions` 新动作（`searchWorkspaceFiles` 复用 `workspace.files` 缓存、`saveAttachment` 调 IPC + 生成 schema 校验、`openModelSettings(paneId)`、`runCommand`）；`SettingsDialog` 增加可选 `initialPane`（pane id 校验后初始选中）；不新增 controller/protocol 文件（附件结果在 App.tsx 按生成 schema 校验） | `apps/desktop/web/src/App.tsx`、`packages/ui/ui-settings/web/src/index.tsx` | 4 | `pnpm -r typecheck && pnpm -r lint`；`pnpm --dir apps/desktop/web test:unit` | ☑ |
| 6 | ui-chat composer：受控原生 textarea（Enter/Shift+Enter/Esc/↑↓/IME 状态机）+ `/`/`@` popup（listbox/aria/↑↓/Enter/Esc/过滤/disabled/空态）+ chip 草稿（引用/附件、**FileReader `data:` 缩略图**/大小/失败重试、≤5）+ 模型标签 + usage 指示 + IME/键盘/六态；`AssistantThread` 用户消息 strip 引用块渲染 chips；i18n 键（zh/en） | `packages/ui/ui-chat/web/src/AssistantComposer.tsx`、`packages/ui/ui-chat/web/src/AssistantThread.tsx`、`apps/desktop/web/src/i18n.tsx` | 3,4 | `pnpm --dir packages/ui/ui-chat/web typecheck && pnpm --dir packages/ui/ui-chat/web lint`；`pnpm -r typecheck && pnpm -r lint`；`bash scripts/i18n-check.sh` | ☑ |
| 7 | UI 单测（新增）：`/` 过滤/键盘/disabled/执行清空；`@` 加 chip/去重/上限；IME 不触发；引用块 compose/strip（含路径换行/控制字符/`@` 开头/伪标记边界）；模型/用量两种状态；缩略图 data URL（无 blob） | `packages/ui/ui-chat/web/tests/composer.test.tsx`（新） | 5,6 | `pnpm --dir packages/ui/ui-chat/web test:unit` | ☑ |
| 8 | e2e（真实 serve + fake upstream）：`/` 执行新建会话；`@` 引用文件并发送；附件 file chooser 上传后经 `/ipc workspace.files` 断言 `.zeokeel/attachments` 存在、chip/引用块刷新后恢复；图片缩略图为 `data:`（非 blob）；axe 0 serious | `e2e/m1-serve.spec.ts` | 3b,5,6 | `pnpm e2e`（m1-serve 部分） | ☑ |
| 9 | 收尾：文档回写（§0 已完成项补证据）、acceptance 填证据、门禁全量；**交互门禁命令依赖 `2026-11-01-ui-interaction-verification` 落地（当前 UNVERIFIED，实现前确认脚本/`e2e:ui` 存在）** | 本 spec `acceptance.md`（只写本文件） | 1–8 | `python3 scripts/spec-check.py`；`python3 scripts/docs-check.py`；`zk compose --check`；`zk capability --check`；**待工具落地**：`pnpm e2e:ui --grep composer`、`python3 scripts/ui-interaction-gate.py`（见 §4/acceptance，当前不可执行） | ☑ |

## 2. 写作用域

- 本变更只写：`packages/contracts/ipc/src/lib.rs`（+生成物由生成器写）、`crates/zk-kit-tools/src/lib.rs`、`tools/zk/src/main.rs`（生成器源，**不手改 `@generated` 模板/产物**）、
  `apps/desktop/src/ipc.rs`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、`packages/ui/ui-chat/web/src/{AssistantComposer.tsx,AssistantThread.tsx}`、
  `packages/ui/ui-chat/web/tests/composer.test.tsx`（新）、`apps/desktop/web/src/{App.tsx,i18n.tsx}`、`packages/ui/ui-settings/web/src/index.tsx`、
  `e2e/m1-serve.spec.ts`、`docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md`、本 spec 目录。
- 明确不写：`packages/ui/ui-commands/**`（命令目录在 app；palette 不改）；`packages/ui/ui-settings/**` 仅允许 task 5 的 `web/src/index.tsx` `initialPane` 深链改动，其余文件不写（模型内联切换留后续）、
  `packages/contracts/agent|session|config`、`packages/client/**`、server/admin、生成物（生成器写）。
- 并行/重叠：任务 1→2→3 串行；4 与 3 可并行（不同类型）；5 依赖 4；6 依赖 3/4；7 依赖 5/6；8 依赖 3/5/6。
  `apps/desktop/web/src/i18n.tsx`、`App.tsx`、`ui-kit/context.tsx` **只允许本 spec 写**（Phase 2b 后置排队，见 §5）。

## 3. 提交顺序

1. 任务 0（文档口径）→ 2. 任务 1（契约）→ 3. 任务 2（生成物）→ 4. 任务 3（host）→ 5. 任务 4（ui-kit）→
6. 任务 5（app catalog/actions）→ 7. 任务 6（composer UI）→ 8. 任务 7（单测）→ 9. 任务 8（e2e）→ 10. 任务 9（证据/门禁）。

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`（draft warning 可接受）
- [ ] `python3 scripts/docs-check.py`
- [ ] `cargo test -p zk-contract-ipc -p zk-desktop`；`cargo test --workspace export_schema && git diff --exit-code`（新增 schema 后 diff 应用作基线，提交后再验幂等）
- [ ] `bash scripts/schema-check.sh`；`zk compose --edition personal --profile default --channel official --check`；`zk capability --check`
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`
- [ ] `bash scripts/i18n-check.sh` exit 0
- [ ] `pnpm e2e`（m1-serve 用例含新增 composer 场景）exit 0；证据 `target/evidence/2026-11-01-client-composer-surface/`
- [ ] *（依赖 `2026-11-01-ui-interaction-verification` 落地；当前 UNVERIFIED，命令尚不存在）* `pnpm e2e:ui --grep composer` exit 0（本 spec `interaction.md` 改 `active` 后，P0/P1 ID 全绿）
- [ ] *（同上，当前 UNVERIFIED）* `python3 scripts/ui-interaction-gate.py` exit 0（契约 ID ↔ 标签 ↔ 证据；无 `waitForTimeout`/`force`/裸 `test.skip`）
- [ ] 无手改 `@generated`；无新增 capability/INDEX 条目；无运行期插件加载（C2）
- [ ] `rg` 确认命令目录只有一处定义；`@` 复用 `workspace.files`；usage 复用既有事件（C17）
- [ ] PR 只包含本 spec 范围；`acceptance.md` 已填证据路径

## 5. 跨 spec 合并顺序与共享文件（占位）

- 顺序：Phase 1 全部（settings-ia → approval → config/mcp）→ **本 spec（Phase 2a）** → `2026-11-01-client-side-panels`（Phase 2b）
  → `2026-11-01-client-activity-panels`（Phase 2b 子 spec，见 side-panels §8）。
- 共享文件：`apps/desktop/web/src/{App.tsx,i18n.tsx}`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、
  `packages/contracts/ipc/src/lib.rs`、`tools/zk/src/main.rs`（生成器 facade 源）、`e2e/m1-serve.spec.ts`、`docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md` 由本 spec 先写；
  side-panels 必须在本 spec 合并后排队改同一文件（其 tasks §0b/§5 登记）；`e2e/m1-serve.spec.ts` 与 `tools/zk/src/main.rs` 由 side-panels/activity 后追加（串行，不并行）。

## 6. 实现备注（2026-11-01，impl-settings）

- `interaction.md` 已由 `planned` 改 `active`；15 个 P0/P1 ID 全部有 `@interaction:<ID>` 测试与证据
  （`target/evidence/2026-11-01-client-composer-surface/interaction/<ID>.json`，15/15 passed）；`ui-interaction-gate.py` + `--strict` 均 exit 0。
- C20：新增 `intuition.md`（12 INT）/ `intuition-walkthrough.md`（3 persona × 3 task + 摩擦表，无 P0 摩擦）；真人用户测试 UNVERIFIED。
- 任务 0：`docs/19` §2.1、`docs/26` §5、`KNOWN-LIMITATIONS.md` 已落定；`docs/28` §4 回写现状。
- 任务 2：`export_schema` + `schema-ts` 已重生成（新增 2 个 attachment schema）；`git diff --exit-code` 需提交后才有意义（UNVERIFIED-until-commit）。
- 任务 8：`pnpm e2e` 原样复跑依赖已提交生成基线（`restore-baseline.sh`）；已用等价序列执行：mock 8/8、serve 19/19（含本 spec 3 个 composer 用例）；
  `pnpm e2e:ui` 完整套件 33 passed / 1 failed（唯一失败 = 基线 `CMP-VIS-01` 旧 composer 像素基线，需人工确认后 `--update-snapshots`；agent 不得自动接受）→ 像素基线 UNVERIFIED；
  本 spec 套件 15/15 passed（`interaction/<ID>.json` 15 份）。
- 模型深链落地为 `UiActions.openModelSettings(paneId)` + `openSettingsPane(paneId)`（保留既有 `openSettings()` 签名以便
  `ui-sessions` 等既有调用点不改；`SettingsDialog.initialPane` 校验 pane id 后选中）——与 spec R4/design D3 语义一致，
  但为避免改动 `ui-sessions`（不在本 spec 写作用域）而没有把 `openSettings` 改成可选参数。
- ARIA 说明：输入框保持 `role=textbox`（既有基线依赖），`aria-expanded` 在 ARIA 中不允许用于 textbox；
  菜单展开经 `aria-haspopup="listbox"` + `aria-controls` + `aria-activedescendant` + `role=status` live region 暴露（`interaction.md` §1 已注明）。
- `zk docs --check` 的漂移来自并行会话已合并的 `ui-settings-mcp` 包（HEAD 有包但 `docs/generated` 未重生成），已由生成器 `zk docs` 写回（C11）。
