# 2026-11-01-client-side-panels · 提案

- 状态：`done`
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：用户已批准的 Phase 2「运行时能力可见面 + composer 体验」（A 方案全量分期中的 Phase 2b）；`research/client-dsh-parity-2026-11-01.md` §4/P1-6/P1-10、§3（右栏/文件/终端/会话）；`target/audits/zk-client-inventory.md` §3（B4/B8/B20）、§4（A3/A4）、§5（C8）；`target/audits/dsh-reference-inventory.md` §4.5（右栏 tab 事实）、§4.7（会话操作）；`docs/16` §2.2（slot）、`docs/26`（C18 workspace 规则）、`docs/28` §4（Inspector/会话操作）；前置：Phase 1 全部（settings-ia/approval/config/mcp）
- 评审：docs/reviews/2026-11-01-client-phase2-specs.md
- 变更预算：**28 个文件**（契约 3、store/kit 5、desktop 2（含生成器 facade 源）、ui-kit 2、ui-inspector 2、ui-sessions 1、desktop-web 4、app 测试 2、e2e 1、文档 6；不含本 spec 目录与生成物）。**能力面板（jobs/subagent/goal/todo/schedule/plan/present）超出预算，按 §8 拆出子 spec**；本 spec 只做右栏 + 会话操作 + 检索入口。

## 1. 背景 / 问题

Phase 1 补完了设置/审批/工具行，但「运行时能力可见面」仍基本为空，右栏与会话侧是审计中 P1/P2 缺口最集中的地方：

- **右栏是只读占位**：`ui-inspector` 固定 288px 单栏（`packages/ui/ui-inspector/web/src/index.tsx:31`），
  文件 tab 是平铺路径列表（`:65-69`，无树/无懒加载/无预览/无刷新），终端 tab 只是一行「终端骨架（只读占位）」文案（`:77`，
  i18n `inspectorTerminalSkeleton`）；`workspace.files` 已存在但只按 depth≤2/≤300 平铺（`apps/desktop/src/ipc.rs:914`）。
  DSH 对照：右栏 per-session 停靠面 + 8 类 tab（files/text/terminal/changes-review/plan/schedule/subagentchat 等，
  `target/audits/dsh-reference-inventory.md` §4.5），其中 files 懒加载目录树、点文件进预览、terminal 是可交互 shell。
- **没有文件预览**：点击文件无任何动作；文档/图片/PDF 无最小预览与诚实降级；无宽度/折叠/宽度持久化（固定 288px），
  窄窗也不自动收起（审计 B20）。
- **会话操作缺失**：会话只有新建/选择；无 rename/fork/copy id/pin/archive（`target/audits/zk-client-inventory.md` §3 B8、
  `research/...` §4 P1-10），与 C18 的 workspace 约束相比至少缺「同 workspace 内 fork / 归档隐藏」。
- **没有会话全文检索入口**：侧栏搜索只匹配标题/路径（`packages/ui/ui-sessions/web/src/index.tsx:168-186`），
  已有 `session_search` 工具能力（`packages/client/tool-session-query/src/lib.rs:12`）但 UI 不可达（审计 A?/B8）。
- **不做的后果**：用户仍然只能通过聊天里的工具行间接感知文件/终端/历史，无法在右栏查看/预览/操作；
  Phase 2 的「能力可见面」在文件与历史两个最高频场景缺席。

## 2. 目标

- **G1 右栏文件树（懒加载）**：files tab 改为目录树（展开/折叠、按需加载子目录、刷新保持展开、错误/空/加载态），
  数据来自扩展后的 `workspace.files`（可选 `path/depth/limit`，不新增搜索 IPC）；隐藏 `.git`/`.zeokeel`。
- **G2 文件预览（最小子集）**：点文件在 files tab 内打开预览（返回树）：文本（UTF-8/截断标记）、图片（data URL），
  其它类型（含 PDF/超大/二进制）**诚实降级**（说明不支持 + 文件大小/路径），不白屏；路径逃逸/符号链接拒绝。
- **G3 真实终端**：terminal tab 接现有持久终端能力（`zk-kit-tools::persistent` 的 open/send/close/list，经新 IPC），
  支持新建/切换/关闭会话、命令输入、输出追加/清空、超时与错误；平台不支持时明确报错；命令**由用户触发**、不作为 agent 工具调用。
- **G4 右栏状态持久化**：当前 tab/宽度/折叠按**会话级**持久化（`localStorage`，app 管理），切会话恢复；
  宽度可拖拽 + 键盘调整，窄窗自动收起，最小/最大宽度与中栏可读性约束。
