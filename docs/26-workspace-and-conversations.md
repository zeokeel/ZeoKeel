# 26 · 工作区与对话（Workspace-scoped conversations）

> 状态：**M1 已实现（2026-10-30）**；规范第七轮评审批次 APPROVED；实现/证据见 §9 与 spec `2026-10-30-workspace-scoped-conversations`。
> 来源：DSH（DeepSeek Harness）`dsh-workspace` / `dsh-session` / `dsh-api-workspace-controller` /
> `dsh-agent-loop` 的公开行为（MIT；**只作事实参考，不作依赖/构建输入**，C15）；取舍与冲突见 §2.5/§2.6。
> **本文是工作区、会话归属、agent cwd 与工作区 schema 的唯一权威**；`24` §7 只保留存储引擎不变量，
> `05`/`19`/`27` 只引用本文，不重复规则。

## 1. 概念

| 概念 | 定义 | 存储 |
| --- | --- | --- |
| **Workspace** | 用户选择/创建的一个本地目录 + 稳定 id（UUIDv4）+ 标题；会话/文件浏览/agent cwd 的作用域 | `workspaces` 表 |
| **Conversation** | 一条消息线程，**必须且只能属于一个 Workspace**；`workspace_id` 创建后不可变 | `conversations.workspace_id` |
| **Agent turn** | 一次"模型请求→工具→落库"循环；cwd = 会话所属 Workspace 根 | `turns`/事件表（**post-M1**；M1 按消息落库） |
| **Workspace switcher** | 侧栏**平铺的 Workspace 列表**（不嵌套/不分组）+ 切换/新建/重命名/移除；默认 `~` | UI（`ui-sessions`） |

**不是安全沙箱**：工作区是上下文与默认根，不是文件/命令隔离边界（弱管控，ADR-030）。
M1 **默认拒绝**绝对路径/工作区外访问；post-M1 才由 `TurnContext.allow_outside` 显式开启。**默认根、相对路径解析、目录浏览、索引、agent cwd 都以工作区为根**。

## 2. 硬规则（M1）

1. **无 Workspace 不建会话**：`session.create` 必须带 `workspace_id`；没有可用 Workspace 时 UI 进入"选择/新建工作区"，不创建空会话。
2. **会话归属不可变（唯一显式例外）**：创建后 `workspace_id` 不得修改；唯一例外是用户二次确认的 `workspace.remove{reassign_to}`（系统内部不得迁移）。换目录 = 新建会话/工作区。
3. **列表按 Workspace 隔离，Workspace 平铺**：侧栏以平铺列表展示**所有** Workspace（不嵌套 Workspace、不做日期分组）；每个 Workspace 行下缩进列出**它自己的**会话（一层缩进，DSH 式）；选择会话时同步 active Workspace；切换 Workspace 只改变展开/当前会话，不把其它 Workspace 的会话混入。
4. **agent cwd = Workspace 根**：本地 agent loop、shell、fs、文件浏览、checkpoint 的默认根均为该会话的 Workspace 根。
5. **相对路径先解析到 Workspace 根；M1 默认拒绝绝对路径/`~`/工作区外路径**（`tool-fs` 根内）；显式越界（`TurnContext.allow_outside`）为 post-M1，未实现前不得宣称支持。
6. **消息不跨 Workspace**：会话命令必须携带当前 `workspace_id` 并与会话归属比对，不匹配 = `session/workspace-mismatch`。
7. **一个 Workspace 可多会话**：共享目录，但各自独立 agent/loop；同一会话同一时刻只允许一个 turn。
8. **首用 Workspace**：首次启动无 Workspace 时创建默认 Workspace = **用户 home 目录 `~`**（canonicalize；`ZEOKEEL_HOME` 仅测试/嵌入 override，不是 UI 默认；home 不可用则让用户选择目录）；**创建默认 Workspace 后不自动建会话、不发消息**。
9. **Workspace 生命周期**：
   - 创建：`path` 必须是**已存在的目录**且可读写；不提供 `create_if_missing`（用户先在文件管理器创建；`workspace.create` 本身不建目录；UI 只能在用户确认后先建目录再调用 create）。UI 的 `~` 是显示简写，IPC 边界会展开为 home 绝对路径后再校验。错误：`workspace/path-not-found`、`workspace/not-a-directory`、`workspace/not-writable`、`workspace/duplicate-path`（同一 canonical path 已登记）。
   - 重命名：只改 `title`，不动 path/id。
   - 移除：只从注册表移除、不删目录。无会话时直接移除；有会话时必须 `reassign_to`（目标存在、≠自身、目录可用），事务内迁移会话后移除，返回 `moved_sessions`；用户需二次确认，写审计日志（M1 可本地日志）。**默认 Workspace 不可直接移除**：需先 `workspace.set_default` 指定另一个 default，再移除；`workspace.set_default` 目标必须存在且可用。
   - 重新登记：移除后同 path 可重新 create，生成**新 id**（旧会话已迁移/不存在）；同 path 已登记时 create 返回 `duplicate-path`，不幂等返回旧对象。
