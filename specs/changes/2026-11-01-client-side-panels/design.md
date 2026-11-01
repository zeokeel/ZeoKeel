# 2026-11-01-client-side-panels · 技术设计

> 规范细节链接 `docs/`，不在这里复制第二份规范。组合/manifest 以 `24` 为准，API 以 `04` 为准。

## 0. 已定决策（用户 2026-11-01 批准推荐项）

| # | 决策 | 落地口径 |
| --- | --- | --- |
| D1 | 终端路径 | 复用 `zk-kit-tools::persistent` 经新 `terminal` IPC（open/send/close/list，≤10s/64 KiB）；不新增 PTY/xterm |
| D2 | 能力面板 | 子 spec `specs/changes/2026-11-01-client-activity-panels/`（≤14 文件）；本 spec 预留 docs/16 activity tab order 40，不实现面板 |
| D3 | 检索/schedule | `session.search` + `zk_kit_session::search` 单一实现；schedule 由 activity-panels 经 `zk-tool-schedule` 公开 `Tool::call_json` 读写，**不新建第二套文件解析/校验** |
| D4 | plan | 不做 plan mode/状态机；activity-panels 的 turnTail 仅展示 assistant Markdown 中可识别的计划标题段落 |
| D5 | archive/fork | archive 默认列表隐藏、可筛选/恢复、不硬删；fork 同 workspace 复制消息（C18） |
| D6 | PDF/预览 | text ≤256 KiB、image ≤8 MiB；PDF/Office/HTML/二进制/超限降级（CSP `frame-src 'none'`，不改 CSP） |

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | 修改 `zeokeel/ui-inspector`（右栏文件树/预览/终端/状态持久化）、`zeokeel/ui-sessions`（会话菜单/归档筛选/内容搜索）、`@zeokeel/ui-kit`（共享 DTO/UiActions）；**不新增包/slot**（保持 `ui.slot.inspector`/`ui.slot.sessions`） |
| 契约 | `zk-contract-session`：`Conversation.pinned/archived`、`FileReadKind`/`WorkspaceFileReadResult`、`SessionSearchHit`/`SessionSearchResult`、`SessionStore` 新默认方法（rename/pin/archive/fork）；`zk-contract-ipc`：新增 `workspace.file_read`、`terminal`、`session.update`、`session.fork`、`session.search` 命令/payload，`WorkspaceFilesPayload` 可选 `path/depth/limit` |
| 路由 | 无 HTTP 路由；本地 IPC 新增 5 命令；`serve`/Tauri invoke 均走同一 dispatch |
| 迁移 | SQLite **v6**：`conversations` 加 `pinned/archived INTEGER NOT NULL DEFAULT 0`（幂等 `has_column`）；无数据回填 |
| 生成物 | `shared/contracts/{schema,ts}`（生成器写）；`apps/desktop/Cargo.toml` 由 `zk compose` 重新生成；本 spec 只改生成器源 `tools/zk/src/main.rs`（生成 `zk-composed` facade 暴露 `zk-kit-tools`），**不手改 @generated 模板/产物** |
| edition/profile/channel | 无 edition 分支；personal/enterprise 同面；不新增运行期加载（C2） |
| 文档 | `docs/16`（右栏 tab order：files 10/changes 20/terminal 30，activity 40 预留；**不新增 settings pane**——settings pane order 70 由 MCP spec 占用，与本 spec 无关）、`docs/19`、`docs/26`、`docs/28`、`KNOWN-LIMITATIONS.md`、`research/...`（缺口状态） |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力（capability id / 路径） | 结论 | 归属 |
| --- | --- | --- | --- |
| 文件列表/懒加载 | `contract.session::FileEntry/WorkspaceFilesResult`（`packages/contracts/session/src/lib.rs:87-124`）；host `workspace.files`（`apps/desktop/src/ipc.rs:914`）；前端 `parseFileEntries`（`protocol.ts:164`） | 扩展（可选 path/depth/limit） | 契约 + host + app |
| 文件读取/预览 | 无现成 IPC；`zk-kit-tools` 有 `safe_path`/`read_text`/路径语义（`crates/zk-kit-tools/src/lib.rs:39-99`），host 经 `zk-composed` 生成 facade 调用（app 不直接依赖 kit） | 扩展（单一 IPC + `workspace.file_read`；复用 kit 路径校验） | `contract.session` + host |
| 终端 | `kit.tools` 持久终端（`crates/zk-kit-tools/src/persistent.rs:293-404`，`tool-terminal` 已包装） | 复用（新 IPC 直调，不写第二套 shell/PTY） | host |
| 会话列表/存储 | `contract.session::SessionStore`（`packages/contracts/session/src/lib.rs:204-240`）、SQLite store（`store-session-sqlite/src/lib.rs:579`） | 扩展（新字段 + 默认方法） | 契约 + store |
| 会话检索 | `session_search` 工具（`packages/client/tool-session-query/src/lib.rs:12-60`；`zk-kit-session::global`） | 提升为 kit 公共函数（`zk_kit_session::search`），工具与 host 共用 | kit.session |
| 会话菜单/Modal/焦点 | `ui-sessions` 现有 workspace `⋯` 菜单与 Modal/焦点管理（`ui-sessions/web/src/index.tsx:125-714`） | 复用模式（不复制 Modal 实现） | ui-sessions |
| 右栏 tabs/⌘B/状态 | `App.tsx:519-580`、`ui-inspector` 现有 tabs/状态 | 扩展（会话级 localStorage + 宽度） | desktop-web |
| 六态/错误边界/a11y | `SlotErrorBoundary`、`useDialogFocus`、现有 inspector 空/loading/error 文案 | 复用 | ui-kit/组件 |
| 命令目录 | Phase 2a 的 `commands` catalog（composer spec） | 不相关（本 spec 不新增命令） | — |