- **G5 会话操作**：会话行菜单 rename / fork / copy id / pin / archive（C18：workspace_id 不可变；fork 仅同 workspace；
  archive 仅从默认列表隐藏、不硬删，可筛选与恢复；pin 置顶）；全部走新 IPC + SQLite v6 增量字段（`serde(default)` 兼容）。
- **G6 会话全文检索入口**：侧栏搜索增加「内容」模式，调用 `session.search` IPC（复用 `session_search` 工具逻辑，
  抽到 `zk-kit-session` 单一实现），返回命中会话/角色/片段，点击打开会话；只做子串匹配、有界（≤50），不做索引/向量。
- **G7 诚实边界与质量**：六态（空/加载/错误/离线/流式中/已取消）、axe 0 serious、i18n 全量键、无新 slot/包/运行期加载（C2）、
  无第二份命令/终端/搜索实现（C17）。

## 3. 非目标

- **能力面板全部拆出**：jobs/subagent/goal/todo/schedule/plan/present/turnTail（见 §8 子 spec `2026-11-01-client-activity-panels`）；
  本 spec 不建面板、不新增 activity IPC、不在右栏加第 4 个 tab（仅在 docs/16 预留 order 40）。
- 不做多 pane/float/fullscreen/拖拽排序/浏览器 tab/Office/HTML/PDF 内嵌预览（PDF 本轮降级；CSP `frame-src 'none'` 也不支持内嵌）。
- 不做会话硬删除/跨 workspace fork/归档保留期策略/会话导出。
- 不做检索索引、向量检索、正则/模糊/高亮、跨 workspace 检索；不做消息内滚动定位（点击仅打开会话）。
- 不新增 `ui.inspector.tab` slot 或 UI 包；不运行时加载 tab 贡献（C2）；不新增终端 PTY 服务（复用既有持久会话）。
- 不改 composer/chat 输入面（Phase 2a 由其 spec 负责）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 浏览大型项目目录 | 右栏树按需展开、刷新保持展开、不卡顿；点文件看文本/图片 |
| 个人用户 | 在项目里跑一条命令确认状态 | 右栏 terminal tab 打开会话、执行、看输出；可切换/关闭 |
| 个人用户 | 文件是 PDF/二进制/过大 | 明确说明不支持在线预览与原因，不白屏、不卡死 |
| 个人用户 | 回到之前会话、改标题、做分支、置顶/归档 | 会话行菜单完成；归档后默认列表隐藏，可筛选恢复 |
| 个人用户 | 只记得某句话想找回会话 | 侧栏搜索切「内容」，列出命中片段，点击打开会话 |
| 企业用户 | 同一界面使用（无本地 provider 差异） | 右栏/会话操作一致；不改网关/权限语义 |

## 5. 成功指标

- 文件树：展开 10 层目录不一次拉全量；每目录点开一次 IPC；刷新不丢展开；`workspace.files` 旧调用（无参）行为不变（兼容单测）。
- 预览：文本 ≤256 KiB 截断可见；图片（≤8 MiB）渲染；PDF/二进制/超限显示降级说明；路径逃逸（`../`、绝对路径、符号链接）拒绝率 100%（host 单测）。
- 终端：打开→执行→输出→关闭全链路 e2e 通过；错误有可读文案；无命令注入（输入原样传给持久 shell，不拼接 host 命令）。
- 持久化：切会话/刷新后 tab/宽度/折叠恢复；窄窗（<768）自动收起；键盘可调宽度（axe/键盘测试）。
- 会话操作：rename/fork/copy pin/archive 全部 e2e；fork 复制消息且 workspace 不变；archive 默认隐藏/可筛选；SQLite v5→v6 迁移幂等（存储单测）。
- 检索：查询命中 ≤50 条、片段稳定、含标题/角色；结果点击打开正确会话；空查询/超长查询有界。
- 质量门禁：`cargo test -p zk-contract-session -p zk-store-session-sqlite -p zk-kit-session -p zk-tool-session-query -p zk-desktop`、`pnpm -r typecheck/lint/test:unit/build`、`pnpm e2e`（m1-serve）、i18n、schema/compose/capability 全绿；无新增 `@generated` 手改。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 终端 IPC 阻塞（同步命令最多 10s） | 中 | host 用 `spawn_blocking`/专用线程？本轮为简单同步 + timeout 上限 10s，UI 显示运行中；e2e 用短命令；记录 KNOWN |
| 终端无审批/无审计（用户主动执行） | 中 | 本地 IPC token 鉴权/CORS；明确「用户命令非 agent 工具」；记录 KNOWN 与 `docs/26` 弱管控口径 |
| 文件预览读大文件/图片内存 | 中 | 文本 256 KiB、图片 8 MiB 上限；超限降级；canonicalize + 前缀校验拒绝逃逸 |
| PDF 预览受 CSP `frame-src 'none'`/object 限制 | 中 | 本轮降级不内嵌（§7-D6）；不改 CSP |
| SQLite v6 迁移与旧库兼容 | 中 | 仅加列（幂等 `has_column`），`serde(default)`；单测 v5→v6 与旧行读回 |
| fork 复制大量消息 | 低 | 同 workspace 内复制；记录消息数；不做流式/断点 |
| `workspace.files` 语义扩展影响既有调用 | 中 | 可选字段缺省 = 现有行为；兼容单测；前端 app 旧调用不改 |
| 右栏宽度/折叠持久化键冲突/脏数据 | 低 | 版本化 key `zk.inspector.v1.<conversationId>`，解析失败回默认；单测 |
| 能力面板拆出后用户预期落差 | 中 | proposal §8 明确子 spec 与顺序；docs/16 预留 order 40；不宣称已交付 |