10. **切换 Workspace = 取消当前活动 turn**（保留已生成部分并标记 cancelled），清空当前草稿；草稿可按 Workspace 暂存，禁止跨 Workspace 发送。
11. **归档/置顶/排序为 post-M1**：M1 不承诺 `archived/pinned/order` 字段与命令（见 §2.5 取舍）；C18 的"按工作区隔离"仍适用于列表/消息。
12. **fork/恢复同一 Workspace**：会话 fork/resume 只允许在源会话所属 Workspace 内；跨 Workspace 复制是显式导出/导入，不是 resume。
13. **文件浏览/越界 post-M1**：M1 不提供目录浏览/监听命令与验收；工具 fs 只做工作区根内的读写（见 §9 差距）。

## 2.5 候选方案与取舍

| 方案 | 结论 | 理由 | 失败/回退 |
| --- | --- | --- | --- |
| A. Workspace 为持久实体 + 会话必属（本文） | **采用** | 与 DSH 一致；列表/审计/cwd 可解释；支持多项目 | 首用目录不可用 → 选择器 + 只读态 |
| B. 仅用"会话 cwd 字符串"，无 Workspace 实体 | 否 | 无法分组/重命名/多会话共享；路径移动即散架；违反"必须归属" | — |
| C. 用 JSON `workspaces.json` + 会话目录（ADR-014 早期想法） | 否 | 与 SQLite 唯一真源冲突；并发/事务/迁移弱 | 改为 SQLite（ADR-058 修订 ADR-014 存储部分） |
| D. 移除 Workspace 时直接删会话/目录 | 否 | 数据丢失风险；DSH 也不删 | 用 reassign/显式删除（M2） |
| E. 允许 ungrouped 会话 | 否（M1） | 与 C18"必须且只能"冲突，UI/查询复杂 | 若将来需要，先改 C18/ADR 再实现 |
| F. Workspace 分组/嵌套树 | 否（M1） | 用户明确要求**平铺**；嵌套导航增加认知成本，M1 不需要 | 若将来需要，先评审再扩展 |

## 2.6 与现有文档/契约的冲突与处理

| 冲突点 | 处理 |
| --- | --- |
| ADR-014：数据目录含 `workspaces.json` | ADR-014 仅保留"配置 JSON/只增不改"；会话/工作区存储由 ADR-058 修订为 SQLite，`workspaces.json` 不再使用 |
| `05` §3.5、`19` §2、`24` §7 重复工作区规则 | 改为摘要 + 链接本文；schema 只在本文 §3，`24` §7 只留引擎不变量 |
| 现有 `Conversation` 无 `workspace_id`、`create_conversation(title)` | 契约升级见 §4；`docs/02` §7 semver，破坏性变更 major 或新增方法 + 兼容层 |
| 现有 `chat.send` 无 `conversation_id` 时自动建会话（`apps/desktop/src/ipc.rs`） | **删除 auto-create fallback**；无 workspace/conversation 一律报错（§9） |
| `05` §5.5：动态内容只放 user message 尾部（prefix cache） | Workspace 路径/时间不写入 system prompt；作为 user message 尾部元数据/工具上下文注入 |
| `tool-fs` 单 root、拒绝绝对路径 | 改为 per-turn workspace root；M1 仍**默认拒绝**绝对路径/越界；`TurnContext.allow_outside` 为 post-M1，见 §4/§9 |
| DSH 移除 project 保留会话（ungrouped） | 我们不允许 ungrouped；移除时 `reassign_to` 迁移会话（C18 优先） |

## 3. 数据模型（本文唯一 schema 来源；引擎不变量见 `24` §7）

```sql
-- schema v3；PRAGMA user_version
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,          -- canonicalize 后的本地绝对路径
  title TEXT NOT NULL,
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id),
  title TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_conversations_workspace ON conversations(workspace_id, updated_at_ms DESC);

CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'done'   -- done | cancelled | error
);
CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at_ms);
```