- 搜索证据：`rg -n "workspace.files|inspector|session_search|list_conversations|persistent|terminal" packages apps crates docs`；
  `./target/debug/zk find kit.session` → `crates/zk-kit-session/src/lib.rs`；`zk find kit.tools`；
  capability 索引 `contract.session`（`shared/contracts/capability-index.json:63`）、`contract.ipc`（`:43`）、`kit.session`（`:123`）、`kit.tools`（`:131`）。
- 为什么不能复用/扩展：`ui-inspector` 现有组件是占位骨架，必须扩展为真实树/预览/终端；会话检索不能只做标题过滤，必须复用
  `session_search` 的逻辑并提升到 kit（否则第二份全文匹配实现，C17）；终端复用现有持久会话，不新建 PTY 服务（D1-A）。
- 新增可复用能力：`zk_kit_session::search`（`kit.session` 内扩展，不新增 capability id）；IPC 命令属既有 `contract.ipc`/`contract.session` 扩展。
  不登记新 `@capability`/INDEX 条目（`zk capability --check` 仍绿）。

## 2. 方案

右栏从占位升级为「文件树 + 预览 + 真实终端」三 tab，会话侧增加会话操作菜单与内容检索；host 新增 5 个只读/用户显式操作命令，
复用现有 store/持久终端/工具搜索逻辑；SQLite 仅加两列实现 pin/archive；fork 由 SQLite store 覆盖实现（事务复制消息，同 workspace）。

### 2.1 关键流程

```text
打开右栏 → 读会话级 localStorage{tab,width,collapsed} → files tab 初始 workspace.files{path:"",depth:1,limit:200}
展开目录 → workspace.files{path, depth:1, limit:200} → 子项并入树（失败仅该行 error+retry）
点文件 → workspace.file_read{path,max_bytes} → text(≤256KiB,truncated)/image(≤8MiB,base64)/unsupported → 预览/降级
terminal tab → terminal{action:list|open|send|close} → 输出追加（≤64KiB/次）+ 超时/平台错误
会话菜单 → session.update{title|pinned|archived} / session.fork / copy id(剪贴板)
内容搜索 → session.search{workspace,query,limit} → 命中列表 → 点击 selectConversation
切会话/刷新 → 恢复该会话 tab/width/collapsed（localStorage，解析失败回默认）
```

### 2.2 数据结构（新增/修改）