## 7. 已拍板决策（原待拍板项；用户 2026-11-01 批准推荐项）

- [x] **D1 终端实现路径**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：复用 `zk-kit-tools::persistent`（现有 bash 持久会话，进程内、每会话多会话）经新 IPC
      `terminal`（open/send/close/list，超时上限 10s）；备选 B：新增 PTY 服务/前端 xterm（新依赖 + Windows ConPTY + 会话生命周期，超预算）。
- [x] **D2 能力面板归属**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：拆出 `specs/changes/2026-11-01-client-activity-panels/`（五件套 draft：jobs/subagent/agent-team/goal/todo/schedule/plan 展示/present，
      右栏第 4 个 tab order 40 + chat turnTail）；备选 B：塞进本 spec（约 +14 文件，超 28 预算）。
- [x] **D3 schedule/检索读写路径**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：检索走 `session.search` IPC（复用 `session_search` 工具逻辑抽到 kit-session，单一实现）；
      schedule 管理留给子 spec，经 `zk-tool-schedule` 的公开 `Tool::call_json`（list/update/delete）读写 `.zeokeel/schedules.json`（不新建第二套 JSON 读写/校验）；
      备选 B：本 spec 直接按文件形状读 schedule（重复解析/校验，不推荐）。
- [x] **D4 plan 是否真做 plan mode**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：不做 plan mode；子 spec 的「计划卡」仅展示模型输出的 Markdown/文本，
      不实现 plan 审批流、不新增 plan 工具/状态机；备选 B：实现 plan mode（agent/契约/UI 新状态，另立 spec）。
- [x] **D5 归档/fork 语义**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：archive 仅从默认列表隐藏（可筛选、可恢复、不硬删）；fork 在同 workspace 内复制消息到新会话
      （C18：workspace_id 不变）；备选 B：fork 仅建空会话引用原会话（不可继续对话，不推荐）。
- [x] **D6 PDF/预览范围**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：文本 + 图片在线预览；PDF/Office/HTML/二进制降级说明（不改 CSP）；备选 B：放宽 CSP/新增
      Windows PDF 渲染（安全与平台成本，超 Phase 2b）。

> 已拍板（D1–D6 均采用推荐项）；状态仍为 `draft`，待独立评审通过后转 `approved`（C16/C19）。

## 8. 变更预算与拆分（含子 spec）

- 本 spec 文件清单（28，实现+文档，不含本 spec 目录/生成物）：
  契约 3（`packages/contracts/ipc/src/lib.rs`、`packages/contracts/session/src/lib.rs`、`packages/contracts/session/tests/conformance.rs`）；
  store/kit 5（`packages/client/store-session-sqlite/src/lib.rs`、`.../tests/conformance.rs`、`crates/zk-kit-session/src/lib.rs`、
  `crates/zk-kit-session/tests/search.rs`（新）、`packages/client/tool-session-query/src/lib.rs`）；
  desktop 2（`apps/desktop/src/ipc.rs`、`tools/zk/src/main.rs` 生成器 facade；**不手改 `@generated` 模板/产物**，`apps/desktop/Cargo.toml` 由 compose 重新生成）；
  UI 5（`packages/ui/ui-kit/web/src/types.ts`、`.../context.tsx`、`packages/ui/ui-inspector/web/src/index.tsx`、
  `.../panels.tsx`（新）、`packages/ui/ui-sessions/web/src/index.tsx`）；
  desktop-web 4（`apps/desktop/web/src/App.tsx`、`.../state/controller.ts`、`.../protocol.ts`、`.../i18n.tsx`）；
  app 测试 2（`apps/desktop/web/tests/inspector-changes.test.tsx`、`.../workspace.test.tsx` 追加）；
  e2e 1（`e2e/m1-serve.spec.ts` 追加）；文档 6（`docs/16`、`docs/19`、`docs/26`、`docs/28`、`KNOWN-LIMITATIONS.md`、
  `research/client-dsh-parity-2026-11-01.md`）。**合计 28**。
