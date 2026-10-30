# 2026-10-30 · 工作区/会话规范与 C19 评审流程评审记录

> 评审类型：只读文档评审（C19 / `docs/11` §9），非实现者。
> 评审对象：工作树 HEAD `f48b87f` 的未提交文档版本；关键文件 SHA256 见 §12。
> 本记录只新增本文件；**未修改任何规范正文、代码或其它文档**。

## 1. 范围

- `AGENTS.md`：C18/C19、当前阶段、§13 workflow；
- `docs/26-workspace-and-conversations.md`（下称 `26`）；
- `docs/11-testing-and-evidence.md` §9（评审流程/门禁/检查表）；
- `docs/13-risks-and-adr.md` ADR-058、ADR-014（对照）；`docs/15-open-questions.md` D66；
- `docs/19-client-ux-flows.md`、`docs/24-composition-and-runtime-spec.md` §7、`docs/12-roadmap.md` 的工作区引用；
- `docs/README.md` SSOT/术语/待定索引登记；
- 事实参考（只读，C15）：`research/dsh.md` 与已安装 DSH 包 `dsh-session`、`dsh-workspace`、`dsh-api-workspace-controller`、`dsh-api-workspace-files`、`dsh-agent-loop` 的 README；
- 现有代码/契约（对照差距）：`packages/contracts/{session,agent,tool,ipc}`、`packages/client/{store-session-sqlite,tool-fs,agent-loop}`、`apps/desktop/src/ipc.rs`、`apps/desktop/web/src/**`。

## 2. 评审者

- 评审者：`audit-arch`（非实现者；本轮只读评审，不修改规范正文）。
- 评审日期：2026-10-30。
- 独立性：本次评审未参与 `docs/26` 的写作；按 C19 要求，修复由实现者完成，评审者只负责复评。

## 3. 结论

**`CHANGES-REQUESTED`**

方向正确：workspace 作为持久实体、会话归属不可变、agent cwd/工具默认根 = workspace、工作区不是沙箱、首用不自动建会话——这些与 DSH 事实一致，也补上了 C18 的空白。但仍不能 APPROVED，原因按 `docs/11` §9.2 检查表：

1. **不满足 §9.1 文档内容要求**：`26` 缺"来源与许可、候选方案与取舍、与现有 ADR/契约的冲突点"三项（`docs/11:167-168`），尤其没有契约/包/角色变更清单；
2. **不可实现/会与 C3/C4 冲突**：`26` 的行为要求没有落到任何契约、包、角色与生成物；当前 `zk-contract-session/agent/tool/ipc` 均无 workspace 形参（§4 P0-1）；
3. **SSOT/ADR 冲突未处理**：ADR-014（JSON 数据目录含 `workspaces.json`）与 ADR-058/`24` §7/`26` §3（SQLite `workspaces` 表）同时 `accepted`，且 `05`/`24`/`26` 三处重复定义工作区规则（§4 P0-2/P0-4）；
4. **数据迁移不可执行**：`26:48` 的 `ALTER TABLE ... ADD COLUMN ... NOT NULL` 对已有数据的 SQLite 库不成立，且无 backfill/删除会话路径（§4 P0-3）；
5. **评审门禁本身可绕过**：`docs/11` §9.3 的"无 APPROVED 评审记录不得开 spec/代码"没有任何脚本/CI 检查，记录也缺"评审的文档版本/哈希"和独立性证据（§5 P1-7）。

因此 `26` 目前是**好草案**，但还不是可据以写 approved spec 的规范。

## 4. P0（阻塞：矛盾/不可实现/安全/许可/SSOT 冲突）

### P0-1 · 缺契约/包/角色/生成物影响清单，C3/C4/C10 无法实施

**问题**：`26:68-82` 只给 IPC 命令表，未说明这些行为由哪个契约、哪个包、哪个角色、哪些生成物承载；`26:6` 自称唯一权威，但没有"与现有契约的冲突点"。

证据（规范侧）：

- `26:61-66` 要求 Agent 绑定 `session_id + workspace_path`、loop cwd=workspace、所有事件带 `workspace_id + conversation_id`；
- `26:68-82` 新增 `workspace.*`、`session.create/list/messages`、`chat.send` 形状；
- `26:5` 只引用 DSH 与 C18/ADR-058；
- `docs/11:167-168` 要求文档至少含"与现有 ADR/契约的冲突点"；
- `docs/11:181` 要求"与契约/组合/角色/生成物所有权一致（C3–C5、C11）"。

证据（当前代码/契约差距）：

- `packages/contracts/session/src/lib.rs:11-20` 的 `Conversation` **没有 workspace_id**；`:51-60` 的 `SessionStore::create_conversation(&self, title)` 没有 workspace 参数；没有 `Workspace` DTO/仓储契约；
- `packages/contracts/agent/src/lib.rs:72-82` 的 `Agent::run(&self, input)` / `run_stream(turn_id, input, sink)` **没有 cwd/workspace/turn 上下文**；
- `packages/contracts/tool/src/lib.rs:35-41` 的 `Tool::call(&self, args)` **没有根目录/工作区上下文**；
- `packages/contracts/ipc/src/lib.rs:71-90` 的命令表只有 `chat.send/chat.cancel/session.*/settings.*/approval.decide/meta.get`，**没有 `workspace.*`**；
- 现有 `tool-fs` 用 `ZEOKEEL_DEMO_ROOT` 或进程 cwd 作为唯一 root（`packages/client/tool-fs/src/lib.rs:14-42`），且明确拒绝绝对路径/`..`/符号链接逃逸（`:58-83`）；`26:26` 却说绝对路径/`~`可显式使用——当前实现与目标规范不一致，必须定义迁移；
- `26` 未定义新包/角色：`docs/24:41-87` 的角色注册表无 workspace 角色，`docs/24:210-224` 的 writer 表也没有 workspace 生成的产物。

**为什么 P0**：按 C3/C4，新增能力必须有四件套包 + 契约单实例；按 C10，IPC 入参/出参必须有 JSON Schema。当前文档无法写出一个不违反 C4/C10 的 approved spec；实现者只能私自改契约（C12/C16 违规）。

**修法（最小）**：在 `26` 增加一节"契约/包/角色/生成物影响"，至少明确：

1. 扩展 `zk-contract-session` 还是新增 `zk-contract-workspace`：`Workspace` DTO、`Conversation.workspace_id`、`SessionStore` 新方法（`create_workspace/rename/remove/list` + `create_conversation(workspace_id, title)`）；`create_conversation` 签名变更若破坏 1.x，按 `docs/02` §7 升 major 或提供新方法 + 兼容层；
2. `agent`/`tool` 的 per-turn 上下文：如 `TurnContext { workspace_id, workspace_root, conversation_id }`，`Agent::run/run_stream` 与 `Tool::call` 如何拿到（是参数、还是包内注入）；明确"未绑定 workspace 时拒绝执行"；
3. IPC：命令名、入参/出参 JSON Schema、`zk.toml [[commands]]` 声明、`commands.gen.ts`/`ipc-commands.json` 生成；
4. 角色/包：新增/扩展哪个角色（建议 `store.workspace` 或并入 `store.session`）、UI 归属包（如 `ui-sessions` 内的 switcher 或新 `ui-workspace`）、`docs/24` §1.5/§3 与 `docs/02` §5 同步；若不需要新包，明确由哪个现有包承担；
5. 迁移/数据：`store-session-sqlite` schema v2 + `workspaces` 表 + `workspace_id` + turns/checkpoints 等 `24` §7 表的关系（见 P0-3）；
6. `docs/12` M1 交付物与 spec 任务拆分。

### P0-2 · ADR-014 与 ADR-058 / `24` §7 / `26` §3 的存储格式冲突

**问题**：ADR-014 `accepted` 说数据目录是 `$ZEOKEEL_HOME` 下 `config.json / workspaces.json / conversations`（JSON 文件），ADR-058 `accepted` 说"session store 增 `workspaces` 表与 `workspace_id` 外键"，`26:38-46` 直接给 SQLite DDL，`24:288` 说 SQLite WAL 唯一真源。两篇 accepted ADR 直接矛盾，且 `docs/11:179` 要求"旧决策冲突先标 `superseded`"。

证据：

- `docs/13:135`（ADR-014）：`$ZEOKEEL_HOME` 下 `config.json / workspaces.json / conversations`；
- `docs/13:495`（ADR-058）："session store 增 `workspaces` 表与 `workspace_id` 外键"；
- `docs/24:284-289`：SQLite WAL 唯一真源，表含 `workspaces`；
- `docs/26:35-46`：`CREATE TABLE workspaces ...`；
- `docs/24:284-289` 与 `docs/26:35-46` 都声称"冻结"，`docs/README:57-58` 又把"SQLite"归 `24`、"工作区"归 `26`，读者无法判断 schema 的真源。

**为什么 P0**：SSOT/ADR 冲突属于 `docs/11` §9.2 明确 P0；实现者会在"JSON 文件"与"SQLite 表"之间二选一，导致数据格式返工。

**修法**：二选一并回写引用：

- 推荐：ADR-014 改为"数据根与 `config.json` 等配置仍 JSON；会话/工作区存储按 ADR-053/ADR-058 用 SQLite"；或把 ADR-014 标记 `superseded-by-ADR-058`（仅存储部分）；`docs/13` §C 决策状态同步；
- `26` 作为 workspace 行为/schema 唯一权威；`24` §7 只保留"SQLite WAL/单实例锁/turn 原子性"等存储引擎不变量并链接 `26`；
- 删除 `05`/`19`/`24` 中重复的工作区规范段落（见 P0-4）。

### P0-3 · 数据模型 SQL 不可执行（已有会话的升级/回填/删除路径缺失）

**问题**：`26:48` 在已有 `conversations` 表上执行：

```sql
ALTER TABLE conversations ADD COLUMN workspace_id TEXT NOT NULL REFERENCES workspaces(id);
```