`turns/tool_calls/approvals/checkpoints` 是 **post-M1**（`24` §7 引擎不变量后置）；M1 只有上述三表。

- `path` 规范化（realpath）后唯一；符号链接解析后登记；目录不存在时 UI 标"目录不可用"，会话只读/禁发，不 fallback 到 cwd。
- M1 不建 `archived/pinned/order` 字段；post-M1 迁移再 `ALTER TABLE`（数据兼容 `serde(default)`）。
- `conversations.workspace_id` 必填；删除 Workspace 前必须 `reassign_to` 迁移其会话。

### 3.1 schema v3 迁移（可执行，SQLite；v2 库补 status 列）

1. **准备**：`PRAGMA wal_checkpoint(TRUNCATE)`；复制 `sessions.db`、`sessions.db-wal`、`sessions.db-shm`（存在则）到 `sessions.db.bak-v<old>`；失败保留备份。
2. **默认 Workspace**：`default_id = uuid v4`；`default_path = canonicalize(dirs::home_dir())`（用户 home `~`；绝对路径；home 不可用 → 迁移失败，让用户选择目录，不猜）。
3. **单事务 + `PRAGMA foreign_keys=OFF`**（必须在 BEGIN 之外设置）：
   ```sql
   PRAGMA foreign_keys=OFF;
   BEGIN;
   CREATE TABLE workspaces (...);
   INSERT INTO workspaces(id,path,title,is_default,created_at_ms,updated_at_ms)
     VALUES (?default_id, ?default_path, '默认工作区', 1, ?now, ?now);
   CREATE TABLE conversations_new (
     id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id),
     title TEXT NOT NULL, created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL);
   INSERT INTO conversations_new(id,workspace_id,title,created_at_ms,updated_at_ms)
     SELECT id, ?default_id, title, created_at_ms, updated_at_ms FROM conversations;
   CREATE TABLE messages_new (
     id TEXT PRIMARY KEY,
     conversation_id TEXT NOT NULL REFERENCES conversations_new(id) ON DELETE CASCADE,
     role TEXT NOT NULL, content TEXT NOT NULL, created_at_ms INTEGER NOT NULL);
   INSERT INTO messages_new SELECT id, conversation_id, role, content, created_at_ms FROM messages;
   DROP TABLE messages;
   DROP TABLE conversations;
   ALTER TABLE conversations_new RENAME TO conversations;
   ALTER TABLE messages_new RENAME TO messages;
   CREATE INDEX idx_conversations_workspace ON conversations(workspace_id, updated_at_ms DESC);
   CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at_ms);
   PRAGMA user_version=2;
   COMMIT;
   PRAGMA foreign_keys=ON;
   ```
4. **校验**：`messages` 行数 = v1 行数；`conversations` 行数 = v1 行数；无孤儿 `workspace_id`；默认 Workspace 恰好 1 个。任一项失败 → 回滚事务、恢复备份、UI 只读并提示导出。
5. **迁移测试**：v1 有会话+消息→v3 归属默认且消息不丢；失败注入→备份恢复；重复迁移幂等（user_version≥2 跳过）。

## 4. 契约 / 包 / 角色 / 生成物影响（C3/C4/C10）

1. **契约扩展 `zk-contract-session`（复用现有契约，不新增契约）**：
   - 新 DTO：`Workspace { id, path, title, is_default, created_at_ms, updated_at_ms }`；
   - `Conversation` 加 `workspace_id: String`；
   - 新 trait `WorkspaceStore { list/create/rename/remove }`（remove 带 `reassign_to: Option<String>`，语义见 §2）；
   - `SessionStore`：`create_conversation(workspace_id, title)`、`list_conversations(workspace_id)`、`messages(workspace_id, conversation_id)`（不匹配 = `SessionError::WorkspaceMismatch`）。
   - 版本：当前契约未发布 1.x，直接改方法面；`docs/02` 发布后按 semver。
2. **包/角色**：`store-session-sqlite`（角色 `store.session`）同时实现 `SessionStore` + `WorkspaceStore`（同一 SQLite/单 writer）；不新增 `store.workspace` 包（M2 若拆再预埋角色）。
   - UI：workspace switcher 归 `@zeokeel/ui-sessions`；不新增 UI 包。
   - 角色注册表 `24` §1.5 不变（`store.session` 契约仍为 `zk-contract-session`）。
