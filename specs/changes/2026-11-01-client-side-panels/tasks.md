# 2026-11-01-client-side-panels · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。
> 状态：`draft`（proposal §7 D1–D6 **已拍板**：用户 2026-11-01 批准推荐项；独立评审通过前不得改产品代码，C16/C19）。
> 范围：右栏（文件树/预览/终端/状态持久化）+ 会话操作 + 检索入口；能力面板见 §8 子 spec。

## 0. 前置：文档/决策口径（C12/C18）

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 0a | **文档先行（C12）**：`docs/26` 更新 schema v6（pinned/archived 列）、§2 硬规则 11/13（归档/置顶/文件浏览从 post-M1 转为 Phase 2b 计划；archive=隐藏不删、fork 同 workspace、检索同 workspace）、§5 错误码表新增 `workspace/path-escape`/`file/too-large`/`file/unsupported`/`terminal/*`/`session/invalid` 等、§2.5 记录取舍；`docs/16` 记录右栏 tab order（files 10/changes 20/terminal 30，activity 40 预留）且不新增 slot/settings pane（MCP pane order 70 不受影响）；`docs/28` §4 同时注明 activity tab（40）由子 spec 实现；`docs/19` §2 记录右栏/会话操作/检索状态口径；`docs/28` §4 更新实现目标 | `docs/26-workspace-and-conversations.md`、`docs/16-ui-packages.md`、`docs/19-client-ux-flows.md`、`docs/28-ui-module-map-and-skeleton.md` | — | `python3 scripts/docs-check.py` | ☑ |
| 0b | 跨 spec 顺序登记：本 spec 在 Phase 2a `client-composer-surface` 之后；`specs/changes/2026-11-01-client-activity-panels/`（子 spec，五件套 draft）在本 spec 之后；共享文件（`ipc.rs`/`App.tsx`/`i18n.tsx`/`ui-kit/context.tsx`/docs/16/28）owner 见 §5 | 本 spec `proposal.md`/`tasks.md`（只读登记；不改其它 spec） | — | `python3 scripts/spec-check.py` | ☑ |

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约 session：`Conversation.pinned/archived`（`#[serde(default)]`）；`FileReadKind`/`WorkspaceFileReadResult`；`SessionSearchHit`/`SessionSearchResult`；`SessionStore` 新增 rename/pin/archive/fork **默认实现**（默认返回 `SessionError::Invalid`，不破坏第三方/测试实现；fork 默认实现 create+messages_in+append） | `packages/contracts/session/src/lib.rs`、`packages/contracts/session/tests/conformance.rs` | 0a | `cargo test -p zk-contract-session` | ☑ |
| 2 | 契约 ipc：`WORKSPACE_FILE_READ`、`TERMINAL`、`SESSION_UPDATE`、`SESSION_FORK`、`SESSION_SEARCH` 命令 + payloads；`WorkspaceFilesPayload`（可选 `path/depth/limit`，缺省=旧行为）；`results::TerminalResult`；不新增 capability | `packages/contracts/ipc/src/lib.rs` | 1 | `cargo test -p zk-contract-ipc` | ☑ |
| 3 | 生成物：重生成 JSON Schema/TS（`export_schema` + `scripts/schema-ts.py`），确认新 DTO 出现且无其它漂移 | `shared/contracts/schema/*`、`shared/contracts/ts/*`（生成器写，不手改） | 2 | `cargo test --workspace export_schema && git diff --exit-code`；`bash scripts/schema-check.sh` | ☑ |
| 4 | store v6：`conversations` 加 `pinned/archived`（幂等 `has_column`）、`row_to_conversation` 读取、`list_conversations_in` 排序 `pinned DESC, updated_at_ms DESC, id DESC`；实现 rename/pin/archive + **覆盖 fork**（事务内复制消息、新 message id、保留原 created_at_ms/status/usage/turn_id；不复制 pinned/archived）；迁移/兼容/排序/fork 单测 | `packages/client/store-session-sqlite/src/lib.rs`、`packages/client/store-session-sqlite/tests/conformance.rs` | 1 | `cargo test -p zk-store-session-sqlite` | ☑ |
| 5 | 检索单一实现：`zk_kit_session::search(store, workspace_id, query, limit) -> Vec<SessionSearchHit>`（复用 contract DTO，大小写不敏感 contains，含 archived，snippet ±60，limit≤50；**另保留完整 content 字段**）；`tool-session-query` 改为调用该函数且输出仍为完整 content（P2-6 兼容断言）；kit 单测 | `crates/zk-kit-session/src/lib.rs`、`crates/zk-kit-session/tests/search.rs`（新）、`packages/client/tool-session-query/src/lib.rs` | 1 | `cargo test -p zk-kit-session -p zk-tool-session-query` | ☑ |
| 6 | host：`workspace.files` 可选 path/depth/limit（默认旧行为，路径校验、symlink 不 follow/不返回）；`workspace.file_read`（复用 `safe_join_under`（经 generated facade）+ canonicalize+前缀/父目录 symlink 拒绝、文本 256 KiB、图片 8 MiB、PDF/二进制 unsupported）；`terminal`（复用 `zk-kit-tools::persistent` open/send/close/list，timeout≤10s，输出≤64 KiB，平台不可用错误）；`session.update/fork/search` dispatch（search 返回 snippet，工具仍完整 content）；host 单测覆盖兼容/边界/安全 | `apps/desktop/src/ipc.rs`、`tools/zk/src/main.rs`（生成 `zk-composed` facade 暴露 `zk-kit-tools`；**不手改 @generated 模板/产物**） | 2,4,5 | `cargo test -p zk-desktop`；`zk compose --edition personal --profile default --channel official --check` | ☑ |
| 7 | ui-kit：共享类型（FileReadResult/Terminal/SessionSearch/SessionOps、Conversation pinned/archived）；`UiActions` 新增 `listDirectory`/`readWorkspaceFile`/`terminal*`/`updateConversation`/`forkConversation`/`searchSessions`；`UiValue` 需要的只读字段（如 `conversations` 已含新字段） | `packages/ui/ui-kit/web/src/types.ts`、`packages/ui/ui-kit/web/src/context.tsx` | 2 | `pnpm --dir packages/ui/ui-kit/web typecheck && pnpm --dir packages/ui/ui-kit/web test:unit` | ☑ |
| 8 | ui-inspector：files tab 目录树（懒加载/展开/刷新/行内错误）、预览（text/image/降级/返回）、terminal tab（会话列表/输出/输入/关闭/运行态/错误）、宽度拖拽+键盘+折叠持久化 props（`width/collapsed/onWidth/onCollapse/actions`），tab order files/changes/terminal；六态/axe/i18n | `packages/ui/ui-inspector/web/src/index.tsx`、`packages/ui/ui-inspector/web/src/panels.tsx`（新） | 7 | `pnpm --dir packages/ui/ui-inspector/web typecheck && pnpm --dir packages/ui/ui-inspector/web lint` | ☑ |
| 9 | ui-sessions：会话行 `⋯` 菜单（rename/fork/copy id/pin/unpin/archive/unarchive）、归档筛选、搜索「标题/内容」模式与命中列表（debounce/取消旧请求/空态/错误态），复用现有 Modal/焦点模式；`⋯` 在 hover/focus 可见 + `aria-label`，不 hover-only | `packages/ui/ui-sessions/web/src/index.tsx` | 7 | `pnpm --dir packages/ui/ui-sessions/web typecheck && pnpm --dir packages/ui/ui-sessions/web lint` | ☑ |
| 10 | desktop-web：`App.tsx` 右栏状态（会话级 localStorage：tab/width/collapsed、窄窗自动收起、⌘B）、注入 ui-inspector/ui-sessions 新 actions/props；`state/controller.ts` 实现 listDirectory/readFile/terminal/updateConversation/fork/search 动作（含 archived 发送阻止）；`protocol.ts` 解析新结果/Conversation 新字段；`i18n.tsx` 键（zh/en） | `apps/desktop/web/src/App.tsx`、`apps/desktop/web/src/state/controller.ts`、`apps/desktop/web/src/protocol.ts`、`apps/desktop/web/src/i18n.tsx` | 6,7,8,9 | `pnpm -r typecheck && pnpm -r lint`；`bash scripts/i18n-check.sh` | ☑ |
| 11 | app 单测（追加）：inspector 树/预览/终端/持久化（mock transport）；会话菜单/pin/archive/fork/search 动作与 archived 发送阻止 | `apps/desktop/web/tests/inspector-changes.test.tsx`、`apps/desktop/web/tests/workspace.test.tsx` | 10 | `pnpm --dir apps/desktop/web test:unit` | ☑ |
| 12 | e2e（m1-serve 追加，真实 SQLite/serve + 临时 HOME）：文件树展开/刷新保持、文本/图片预览/PDF 降级、终端 open→send→输出→close、rename/fork（复制消息）/copy id/pin/archive+筛选、内容搜索命中打开会话、tab/宽度/折叠刷新恢复、窄窗收起、axe 0 serious | `e2e/m1-serve.spec.ts` | 6,10,11 | `pnpm e2e`（m1-serve 用例）；**待工具落地（依赖 `ui-interaction-verification`，当前 UNVERIFIED）**：`pnpm e2e:ui --grep "inspector|sessions"`（本 spec `interaction.md` 改 `active` 后） | ☑ |
| 13 | 文档回写：`docs/16/19/26/28` 完成项、`KNOWN-LIMITATIONS`（终端无审批/审计、同步阻塞、PDF 降级、无多 pane/float、检索无索引/脱敏边界、fork 复制成本、symlink 不展示、附件/隔离边界）、parity 报告状态；acceptance 填证据并跑门禁 | `docs/16-ui-packages.md`、`docs/19-client-ux-flows.md`、`docs/26-workspace-and-conversations.md`、`docs/28-ui-module-map-and-skeleton.md`、`KNOWN-LIMITATIONS.md`、`research/client-dsh-parity-2026-11-01.md`、本 spec `acceptance.md` | 1–12 | `python3 scripts/docs-check.py`；`python3 scripts/spec-check.py`；`bash scripts/i18n-check.sh`；`zk capability --check`；**待工具落地**：`pnpm e2e:ui --grep "inspector|sessions"`、`python3 scripts/ui-interaction-gate.py` | ☑ |