SQLite 不允许对已有行添加无默认值的 `NOT NULL` 列（会报 `Cannot add a NOT NULL column with default value NULL`；外键列同理），且没有 `user_version`/迁移顺序/回填默认 workspace 的步骤。当前 `store-session-sqlite` 只有 `CREATE TABLE IF NOT EXISTS conversations/messages`，无 schema 版本（`packages/client/store-session-sqlite/src/lib.rs:34-52`），已有用户库无法安全升级。`26:55` 要求"删除 Workspace 前必须先删除/导出其会话"，但 `26:75` 又规定有会话则拒绝移除，而 `26:68-82` 没有 `session.delete`/export/import 命令——有会话的工作区将永久不可移除，形成死路。ADR-058:496 还承诺"历史 demo 数据迁移到默认工作区"，`26`/`24` §7 均未给出迁移步骤或验收。

**为什么 P0**：这属于"不可实现/数据安全"；按 `docs/11` §9.2 必须有失败/回退路径，当前没有。若照文档直接迁移，线上/演示数据可能丢失或升级失败。

**修法（最小）**：在 `26` 增加"存储迁移（schema v2）"小节，明确：

1. 顺序：`PRAGMA user_version=1` 检测 → 事务内创建 `workspaces`（含默认 workspace 行）→ 重建 `conversations_new`（`workspace_id TEXT NOT NULL REFERENCES workspaces(id)`）→ 用默认/用户选择 workspace 回填 → 校验无孤儿 → rename/index → `user_version=2`；
2. 旧数据归属：默认 workspace（`default-workspace`）或首启引导选择；失败回滚/备份策略；迁移不可逆时的只读兜底；
3. `session.delete`（或 export + delete）命令与错误码，或明确允许"移除 workspace → 会话 ungrouped/归档"的替代语义（见 P1-2）；
4. 迁移验收：`当前 schema + 有会话` → 迁移 → 两工作区隔离 e2e；新装库路径；失败注入；
5. 与 `24` §7 的 turns/tool_calls/approvals/checkpoints 表关系（当前 `26`/`24` 都没有这些表的 DDL/迁移细节，而 `24:288` 已把它们列为 SQLite 真源）。

### P0-4 · 工作区规则三处重复，违反"唯一 SSOT"（`docs/11` §9.2 第 1 条）

**问题**：`26:6` 自称"工作区、对话归属、agent cwd 的唯一权威"，但 `24:284-289`、`05:145-150`、`19:33` 各自重复了规范正文（不是摘要+链接），`docs/README:109-110` 又重复术语定义；`24` §7 还单独"冻结"了 workspace 表。`docs/11:169` 明确要求"同一结论不在多篇文档重复（只引用编号）"。

证据：

- `26:6`；`24:284-289`；`05:145-150`；`19:33-34`；`README:109-110`；
- `docs/README:57-58` 把 SQLite 归 24、工作区归 26，边界重叠；
- `docs/24:284` 标题即"客户端会话存储（冻结）"，与 `26:35` "数据模型（M1 冻结）"重复冻结同一 schema。

**为什么 P0**：这是 reviewer 检查表第一条"唯一 SSOT"；重复规范会在三处漂移，且 `docs/26` 的变更无法同步保证。

**修法**：确定 `26` 为 workspace 行为 + schema 的唯一权威；`24` §7 改为"存储引擎不变量（WAL/单实例/turn 原子/JSONL 仅导入导出）见 24；工作区 schema 见 26 §3"；`05` §3.5 精简为一句"见 26"（可保留 3.5 编号但只放 link）；`19:33-34` 已基本是摘要，补"详见 26"；`README:109-110` 术语保留定义但删重复规范；`24` §7 中 workspace 表定义移入 26 或明确引用。

## 5. P1（应修）

### P1-1 · 归档/置顶/排序在规则、数据模型、验收里存在，但 API 完全缺失

- `26:15`（`workspaces.order`、`conversations.archived/pinned`）、`26:32`（归档/置顶/排序规则）、`26:100`（验收"归档/恢复会话"）要求 M1 有归档能力；
- `26:50` 注释又说"会话归档/置顶（M1 可后置）"；
- `26:68-82` 没有任何 `session.archive/unarchive/pin/reorder` 或 `workspace.reorder` 命令，`26:81` 的错误码也没有归档活动态错误（DSH 为 `workspace/session-active`，带 `stopActivity`）。

**修法**：二选一：① M1 做归档/置顶/排序：加 `session.archive/unarchive/pin/reorder`、`workspace.reorder`、`workspace/session-active`（含 activity 家族 turn/subagent/job/schedule 与 `stopActivity`）、验收与测试；② 明确 M1 不做：删 `26:100` 验收、把 §2.11 标"post-M1"，并从 §1 表移除 `archived/pinned` 承诺（或标 later），以免"规范承诺 vs 验收"打架。

### P1-2 · 移除有会话的 Workspace 是死路；且与 DSH "移除项目保留会话"的语义相反

- `26:30`：移除 = 只从注册表移除、不删目录；`26:55`：删除 Workspace 前必须先删除/导出会话；`26:75`：有会话则拒绝/需先处理；`26:98`：验收"移除有会话被拒绝"；
- 不存在 `session.delete`、export/import、ungroup 命令（`26:68-82`），也不存在"移除后会话 ungrouped"的表示（C18 又要求会话必须属于 workspace）。结果是：只要曾经建过会话，该 Workspace 就永远不能移除。
- DSH 事实：移除 project **不删目录/会话**，会话变为 ungrouped，可重新添加目录得到新 project（`dsh-workspace` README:12/77/168-169；`dsh-api-workspace-controller` README:25-27），与 `26` 的拒绝语义不同。`26:103-109` 只列了相同点，没有列这个差异。

**修法**：明确产品选择并写进 `26`：① 保持"必须归属"（C18 不变）：提供 `session.delete` + `session.export/import` + `workspace.remove` 在无会话时可用，并说明"要删工作区先删/导出会话"的用户流程；或 ② 允许移除并 ungrouped（需修改 C18 的"必须且只能"为"未移除时必须"、增加 ungrouped 状态/UI 恢复），否则不要承诺 DSH 同构。无论哪种，`26:30/55/75/98` 要统一，且补充错误码 `workspace/has-sessions` 之外的 `session/active` 等。

### P1-3 · 文件浏览/监听/符号链接边界没有定义（但被声明为工作区能力）

- `26:12` 把"文件浏览"列为 Workspace 作用域，`26:18/25` 说目录浏览/监听以工作区为根，`26:108` 引用 DSH workspace-files；
- `26:68-82` 没有任何文件浏览/读取/监听命令，`26:93-101` 没有文件浏览验收；`26:56` 只规定"workspace path 的符号链接解析后登记"，没有文件浏览的 symlink/`lstat`/越界规则。
- DSH 事实更严格：目录 listing/watch 必须留在 workspace root；文件 read 可按绝对路径（非沙箱）；文件读取拒绝最终 symlink，目录 symlink 只有解析到 root 内才允许 list；错误码有 `workspace-file/outside-workspace`、`not-regular-file`、`not-found`、`too-large` 等（`dsh-api-workspace-files` README:12/40/54/73）。

**修法**：要么把"文件浏览"明确标为 post-M1 并从 `26:12/18/108` 的能力承诺/验收里删掉；要么补 §"文件浏览 API/错误/边界"：命令、路径解析（workspace-relative vs explicit absolute）、`lstat`/realpath、最终 symlink 策略、目录 symlink 仅 root 内、watch 目录范围、TOCTOU/并发写、错误码，以及越界仅在 UI 提示不阻断（弱管控）的验收。建议先参考 DSH 的 typed error 与 listing 规则，避免后续安全评审返工。

### P1-4 · agent cwd/工具根与现有实现矛盾，缺少迁移计划

- `26:25/61/65` 要求 agent loop cwd 与 fs/shell/搜索/附件/诊断默认根 = workspace；`26:26` 允许显式绝对路径/`~` 并在 UI 提示"在工作区外"；
- 当前 `tool-fs` 是**单 root、进程级**（`ZEOKEEL_DEMO_ROOT`/`current_dir`，`tool-fs/src/lib.rs:14-42`），且**拒绝绝对路径、`..`、symlink 逃逸**（`:58-83`），与 `26:26` 的"可显式绝对路径"相反；
- `agent` 契约 `run(input)`、`tool` 契约 `call(args)` 无 workspace 上下文（`agent/lib.rs:72-82`、`tool/lib.rs:35-41`）；
- `agent-loop` 当前 prompt 与工具调用完全不含 workspace（`agent-loop/src/lib.rs:19-36,44-60`）。

**修法**：在 P0-1 的契约变更中明确 per-turn workspace 上下文；在 `26` 增加"当前实现差距与迁移"：`tool-fs` 从环境 root 改为 workspace root；绝对路径/越界策略与弱管控声明一致；旧 `ZEOKEEL_DEMO_ROOT` 迁移/废弃；并加对应 conformance/负向测试（越界、symlink、绝对路径、workspace 缺失）。

### P1-5 · `session.messages`/`chat.send` 没有 workspace 参数与跨工作区错误，隔离只在 UI 层

- `26:24/27` 要求列表/消息/归档按 workspace 隔离；`26:78` `session.messages {conversation_id}` 只"校验归属"却不传当前 workspace；`26:79` `chat.send {conversation_id,text}` 同样不校验当前 workspace；`26:81` 没有 `session/workspace-mismatch`/`session/not-found` 等错误；
- 攻击/UI bug 可以拿 A 工作区的 conversation_id 在 B 工作区上下文里读/发，违反 C18"消息不跨工作区"。

**修法**：在 `26` §5 明确每个会话命令都携带 `workspace_id`（或 "current workspace" 上下文）并与 `conversation.workspace_id` 比对；定义 `session/workspace-mismatch`、`session/not-found`、`workspace/path-unavailable` 在 chat 路径的映射；加负向测试。

### P1-6 · 并发/切换/流式语义未定义（注册表并发、切换时活动 turn）