```text
# zk-contract-session
Conversation { id, workspace_id, title, created_at_ms, updated_at_ms,
               #[serde(default)] pinned: bool, #[serde(default)] archived: bool }

FileReadKind = text | image | unsupported
WorkspaceFileReadResult {
  path: String,            # workspace 相对路径（正斜杠）
  kind: FileReadKind,
  mime: String,            # text/*、image/*；unsupported 时可为空/通用
  size: u64,
  truncated: bool,         # 文本截断/输出截断
  encoding: "utf8"|"base64"|"", content: String,
  reason: String,          # unsupported 的用户可读原因（空）
}

SessionSearchHit { conversation_id, title, message_id, role, snippet, archived }
SessionSearchResult { items: Vec<SessionSearchHit>, truncated: bool }

trait SessionStore {
  fn rename_conversation(&self, workspace_id, conversation_id, title) -> Result<Conversation, SessionError> {
     Err(SessionError::Invalid("rename_conversation unsupported".to_owned()))   # 默认实现保持旧实现可编译
  }
  fn set_pinned(&self, workspace_id, conversation_id, pinned: bool) -> Result<Conversation, SessionError> { ...default Invalid("set_pinned unsupported")... }
  fn set_archived(&self, workspace_id, conversation_id, archived: bool) -> Result<Conversation, SessionError> { ...default Invalid("set_archived unsupported")... }
  fn fork_conversation(&self, workspace_id, conversation_id, title: Option<&str>) -> Result<Conversation, SessionError> {
     /// 默认实现：返回 `SessionError::Invalid("fork_conversation unsupported")`（保持第三方/测试实现可编译）；
     /// SQLite store 覆盖实现：事务内 create + messages_in + 逐条 append（新 message id；保留原 created_at_ms 排序）
  }
}

# zk-contract-ipc（commands/payloads）
workspace.file_read  WorkspaceFileReadPayload { workspace_id, path, #[serde(default)] max_bytes: Option<u64> }
terminal             TerminalPayload { workspace_id, conversation_id, action: open|send|close|list|status,
                                       #[serde(default)] session: Option<String>,
                                       #[serde(default)] command: Option<String>,
                                       #[serde(default)] timeout_ms: Option<u64> }
session.update       SessionUpdatePayload { workspace_id, conversation_id,
                                            #[serde(default)] title: Option<String>,
                                            #[serde(default)] pinned: Option<bool>,
                                            #[serde(default)] archived: Option<bool> }
session.fork         SessionForkPayload { workspace_id, conversation_id, #[serde(default)] title: Option<String> }
session.search       SessionSearchPayload { workspace_id, query, #[serde(default)] limit: Option<u32> }
workspace.files      WorkspaceFilesPayload 扩展：#[serde(default)] path: Option<String>,
                     #[serde(default)] depth: Option<u32>, #[serde(default)] limit: Option<u32>
# 结果：session.update/fork 返回 Conversation JSON（session schema 校验）；file_read/search 返回 session 契约 DTO；
# terminal 结果在 ipc::results::TerminalResult { sessions: Vec<String>, output: String, truncated: bool, exit_code: Option<i32> }
```

限制/规则：
- `workspace.files`：`path` 默认 `""`（workspace 根）；`depth` 1..=2（默认 2）；`limit` 1..=500（默认 300）；
  `path` 必须是 workspace 相对目录（canonicalize + 前缀校验），**无参调用=旧行为**（depth 2/300）。
- `workspace.file_read`：`max_bytes` 默认/上限 256 KiB（文本）或 8 MiB（图片/其它判定的原始字节上限）；文本 UTF-8 解码失败→unsupported；
  图片 MIME 仅 png/jpeg/webp/gif；PDF/Office/HTML 等→unsupported（D6-A）；路径 canonicalize 后必须在 workspace 根内且拒绝符号链接逃逸。
- `terminal`：action 必填；`session` 默认 `"default"`，id 规则沿用持久终端（ASCII 字母数字`-_.`、≤64、≤8/会话）；
  `command` 仅 send 必填（非空、≤16 KiB）；`timeout_ms` 1_000..=10_000（默认 10_000）；输出上限 64 KiB（超出 truncated）；
  open/close/list 幂等性：open 已存在=成功返回；close 不存在=`terminal/session-not-open`。平台 shell：unix bash、Windows pwsh（由 host cfg 选择，沿用现有 shell kind）。
- `session.update`：至少一个字段；title trim 非空 ≤200；pin/archive 幂等；归档中的会话仍可被显式打开（读取），但 `chat.send` 对归档会话由 app 阻止（提示 unarchive；本 spec 不改 host chat.send，UI 层阻止）——**design 决定**：在 app 发送前置检查 active conversation.archived，阻止并提示；host 不加规则（避免跨 spec 改 agent 链）。
- `session.fork`：源会话必须属于 workspace；新会话 title 默认 `"{源标题} · fork"`，≤200；复制全部消息（新 message id、原 created_at_ms 排序、原 status/usage/turn_id 保留）；
  不复制 pinned/archived；返回新 Conversation。
- `session.search`：query trim 后 1..=200；limit 1..=50（默认 20）；复用 `zk_kit_session::search`（大小写不敏感 contains，遍历当前 workspace 会话与消息，最多 50 hits）；
  包含 archived 会话并带标记；`truncated` 表示达到 limit。