## 2. 写作用域

- 本变更只写：`packages/contracts/{ipc,session}/**`、`packages/client/store-session-sqlite/**`、`packages/client/tool-session-query/src/lib.rs`、
  `crates/zk-kit-session/**`、`apps/desktop/src/ipc.rs`、`tools/zk/src/main.rs`（生成 `zk-composed` facade；**不手改 @generated 模板/产物**）、
  `packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、`packages/ui/ui-inspector/web/src/**`、`packages/ui/ui-sessions/web/src/index.tsx`、
  `apps/desktop/web/src/{App.tsx,state/controller.ts,protocol.ts,i18n.tsx}`、`apps/desktop/web/tests/{inspector-changes,workspace}.test.tsx`、
  `e2e/m1-serve.spec.ts`、文档（`docs/16/19/26/28`、`KNOWN-LIMITATIONS.md`、parity 报告）、本 spec 目录。
- 明确不写：能力面板（jobs/subagent/goal/todo/schedule/plan/present）与子 spec 目录、`packages/ui/ui-chat/**`、`packages/client/tool-schedule/**`、
  `apps/desktop/src/automation.rs`、server/admin、生成物（生成器写）、`docs/36`/`docs/40`（只读摘要）。
- 并行/重叠：1→2→3 串行；4/5 可并行（依赖 1）；6 依赖 2/4/5；7 依赖 2；8/9 依赖 7；10 依赖 6/7/8/9；
  11 依赖 10；12 依赖 6/10/11；13 收尾。`apps/desktop/web/src/{App.tsx,controller.ts,protocol.ts,i18n.tsx}`、
  `packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、`packages/contracts/ipc/src/lib.rs`、`docs/16/28` **只允许本 spec 写**（Phase 2a 后置排队，见 §5）。

## 3. 提交顺序

1. 任务 0a/0b（文档/顺序）→ 2. 任务 1（session 契约）→ 3. 任务 2（ipc 契约）→ 4. 任务 3（生成物）→
5. 任务 4（store v6）＋任务 5（检索单一实现）→ 6. 任务 6（host）→ 7. 任务 7（ui-kit）→ 8. 任务 8/9（inspector/sessions UI）→
9. 任务 10/11（app/单测）→ 10. 任务 12（e2e）→ 11. 任务 13（文档/证据/门禁）。

## 4. 验收前检查

- [ ] `intuition.md`/`intuition-walkthrough.md`（planned）已评审：P0 全覆盖、无 OPEN P0；实现开始时随 `interaction.md` 改 `active`
- [ ] `python3 scripts/spec-check.py`（draft warning 可接受）；`python3 scripts/docs-check.py`
- [ ] `cargo test -p zk-contract-session -p zk-contract-ipc -p zk-store-session-sqlite -p zk-kit-session -p zk-tool-session-query -p zk-desktop`
- [ ] `cargo test --workspace export_schema && git diff --exit-code`（新增 schema 后作基线，提交后验幂等）；`bash scripts/schema-check.sh`
- [ ] `zk compose --edition personal --profile default --channel official --check`（模板/Cargo 生成一致）；`zk capability --check`（无新增条目）
- [ ] `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh` exit 0
- [ ] `pnpm e2e` exit 0（m1-serve 追加右栏/会话/检索场景）；证据 `target/evidence/2026-11-01-client-side-panels/`
- [ ] *（依赖 `2026-11-01-ui-interaction-verification` 落地；当前 UNVERIFIED）* `pnpm e2e:ui --grep "inspector|sessions"` exit 0（本 spec `interaction.md` 改 `active` 后，P0/P1 ID 全绿）
- [ ] *（同上，当前 UNVERIFIED）* `python3 scripts/ui-interaction-gate.py` exit 0（契约 ID ↔ 标签 ↔ 证据；无 `waitForTimeout`/`force`/裸 `test.skip`）
- [ ] 无手改 `@generated`；无新增运行期加载（C2）；无第二份终端/检索/文件列表实现（C17）；C18 规则测试（fork/archive/搜索均 workspace 内）
- [ ] `cargo check --workspace --all-targets`；PR 只包含本 spec 范围；`acceptance.md` 已填证据路径

## 5. 跨 spec 顺序与共享文件

- 顺序：Phase 1 全部 → Phase 2a `client-composer-surface` → **本 spec** → 子 spec `specs/changes/2026-11-01-client-activity-panels/`（五件套 draft 已写：jobs/subagent/agent-team/goal/todo/schedule/plan 展示/present）。
- 本 spec 独占共享文件：`packages/contracts/ipc/src/lib.rs`、`packages/contracts/session/src/lib.rs`、`apps/desktop/src/ipc.rs`、
  `apps/desktop/web/src/{App.tsx,controller.ts,protocol.ts,i18n.tsx}`、`packages/ui/ui-kit/web/src/{types.ts,context.tsx}`、`docs/16/26/28`；
  **`e2e/m1-serve.spec.ts` 由 composer 先追加、本 spec 后追加（串行，不并行）**；**`tools/zk/src/main.rs` 由 composer 先改 kit facade，本 spec 后追加 kit 能力，activity 再追加 jobs/subagent/schedule facade（严格串行）**；docs/19、docs/28、KNOWN 的最终 owner 为本 spec（composer 只改其 §2/边界摘要）。
  子 spec 必须在本 spec 合并后排队改上述文件。
- Phase 2a 已占用 `packages/contracts/ipc/src/lib.rs`（attachment_save）、`App.tsx`、`controller.ts`、`protocol.ts`、`i18n.tsx`、ui-kit context：
  本 spec 必须在其合并后开始（tasks §0b 登记）。

## 6. 子 spec 占位（本 spec 不实现）

- `specs/changes/2026-11-01-client-activity-panels/`（五件套 draft，与本 spec 同批评审；预算 ≤14 文件）：右栏第 4 tab `activity`（docs/16 order 40）、jobs/subagent/agent-team/goal/todo/schedule 面板、
  chat turnTail（present 交付卡/改动卡）、plan 文本展示（非 plan mode）；预计约 14 文件。本 spec 的 docs/16 先预留 order 40。

## 7. 实现备注（2026-11-01，impl-settings）

- 契约：`Conversation.pinned/archived`（serde default）+ `FileReadKind`/`WorkspaceFileReadResult`/`SessionSearchHit`/`SessionSearchResult`；`SessionStore` 增加 rename/pin/archive/fork **默认实现**（Invalid，SQLite 覆盖），conformance 增加 legacy JSON 与默认实现断言。
- IPC：新增 `workspace.file_read`/`terminal`/`session.update`/`session.fork`/`session.search` 命令 + payloads（`WorkspaceFilesPayload` 扩展 path/depth/limit），`results::TerminalResult`；`WORKSPACE_FILE_READ` 等进入命令唯一性列表；`export_schema` 新增 10 个 schema（107 schemas）+ `schema-ts`，`schema-check` 绿。
- store v6：幂等加 `pinned/archived` 两列（user_version 6）；`list_conversations_in` 排序 `pinned DESC, updated_at_ms DESC, id DESC`；rename/pin/archive；fork 覆盖实现（事务复制消息、新 message id、不复制 pin/archive）。
- 检索：`zk_kit_session::search`（大小写不敏感 char 子串、snippet ±60、limit ≤50、含 archived），`tool-session-query` 改调该函数且输出完整 content；host `session.search` 返回 snippet。
- host：`workspace.files` 旧无参行为不变 + 懒加载 path/depth/limit；`workspace.file_read` 经 kit 新增 `safe_resolve_under`（只读、逐组件拒绝 symlink/`..`/绝对、不创建目录）；文本 256 KiB 截断/图片 8 MiB/base64/PDF 等诚实降级；`terminal` 复用 `zk-kit-tools::persistent`（open/send/close/list/status，输出 64 KiB 标记截断，错误码映射）；session update/fork/search 经 `session_err`（稳定错误码）。
- generator：`zk-kit-session` 加入 zk-composed facade（app 不直接依赖 kit；C4/C2）；`zk compose` 重生成 17 文件。
- UI：`@zeokeel/ui-inspector` 重写（懒加载树/预览/变更/用户显式终端/会话级 tab/宽度/折叠 localStorage/窄窗收起/键盘与 axe）；`@zeokeel/ui-sessions` 增加 `⋯` 菜单（rename/fork/copy id/pin/archive，流式中禁用 archive/fork）、显示已归档、内容搜索模式（250ms 防抖+旧请求丢弃）；app 增加 `updateConversation/forkConversation/copyConversationId/searchSessions`、controller `refreshActiveConversations`、`inspectorApi`（app 侧 Ajv 校验后交给包）。
- 偏差/说明：终端无 PTY（用户显式命令、无审批/审计）；预览不含 PDF 内嵌（CSP 不变）；检索无索引/高亮；`INS2-VIS-01` 像素基线需人工确认（UNVERIFIED），`INS2-A11Y-01` 用 axe+显式 ARIA 断言；真人用户测试 UNVERIFIED。