- `26:28` 说并行会话"各自有独立 agent/loop/文件锁"，但没定义"文件锁"是什么（session DB 锁？workspace 级锁？工具写冲突？）；`26:90` 说切换 Workspace 必须清空流式/草稿，但没定义活动 turn 是取消、后台继续还是拒绝切换；
- DSH 的 workspace 注册表串行化 mutation，archive 的活动检查与写入不是原子（`dsh-workspace` README:96-97/103/172），且活动家族包括 turn/subagent/job/schedule；
- 本规范只有"同一会话一个 turn"与"归档拒绝活动 turn"，没有 Workspace 创建/删除/重命名与 active turn/streaming 的先后关系。

**修法**：定义切换策略（建议：切换即取消当前 turn，与 `26:90` "清空流式"一致；或显式"后台继续"并在 UI 标后台）；定义 workspace mutation 的串行化（按 workspace id/全局队列）；定义"active turn/stream"错误码；补 e2e：切换/切换回/流式中删除/重命名。

### P1-7 · 评审门禁无法强制，且记录缺"版本/独立性/P0-P1 关闭证据"

- `docs/11:187-191` 规定"没有评审记录（APPROVED）的新方案 → 不得开 spec/代码；记录与 spec/PR 不对应 → 拒绝合并"；但 `scripts/docs-check.py:108-118` 只检查 SSOT 登记、`:166-181` 只检查 ADR/D 编号；`scripts/spec-check.py:28-54` 只检查模板/状态，**没有任何 review/APPROVED/评审者检查**。C19 门禁目前纯靠自觉。
- `docs/reviews/README:5-6` 只要求"范围、评审者、结论、P0/P1/P2、复评结果、关联 spec"，**不要求**：被评审文档的 commit/路径+SHA256、评审者与实现者身份分离的证据、每条 P0/P1 的修复证据/复评状态、评审后文档再变更则失效的规则、同一天同 topic 的重名处理。
- `AGENTS.md:54`（C19）写"由非实现者只读评审**并修复** P0/P1"——评审者不应改规范正文（与本任务"非实现者角色"冲突），修复应归属实现者；`docs/11:173` 已写"实现者修 P0/P1 后由同一评审者复评"，两处措辞需要统一。

**修法**：
1. `docs/reviews/README` 增必填字段：评审对象 commit+每文件 SHA256、评审者、与实现者关系/独立性声明、P0/P1 清单及逐条状态、复评日期/结论、关联 spec id；文件命名加 `<topic>-<rev>` 避免同日冲突。
2. 增加轻量门禁：`scripts/review-check.py` 或 `spec-check` 扩展——有 `docs/reviews/*.md` 且结论 `APPROVED`，其中"关联 spec"与 `specs/changes/<id>` 对应；spec `proposal.md` 必须引用评审记录路径；未 APPROVED 的 spec 状态不得 `approved`（或 CI 警告/失败）。
3. `AGENTS.md:54` C19 改为"由非实现者只读评审；实现者修复 P0/P1；同一评审者复评通过后，才可写 approved spec"。
4. 评审后文档若变更（hash 不匹配）需要复评，否则记录失效——写入门禁。

### P1-8 · `26` 不满足 `docs/11` §9.1 的内容要求（来源/许可、候选方案与取舍、冲突点），且无状态标记

- `docs/11:167-168` 要求至少包含"来源与许可、候选方案与取舍、与现有 ADR/契约的冲突点、失败/回退条件、验收方式"；
- `26` 有验收（§7）、部分失败路径（§3/§6），但**没有**：候选方案与取舍（例如为什么不用 DSH 的 session meta.cwd 单点、为什么选 SQLite workspace 表、为什么拒绝移除会话、为什么 workspace 实体而非纯目录）；来源/许可（DSH 包许可证未记录，只有 C15 一句"参考"）；冲突点（ADR-014/C4/05 prefix cache 均未列）；
- `26` 无"状态：草案/待评审/M1 冻结"横幅（§3/§5 却写 M1 冻结），与 `docs/27:4` 的"实现目前对应 M1"写法不一致。

**修法**：在 `26` 增加 `## 0. 状态与来源`（状态：草案，待 C19 评审；参考 DSH 包与许可证；只作事实参考）与 `## 2.5 候选方案与取舍`（至少列出 2–3 个被否方案与理由、失败/回退条件）；在"冲突点"中逐条列 ADR-014、contract session/agent/tool、05 §5.5 prefix cache、C18；验收加证据命令/路径（见 P1-9）。

### P1-9 · UI/UX 与验收证据不足（`docs/11` §9.2 的 UI 检查项）

- `26:84-91` 的 UI 只有文字要求；缺六态（空/加载/错误/离线/流式中/已取消）、a11y、i18n、明暗主题、mock 与真实后端边界、窄窗键盘细节；`docs/11:184` 要求这些；
- `26:93-101` 验收没有命令/截图路径/证据外置，`docs/11:185` 要求"证据计划可执行（命令/截图/门禁）"；
- 与 `19`/`27` 的关系未写（`26` 应链到 `19` 的页面状态与 `27` 的视觉/评审证据）。

**修法**：在 `26` §6 增加"与 19/27 的关系、六态矩阵、a11y/键盘/i18n/主题要求"；§7 增加每条的验证命令/截图目录（如 `apps/desktop/web/screenshots/workspace-*`、axe.json、Playwright project），或明确把 UI 验收委托给 `19` 并只留链接。

### P1-10 · 状态标注与实现差距清单不足：`24` §7/`05` §3.5/`19:33` 像"已实现"

- 正确标注：`AGENTS.md:27`（M1 工作区/会话归属尚未完成）、`docs/12:78/85`（M1 交付/退出条件）、`26:93` 未勾选验收——这三点说明没有假装完成；
- 但 `24:284` 标题"客户端会话存储（冻结）"、`05:145-150`"工作区绑定"、`19:33-34`"工作区前置"都没有"M1 待实现/当前代码未实现"的标记；`24:288` 列出 `workspaces/...` 表，当前 `store-session-sqlite` 只有 conversations/messages（`lib.rs:34-52`），读者可能误以为已落地；
- 缺一份"现有代码 → 目标"的差距/迁移清单（contract/store/UI/IPC/auto-create 兼容路径），`26` 未提供；当前 `session.create` 无 workspace（`apps/desktop/src/ipc.rs:54-59`），`chat.send` 在无 conversation_id 时**自动创建会话**（`apps/desktop/src/ipc.rs:132-141`；前端也会在无 active 会话时自动 `session.create`，`apps/desktop/web/src/state/controller.ts:185/231`），这与 C18"无工作区不建会话"相反；`docs/26` 没有把"移除自动建会话 fallback"列为任务/验收。

**修法**：在 `24` §7、`05` §3.5、`19` §2 顶部各加一行"M1 规范，代码未实现；实现清单见 `12` M1 与 `26` §9"；在 `26` 增加 §9"现有实现差距与迁移"（contract/store/ipc/ui/legacy auto-create/DB migration）；在 `12` M1 任务/验收中明确"无 workspace 时 chat.send/session.create 必须失败（删除 auto-create fallback）"。

## 6. P2（建议）

1. **字段命名不一致**：`26:12/15` 用 `default/order/archived/pinned`，`26:42-43/51-52` 用 `is_default/sort_order/archived_at_ms/pinned_at_ms`；`26:5` 的 workspace 字段应是 schema 的稳定名。修法：以 DDL 为准统一术语表与 §1 表。
2. **DTO/JSON Schema/生成物未定义**：`26:72-79` 只给命令表，`Workspace`/`Conversation`/`workspace_id` 的 JSON Schema、ts-rs 导出、`ipc-commands.json` 条目未列；按 C10/`24` §3/§4 补。
3. **DSH 引用不完整/无许可记录**：`26:105-109` 的引用基本正确，但遗漏 `initializeDefault` 的资格条件（空注册表且无 live/persisted/archived session）、可能返回 `undefined`、Documents 平台策略（Linux 需 `xdg-user-dir`；`dsh-workspace` README:65-69；`dsh-api-workspace-controller` README:34-36）、默认标题取自请求目录名而非 canonical、移除项目不删会话等差异；建议在 §8 加"与 DSH 的差异"并记录 DSH 包许可证；`26:29` 的默认路径 `Documents/ZeoKeel/default-workspace` 需补平台策略与"不可用"判定。
4. **SSOT 计数过期**：`docs/README:107` 仍写"D1–D65 + T1–T21"（现为 D1–D66；`docs/15:158` D66）；`AGENTS.md:303` 仍写"D1–D65"；`docs/15` T 索引到 T21 检查；修 D66/计数或用生成器。
5. **首用/重复路径语义内部矛盾**：`26:30` 创建"目录必须在本地、可读写"、`26:73` 却"目录不存在则创建"；`26:30`"重新登记（同路径幂等）"与`26:73`"重复路径报错"需明确（已存在注册 → 报错还是返回既有？移除后重加 → 新 id？）；`26:73` 与 DSH"create 只接受现有目录、重复返回 existing project"（`dsh-workspace` README:51）不同，需写产品选择。
6. **`inject: ["session","workspace"]` 无服务/契约定义**：`docs/16:64` 的 UI 包清单声明注入 workspace 上下文，但 `24` §1.5/契约清单没有 workspace context 服务与包/角色；M1 若使用该清单，需要定义；否则在 `16` 标 post-M1。
7. **Workspace id 方案与默认 title 未定义**：`26:12` 只说稳定 id；建议明确 UUIDv4/ULID、不可猜测/本地生成；默认 title 规则（目录名 vs 用户输入）；`workspace.create` 缺 title 时的行为。

## 7. 与 DSH 事实的核对（只作事实参考，C15）