- 错误码：以上命令的错误为 `CommandResponse.error` 字符串（封闭约定），新增 `workspace/path-escape`、`file/too-large`、`file/unsupported`、
  `terminal/session-invalid`、`terminal/session-not-open`、`terminal/timeout`、`terminal/truncated`、`terminal/unavailable`、
  `session/invalid`（沿用既有）、`session/not-found`/`session/workspace-mismatch`（既有）；在 `docs/26` §5 登记。

### 2.3 右栏实现

- `ui-inspector` props 新增（由 app 注入，不 import app）：
  `width: number; collapsed: boolean; onWidth: (w)=>void; onCollapse: ()=>void;
   listDirectory(path: string) => Promise<FileEntry[]>; readFile(path) => Promise<WorkspaceFileReadResult>;
   terminal: { list/open/send/close; sessions: string[]; output: string; state };
   previewRequest?: { path: string; nonce: number } | null; onPreviewConsumed?: () => void`
  （`previewRequest` 供 activity-panels 的 turnTail「在右栏查看」调用：tab 切 files + 打开预览；本 spec 只提供入口，不实现该调用方）。
  为控制 props 数量，`listDirectory/readFile/terminal` 以 `actions` 对象注入（ui-kit 类型）。
- 符号链接：列表阶段 **不展示 symlink**（host 返回条目时 UI 用 kind/预览拒绝的双保险；host 侧 `read_dir` 后不 follow symlink）；`workspace.file_read` 对目标/父目录 symlink 一律拒绝（canonicalize + starts_with）。
  刷新清空缓存但保留 expanded；`aria-expanded`/`role="treeitem"`/`level`；目录/文件按 kind 排序（目录优先，名称 localeCompare）。
- 预览：`previewPath: string|null` + `previewState`（loading/ok/error/unsupported）；文本 `<pre>`（保留换行、截断提示）；
  图片 `<img src={"data:"+mime+";base64,"+content}>`（CSP `img-src 'self' data:` 已允许）；不支持显示降级块 + 返回树。
- 终端：会话列表（tabs/select）、输出 `<pre>`（自动滚底、最多保留 N 行/64 KiB）、输入行（Enter 发送、↑ 历史可选不做）；
  执行中禁用输入/显示 spinner；关闭会话确认（简单确认，不新增 Modal 组件：复用 `ui-kit` 现有 primitive 或浏览器 confirm？**design 决定**：
  复用 ui-kit 的按钮 + 内联确认（无全局 confirm）避免新组件）；
- 状态持久化：app `useEffect` 读写 `localStorage["zk.inspector.v1."+conversationId] = {tab,width,collapsed}`；
  解析失败/无记录→默认；宽度 drag handle（pointer events）+ keyboard ArrowLeft/Right（每次 16px，Shift 64px）；
  视口 <768px 时 app 强制 collapsed=true（保留存储值，变宽恢复）。
- tab order：`files(10) → changes(20) → terminal(30)`；docs/16 预留 `activity(40)` 给子 spec；无 tab registry/slot（组件内固定顺序，C2）。
  **不新增 settings pane**：MCP 的 `ui.settings.pane` order 70 由 `2026-11-01-client-mcp-config` 占用，本 spec 不改设置面。

### 2.3.1 生成器 facade（C4/C11，P2-11）

- **生成物归属澄清（P2-11）**：`tools/zk/templates/app_desktop.Cargo.toml` 是**生成器输入源**（其首行 `@generated` 为历史遗留注释，不代表它是本 spec 的生成物）；本 spec **不改该模板**，只改生成器源 `tools/zk/src/main.rs` 并让 `zk compose` 生成 `zk-composed`/`apps/desktop/Cargo.toml`；二者仍是 @generated，禁止手改，`--check` 必须绿。
- `apps/desktop` 不直接依赖 `zk-kit-tools`/tool 包；`tools/zk/src/main.rs` 生成 `zk-composed` 的 kit 依赖与 `pub use`（`zk_kit_tools`），
  host 通过 `zk_composed::zk_kit_tools::{safe_path,read_text,persistent}` 调用；只改生成器源，不手改 `@generated` 模板/产物；
  **`tools/zk/templates/app_desktop.Cargo.toml` 保持不动（不是本 spec 写作用域）**；`zk compose --check` 绿。

### 2.4 会话操作/检索实现