- **拆分记录（P1-3 口径，Lead 2026-11-01 允许）**：原 Phase 2b 合计约 42 文件，超 28 阈值，按子域拆为：
  - 本 spec `2026-11-01-client-side-panels`：**右栏（文件树/预览/终端/状态持久化）+ 会话操作（rename/fork/copy/pin/archive）+ 检索入口**（28）。
  - 子 spec **`specs/changes/2026-11-01-client-activity-panels/`**（五件套 draft 已写，与本 spec 同批评审）：jobs/subagent/agent-team/goal/todo/schedule/plan 展示/present
    + 右栏第 4 tab（order 40）+ chat turnTail；预算 ≤14 文件。顺序在本 spec 之后（共享 `ipc.rs`/`App.tsx`/`i18n.tsx`/`ui-inspector`/`ui-chat`/docs/16/28）；本 spec 的 docs/16 先预留 activity tab order 40。
- 不拆的子理由：右栏文件/预览/终端与「会话操作/检索」共享 `session`/`ipc`/`App` 状态与 e2e，拆开会造成共享文件竞写与
  双份会话搜索实现；能力面板使用另一组 IPC/数据源（jobs/subagent/goal/schedule），可独立且可后置。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "workspace.files|inspector|session_search|list_conversations|Conversation|terminal|persistent" packages apps crates docs`；
  `./target/debug/zk find kit.session` → `crates/zk-kit-session/src/lib.rs`；`zk find kit.tools` → `crates/zk-kit-tools/src/lib.rs`；
  capability 索引 `contract.session`（`shared/contracts/capability-index.json:63`）、`contract.ipc`（`:43`）、`kit.session`（`:123`）、`kit.tools`（`:131`）。
- 可复用：
  - 文件列表：`workspace.files`（`FileEntry`/`WorkspaceFilesResult`，`packages/contracts/session/src/lib.rs:87-124`；host `ipc.rs:914`；
    前端 `parseFileEntries` `protocol.ts:164`）→ 扩可选 `path/depth/limit`，不新增搜索 IPC。
  - 终端：`zk-kit-tools::persistent`（`crates/zk-kit-tools/src/persistent.rs:293-404` 的 open/run/close/list；`tool-terminal` 已包装）→ 新 IPC 直接调用，不写第二套 shell。
  - 会话存储/列表：`SessionStore`（`packages/contracts/session/src/lib.rs:222-240`）、SQLite（`packages/client/store-session-sqlite/src/lib.rs:579`）；
    fork 默认实现复用 `messages_in`+`append_message`。
  - 检索：`session_search` 工具逻辑（`packages/client/tool-session-query/src/lib.rs:28-60`）→ 抽到 `zk-kit-session` 公共函数，
    工具与 host IPC 共用（不复制循环）。
  - 会话菜单交互：`ui-sessions` 现有 workspace `⋯` 菜单/Modal/焦点管理（`packages/ui/ui-sessions/web/src/index.tsx:125-714`）→ 复用模式。
  - 右栏状态/快捷键：`App.tsx:519-580`（inspectorOpen/tab/⌘B）、`ui-inspector` 现有 tabs/错误态 → 扩展，不新增 slot。
  - 六态/a11y：`SlotErrorBoundary`、`useDialogFocus`、现有 inspector loading/error/empty 文案模式。
- 本次新增可复用能力：**无新 kit capability**；新增 1 个 kit 公共函数 `zk_kit_session::search`（已有 `kit.session` capability 内扩展，不新增 INDEX 条目），
  新增 IPC 命令属 `contract.ipc`/`contract.session` 既有契约扩展（不新增 capability id）。
- 为什么不能复用/扩展：右栏树/预览/终端在 `ui-inspector` 自有组件内扩展（不新增 slot，C2）；job/subagent 等能力面板不在本 spec（拆分）；
  会话检索不能只复用侧栏标题过滤（无全文能力），必须复用 `session_search` 的逻辑并提升为 kit 公共函数（C17）。