| `26` 的引用 | DSH 事实 | 结论 |
| --- | --- | --- |
| `26:105` session 创建带 `meta.cwd`，事件日志可重建 | `dsh-session` README:12/35 | 正确 |
| `26:106-107` controller create/rename/remove/reorder、session archive/pin/sort、`workspace/follow`、`initializeDefault` 不建 session/message | `dsh-api-workspace-controller` README:11/25-36；`dsh-workspace` README:65-69 | 基本正确；遗漏 `initializeDefault` 资格/undefined/Documents 策略/默认标题取自请求名，以及 archive 的 `stopActivity`/`workspace/session-active` 与活动家族 turn/subagent/job/schedule（`dsh-workspace` README:103） |
| `26:108` 目录浏览/监听以 workspace root 为界，读取可绝对路径 | `dsh-api-workspace-files` README:12/40/54/73 | 正确；但本文没有实现这些边界/错误/符号链接规则（P1-3） |
| `26:109` agent-loop `agents[].cwd`、创建/resume 绑定、取消保留流式文本 | `dsh-agent-loop` README:12/53/75 | 正确 |
| `26:30/55/75/98` 移除有会话的 workspace 被拒绝 | DSH：移除 project 保留目录/会话，会话变 ungrouped（`dsh-workspace` README:12/77/168-169） | **有意/无意的差异**；本文未说明这是产品选择，且与 C18"必须属于"组合后不可移除；必须明确（P1-2） |
| `26:30/73` 创建目录不存在则创建 / 同路径 | DSH：create 只接受已存在 fully-qualified directory；missing/file/relative 拒绝；重复路径返回 existing（`dsh-workspace` README:51） | 产品选择需明确，当前文字内部矛盾（P2-5） |

结论：**没有把 DSH 的"参考"误当作依赖**（C15 注解在 `26:103`），但对 DSH 事实的引用是"简化版"，关键差异（remove 语义、initializeDefault 资格、symlink/file 边界）没有列出，容易被误读为同构。

## 8. 问题 2：与现有代码/契约的差距是否被正确标注为 M1 待实现？

**部分正确，需补差距清单与状态标记。**

- 已正确标注：`AGENTS.md:27`（尚未完成 M1 工作区/会话归属）、`docs/12:78/85`（M1 交付与退出条件）、`26:93-101`（验收未勾选）、ADR-058 自身定位为 M1 规范。
- 未标注/会误导：`24:284-289` 与 `05:145-150` 用"冻结/绑定"的现在时；`19:33-34` 以"工作区前置"直接进入页面规范；当前实现明显未满足（见下表），但文档没有"M1 待实现/当前代码差距"章节。
- 当前实现差距（只列事实，不在本评审修复）：

| 目标（`26`） | 当前代码/契约 | 状态 |
| --- | --- | --- |
| `Conversation` 有 `workspace_id` | `packages/contracts/session/src/lib.rs:11-20` 无该字段 | 未实现 |
| `SessionStore` 以 workspace 建会话 | `contracts/session:54` `create_conversation(title)` 无 workspace | 未实现/需改契约 |
| agent cwd = workspace | `contracts/agent:72-82` 无 cwd/workspace 上下文 | 未实现/需改契约 |
| tool 默认根 = workspace | `contracts/tool:35-41` 无上下文；`tool-fs:14-42` 进程 root、拒绝绝对路径 | 未实现/行为相反 |
| IPC `workspace.*`/`session.archive` | `contracts/ipc:71-90` 无这些命令 | 未实现 |
| SQLite `workspaces` + `workspace_id` + turns 等 | `store-session-sqlite/src/lib.rs:34-52` 仅 conversations/messages；无 user_version/迁移 | 未实现 |
| 无 workspace 不建会话 | `apps/desktop/src/ipc.rs:132-141` 无 conversation_id 时自动 create；前端 `controller.ts:185/231` 同样自动建 | **与 C18 相反，需迁移** |
| UI workspace switcher | `apps/desktop/web/src` 无 workspace 切换到（`rg workspace` 无业务命中） | 未实现 |
| docs/24 §7 的 turns/tool_calls/approvals/checkpoints | 当前 store 无这些表；`26`/`24` 也未给 DDL | 数据模型缺口 |

**结论**：仓库层面没有"假装完成"（AGENTS/12/26 都承认未做），但 `24`/`05`/`19` 需要状态标记，`26` 需要"差距与迁移"清单，且当前 `chat.send` 自动建会话的兼容路径必须在 spec 中明确移除/迁移，否则实现会违背 C18。

## 9. 问题 4：评审流程本身是否可执行、是否有绕过点？

**人工流程可执行，但门禁不可自动执行，且有几个绕过点。**

- 可执行部分：`docs/11` §9.1 的步骤（落文档 → SSOT 登记 → 独立评审 → 修复复评 → spec → 收尾）定义完整；`docs/reviews/README` 给了最小格式；C19/`27` §5 都引用同一流程；本记录即按此流程产生。
- 绕过点/缺口（按严重度）：
  1. **无自动门禁**：`docs-check.py` 不检查 `docs/reviews/`；`spec-check.py` 不校验评审记录与 `approved` 的对应（`scripts/docs-check.py:108-118/166-181`、`scripts/spec-check.py:28-54`）；"没有 APPROVED 记录不得开 spec/代码"只能靠人遵守。
  2. **无评审对象版本**：记录不要求 commit/文件 hash；文档评审通过后若再改，APPROVED 仍然"有效"（无法发现）。本记录已在 §12 主动记录 hash，但流程未强制。
  3. **独立性不可验证**：`docs/11:170` 只要求"非实现者"；`docs/reviews/README` 只要求"评审者"字段，无身份/角色/与实现者关系声明；实现者可以自写一条"评审者：someone"的记录。
  4. **P0/P1 关闭无证据要求**：`docs/11:173` 只说"修复后复评"，记录格式不要求逐条 P0/P1 的修复证据/验证命令；复评结论可以只写"通过"。
  5. **AGENTS C19 措辞**：`AGENTS.md:54` 写"由非实现者只读评审并修复 P0/P1"，与本任务"非实现者角色、不改规范正文"冲突；应改为"评审者只评审，实现者修复"（与 `docs/11:173` 一致）。
  6. **评审者不可用/记录重名**：没有 fallback（同一评审者必须复评）；`YYYY-MM-DD-<topic>.md` 同日同 topic 会覆盖；需要 `<rev>`/序号。
  7. **"新方案"定义模糊**：C19 覆盖"库/设计系统/DSH 模式/颜色/品牌规则等"，但"bug 修复/重构/紧急修复"是否豁免未写；实现者可以以"不是新方案"绕过。
  8. **C16 approved 与 C19 APPROVED 的关系**：`docs/README` 已注明 spec-check 仅结构检查，C16 approved 由评审执行；但 `spec-check.py` 对 `approved` 完全放行（`:49-52`），建议至少要求 proposal 里出现 `review: docs/reviews/...` 链接（存在即可，不校验内容）。

**修法**：见 P1-7；优先做"记录模板字段 + spec proposal 必填 review 链接 + 轻量 review-check"三件事即可把绕过成本提高；不必上重型审批系统。

## 10. 问题 3：缺哪些验收/失败路径/迁移/边界（对照表）

| 边界/失败路径 | 文档现状 | 缺口 → 建议 |
| --- | --- | --- |
| 工作区不存在 | `26:57` 目录不存在→只读/不可发新 turn；`26:89` 横幅；`26:99` 验收 | 缺 chat/session.create 的具体错误映射与 UI 恢复流程；`workspace/path-unavailable` 未进 `26:81` 的命令错误说明；补 e2e |
| 路径符号链接 | `26:56` 仅 registration "符号链接解析后登记" | 缺 `fs.realpath` 唯一性、Windows case/UNC、目录 symlink/list、文件 read 最终 symlink、watch root、TOCTOU（见 P1-3） |
| 删除有会话 | `26:30/55/75/98` | 拒绝 + 无 `session.delete`/export = 永久不可移除；与 DSH ungrouped 语义不同（P1-2） |
| 并发 | `26:28` 每会话单 turn；"独立文件锁"未定义；`26:90` 切换清空 | 缺 workspace mutation 串行化、切换/流式中 turn 的取消/后台语义、文件写冲突（P1-6） |
| agent cwd | `26:25/61/65` 有规则 | 契约/实现无入口；tool-fs 行为相反；见 P0-1/P1-4 |
| 工作区不是沙箱 | `26:17-18` 明写；`26:65` 越界显式并标注 | 方向正确；但当前 tool-fs 拒绝绝对路径（更严），需迁移/统一；补"越界在 UI/审计可见"的验收 |
| 跨工作区隔离 | `26:24/27` 规则；`26:78` 仅 conversation_id | 缺 workspace_id 参数/`session/workspace-mismatch`/负向 e2e（P1-5） |
| 迁移 | `ADR-058:496` 承诺历史 demo 数据迁移；`26` 无步骤 | 无 schema version/backfill/rollback/验收（P0-3） |
| 首用/默认工作区 | `26:29/87` | 缺 DSH 式资格（无 session）、`initializeDefault` 失败返回 undefined/平台 Documents 策略、默认 title（P1-8/P2-3） |
| 归档/置顶 | `26:15/32/100`，`26:50` 又说 M1 可后置 | 规则/验收/API 三处不对齐（P1-1） |
| 工作区重命名/排序 | `26:30/15` | `workspace.reorder` 不在 API；`workspaces.order` 无更新命令（P1-1） |
| 证据/验收 | `26:93-101` 只有 checklist | 缺命令/截图/axe/Playwright 证据路径（P1-9） |

## 11. 复评结果

- 本轮：首次评审，`CHANGES-REQUESTED`。
- 复评条件：实现者修复 P0-1～P0-4 与 P1-1～P1-10 后，由**同一评审者**（audit-arch）按本文件逐项复评；复评时需提供：
  - `26` 的新版本 SHA256 + 变更摘要；
  - 每个 P0/P1 的修法落点（文件:行）与验证命令（`docs-check`/`spec-check`/相关脚本）；
  - `docs/reviews/README` 要求的字段（含关联 spec id）；
  - 若 `26` 在复评通过后再次改动，本记录失效，需重新评审。
- 复评结论只有两种：`APPROVED`（P0 全关、P1 全关或有用户接受的例外）或再次 `CHANGES-REQUESTED`；本文件不预填"通过"。

## 12. 关联 spec / 版本证据