- `ui-sessions` 会话行 `⋯` 菜单（复用现有 workspace 菜单交互）：rename（内联输入/Modal）、fork、copy id（`navigator.clipboard.writeText`，
  成功后 toast/aria-live）、pin/unpin、archive/unarchive（归档筛选开关「显示已归档」）；菜单项 disabled（流式中 fork/archive 拒绝）。
- `UiActions` 新增：`updateConversation({workspaceId, conversationId, title?, pinned?, archived?})`、`forkConversation(...)`、
  `searchSessions(workspaceId, query, limit?)`；App 实现 transport 调用 + `protocol` 解析 + 刷新会话列表/选中新会话。
- 内容搜索 UI：侧栏搜索框旁模式切换（标题/内容，segmented button）；内容模式 debounce 250ms（AbortController/序列号丢弃旧结果），
  命中列表显示会话标题 + role + snippet（`mark` 不高亮）；点击 `selectConversation` 并关闭搜索；archived 命中标注。
- host `session.search`：调用 `zk_kit_session::search(global store, workspace, query, limit)`，映射为 `SessionSearchResult`。

### 2.5 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| A. 单栏右栏 + 3 tab + 会话级宽度持久化；终端复用持久会话；检索复用工具逻辑；SQLite 加列 | 复用最大、文件预算内、无新依赖 | 无多 pane/float；终端同步阻塞；PDF 不预览 | **采用** |
| B. 引入 xterm.js + PTY 服务 + 右栏多 pane/float | 与 DSH 形态接近 | 新依赖/跨平台 PTY/生命周期/预算超 28 | 放弃（Phase 3 再评估） |
| C. 会话检索前端遍历已加载消息 | 无 IPC | 只能搜当前会话/已加载窗口，不等于全文 | 放弃（与 `session_search` 能力不一致） |
| D. archive 硬删除/单独归档表 | 数据清晰 | 数据丢失风险，C18 无回退 | 放弃（archive=隐藏） |
| E. PDF 内嵌（放宽 CSP frame-src/object-src） | 预览完整 | 安全面扩大/平台差异 | 放弃（D6-A 降级） |

### 2.6 数据与迁移

```sql
-- schema v6（幂等：has_column 检查后 ALTER）
ALTER TABLE conversations ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE conversations ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
-- 列表：SELECT ... ORDER BY pinned DESC, updated_at_ms DESC, id DESC
```

- `row_to_conversation` 读取 pinned/archived；`append_message`/`messages_in` 不变（v5 的 turn_id 保持）。
- fork 由 SQLite store **覆盖实现**（事务内复制消息；contract 默认实现返回 Invalid，保持第三方/测试实现兼容）；消息 id 用 `uuid`（store 已有依赖）。
- 检索函数：`zk_kit_session::search(store, workspace_id, query, limit) -> Result<Vec<SessionSearchHit>>`（复用 contract DTO；
  遍历 `list_conversations_in` + `messages_in`，contains 匹配，截断 snippet ±60 字符，含 archived）。

### 2.7 安全与隐私

- 文件预览：只读、workspace 相对路径；canonicalize 后必须以 workspace canonical root 为前缀；拒绝符号链接/absolute/`..`；
  文本/图片大小上限；不返回可执行内容 MIME；不写盘。父目录 symlink 同样拒绝（不能只检查文件本身）。
- 检索隐私：只查当前 workspace；snippet 由 host 截取（±60 字符），工具 `session_search` 仍返回**完整 content**（兼容，P2-6）；
  UI 不把结果写日志/上报；与打开会话同权限，不额外脱敏（正文本身在 UI 可见），但拒绝跨 workspace 返回。
- 终端：用户显式命令，原样交给持久 shell；IPC 由 token/CORS 保护（既有 serve 安全）；host 不拼接/不解释命令；
  不做审批（非 agent 工具）；记录 KNOWN「右栏终端无审批/审计」；命令输入 ≤16 KiB、输出 ≤64 KiB、超时 ≤10s。
- 会话操作：只允许当前 workspace 内；C18 workspace_id 不可变；fork 复制消息不复制归档/置顶；检索只读当前 workspace。
- 审计：无新增服务端审计（本地个人/员工操作）；错误记录本地日志（不含文件内容/命令输出全文）。

### 2.8 发布与回滚

- 无灰度/强制更新；IPC additive、SQLite v6 仅加列、旧客户端忽略新字段。
- 回滚：回退 commit；v6 加列对旧二进制可读（固定列 SELECT 不选新列）；已 pin/archive 数据在回滚后被忽略（列表恢复原行为）；
  已 fork 的新会话仍存在（数据无破坏）。