3. **Agent/Tool per-turn 上下文**：契约新增 `zk-contract-tool::TurnContext { conversation_id, workspace_id, workspace_root }`
   （M1 无 `allow_outside`，越界 post-M1；agent/tool 均依赖 tool 契约）；
   `Agent::run/run_stream(ctx, input, ...)`、`Tool::call(ctx, args)`；未绑定 Workspace = `AgentError::WorkspaceRequired` / `ToolError::WorkspaceRequired`。
   `agent-loop` prompt 只含稳定部分；workspace 路径/时间作为 user message 尾部元数据或 tool 上下文（§2.6 prefix cache）。
4. **IPC**：`CommandRequest/Response` 形状不变；命令清单见 §5；`shared/contracts/ipc-commands.json` 与 `@zeokeel/ui-kit` 类型由 `zk compose` 生成；命令全集/错误码在 T4（M1 spec）冻结。
5. **生成物**：`zk compose` 无新增生成器；`shared/contracts/roles.json`/`packages.json` 随包扫描更新；`ipc-commands.json`/错误码为 T4（M1 spec）待补。
6. **包四件套**：store 包已有 `zk.toml`/README/conformance；契约变更需 conformance 覆盖 workspace/负向用例；UI 包不需新包。

## 5. API / IPC（M1）

| 命令 | 输入 | 输出 | 错误 |
| --- | --- | --- | --- |
| `workspace.list` | — | `Workspace[]` | — |
| `workspace.create` | `{path,title?}` | `Workspace` | `workspace/path-not-found`、`workspace/not-a-directory`、`workspace/not-writable`、`workspace/duplicate-path` |
| `workspace.rename` | `{workspace_id,title}` | `Workspace` | `workspace/not-found` |
| `workspace.set_default` | `{workspace_id}` | `Workspace[]` | `workspace/not-found`、`workspace/path-unavailable` |
| `workspace.remove` | `{workspace_id, reassign_to?, confirm:true}` | `{removed:true, moved_sessions:n}` | `workspace/not-found`、`workspace/has-sessions`、`workspace/default`、`workspace/reassign-target-invalid` |
| `session.create` | `{workspace_id,title?}` | `Conversation` | `workspace/not-found`、`session/workspace-required` |
| `session.list` | `{workspace_id}` | `Conversation[]` | `workspace/not-found` |
| `session.messages` | `{workspace_id, conversation_id}` | `Message[]` | `session/not-found`、`session/workspace-mismatch` |
| `chat.send` | `{workspace_id, conversation_id, text}` | `{stream_id,workspace_id}` | 同上 + `session/active-turn` |
| `chat.cancel` | `{workspace_id, stream_id}` | `{cancelled:true}` | `workspace/not-found` |

- 所有会话命令必须带 `workspace_id` 并校验归属；`chat.send` **不再自动建会话**。
- 并发：Workspace mutation 全局串行（单 writer/互斥）；同一会话单 turn；切换 Workspace 取消活动 turn 并广播 `turn_cancelled`。
- 错误码为封闭集合，M1 落在 `shared/contracts/errors.json`（T4）。
- 默认 Workspace 规则：始终恰好 1 个；`set_default` 切换；默认不可移除，先切 default 再移除；reassign 目标可为默认（含默认作为接收方），但不能是自身/不可用。

## 6. UI / UX（M1）

- 侧栏 = DSH 式：顶部 brand/搜索/新建；全宽“新会话”；“工作区” section 头（搜索/筛选/添加）；下方**平铺 Workspace 行**（默认 `~` 第一行），每个 Workspace 行下缩进列出其会话（一层；状态点/标题/时间；当前会话高亮）；没有独立“会话”section、没有日期分组、Workspace 不嵌套；底部设置。
- 首次启动无 Workspace：全屏/空态"选择工作区"（使用默认 `~` / 打开目录）；成功后进入空会话态（不自动建会话）。
- 会话头部显示 Workspace 名称；设置/诊断显示工作区路径、可读写状态。
- 目录不可用：横幅"工作区目录不可用"+ 重新选择；禁用发送；消息只读可导出（导出 post-M1 则暂只读）。
- 六态（空/加载/错误/离线/流式中/已取消）与 a11y/键盘/i18n/明暗/窄窗遵循 `19`/`27`；workspace 操作必须有 focus-visible、aria-label、Esc 关闭弹窗。
- mock 与真实 serve 载体都必须可完成"建工作区→建会话→发消息→切工作区隔离"；截图/axe 交付见 §7。
- 切 Workspace：取消活动 turn、清空草稿（按 workspace 暂存草稿为 post-M1）。