- 关联 spec：**尚未创建**；APPROVED 后建议 `specs/changes/2026-10-30-workspace-scoped-conversations/`（C16 五件套），proposal 必须引用本记录，acceptance 必须回链本记录与 P0/P1 关闭证据。
- 评审对象（当前工作树 HEAD `f48b87f`）：
  - `AGENTS.md` `b8d15a890f7da64721235f3292b324b7fe67ed69feb68dbe2965d0efb60200bc`
  - `docs/26-workspace-and-conversations.md` `2185b49e33a83b31e41b09191b2b41f794e93ab6fffce1b38315edefaef8c6d6`
  - `docs/11-testing-and-evidence.md` `57bf77d9bf1c00d79513be6a222ed2b25eba4bcaf8b950b01ef82eeb765f3469`
  - `docs/13-risks-and-adr.md` `a38d33e593f082b57d859e8fdc61c91e999c1cbe77e7ba9af11578b0910b590c`
  - `docs/15-open-questions.md` `280cc564ff1b9f1bbc7d2381ab0802369e13f1cd95da2f1dc1efc0ad1dff08d4`
  - `docs/24-composition-and-runtime-spec.md` `b8a99cee30a21b9bb5e58f32bceaa411d0d9e8dee2b57c456d0077c8ecccce81`
  - `docs/05-client-runtime.md` `f7c5f187070dd47d14db9161d4393ce95a6bb3fc9e88c5b5b68bd91adc6d33c3`
  - `docs/19-client-ux-flows.md` `4d8dc97032665b344fab1d9223f69abf82f317302b71053b59c6dea59b1c2479`
  - `docs/README.md` `c535b9f4551eedcee89b407eed07b79affac4e7db68d333ec1c157c564ef4473`
- 门禁复核：`python3 scripts/docs-check.py` = 0 errors/0 warnings（32 files）；`python3 scripts/spec-check.py` = 0 errors/0 warnings。两者均**不校验**本评审流程（见 §9）。
- 本记录只新增 `docs/reviews/2026-10-30-workspace-and-process.md`；未改任何其它文件。

---

## 复评（2026-10-30，第二轮）

> 复评对象：本轮修复后的工作树（`docs/26` 重写、`13` ADR-014/058、`24` §7、`05` §3.5、`19` §2、`README`、`11` §9、`review-check.py`、AGENTS C18/C19、spec 模板）。
> 复评方式：只读文档 + 代码对照 + 门禁执行；未改规范正文/代码，只追加本节的评审记录。
> 版本证据（SHA256）：`26` `872d64b7…`；`13` `b20d8851…`；`24` `bdcecc80…`；`05` `3ef8d213…`；`19` `39346fd8…`；`README` `3bc66f71…`；`11` `14b40b03…`；`AGENTS` `147a538a…`；`review-check.py` `361fc91f…`；`02` `0fd1905f…`；`15` `280cc564…`。

### R0. 复评结论

**`CHANGES-REQUESTED`（仍不能 APPROVED）**

上一轮 4 个 P0 中，P0-2（ADR 冲突）、P0-4（SSOT 重复）**已关闭**；P0-1（契约/包/角色/生成物）**文档层基本关闭**；P0-3（迁移）**部分关闭但方案仍不可执行**。另发现一个**新的 P0**：`reassign_to` 会修改会话的 `workspace_id`，与 C18/ADR-058/`26` 自身"`workspace_id` 不可变、不迁移历史消息"直接矛盾。

**文档能否据以写 approved spec**：**还不能**。方向、契约/包/角色影响、UI/验收、DSH 差异已足够作为 spec 输入，但 P0-A（归属不可变 vs reassign）必须先解决；P1-A～P1-F 中至少迁移与契约清单两项建议在 spec 前修，否则 spec 会把错误迁移/旧契约面固化。

### R1. 上一轮问题关闭情况

| 上轮项 | 复评状态 | 依据 |
| --- | --- | --- |
| P0-1 契约/包/角色/生成物影响 | **RESOLVED（文档层）** | `26:99-116` 已定义 `Workspace`/`WorkspaceStore`/`SessionStore` 方法、`TurnContext`、agent/tool 签名、包/角色、IPC/生成物；仍有 P1-B/P1-C 的契约清单与归属细节 |
| P0-2 ADR-014 vs ADR-058/24/26 | **RESOLVED** | `13:133-137` ADR-014 改为"配置 JSON；会话/工作区按 ADR-058 SQLite，`workspaces.json` 不再使用"；ADR-058:491/494 同步 |
| P0-3 迁移不可执行 | **PARTIAL → P1-A** | `26:86-97` 有了 v2 步骤/回滚/测试承诺，但 SQL/备份/外键细节仍会失败（见 P1-A） |
| P0-4 工作区规则重复 SSOT | **RESOLVED** | `26:6-7` 唯一权威；`24:284-292`、`05:145-150`、`19:32-34` 改为摘要+链接；`README:57-58` 行 27 指向 26；`24` §7 只留引擎不变量 |
| P1-1 归档/置顶 API 缺失 | **RESOLVED（scope-out）** | `26:33`、`AGENTS:53`、ADR-058:494 明确 `archive/pin/order` post-M1；`26:83` 不建字段；验收不再承诺归档；API 不列 |
| P1-2 移除有会话死路 | **PARTIAL → P0-A** | `26:31/84/104/125` 用 `reassign_to` 解决死路，但与"`workspace_id` 不可变"（`26:14/24`、`AGENTS:53`、ADR-058:491）冲突；默认 workspace/reassign 目标规则仍缺 |
| P1-3 文件浏览/symlink 边界 | **RESOLVED（scope-out）** | `26:35` 文件浏览 post-M1、`26:162` DSH 差异；但 §1/§2 的"工具可访问工作区外"与 §2.13/验收仍冲突（P1-C） |
| P1-4 agent/tool cwd 与实现矛盾 | **PARTIAL** | `26:110-113` 定义 `TurnContext`/签名，`26:169` §9 列迁移；当前 `tool-fs` 行为/契约旧面仍需 spec 落地（施工阶段），但文档的 outside 策略自相矛盾（P1-C） |
| P1-5 跨工作区隔离 | **RESOLVED** | `26:28/128-130` 所有会话命令带 `workspace_id`，`session/workspace-mismatch`；`chat.send` 不再 auto-create（`26:132`） |
| P1-6 并发/切换 | **PARTIAL（细节后置）** | `26:32/133` 定义切换取消活动 turn、mutation 全局串行；具体锁/取消实现属施工阶段（`26:169-171`） |
| P1-7 prompt/prefix cache 冲突 | **RESOLVED** | `26:55/113` 明确 workspace 路径/时间不进 system prompt，放 user message 尾部/工具上下文；与 `05` §5.5 对齐 |
| P1-8 11 §9.1 内容要求 | **RESOLVED** | `26:3-5` 状态/来源/MIT；`26:37-57` 候选方案与冲突；`26:146-156` 验收与证据；DSH 差异 `26:158-163` |
| P1-9 UI/六态/证据 | **RESOLVED** | `26:136-156` 六态/a11y/i18n/明暗/窄窗/mock-real/截图/axe/命令 |
| P1-10 状态标注/实现差距 | **RESOLVED** | `24:289`、`05:147`、`19:33` 明确 M1 代码未实现；`26:165-172` 实现差距/迁移任务；AGENTS:27/12 M1 保持未完成 |
| P1-7（流程）评审门禁 | **PARTIAL → P1-E** | `scripts/review-check.py` 已存在并被 AGENTS §6/§8 引用；但仍可绕过且与 `11:189` 声明不一致（P1-E） |

### R2. 仍存在的 P0

#### P0-A · `reassign_to` 与"会话归属不可变"直接矛盾（C18/ADR-058/26）

**证据**：

- `26:14`："`workspace_id` 创建后不可变"；`26:24`："创建后 `workspace_id` 不得修改；换目录 = 新建会话/工作区，不迁移历史消息"；
- `26:31`：移除时"有会话时必须 `reassign_to` 到另一 Workspace，事务内迁移会话后移除"；`26:84`、`26:104`、`26:125`、`26:152` 同；
- `AGENTS:53` C18："会话 `workspace_id` 不可变"；`13:491` ADR-058："`conversations.workspace_id` 不可变"；`24:292`："`conversations.workspace_id` 必填、不可变"；
- `26:57` 用"reassign 迁移会话"解释，但"迁移会话到另一 Workspace"必然改写 `workspace_id`，与上述四条相抵触。

**为什么 P0**：C18 是 binding 约束，ADR-058 是 accepted 决策；同一规范同时规定"不可变"和"移除时改归属"，实现者无法同时满足。这不是措辞问题，而是数据模型/API 语义冲突；approved spec 无法写出不违反 C18 的 `workspace.remove`。

**修法（二选一，并回写 C18/ADR-058/24/26）**：

1. **保留 reassign（推荐，解决死路）**：把 C18/ADR-058/`24:292`/`26:24` 的"不可变"改为"**创建后不可变；唯一例外是用户显式 `workspace.remove(reassign_to)` 的迁移，须二次确认/审计，且不用于普通换目录**"；`26:24` 的"不迁移历史消息"改成"普通路径不迁移；移除 Workspace 的 reassign 是唯一例外"；`26:5` 概念表同步。
2. **禁止 reassign**：移除 `reassign_to`，改为"有会话必须 `session.delete`/export 后才能移除"（需新增 M1 会话删除/导出命令），或允许 ungrouped（同时改 C18，不推荐）。
   同时补默认 Workspace 规则：`workspace/default` 错误码何时触发（默认 Workspace 是否可移除/是否可作为 reassign 目标），reassign 目标必须存在、非自身、可写，错误码 `workspace/invalid-reassign`/`workspace/default`。

### R3. 仍存在的 P1（文档问题）

#### P1-A · schema v2 迁移仍不可执行：`DROP TABLE` 会清空 messages，备份/占位/默认路径也不成立

**证据**：