### 2.9 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 组合 | compose/schema/capability | `zk compose --edition personal --profile default --channel official --check`；`bash scripts/schema-check.sh`；`zk capability --check` |
| T1 契约 | Conversation 默认/旧 JSON；trait 默认方法；DTO schema | `cargo test -p zk-contract-session` |
| T1 store | v5→v6 幂等/旧行 false/排序/rename/pin/archive/fork | `cargo test -p zk-store-session-sqlite` |
| T1 kit/tool | search 与 session_search 一致；边界 | `cargo test -p zk-kit-session -p zk-tool-session-query` |
| T1 host | workspace.files 扩展兼容 + file_read + terminal + session.* + search | `cargo test -p zk-desktop` |
| T2 UI | inspector 树/预览/终端/持久化；sessions 菜单/search（app 单测 + e2e；UI 包仅 contract 测试，不新增 UI 包测试文件） | `pnpm --dir apps/desktop/web test:unit` |
| T3 e2e | m1-serve 追加场景（树/预览/终端/会话操作/搜索/持久化/窄窗） | `pnpm e2e` |
| T4 门禁 | typecheck/lint/i18n/workspace | `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit && pnpm -r build`；`bash scripts/i18n-check.sh` |

## 3. 备选与取舍（细节）

- 文件树数据：每次展开调用 `workspace.files{path,depth:1}`（N 次 IPC）vs 一次 depth=2 缓存（移动/新增文件后 stale）；
  采用前者（懒加载符合需求，host 有界）。
- 预览图片：base64 经 IPC（CSP 允许 data: img）vs 新增自定义协议；采用 base64（简单、无协议注册）。
- 终端输出：轮询 vs 同步返回；采用同步（每次 send 返回本次输出，UI 追加），避免轮询线程。
- 会话搜索：放 app 侧遍历 store（重复实现）vs kit 公共函数；采用 kit 公共函数（C17）。

## 4. 文档更新

- [ ] `docs/16` §2.2/§2.4：右栏 tab order（files 10/changes 20/terminal 30，activity 40 预留给子 spec）；无新 slot。
- [ ] `docs/19` §2：`ui.inspector`/会话操作状态口径（六态、归档筛选、检索模式）。
- [ ] `docs/26`：schema v6（pinned/archived）、会话操作规则（C18：workspace 不变；archive 隐藏不删；fork 同 workspace；检索同 workspace）、§5 错误码新增项、M1「post-M1」表述更新。
- [ ] `docs/28` §4：Inspector 与会话操作实现状态（Phase 2b），`docs/28` §4.7 后置项更新。
- [ ] `KNOWN-LIMITATIONS.md`：终端无审批/审计、同步阻塞、PDF 降级、无多 pane/float、检索子串无索引、fork 复制成本。
- [ ] `research/client-dsh-parity-2026-11-01.md`：P1-6/P1-10/B4/B8/B20/A? 状态回写（右栏/会话操作/检索已实现子集）。
- [ ] ADR/D：无新增；C18 语义不变（archive 非删除、fork 同 workspace）。

## 5. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| `workspace.files` 扩展破坏旧调用 | 中 | 中 | 可选字段缺省=旧行为；兼容单测；app 旧代码不改 | host 单测 + e2e |
| 终端 IPC 同步阻塞/超时 | 中 | 中 | timeout ≤10s；UI 运行态；e2e 短命令；KNOWN 记录 | host 单测 + e2e |
| 文件预览逃逸/大文件 | 中 | 高 | canonicalize+前缀、符号链接拒绝、大小上限、降级 | host 安全单测 |
| v6 迁移损坏旧库 | 低 | 高 | 仅加列+幂等；迁移前备份（沿用既有备份）；v5 库测试 | store 单测 |
| fork 大量消息阻塞 IPC | 低 | 中 | 同 workspace 复制；消息数上限/超时提示；KNOWN | host 单测 |
| 搜索遍历大 workspace 慢 | 中 | 中 | limit≤50、query 长度限制、只读；MVP 可接受，索引留后续 | host 单测 + e2e |
| 归档会话发送语义 | 低 | 中 | UI 阻止发送并提示 unarchive；host 不新增规则 | app 单测 |
| 能力面板拆出后验收口径混淆 | 中 | 中 | proposal §8/§2 明确非目标；子 spec 列出；docs 不宣称已交付 | 评审 |