## 7. 验收（证据外置）

- [ ] `store-session-sqlite` v2 迁移：旧库（有会话）→ 默认 Workspace 回填 → 两会话归属正确；失败注入回滚/只读兜底。证据：`cargo test -p zk-store-session-sqlite`。
- [ ] 无 Workspace/无 conversation 时 `session.create`/`chat.send` 失败（删除 auto-create）。证据：bridge 单测 + Playwright。
- [ ] 默认 Workspace `path == canonicalize(dirs::home_dir())`（`~`），且首用**不自动建会话**；`ZEOKEEL_HOME` 只作测试/嵌入 override。证据：store 测试 + API 断言。
- [ ] Workspace 列表**平铺**（不嵌套/不分组），每行可切换/重命名/移除；会话行平铺（M1 不做日期分组）。证据：Playwright 截图 + DOM 断言。
- [ ] home 不可用：返回 `workspace/path-unavailable`，UI 进入选择目录，不静默回退 data 目录。证据：store 负向测试 + e2e 错误态截图。
- [ ] 两个 Workspace 隔离：切换后列表/消息互不可见；跨工作区 `session.messages`/`chat.send` 返回 mismatch。证据：Rust 负向测试 + Playwright。
- [ ] agent cwd/工具相对路径读取该 Workspace 根下文件；M1 绝对路径/越界默认拒绝（post-M1 `allow_outside`）。证据：agent-loop/tool-fs conformance。
- [ ] Workspace 移除：空 Workspace 移除不删目录；默认 Workspace 需先 `set_default`；有会话时无 `reassign_to` 报错，有则迁移后移除；reassign 目标非法报错。证据：store 测试。
- [ ] 重启后归属不丢；默认 Workspace 不自动建会话。证据：Playwright serve 重启。
- [ ] 目录不可用：发送禁用 + 横幅 + 可恢复。证据：Playwright 截图。
- [ ] 截图/axe：`apps/desktop/web/screenshots/workspace-*`（light/dark/narrow/六态）+ `screenshots/axe.json` 0 serious/critical；真实后端 `13-modular-chat.png` 更新。
- [ ] `python3 scripts/docs-check.py`、`spec-check.py`、`review-check.py`、`zk compose --check` 绿。

## 8. 与 DSH 的差异（事实参考，MIT）

- DSH 移除 project 保留会话为 ungrouped；我们为满足 C18 **不允许 ungrouped**，移除时要求 `reassign_to`。
- DSH `initializeDefault` 仅在空注册表且无 live/persisted/archived session 时创建默认 project；我们 M1 仅"无 Workspace 时创建默认 Workspace"，不自动建会话。
- DSH 有 workspace files listing/watch 与 typed errors（`workspace-file/outside-workspace` 等）；我们 M1 不做文件浏览，post-M1 按 `26` §13 扩展。
- DSH 默认项目路径为 `<Documents>/deepseek-harness`；我们按用户决策默认 **home `~`**，且 Workspace 列表平铺（DSH 也支持 flat 列表，方向一致）。
- DSH session log 是事件日志；我们 M1 仍是 SQLite **消息表**（`conversations/messages`）；`turns`/事件溯源/tool_calls 等表 post-M1（`24` §7）。

## 9. 实现记录与迁移（M1，已落地）

1. `zk-contract-session`：加 Workspace/WorkspaceStore、workspace_id、方法签名；conformance 负向。
2. `store-session-sqlite`：schema v2 迁移（`user_version`、重建表、默认 workspace 回填、备份/回滚）；`list/create/messages` 带 workspace；remove/reassign。
3. `zk-contract-agent`/`zk-contract-tool`/`agent-loop`/`tool-fs`：TurnContext、per-turn root、移除以进程 env 为 root 的 `ZEOKEEL_DEMO_ROOT` fallback。
4. `apps/desktop/src/ipc.rs` + `apps/desktop/web`：workspace 命令、session 命令带 workspace、删除 auto-create、UI switcher/首用/切工作区取消。
5. 迁移旧 demo 数据：默认 Workspace + 归属回填；验收见 §7。
6. 兼容：旧客户端无 workspace 字段的库按 v2 迁移；旧 IPC 调用无 workspace 一律报错（不静默）。