- `26:86` 标题称"可执行"；`26:88` 只复制 DB 主文件（WAL 模式下 `-wal` 未 checkpoint 时备份不完整）；`26:90` 插入默认 Workspace 的 `path=default-workspace` 是相对名，与 `26:65` "canonicalize 后的本地绝对路径"、`26:82` 规范化冲突；`26:92` `?default_workspace_id` 是未绑定占位；`26:93` `DROP TABLE conversations`；
- 当前 `messages` 表 `conversation_id ... REFERENCES conversations(id) ON DELETE CASCADE`（`packages/client/store-session-sqlite/src/lib.rs:41-49`）；SQLite 在 `foreign_keys=ON` 下 `DROP TABLE conversations` 会先隐式删除父行并级联删除 messages（本地 `sqlite3 :memory:` 实测：DROP 后 `messages` 计数为 0）。`26:95` 的"消息数量不变"校验会因此失败并回滚——迁移对任何有消息的旧库**必然失败**；若事务/备份实现不严，存在数据风险。
- `26:86-97` 未处理 `messages` 重建/复制、`turns`/`tool_calls`/`approvals`/`checkpoints` 的 FK（`24:288` 声称这些表属于 SQLite 真源，但 `26` §3 未给 DDL）。
- 迁移前的备份在 WAL 下应使用 SQLite backup API 或 `wal_checkpoint(TRUNCATE)` 后复制 `db/-wal/-shm`，不能只 `cp sessions.db`。

**修法**：把 §3.1 改成真正的迁移文件级步骤：显式 `default_workspace_id`/默认绝对 path；`PRAGMA foreign_keys=OFF`（事务内）或重建 `messages`/所有子表并复制；`CREATE TABLE ...; INSERT ... SELECT; DROP/RENAME` 覆盖 messages 及其索引；迁移前后校验 conversations/messages 行数与 FK；备份用 backup API/checkpoint；给出 `user_version` 与失败回滚的精确 SQL。否则 P1-A 不能视为关闭。

#### P1-B · 契约/命令清单未同步到 `02`/`05`/`24`/`15`，且 T21 引用错误

**证据**：

- `26:101-105` 要改 `zk-contract-session`（Workspace/WorkspaceStore/会话方法）、`26:110-112` 要改 agent/tool 签名；但 `02:122-152` 的"契约清单（初版，M0 定型）"仍只写 `zk-contract-session` 的 `SessionStore/Conversation/Message/Checkpoint`（`02:136`），`zk-contract-agent`（`02:135`）与 `zk-contract-tool`（`02:133`）仍无 `TurnContext`/ctx 参数；未出现 `Workspace`/`WorkspaceStore`；
- `05` §6（`05:270-310`）命令声明/命令全集仍是旧集合，未包含 `workspace.*`/带 `workspace_id` 的 session/chat；`24:255-258` 仍只讲 `[[commands]]` 静态声明，未列 workspace 命令的声明归属；
- `26:114` 说命令清单见 §5、`ipc-commands.json` 由 `zk compose` 生成，但未说明哪个包在 `zk.toml` 声明 `[[commands]]`（store？desktop？agent？），也未给 args/result schema（C10）；
- `26:115`、`26:134` 把 `ipc-commands.json`/`errors.json` 的待办指向 **T21**；`15:146` 的 T21 是"kit 清单与 @capability"，不是 IPC/错误码（T4 才是 IPC 命令全集，`15:129`）；
- `15:129/130` 的 T4/T5 仍写"IPC 命令全集...M1 spec"/"SQLite schema v1...M1 spec"，与 `26` 已冻结 schema v2/命令集冲突（T5 的归属 `24 §7` 现在又指向 26）。
- `26:110` "契约新增 TurnContext"未说明放在哪个契约 crate（`zk-contract-agent`? `zk-contract-tool` 依赖 agent? 新 `zk-contract-workspace`?），这是 C4 单实例/依赖方向的未决点。

**修法**：更新 `02` §5（Workspace/WorkspaceStore/TurnContext/新签名，标明契约 crate 与版本策略）；更新 `05` §6（command 列表/`[[commands]]` 归属与 schema）；更新 `24` §4（workspace 命令静态声明样例）与 §3 writer（若新增 `commands.gen.ts`/schema）；把 T4/T5 改为"其余命令/其余表在 M1 spec 冻结；workspace 子集见 26"；把 `26:115/134` 的 T21 改为 T4（IPC）并明确错误码属 T4/错误码集合；明确 `TurnContext` 所属契约 crate（建议 `zk-contract-agent` 或 `zk-contract-session`，并说明 tool 契约的依赖方向）。

#### P1-C · 工作区外路径策略仍三处冲突（M1 允许/默认拒绝/fs 仅 root）

**证据**：

- `26:19`："工具可以按显式绝对路径访问工作区外"；`26:27`："绝对路径/`~` 必须显式，并在 UI/审计标注"；
- `26:56`："显式绝对路径需 `TurnContext.allow_outside`（**M1 只读/默认拒绝**）"；`26:151` 验收："绝对路径**默认拒绝**并提示"；
- `26:35`："工具 fs 只做工作区根内的读写"；`26:169` §9 说 tool-fs 改为 per-turn root、移除 env fallback；当前 `tool-fs` 明确拒绝绝对路径/`..`/symlink 逃逸（`packages/client/tool-fs/src/lib.rs:14-16,58-83`）。

**修法**：确定 M1 策略并统一：建议 M1 = 默认拒绝、仅 `allow_outside` 显式开启（当前 §2.6/§7 的口径），把 `26:19`/`26:27` 标为 post-M1 或"仅当 `allow_outside=true`"；§2.13 的"fs 只做根内"作为 M1 事实；`26:151` 验收与 §9 迁移一致。否则实现者会按 §1 允许越界、按 §3.1/§7 拒绝，产生安全/行为差异。

#### P1-D · Workspace 创建/重复/默认/移除目标规则不闭合

**证据**：

- `26:31`："创建（目录必须存在或由用户显式创建；可读写）...重新登记（同路径幂等：返回既有 Workspace）"；
- `26:123`：`workspace.create` 错误 `workspace/path-unavailable`、`workspace/duplicate-path`（重复路径=错误），与"同路径幂等返回既有"矛盾；`26:30` 默认 Workspace 由平台 Documents 创建，与"目录必须存在或由用户显式创建"未说明谁创建/失败回退；
- `26:125` 列 `workspace/default`，但没有"默认 Workspace 是否可移除/是否可作为 `reassign_to` 目标"的规则；`workspace.remove` input `reassign_to?` 无目标校验/错误码（除了 has-sessions/default）；
- `26:82` 只规定 workspace path canonicalize，未定义 `workspace.create` 对文件/非目录/不可写/符号链接/Windows case/UNC 的错误码；`26:123` 仅 `path-unavailable`/`duplicate-path`。

**修法**：明确 `workspace.create` 只接受已存在目录（DSH 语义）或增加 `create_if_missing`；重复路径返回既有（幂等）或统一为 duplicate-path，二者择一；默认 Workspace 由 `initializeDefault` 等价逻辑创建（平台 Documents 不可用时返回 undefined/选择器，不自动建会话）；定义 `workspace/default` 规则（默认不可移除或需先改默认）、`workspace/invalid-reassign`、`workspace/not-writable`、`workspace/not-directory`、`workspace/symlink` 等错误码；补 §7 验收。

#### P1-E · `review-check.py` 可被绕过，且与 `docs/11` §9.3 声明不符

**证据**：

- `scripts/review-check.py:30` 对 `- 评审：n/a` 一律 `continue`（模板 `specs/templates/proposal.md:6` 允许 `n/a` "C19 前遗留"，但脚本没有新 spec 截止判断）；任何新 spec 写 `n/a` 即绕过评审。
- `scripts/review-check.py:38` 只做 `if "APPROVED" not in body` 的**子串**判断；本记录作为 `CHANGES-REQUESTED` 仍包含 "APPROVED" 字样（§11 的"只有两种"），若某个 spec 链接本记录，脚本会误判通过。
- `scripts/review-check.py:40-42` 只要记录里有 "关联 spec" 或 change id 即通过，不核对 `- 评审：` 指向的记录是否对应此 spec；`11:189` 等声明"校验记录结论与关联 spec"并未实现；无 review 对象 hash/复评结论/P0-P1 关闭证据校验。
- `docs/README:169` 维护门禁仍只列 `docs-check` + `spec-check`，未列 `review-check.py`；`docs/11:175` §9.1 收尾也只写 docs-check/spec-check；`docs/11:189` 声称 review-check 校验"记录结论与关联 spec"，与脚本行为不符。

**修法**：`review-check.py` 解析记录的 `## 结论`/`结论：` 行并要求精确 `APPROVED`；新 spec（创建日期 > C19 生效日）禁止 `n/a`（或要求 n/a 仅用于 C19 前已 created 的 spec，脚本按 `proposal.md` 日期判断）；校验 `评审：` 的 record 内 `关联 spec` 与 spec id 匹配；校验记录 hash/复评结论/P0/P1 状态（至少 `复评` 节存在）；把 review-check 加入 `docs/README` §6、`docs/11` §9.1 step 6 与 AGENTS workflow step 8。

#### P1-F · `24 §7` 的表清单与 `26 §3` 不匹配，schema/迁移范围不闭合

**证据**：

- `24:288`："表清单/DDL 见 `26` §3（`workspaces/conversations/...`）"；`24:288-292` 仍要求 `workspaces/conversations/messages/turns/tool_calls/approvals/checkpoints` 的 SQLite 真源与 turn 原子提交/checkpoint；
- `26` §3（`26:59-84`）只给 `workspaces` + `conversations`；没有 `messages`、`turns`、`tool_calls`、`approvals`、`checkpoints` 的 DDL/迁移；`26:15/163` 却引用 `turns`/SQLite 消息/turn 表；`26:169-172` §9 任务也只写 conversations/messages 迁移，未列 turns/checkpoints；
- 当前 store 只有 conversations/messages（`store-session-sqlite/src/lib.rs:34-52`），`turns` 等未实现；若 spec 按 `26` 只迁移两张表，`24 §7` 的 turn 原子性/checkpoint 验收没有 schema。

**修法**：二选一：`26` §3 补全 messages/turns/tool_calls/approvals/checkpoints（或明确这些是 M2/后续，`24 §7` 同步标 post-M1）；`24:288` 改成"表清单/DDL 见 26 §3；M1 只含 workspaces/conversations/messages，turns/checkpoints 见 M2"；§7 验收相应调整。

### R4. 施工阶段才解决的代码问题（不作为本轮文档 P0/P1，但 spec 必须列）

以下已在文档标注（AGENTS:27、`24:289`、`05:147`、`26:165-172`、`26:149-152`），**不是本轮文档 P0**，但 approved spec/实现必须覆盖：

- 当前 `zk-contract-session` 无 workspace；`store-session-sqlite` 只有两表、无 user_version/迁移；`agent`/`tool` 无 `TurnContext`；IPC 无 `workspace.*`；UI 无 switcher；`chat.send` 自动建会话（`apps/desktop/src/ipc.rs:132-141`；前端 `controller.ts:185/231`）；`tool-fs` 进程 root（`tool-fs/src/lib.rs:14-42`）。
- 建议 spec 在 tasks 中逐条列"删除 auto-create fallback、tool-fs per-turn root、workspace switcher、store v2 迁移、contract conformance"，并在 acceptance 里用 `26:148-156` 的命令/截图证据。

### R5. 门禁与复评记录

- `docs-check` 0/0、`spec-check` 0/0、`review-check` `ok (3 records)` 均通过；但 review-check 的通过不能作为 APPROVED 证据（P1-E）。
- 本轮复评对象版本（最新）：`26` `872d64b742c23b7c419ee1fcb02fcb9aa621563389e71a6f8ed0a8c754a60b6a`；`13` `b20d885140b392e0b147499258f2d9b8a9956dc4e1603ce8048243ba03de3eee`；`24` `bdcecc80651b2f6a7f2d64c01f33a28e4aad59d3591900e6e0925d5ae6394f5b`；`05` `3ef8d213a416bed54efd61f7cbc11b8a0ae0e6cf1147a1a70d3f6cd0bfdabcd2`；`19` `39346fd896a091a417e0aa2d1c09947b309b227ee375d0ca1de6ed678dc10403`；`README` `3bc66f7104bd1fb2429115fe00d468570e47041cd2f974d1878f8d3839f5bb08`；`11` `14b40b03c042ce4e0a4a73444b569dc9cbbf9f2b3a555c058d7112ac457bf9b1`；`AGENTS` `147a538a14d3e5dce416318ff45b5c0747062166876a8d2f62e78076ed085d84`；`review-check.py` `361fc91fa003a343e558d807427e1de4afedc354696c2429c2c99b5903342da1`；`02` `0fd1905f628965af6a4a42e484cde80b42e7332248841f97420cf17f4b193016`。
- 复评结论：**CHANGES-REQUESTED**；实现者修复 P0-A + P1-A～P1-F 后，由同一评审者（audit-arch）再次复评。复评通过后，本记录追加 `## 复评` 的最终状态为 `APPROVED`，才允许写 `specs/changes/2026-10-30-workspace-scoped-conversations/` 的 approved spec。

---

### 第三轮复评（2026-10-30，R3）

> 复评对象：P0-A + P1-A/B/C/D/E/F 修复后的文档（`docs/26` 重写、`24` §7、`02` §5、`05` §6、`15` T4/T5、`README`、`11` §9、`review-check.py`、AGENTS C18/C19）。
> 方式：只读文档 + 代码对照 + 门禁执行；未改规范正文/代码。
> 版本（SHA256）：`26` `99745c53…`；`24` `cbe4bc07…`；`02` `8a6278ea…`；`05` `6e14aed3…`；`15` `60cc5b0d…`；`README` `28a693db…`；`11` `f47a8ae2…`；`AGENTS` `b6ab9ffc…`；`13` `55a3e887…`；`review-check.py` `83145149…`。

#### R3.0. 结论

**`CHANGES-REQUESTED`（尚未 APPROVED）**

P0-A、P1-A、P1-B、P1-E 核心已关闭；P1-C/P1-D/P1-F 仍有小型但明确的文档矛盾/断链，P1-G 是评审门禁文档本身被改坏。修完这 4 项即可 APPROVED；其中没有新的架构级 P0。

**文档能否据以写 approved spec**：**修完 P1-C/P1-D/P1-F/P1-G 后可以**。契约/包/角色/生成物、迁移可执行性、DSH 差异、验收/证据、实现差距已足够作为 spec 输入；当前剩余问题都会直接进入 spec 的设计/验收文本，不能带着写 approved spec。

#### R3.1. 已关闭

- **P0-A 不可变 vs reassign：RESOLVED**。`AGENTS:53` C18、`13:491` ADR-058、`26:24` 统一为"`workspace_id` 创建后不可变；唯一例外是用户二次确认的 `workspace.remove{reassign_to}`，系统内部不得迁移"；`26:34/172` 补默认 Workspace/`set_default`/reassign 目标规则、`26:162` 补 `workspace/reassign-target-invalid`、`confirm:true`。剩余 `24:293` 未写例外（P2-G）。
- **P1-A 迁移：RESOLVED（核心）**。`26:103-133` 已含 WAL checkpoint + 备份 db/-wal/-shm、默认 Workspace 的 id/绝对 canonical path、`PRAGMA foreign_keys=OFF` 在 BEGIN 外、**重建 conversations+messages**（避免对 messages 的 CASCADE）、显式绑定、行数/孤儿/默认唯一校验、失败恢复、幂等。此前实测 `DROP TABLE conversations` 清空 messages 的问题已由"重建 messages"消除。
- **P1-B 契约/命令同步：RESOLVED**。`02:133/135/136` 已补 `Tool/TurnContext`、agent run/run_stream 带 TurnContext、`WorkspaceStore/Workspace/Conversation{workspace_id}`；`05:305-307`、`24:259` 指向 26 §5/T4；`15:129/130` T4/T5 已改为含 workspace 命令与 schema v2；`26:150/151` 改为 T4，不再错指 T21。
- **P1-E review-check 核心绕过：RESOLVED**。`scripts/review-check.py:31` `n/a` 仅允许两个 legacy spec 且必须带"C19 前遗留"；`:42-59` 要求记录含 `关联 spec` + change id，并由 `latest_conclusion()`（`:30-37`）解析最后一条含"结论"的行、精确比对 `APPROVED`；CHANGES 记录中的 APPROVED 字样不再误判。剩余 P2-H（hash/复评未强制）。
- **P1-D 创建/重复/默认规则：大部分 RESOLVED**。`26:32` 明确 create 只接受已存在可读写目录、不提供 `create_if_missing`；`26:159` 补 path-not-found/not-a-directory/not-writable/duplicate-path；`26:35` 明确同 path 重新登记生成新 id、已登记返回 duplicate-path；`26:34/172` 默认规则与 reassign 目标规则已补。仅剩 `26:31` 的"或由用户显式创建"与 `26:32` 的"已存在"矛盾（P1-D）。
- **P1-F schema 范围：大部分 RESOLVED**。`24:292` 已明确 M1 只有 workspaces/conversations/messages，turns/tool_calls/approvals/checkpoints post-M1；`26:95` 同步；`26:15` 标 turns post-M1。剩余 `26:201` 仍说 M1 有 turn 表（P1-F）。

#### R3.2. 仍存在的 P1（文档）

**P1-C · `26:19` 仍与 M1"默认拒绝越界"冲突**

- `26:19`："工具可以按显式绝对路径访问工作区外"；
- 但 `26:27`："M1 默认拒绝绝对路径/`~`/工作区外路径（tool-fs 根内）；显式越界（`TurnContext.allow_outside`）为 post-M1"；`26:39` 文件/越界 post-M1；`26:60` 同；`26:189` 验收"默认拒绝"。
- 这是规范正文内的硬规则冲突；实现者会按 §1 允许越界、按 §2/§7 拒绝。修法：`26:19` 改为"M1 默认拒绝；post-M1 通过 `allow_outside` 显式开启"或直接删除该句，改成 §2 的措辞。

**P1-D · `26:31` 的"或由用户显式创建"与 `26:32`"必须已存在"矛盾**

- `26:31`："创建（目录必须存在**或由用户显式创建**；可读写）"；
- `26:32`："`path` 必须是**已存在的目录**且可读写；不提供 `create_if_missing`（用户先在文件管理器创建，或后续 UI 显式建目录）"。
- 同一节对 create 是否创建目录给出两种说法。修法：`26:31` 改为"目录必须已存在且可读写（用户先在文件管理器创建）"；把"后续 UI 显式建目录"移出 create API 或写成独立 `workspace.create_directory`（M2）。

**P1-F · `26:201` 仍说 M1 有 `turn` 表，与 schema/turns post-M1 冲突**

- `26:201`："我们 M1 仍是 SQLite **消息/turn 表**（`24` §7），事件溯源后置"；
- `26:15`："`turns`/事件表（**post-M1**；M1 按消息落库）"；`26:95`："turns/tool_calls/approvals/checkpoints 是 **post-M1**；M1 只有上述三表"；`24:292` 同。
- 修法：`26:201` 改为"M1 是 SQLite 消息表；turn/事件表 post-M1"。

**P1-G · `docs/README` 的评审门禁命令被改坏（换行落在 inline code 内）**

- `docs/README:166-167`：`` `python3 scripts/spec-check.py\npython3 scripts/review-check.py` ``；`:170-171`：`` `python3 scripts/docs-check.py` + `python3 scripts/spec-check.py\npython3 scripts/review-check.py` ``。Markdown 会把换行并入同一 code span，渲染成一条不存在/错误的命令；这是本轮修改引入的文档缺陷，`docs-check` 不检查。
- 修法：拆成独立 code span：`` `python3 scripts/spec-check.py` + `python3 scripts/review-check.py` ``、`` `python3 scripts/docs-check.py` + `python3 scripts/spec-check.py` + `python3 scripts/review-check.py` ``。

#### R3.3. 仍存在的 P2（建议）

- **P2-G · 例外未传播**：`24:293` 仍写"`conversations.workspace_id` 必填、不可变"而未提 C18/26 的唯一 reassign 例外；建议补"（唯一例外见 `26` §2）"。
- **P2-H · review-check 仍不校验记录 hash/复评节**：`scripts/review-check.py` 不校验 `docs/reviews/README:8-9` 要求的评审对象 hash/复评/P0-P1 状态；可在 spec 阶段前补，属于流程加固，不阻塞本轮。
- **P2-I · 错误码命名不一致**：`26:34` 写 `workspace/set-default`，API 表 `26:161` 用 `workspace/not-found`/`workspace/path-unavailable`；建议统一 `workspace/set-default-*` 或写 `workspace/not-found`。
- **P2-J · 迁移脚本占位**：`26:109` `CREATE TABLE workspaces (...)` 是占位（schema 在 §3 已给），在"可执行"标题下应直接引用 §3 DDL 或写全。
- **P2-K · 计数/范围陈旧**：`13:552` 仍写"域内补充规范见 `19–26`"（现有 27）；建议改 `19–27`。
- **P2-L · `06`/`08` 交叉引用**：本记录未复评其它文档；若 `06` §3 仍以旧 schema/迁移口径描述，需在 spec 时同步（非本轮 P1）。

#### R3.4. 施工阶段代码问题（仍不在本轮文档 P0/P1）

代码仍未按新规范施工（符合 C19 的"评审通过后再写 spec/实现"）。以下差距已在 `26:203-210`、`24:290`、`05` §3.5/§6、`19:33` 标注，**属于 approved spec/实现任务**，不是本轮文档阻塞：contract session/agent/tool 的 Workspace/TurnContext、store v2 迁移、agent-loop/tool-fs per-turn root、workspace IPC、UI switcher、删除 `chat.send` auto-create、`tool-fs` 进程 root/`ZEOKEEL_DEMO_ROOT`。spec acceptance 应按 `26:184-194` 的命令/截图/axe 证据执行。

#### R3.5. 门禁与复评记录

- `docs-check` 0/0、`spec-check` 0/0、`review-check` `ok (3 records)` 均通过（P1-G 是 README 文本缺陷，不影响脚本执行）。
- 本轮复评对象版本：`26` `99745c531eb9156a89e121ceb2973a2d5a653d0d0850e668c47c20c43a1bcf34`；`24` `cbe4bc073b17a48cbe64537fd165d4807230b02f414dcf7da31ffeae94239769`；`02` `8a6278eace739cf6f872f0d21ff2f41d236e9033a93ef1510890609a1aff5495`；`05` `6e14aed3e6c18837ec2ddcfa9219a6ce901098e6d88eb98148398f9812c44a37`；`15` `60cc5b0d0ad4793c18e8ad0a05e78ea7c82c17cafa8ebad3b3d43054ed825a9d`；`README` `28a693db26d853848990f8e3a6f8c8a195e13b3e0eecc1b6f571aad4eecddc17`；`11` `f47a8ae2b6461d2dbfc3f022d39148f8e78cb37b2f45a8bcddf22cb44efb9a64`；`AGENTS` `b6ab9ffc9c7c24a7cba68a5df18bc6cf234f2e8da99e39d2600ef792e9daf7b8`；`13` `55a3e8870ff03ba3ff2bef2fb876586e729c85d01ce4c642cfb28e25e56fe20d`；`review-check.py` `83145149798368bf1e3e252dc7a1ada3461c87af412379a81a294d0fe6d6d2d6`。
- 关联 spec：`specs/changes/2026-10-30-workspace-scoped-conversations`（APPROVED 后创建；proposal 必须引用本记录，acceptance 回链 P0/P1 关闭证据）。
- 复评结论（第三轮）：**CHANGES-REQUESTED**

---

### 第四轮复评（2026-10-30，R4）

> 复评对象：第三轮剩余 4 个 P1 修复后的文档（`26`、`README`、`24`、`13` 等）。
> 方式：只读文档 + 门禁执行；未改规范正文/代码。
> 版本（SHA256）：`26` `5782f426…`；`24` `383b4f8c…`；`02` `8a6278ea…`；`05` `6e14aed3…`；`15` `60cc5b0d…`；`README` `30a59488…`；`11` `f47a8ae2…`；`AGENTS` `b6ab9ffc…`；`13` `4db3af1b…`；`review-check.py` `83145149…`。

#### R4.0. 结论

**`APPROVED`**

- **P0：无。P1：无。**
- 第三轮 4 个 P1 均已核验关闭（见 R4.1）。
- **文档可以据以写 approved spec**：`specs/changes/2026-10-30-workspace-scoped-conversations/` 可创建，proposal 引用本记录，acceptance 回链本记录与施工阶段验收（`26` §7/§9）。
- 施工阶段代码差距（contract/store/agent/tool/IPC/UI/auto-create/tool-fs）**不是文档阻塞**：`26:203-210`、`24:290`、`05`/`19` 已明确 M1 代码未实现，spec 按 §9 tasks/§7 证据执行。

#### R4.1. 第三轮 P1 核验

| P1 | 状态 | 核验 |
| --- | --- | --- |
| P1-C `26:19` 与默认拒绝冲突 | **RESOLVED** | `26:19` 已改为"M1 **默认拒绝**绝对路径/工作区外访问；post-M1 才由 `TurnContext.allow_outside` 显式开启"；与 `26:27/39/60/189` 一致 |
| P1-D `26:31` vs `26:32` 创建规则 | **RESOLVED** | `26:32` 明确 path 必须**已存在**目录且可读写、不提供 `create_if_missing`；同 path 已登记返回 `duplicate-path`、不幂等（`26:35`）；错误码补齐 |
| P1-F `26:201` turn 表范围 | **RESOLVED** | `26:201` 已改为"M1 仍是 SQLite 消息表（`conversations/messages`）；`turns`/事件溯源/tool_calls 等表 post-M1"；与 `26:15/95`、`24:292` 一致 |
| P1-G `README` 门禁命令换行 | **RESOLVED** | `README:166/169` 已拆成独立 code span：`spec-check.py` 与 `review-check.py` 分开；docs/11 §9 与 AGENTS §6/§8 也已引用 review-check |
| 附带：`24:293` 例外、`13:552` 范围、`26:34` set_default 命名 | **RESOLVED** | `24:293` 补"唯一例外 `workspace.remove{reassign_to}`，见 26 §2"；`13:552` 改 `19–27`；`26:34/161` 统一 `workspace.set_default` |

#### R4.2. 剩余 P2（建议，不阻塞 APPROVED）

- **P2-1 `13:552` 计数**：仍写 `D1–D66`，但 `15:1/159`、`README:107`、`AGENTS:273/305` 已是 `D1–D67`（新增 D67=C10 例外）。修法：`13:552` 改 `D1–D67`。
- **P2-2 `README:169` 措辞**：`docs-check + spec-check + review-check` 后的"（后者仅结构检查，不校验 C16 的 approved 状态）"会把 review-check 说成仅结构检查；实际 spec-check 才是结构检查、review-check 校验 C19/APPROVED。修法：改为"（spec-check 仅结构检查；review-check 校验 C19/approved）"。
- **P2-3 `26:109` 迁移占位**：`CREATE TABLE workspaces (...)` 在"可执行"脚本里是占位（完整 schema 在 §3 已给）。修法：直接引用 §3 DDL 或写全。
- **P2-4 `26:32` 括注**："用户先在文件管理器创建，或后续 UI 显式建目录"中"后续 UI 显式建目录"是 out-of-band 行为，建议注明"UI 先建目录再调用 create；create 本身不建目录"，避免读成 `create_if_missing`。
- **P2-5 review-check 记录完整性**：`scripts/review-check.py` 仍不校验评审对象 hash/复评节/P0-P1 关闭证据（`docs/reviews/README:8-9` 要求），属流程加固。
- **P2-6 `24:296`**："崩溃恢复扫描未完成 turn"在 M1 表范围（turns post-M1）下应标 post-M1 或删除；不影响本轮 workspace spec。

#### R4.3. 门禁与复评记录

- `docs-check` 0/0、`spec-check` 0/0、`review-check` `ok (3 records)` 均通过；`review-check` 的 `latest_conclusion()` 会读取本节末尾的 `APPROVED`（但 proposer 必须等本记录落盘后再写 spec）。
- 本轮复评对象版本：`26` `5782f426385769e3d571ce1cc744f9e722dd571dd50b732035f13a6a7a87a1bc`；`24` `383b4f8cbf4bf2a4eee1971c4099e9acc5f47fe8c6e8346a611f4e0557f485e6`；`02` `8a6278eace739cf6f872f0d21ff2f41d236e9033a93ef1510890609a1aff5495`；`05` `6e14aed3e6c18837ec2ddcfa9219a6ce901098e6d88eb98148398f9812c44a37`；`15` `60cc5b0d0ad4793c18e8ad0a05e78ea7c82c17cafa8ebad3b3d43054ed825a9d`；`README` `30a594886cc35df3a75a6752b4e3d2167b4b36b1a7800bf83efe42129b602346`；`11` `f47a8ae2b6461d2dbfc3f022d39148f8e78cb37b2f45a8bcddf22cb44efb9a64`；`AGENTS` `b6ab9ffc9c7c24a7cba68a5df18bc6cf234f2e8da99e39d2600ef792e9daf7b8`；`13` `4db3af1b508a1aa6104c275f3c65a09c493066dbfae3781efca27d10fe0cd36d`；`review-check.py` `83145149798368bf1e3e252dc7a1ada3461c87af412379a81a294d0fe6d6d2d6`。
- 关联 spec：`specs/changes/2026-10-30-workspace-scoped-conversations`（proposal 引用本记录；acceptance 回链本记录与 `26` §7/§9 证据）。
- 本记录只追加评审文本；未改任何规范正文/代码。

复评结论（第四轮）：**APPROVED**
